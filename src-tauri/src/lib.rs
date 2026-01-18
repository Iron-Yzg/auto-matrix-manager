// Core modules
// 核心模块
pub mod core;
pub mod storage;
pub mod browser;
pub mod platforms;
pub mod commands;

// Re-export browser types for easy access
pub use browser::{BrowserAutomator, BrowserAuthResult, BrowserAuthStep};

// Re-export commands for easy access
// 重新导出命令以便轻松访问
pub use commands::*;

// Also explicitly export AppState for state management
pub use commands::AppState;

use std::sync::Arc;
use tokio::sync::Mutex;

// Run the Tauri application
// 运行 Tauri 应用
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use crate::storage::DatabaseManager;
    use crate::commands::AppState;
    use crate::browser::{BrowserAutomator, check_playwright_env, ensure_playwright_env};
    use tauri::Manager;

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // 使用 Tauri 的应用数据目录
            let data_path = app.path()
                .app_data_dir()
                .unwrap_or_else(|_| {
                    // 如果获取失败，使用默认路径
                    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                    std::path::PathBuf::from(home)
                        .join("Library")
                        .join("Application Support")
                        .join("com.tauri.auto-matrix-manager")
                });

            // Create data directory if needed
            std::fs::create_dir_all(&data_path).ok();

            eprintln!("[App] Database path: {:?}", data_path.join("matrix.db"));

            // 检查 Playwright 环境（非阻塞方式）
            match check_playwright_env() {
                Ok(_) => {
                    eprintln!("[App] Playwright 环境检查通过");
                }
                Err(e) => {
                    eprintln!("[App] Playwright 环境检查失败: {}，正在后台安装...", e);

                    // 在后台线程安装
                    std::thread::spawn(move || {
                        if let Err(e) = ensure_playwright_env() {
                            eprintln!("[App] Playwright 环境安装失败: {}", e);
                        } else {
                            eprintln!("[App] Playwright 环境安装完成");
                        }
                    });
                }
            }

            let db_manager = Arc::new(DatabaseManager::new(data_path.clone()));
            let browser_automator = Arc::new(Mutex::new(BrowserAutomator::new()));
            let app_state = AppState {
                db_manager,
                browser_automator,
            };

            app.manage(app_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            get_supported_platforms,
            get_accounts,
            add_account,
            delete_account,
            open_file_dialog,
            // Publication task commands (new main + sub table structure)
            get_publication_tasks,
            get_publication_task,
            create_publication_task,
            delete_publication_task,
            publish_video,
            publish_saved_video,
            start_browser_auth,
            check_browser_auth_status,
            cancel_browser_auth,
            get_extractor_configs,
            get_extractor_config,
            save_extractor_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
