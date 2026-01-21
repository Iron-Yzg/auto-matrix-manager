// Commands module - Tauri commands for the application
// 命令模块 - Tauri 应用命令

use crate::core::*;
use crate::platforms::douyin::DouyinPlatform;
use crate::storage::{DatabaseManager, ExtractorConfig};
use crate::browser::{BrowserAutomator, BrowserAuthResult, BrowserAuthStep};
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use serde::Serialize;
use crate::core::{PublicationTask, PublicationAccountDetail, PublicationTaskWithAccounts};

// App state
// 应用状态
#[derive(Clone)]
pub struct AppState {
    pub db_manager: Arc<DatabaseManager>,
    pub browser_automator: Arc<tokio::sync::Mutex<BrowserAutomator>>,
}

// Required for Tauri state management
unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// Platform management commands
// 平台管理命令
#[tauri::command]
pub fn get_supported_platforms() -> Vec<PlatformInfo> {
    vec![
        PlatformInfo {
            id: "douyin",
            name: "抖音",
            icon: "/src/assets/icons/douyin.png",
            color: "#000000",
        },
        PlatformInfo {
            id: "xiaohongshu",
            name: "小红书",
            icon: "/src/assets/icons/xiaohongshu.ico",
            color: "#FE2C55",
        },
        PlatformInfo {
            id: "kuaishou",
            name: "快手",
            icon: "/src/assets/icons/kuaishu.ico",
            color: "#FF4906",
        },
        PlatformInfo {
            id: "bilibili",
            name: "B站",
            icon: "/src/assets/icons/bilibili.ico",
            color: "#00A1D6",
        },
    ]
}

#[derive(Serialize)]
pub struct PlatformInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub icon: &'static str,
    pub color: &'static str,
}

// Account management commands
// 账号管理命令
#[tauri::command]
pub fn get_accounts(
    app: AppHandle,
    platform: &str,
) -> Result<Vec<UserAccount>, String> {
    let platform_type = match platform {
        "douyin" => PlatformType::Douyin,
        "xiaohongshu" => PlatformType::Xiaohongshu,
        "kuaishou" => PlatformType::Kuaishou,
        "bilibili" => PlatformType::Bilibili,
        _ => return Err(format!("Unknown platform: {}", platform)),
    };

    let data_path = app.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("data"));
    let db_manager = DatabaseManager::new(data_path);
    db_manager.get_accounts_by_platform(platform_type)
        .map_err(|e| e.to_string())
}

/// Get all accounts across all platforms
/// 获取所有平台的账号
#[tauri::command]
pub fn get_all_accounts(app: AppHandle) -> Result<Vec<UserAccount>, String> {
    let data_path = app.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("data"));
    let db_manager = DatabaseManager::new(data_path);
    db_manager.get_all_accounts()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_account(
    app: AppHandle,
    account_id: &str,
) -> Result<bool, String> {
    let data_path = app.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("data"));
    let db_manager = DatabaseManager::new(data_path);
    db_manager.delete_account(account_id)
        .map_err(|e| e.to_string())
}

/// Add a new account via JSON params
/// 添加账号（通过JSON参数）
#[tauri::command]
pub fn add_account(
    app: AppHandle,
    platform: &str,
    username: &str,
    nickname: &str,
    avatar_url: &str,
    params: &str,
) -> Result<UserAccount, String> {
    let platform_type = match platform {
        "douyin" => PlatformType::Douyin,
        "xiaohongshu" => PlatformType::Xiaohongshu,
        "kuaishou" => PlatformType::Kuaishou,
        "bilibili" => PlatformType::Bilibili,
        _ => return Err(format!("Unknown platform: {}", platform)),
    };

    // Validate params is valid JSON
    let _: serde_json::Value = serde_json::from_str(params)
        .map_err(|e| format!("Invalid params JSON: {}", e))?;

    let account = UserAccount {
        id: uuid::Uuid::new_v4().to_string(),
        username: username.to_string(),
        nickname: nickname.to_string(),
        avatar_url: avatar_url.to_string(),
        platform: platform_type,
        params: params.to_string(),
        status: AccountStatus::Active,
        created_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    };

    let data_path = app.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("data"));
    let db_manager = DatabaseManager::new(data_path);
    db_manager.save_account(&account)
        .map_err(|e| e.to_string())?;

    Ok(account)
}

// Publication management commands
// 发布管理命令

/// Get all publication tasks with their account details
/// 获取所有作品发布任务及其账号详情
#[tauri::command]
pub fn get_publication_tasks(app: AppHandle) -> Result<Vec<PublicationTaskWithAccounts>, String> {
    let data_path = app.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("data"));
    let db_manager = DatabaseManager::new(data_path);
    db_manager.get_all_publication_tasks()
        .map_err(|e| e.to_string())
}

/// Get a single publication task with its account details
/// 获取单个作品发布任务及其账号详情
#[tauri::command]
pub fn get_publication_task(app: AppHandle, task_id: &str) -> Result<Option<PublicationTaskWithAccounts>, String> {
    let data_path = app.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("data"));
    let db_manager = DatabaseManager::new(data_path);

    // Get the main task
    let task = match db_manager.get_publication_task(task_id).map_err(|e| e.to_string())? {
        Some(t) => t,
        None => return Ok(None),
    };

    // Get account details
    let task_id_clone = task.id.clone();
    let all_tasks = db_manager.get_all_publication_tasks().map_err(|e| e.to_string())?;

    // Find the task with accounts
    for t in all_tasks {
        if t.id == task_id_clone {
            return Ok(Some(t));
        }
    }

    // If no accounts found, return task with empty accounts
    Ok(Some(PublicationTaskWithAccounts {
        id: task.id,
        title: task.title,
        description: task.description.unwrap_or_default(),
        video_path: task.video_path,
        cover_path: task.cover_path.unwrap_or_default(),
        hashtags: task.hashtags,
        status: task.status,
        created_at: task.created_at,
        published_at: task.published_at.unwrap_or_default(),
        accounts: Vec::new(),
    }))
}

/// Create a publication task with account details (main + sub tables)
/// 创建作品发布任务（主表+子表）
#[tauri::command]
pub fn create_publication_task(
    app: AppHandle,
    title: &str,
    description: &str,
    video_path: &str,
    cover_path: Option<&str>,
    account_ids: Vec<String>,
    platforms: Vec<String>,
    hashtags: Vec<Vec<String>>,
) -> Result<PublicationTaskWithAccounts, String> {
    let data_path = app.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("data"));
    let db_manager = DatabaseManager::new(data_path);

    let task_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // Create main task (flatten hashtags from Vec<Vec<String>> to Vec<String>)
    let hashtags: Vec<String> = hashtags.into_iter().flatten().collect();
    let task = PublicationTask {
        id: task_id.clone(),
        title: title.to_string(),
        description: Some(description.to_string()),
        video_path: video_path.to_string(),
        cover_path: cover_path.map(|s| s.to_string()),
        hashtags,
        status: PublicationStatus::Draft,
        created_at: now.clone(),
        published_at: None,
    };

    // Create account details (only store account info, title/description/hashtags are in main table)
    // 冗余 account_name 字段便于直接显示
    let mut account_details = Vec::new();
    for (i, account_id) in account_ids.iter().enumerate() {
        let platform_str = platforms.get(i).map(|s| s.as_str()).unwrap_or("douyin");
        let platform_type = match platform_str {
            "douyin" => PlatformType::Douyin,
            "xiaohongshu" => PlatformType::Xiaohongshu,
            "kuaishou" => PlatformType::Kuaishou,
            "bilibili" => PlatformType::Bilibili,
            _ => PlatformType::Douyin,
        };

        // Get account name from database for redundancy
        let account_name = match db_manager.get_account(account_id).map_err(|e| e.to_string())? {
            Some(acc) => acc.nickname.clone(),
            None => format!("账号{}", &account_id[..8]),
        };

        let detail = PublicationAccountDetail {
            id: uuid::Uuid::new_v4().to_string(),
            publication_task_id: task_id.clone(),
            account_id: account_id.clone(),
            account_name,  // 冗余的账号名称
            platform: platform_type,
            status: PublicationStatus::Draft,
            created_at: now.clone(),
            published_at: None,
            publish_url: None,
            stats: PublicationStats::default(),
        };

        account_details.push(detail);
    }

    // Save both main task and all account details in a transaction
    db_manager.save_publication_with_accounts(&task, &account_details)
        .map_err(|e| e.to_string())?;

    Ok(PublicationTaskWithAccounts {
        id: task_id,
        title: title.to_string(),
        description: description.to_string(),
        video_path: video_path.to_string(),
        cover_path: cover_path.map(|s| s.to_string()).unwrap_or_default(),
        hashtags: task.hashtags,
        status: PublicationStatus::Draft,
        created_at: now,
        published_at: String::new(),
        accounts: account_details,
    })
}

/// Delete a publication task and all its account details
/// 删除作品任务及其所有账号详情
#[tauri::command]
pub fn delete_publication_task(app: AppHandle, task_id: &str) -> Result<bool, String> {
    let data_path = app.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("data"));
    let db_manager = DatabaseManager::new(data_path);
    db_manager.delete_publication_task(task_id)
        .map_err(|e| e.to_string())
}

/// Get a publication task with all account details
/// 获取作品任务及其所有账号详情
#[tauri::command]
pub fn get_publication_task_with_accounts(app: AppHandle, task_id: &str) -> Result<Option<PublicationTaskWithAccounts>, String> {
    let data_path = app.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("data"));
    let db_manager = DatabaseManager::new(data_path);
    db_manager.get_publication_task_with_accounts(task_id)
        .map_err(|e| e.to_string())
}

/// Get a single publication account detail by ID
/// 根据ID获取单个作品账号发布详情
#[tauri::command]
pub fn get_publication_account_detail(app: AppHandle, detail_id: &str) -> Result<Option<PublicationAccountDetail>, String> {
    let data_path = app.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("data"));
    let db_manager = DatabaseManager::new(data_path);
    db_manager.get_publication_account_detail(detail_id)
        .map_err(|e| e.to_string())
}

/// Result of publishing a task
/// 发布任务结果
#[derive(Serialize, Clone)]
pub struct PublishTaskResult {
    pub success: bool,
    pub detail_id: String,
    pub publish_url: Option<String>,
    pub error: Option<String>,
}

/// Publish a publication task to all accounts
/// 发布作品到所有账号
#[tauri::command]
pub async fn publish_publication_task(
    app: AppHandle,
    task_id: &str,
    _title: &str,
    _description: &str,
    _video_path: &str,
    _hashtags: Vec<String>,
) -> Result<Vec<PublishTaskResult>, String> {
    eprintln!("[Publish] Starting publish for task: {}", task_id);
    let data_path = app.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("data"));
    let db_manager = DatabaseManager::new(data_path.clone());

    // Get the task with accounts
    let task = match db_manager.get_publication_task_with_accounts(task_id).map_err(|e| e.to_string())? {
        Some(t) => t,
        None => {
            eprintln!("[Publish] Task not found: {}", task_id);
            return Err("Task not found".to_string());
        }
    };
    eprintln!("[Publish] Found task with {} accounts", task.accounts.len());

    // Get main task for video path and title
    let main_task = match db_manager.get_publication_task(task_id).map_err(|e| e.to_string())? {
        Some(t) => t,
        None => return Err("Task not found".to_string()),
    };

    let mut results = Vec::new();

    // Publish to each account
    for account_detail in &task.accounts {
        eprintln!("[Publish] Processing account: {}, platform: {:?}", account_detail.account_id, account_detail.platform);
        // Skip if already published
        if account_detail.status == PublicationStatus::Completed {
            eprintln!("[Publish] Account {} already published, skipping", account_detail.account_id);
            results.push(PublishTaskResult {
                success: true,
                detail_id: account_detail.id.clone(),
                publish_url: account_detail.publish_url.clone(),
                error: None,
            });
            continue;
        }

        // Get the account (unused but kept for future use)
        let _account = match db_manager.get_account(&account_detail.account_id).map_err(|e| e.to_string())? {
            Some(acc) => acc,
            None => {
                eprintln!("[Publish] Account not found: {}", account_detail.account_id);
                results.push(PublishTaskResult {
                    success: false,
                    detail_id: account_detail.id.clone(),
                    publish_url: None,
                    error: Some("Account not found".to_string()),
                });
                continue;
            }
        };

        // Build publish request
        let platform_str = format!("{:?}", account_detail.platform);
        eprintln!("[Publish] Building request for platform: {}", platform_str);
        let request = PublishRequest {
            account_id: account_detail.account_id.clone(),
            video_path: main_task.video_path.clone().into(),
            cover_path: main_task.cover_path.clone().map(|p| p.into()),
            title: main_task.title.clone(),
            description: main_task.description.clone(),
            hashtags: main_task.hashtags.clone(),
            visibility_type: 0,
            download_allowed: 0,
            timeout: 0,
            record_id: None,
            send_time: None,
            music_info: None,
            poi_id: None,
            poi_name: None,
            anchor: None,
            extra_info: None,
            platform_data: None,
        };

        // Publish based on platform
        let publish_result = match account_detail.platform {
            PlatformType::Douyin => {
                eprintln!("[Publish] Starting Douyin publish...");
                let douyin_platform = DouyinPlatform::with_storage(db_manager.clone());
                // 注意：直接使用 .await，不需要手动创建 runtime
                // 因为 publish_publication_task 本身是 async 函数，已经在 runtime 中
                match douyin_platform.publish_video(request).await {
                    Ok(r) => {
                        eprintln!("[Publish] Douyin publish success: {}", r.success);
                        r
                    }
                    Err(e) => {
                        eprintln!("[Publish] Douyin publish error: {}", e);
                        results.push(PublishTaskResult {
                            success: false,
                            detail_id: account_detail.id.clone(),
                            publish_url: None,
                            error: Some(e.to_string()),
                        });
                        continue;
                    }
                }
            }
            _ => {
                results.push(PublishTaskResult {
                    success: false,
                    detail_id: account_detail.id.clone(),
                    publish_url: None,
                    error: Some(format!("Unsupported platform: {}", platform_str)),
                });
                continue;
            }
        };

        // Update status
        let publish_url = publish_result.item_id.clone()
            .map(|id| format!("https://v.douyin.com/{}", id));

        let new_status = if publish_result.success {
            PublicationStatus::Completed
        } else {
            PublicationStatus::Failed
        };

        if let Err(e) = db_manager.update_publication_account_status(
            &account_detail.id,
            new_status,
            publish_url.clone(),
        ) {
            eprintln!("Failed to update status: {}", e);
        }

        results.push(PublishTaskResult {
            success: publish_result.success,
            detail_id: account_detail.id.clone(),
            publish_url,
            error: publish_result.error_message,
        });
    }

    Ok(results)
}

#[tauri::command]
pub fn publish_video(
    app: AppHandle,
    platform: &str,
    account_id: &str,
    video_path: &str,
    title: &str,
    description: &str,
    hashtags: Vec<String>,
) -> Result<PublishResult, String> {
    let platform_type = match platform {
        "douyin" => PlatformType::Douyin,
        _ => return Err(format!("Unsupported platform: {}", platform)),
    };

    let data_path = app.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("data"));
    let db_manager = DatabaseManager::new(data_path.clone());

    let _account = match db_manager.get_account(account_id) {
        Ok(Some(acc)) => acc,
        Ok(None) => return Err("Account not found".to_string()),
        Err(e) => return Err(format!("Database error: {}", e)),
    };

    let request = PublishRequest {
        account_id: account_id.to_string(),
        video_path: video_path.into(),
        cover_path: None,
        title: title.to_string(),
        description: Some(description.to_string()),
        hashtags,
        visibility_type: 0,
        download_allowed: 0,
        timeout: 0,
        record_id: None,
        send_time: None,
        music_info: None,
        poi_id: None,
        poi_name: None,
        anchor: None,
        extra_info: None,
        platform_data: None,
    };

    match platform_type {
        PlatformType::Douyin => {
            let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
            let douyin_platform = DouyinPlatform::with_storage(db_manager);
            rt.block_on(async {
                douyin_platform.publish_video(request).await
            })
            .map_err(|e| e.to_string())
        }
        _ => Err("Unsupported platform".to_string()),
    }
}

/// Publish a saved publication by ID
/// 发布保存的作品
#[tauri::command]
pub fn publish_saved_video(
    app: AppHandle,
    publication_id: &str,
) -> Result<PublishResult, String> {
    let data_path = app.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("data"));
    let db_manager = DatabaseManager::new(data_path.clone());

    // Get the publication
    let publication = match db_manager.get_publication(publication_id) {
        Ok(Some(p)) => p,
        Ok(None) => return Err("Publication not found".to_string()),
        Err(e) => return Err(format!("Database error: {}", e)),
    };

    // Get the account
    let _account = match db_manager.get_account(&publication.account_id) {
        Ok(Some(acc)) => acc,
        Ok(None) => return Err("Account not found".to_string()),
        Err(e) => return Err(format!("Database error: {}", e)),
    };

    // Build publish request
    let platform_str = format!("{:?}", publication.platform);
    let request = PublishRequest {
        account_id: publication.account_id.clone(),
        video_path: publication.video_path.clone().into(),
        cover_path: publication.cover_path.clone().map(|p| p.into()),
        title: publication.title.clone(),
        description: Some(publication.description.clone()),
        hashtags: vec![],
        visibility_type: 0,
        download_allowed: 0,
        timeout: 0,
        record_id: None,
        send_time: None,
        music_info: None,
        poi_id: None,
        poi_name: None,
        anchor: None,
        extra_info: None,
        platform_data: None,
    };

    // Publish based on platform
    let result = match publication.platform {
        PlatformType::Douyin => {
            let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
            let douyin_platform = DouyinPlatform::with_storage(db_manager.clone());
            rt.block_on(async {
                douyin_platform.publish_video(request).await
            })
            .map_err(|e| e.to_string())?
        }
        _ => return Err(format!("Unsupported platform: {}", platform_str)),
    };

    // Update publication status
    let item_id = result.item_id.clone();
    let updated_publication = PlatformPublication {
        id: publication.id,
        account_id: publication.account_id,
        platform: publication.platform,
        title: publication.title,
        description: publication.description,
        video_path: publication.video_path,
        cover_path: publication.cover_path,
        status: if result.success { PublicationStatus::Completed } else { PublicationStatus::Failed },
        created_at: publication.created_at,
        published_at: Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()),
        publish_url: item_id.map(|id| format!("https://v.douyin.com/{}", id)),
        stats: PublicationStats::default(),
    };

    db_manager.save_publication(&updated_publication)
        .map_err(|e| e.to_string())?;

    Ok(result)
}

// ============================================================================
// Browser automation authentication commands
// 浏览器自动化授权命令
// ============================================================================

/// 浏览器认证状态查询结果
#[derive(Serialize, Clone)]
pub struct BrowserAuthStatusResult {
    pub step: String,
    pub message: String,
    pub current_url: String,
    pub screenshot: Option<String>,
    pub need_poll: bool,
    pub cookie: String,
    pub local_storage: String,
    pub nickname: String,
    pub avatar_url: String,
    pub third_id: String,
    pub sec_uid: String,
    pub error: Option<String>,
}

/// 启动浏览器授权流程
/// 如果传入了 account_id，则会更新现有账号而不是创建新账号
#[tauri::command]
pub async fn start_browser_auth(_app: AppHandle, state: tauri::State<'_, AppState>, platform: &str, account_id: Option<&str>, _chrome_path: Option<&str>) -> Result<BrowserAuthStatusResult, String> {
    eprintln!("[Command] start_browser_auth called for platform: {}, account_id: {:?}", platform, account_id);

    let mut automator = state.browser_automator.lock().await;

    // 使用通用规则引擎启动授权
    automator.start_authorize(&state.db_manager, platform, account_id)
        .await
        .map_err(|e| format!("启动浏览器失败: {}", e))?;

    let result = automator.get_result().clone();
    eprintln!("[Command] start_browser_auth result: step={}, need_poll={}", result.step, result.need_poll);

    // 如果在初始化阶段就完成了授权，保存到数据库
    eprintln!("[Command] Debug: result.step={:?}, cookie.len()={}", result.step, result.cookie.len());
    eprintln!("[Command] Debug: matches step = {}, cookie empty = {}",
        matches!(result.step, BrowserAuthStep::Completed), result.cookie.is_empty());

    if matches!(result.step, BrowserAuthStep::Completed) && !result.cookie.is_empty() {
        eprintln!("[Command] Authorization completed in initialization, saving to database...");

        match save_browser_credentials(&_app, &result, platform, account_id) {
            Ok(account) => {
                eprintln!("[Command] Account saved successfully: {}", account.nickname);
                return Ok(BrowserAuthStatusResult {
                    step: "Completed".to_string(),
                    message: format!("授权完成！账号: {}", account.nickname),
                    current_url: result.current_url,
                    screenshot: result.screenshot,
                    need_poll: false,
                    cookie: result.cookie,
                    local_storage: result.local_storage,
                    nickname: result.nickname,
                    avatar_url: result.avatar_url,
                    third_id: result.third_id,
                    sec_uid: result.sec_uid,
                    error: None,
                });
            },
            Err(e) => {
                eprintln!("[Command] Failed to save account: {}", e);
                return Ok(BrowserAuthStatusResult {
                    step: "Completed".to_string(),
                    message: format!("授权完成但保存失败: {}", e),
                    current_url: result.current_url,
                    screenshot: result.screenshot,
                    need_poll: false,
                    cookie: result.cookie,
                    local_storage: result.local_storage,
                    nickname: result.nickname,
                    avatar_url: result.avatar_url,
                    third_id: result.third_id,
                    sec_uid: result.sec_uid,
                    error: Some(e),
                });
            }
        }
    }

    Ok(BrowserAuthStatusResult {
        step: format!("{:?}", result.step),
        message: result.message,
        current_url: result.current_url,
        screenshot: result.screenshot,
        need_poll: result.need_poll,
        cookie: result.cookie,
        local_storage: result.local_storage,
        nickname: result.nickname,
        avatar_url: result.avatar_url,
        third_id: result.third_id,
        sec_uid: result.sec_uid,
        error: result.error,
    })
}

/// 检查浏览器授权状态并提取凭证
#[tauri::command]
pub async fn check_browser_auth_status(app: AppHandle, state: tauri::State<'_, AppState>) -> Result<BrowserAuthStatusResult, String> {
    eprintln!("[Command] check_browser_auth_status called");
    let mut automator = state.browser_automator.lock().await;

    // 检查登录状态并提取凭证
    let need_poll = automator.check_and_extract()
        .await
        .map_err(|e| format!("检查状态失败: {}", e))?;

    let result = automator.get_result().clone();

    // 如果已完成，保存凭证到数据库
    if !need_poll && !result.cookie.is_empty() {
        eprintln!("[Command] Authorization completed, saving to database...");
        eprintln!("[Command] nickname: '{}'", result.nickname);
        eprintln!("[Command] cookie.len(): {}", result.cookie.len());
        eprintln!("[Command] account_id to update: {:?}", automator.account_id);

        // 从automator获取account_id
        let account_id = automator.account_id.as_deref();
        match save_browser_credentials(&app, &result, "douyin", account_id) {
            Ok(account) => {
                eprintln!("[Command] Account saved successfully: id={}, nickname={}", account.id, account.nickname);
                // 返回完成的账号信息
                return Ok(BrowserAuthStatusResult {
                    step: "Completed".to_string(),
                    message: format!("授权成功！账号: {}", account.nickname),
                    current_url: result.current_url,
                    screenshot: result.screenshot,
                    need_poll: false,
                    cookie: result.cookie,
                    local_storage: result.local_storage,
                    nickname: result.nickname,
                    avatar_url: result.avatar_url,
                    third_id: result.third_id,
                    sec_uid: result.sec_uid,
                    error: None,
                });
            }
            Err(e) => {
                eprintln!("[Command] Failed to save account: {}", e);
                return Err(format!("保存凭证失败: {}", e));
            }
        }
    }

    Ok(BrowserAuthStatusResult {
        step: format!("{:?}", result.step),
        message: result.message,
        current_url: result.current_url,
        screenshot: result.screenshot,
        need_poll,
        cookie: result.cookie,
        local_storage: result.local_storage,
        nickname: result.nickname,
        avatar_url: result.avatar_url,
        third_id: result.third_id,
        sec_uid: result.sec_uid,
        error: result.error,
    })
}

/// 取消浏览器授权
#[tauri::command]
pub async fn cancel_browser_auth(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut automator = state.browser_automator.lock().await;
    automator.cancel().await;
    Ok(())
}

/// 保存从浏览器提取的凭证到数据库
/// 如果传入了 account_id，则更新现有账号而不是创建新账号
fn save_browser_credentials(app: &AppHandle, result: &BrowserAuthResult, platform: &str, account_id: Option<&str>) -> Result<UserAccount, String> {
    eprintln!("[Save] save_browser_credentials called");
    eprintln!("[Save] account_id to update: {:?}", account_id);
    eprintln!("[Save] nickname: '{}'", result.nickname);
    eprintln!("[Save] avatar_url: '{}'", result.avatar_url);
    eprintln!("[Save] third_id: '{}'", result.third_id);
    eprintln!("[Save] sec_uid: '{}'", result.sec_uid);
    eprintln!("[Save] cookie.len(): {}", result.cookie.len());

    // 构建 third_param - 直接使用 request_headers (JSON string)
    let third_param: serde_json::Value = serde_json::from_str(&result.request_headers)
        .unwrap_or(serde_json::json!({}));

    // 添加 cookie 和 local_storage 到 third_param
    let mut third_param_obj = third_param.as_object()
        .cloned()
        .unwrap_or_else(|| serde_json::Map::new());

    third_param_obj.insert("cookie".to_string(), serde_json::json!(result.cookie));
    third_param_obj.insert("local_data".to_string(), serde_json::json!(result.local_storage));

    // 直接从 result 读取字段
    let third_id = result.third_id.clone();
    let nickname = if !result.nickname.is_empty() {
        result.nickname.clone()
    } else {
        format!("{}用户", get_platform_name(platform))
    };
    let avatar_url = result.avatar_url.clone();

    eprintln!("[Save] third_id: {}", third_id);
    eprintln!("[Save] nickname: {}", nickname);
    eprintln!("[Save] avatar_url: {}", avatar_url);

    // 构建 params JSON
    let params = serde_json::json!({
        "third_id": third_id,
        "sec_uid": result.sec_uid,
        "third_param": serde_json::Value::Object(third_param_obj)
    });

    // 使用传入的account_id或生成新的UUID
    let account_id = account_id.map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let account = UserAccount {
        id: account_id,
        username: nickname.clone(),
        nickname,
        avatar_url,
        platform: match platform {
            "douyin" => PlatformType::Douyin,
            "xiaohongshu" => PlatformType::Xiaohongshu,
            "kuaishou" => PlatformType::Kuaishou,
            "bilibili" => PlatformType::Bilibili,
            _ => PlatformType::Douyin,
        },
        params: params.to_string(),
        status: AccountStatus::Active,
        created_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    };

    // 保存到数据库
    eprintln!("[Save] Account to save: id={}, nickname={}, avatar_url={}", account.id, account.nickname, account.avatar_url);
    let data_path = app.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("data"));
    eprintln!("[Save] Database path: {:?}", data_path);
    let db_manager = DatabaseManager::new(data_path.clone());
    db_manager.save_account(&account)
        .map_err(|e| e.to_string())?;
    eprintln!("[Save] Account saved successfully!");

    Ok(account)
}

/// 获取平台名称
fn get_platform_name(platform: &str) -> &'static str {
    match platform {
        "douyin" => "抖音",
        "xiaohongshu" => "小红书",
        "kuaishou" => "快手",
        "bilibili" => "B站",
        _ => "未知",
    }
}

// ============================================================================
// Extractor Config Management Commands
// 提取引擎配置管理命令
// ============================================================================

/// 获取所有平台提取引擎配置
#[tauri::command]
pub fn get_extractor_configs(app: AppHandle) -> Result<Vec<serde_json::Value>, String> {
    let data_path = app.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("data"));
    let db_manager = DatabaseManager::new(data_path);

    db_manager.get_all_extractor_configs()
        .map_err(|e| e.to_string())
        .map(|configs| {
            configs.into_iter().map(|c| {
                serde_json::json!({
                    "id": c.id,
                    "platform_id": c.platform_id,
                    "platform_name": c.platform_name,
                    "login_url": c.login_url,
                    "login_success_pattern": c.login_success_pattern,
                    "redirect_url": c.redirect_url,
                    "extract_rules": c.extract_rules,
                    "is_default": c.is_default,
                    "created_at": c.created_at,
                    "updated_at": c.updated_at,
                })
            }).collect()
        })
}

/// 获取指定平台的提取引擎配置
#[tauri::command]
pub fn get_extractor_config(app: AppHandle, platform_id: &str) -> Result<Option<serde_json::Value>, String> {
    let data_path = app.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("data"));
    let db_manager = DatabaseManager::new(data_path);

    db_manager.get_extractor_config(platform_id)
        .map_err(|e| e.to_string())
        .map(|config| {
            config.map(|c| {
                serde_json::json!({
                    "id": c.id,
                    "platform_id": c.platform_id,
                    "platform_name": c.platform_name,
                    "login_url": c.login_url,
                    "login_success_pattern": c.login_success_pattern,
                    "redirect_url": c.redirect_url,
                    "extract_rules": c.extract_rules,
                    "is_default": c.is_default,
                    "created_at": c.created_at,
                    "updated_at": c.updated_at,
                })
            })
        })
}

/// 保存提取引擎配置
#[tauri::command]
pub fn save_extractor_config(
    app: AppHandle,
    platform_id: &str,
    platform_name: &str,
    login_url: &str,
    login_success_pattern: &str,
    redirect_url: Option<&str>,
    extract_rules: &str,
) -> Result<bool, String> {
    let data_path = app.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("data"));
    let db_manager = DatabaseManager::new(data_path);

    // 解析 extract_rules JSON
    let rules: serde_json::Value = serde_json::from_str(extract_rules)
        .map_err(|e| format!("Invalid extract_rules JSON: {}", e))?;

    let config = ExtractorConfig {
        id: format!("config_{}", platform_id),
        platform_id: platform_id.to_string(),
        platform_name: platform_name.to_string(),
        login_url: login_url.to_string(),
        login_success_pattern: login_success_pattern.to_string(),
        redirect_url: redirect_url.map(|s| s.to_string()),
        extract_rules: rules,
        is_default: false,
        created_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        updated_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    };

    db_manager.save_extractor_config(&config)
        .map_err(|e| e.to_string())?;

    Ok(true)
}

// ============================================================================
// File selection commands
// 文件选择命令
// ============================================================================

/// Select a file and return its path
/// 选择文件并返回路径
#[tauri::command]
pub fn select_file(
    _app: AppHandle,
    title: &str,
    filter_extensions: Option<Vec<&str>>,
) -> Result<Option<String>, String> {
    // Since we can't use tauri::api::dialog directly in commands,
    // we'll return the filter info for frontend to handle
    // Or we can use webview window to open dialog
    let extensions = filter_extensions.unwrap_or_default();
    Ok(Some(format!(
        "FILE_DIALOG:{}:{}",
        title,
        extensions.join(",")
    )))
}

/// Result of file selection
#[derive(Serialize, Clone)]
pub struct FileSelectionResult {
    pub path: String,
    pub name: String,
}

/// Result of file selection with content for preview
#[derive(Serialize, Clone)]
pub struct FileSelectionWithContentResult {
    pub path: String,
    pub name: String,
    pub content: String, // Base64 encoded content
    pub mime_type: String,
}

/// Open native file dialog and return selected file path
/// 打开系统文件对话框并返回选中的文件路径
#[tauri::command]
pub async fn open_file_dialog(
    _window: tauri::Window,
    title: &str,
    _multiple: bool,
    filters: Option<Vec<String>>,
) -> Result<Option<FileSelectionResult>, String> {
    // Use rfd for native file dialog
    let mut dialog = rfd::AsyncFileDialog::new()
        .set_title(title);

    // Add filters
    if let Some(filters_str) = filters {
        let extensions: Vec<&str> = filters_str
            .iter()
            .flat_map(|f| f.split(','))
            .map(|s| s.trim().trim_start_matches('.'))
            .filter(|s| !s.is_empty())
            .collect();
        if !extensions.is_empty() {
            dialog = dialog.add_filter("Files", &extensions);
        }
    }

    // Single file selection for now
    let result = dialog
        .pick_file()
        .await;

    match result {
        Some(file) => Ok(Some(FileSelectionResult {
            path: file.path().to_string_lossy().to_string(),
            name: file.file_name().to_string(),
        })),
        None => Ok(None),
    }
}

/// Open native file dialog and return file with content for preview
/// 打开系统文件对话框并返回文件内容（用于前端预览）
#[tauri::command]
pub async fn select_file_with_content(
    _window: tauri::Window,
    title: &str,
    file_type: &str, // "video" or "image"
    _filters: Option<Vec<String>>,
) -> Result<Option<FileSelectionWithContentResult>, String> {
    // Use rfd for native file dialog
    let mut dialog = rfd::AsyncFileDialog::new()
        .set_title(title);

    // Add filters based on file type
    let extensions: Vec<&str> = match file_type {
        "video" => vec!["mp4", "mov", "avi", "mkv", "webm", "3gp", "m4v", "wmv"],
        "image" => vec!["png", "jpg", "jpeg", "webp", "gif", "bmp", "tiff", "heic"],
        _ => vec!["*"],
    };

    if !extensions.is_empty() {
        dialog = dialog.add_filter("Files", &extensions);
    }

    let result = dialog
        .pick_file()
        .await;

    match result {
        Some(file) => {
            let path = file.path().to_string_lossy().to_string();
            let name = file.file_name().to_string();

            // Read file content
            let bytes = tokio::fs::read(&path).await
                .map_err(|e| format!("Failed to read file: {}", e))?;

            // Determine mime type
            let mime_type = match file_type {
                "video" => {
                    let ext = std::path::Path::new(&path)
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("");
                    match ext.to_lowercase().as_str() {
                        "mp4" => "video/mp4",
                        "mov" => "video/quicktime",
                        "avi" => "video/x-msvideo",
                        "mkv" => "video/x-matroska",
                        "webm" => "video/webm",
                        "3gp" => "video/3gpp",
                        "m4v" => "video/x-m4v",
                        "wmv" => "video/x-ms-wmv",
                        _ => "application/octet-stream",
                    }.to_string()
                }
                "image" => {
                    let ext = std::path::Path::new(&path)
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("");
                    match ext.to_lowercase().as_str() {
                        "png" => "image/png",
                        "jpg" | "jpeg" => "image/jpeg",
                        "webp" => "image/webp",
                        "gif" => "image/gif",
                        "bmp" => "image/bmp",
                        "tiff" | "tif" => "image/tiff",
                        "heic" => "image/heic",
                        _ => "application/octet-stream",
                    }.to_string()
                }
                _ => "application/octet-stream".to_string(),
            };

            // Encode to base64
            let content = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);

            Ok(Some(FileSelectionWithContentResult {
                path,
                name,
                content,
                mime_type,
            }))
        }
        None => Ok(None),
    }
}

