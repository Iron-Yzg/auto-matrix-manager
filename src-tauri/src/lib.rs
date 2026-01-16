// Core modules
// 核心模块
pub mod core;
pub mod storage;
pub mod browser;
pub mod platforms;
pub mod commands;

// Re-export browser types for easy access
pub use browser::{BrowserAutomator, BrowserAuthResult, BrowserAuthStep, BrowserFactory, PlatformBrowser};

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
    use crate::platforms::douyin::DouyinPlatform;
    use crate::commands::AppState;
    use crate::browser::BrowserAutomator;

    let base_path = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    let data_path = base_path.join("data");

    // Create data directory if needed
    std::fs::create_dir_all(&data_path).ok();

    let db_manager = Arc::new(DatabaseManager::new(data_path.clone()));
    let douyin_platform = Arc::new(DouyinPlatform::with_storage((*db_manager).clone()));
    let browser_automator = Arc::new(Mutex::new(BrowserAutomator::new()));
    let app_state = AppState {
        db_manager,
        douyin_platform,
        browser_automator,
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            greet,
            get_supported_platforms,
            get_accounts,
            add_account,
            delete_account,
            get_publications,
            publish_video,
            start_browser_auth,
            check_browser_auth_status,
            cancel_browser_auth,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
