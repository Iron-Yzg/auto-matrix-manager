// Commands module - Tauri commands for the application
// 命令模块 - Tauri 应用命令

use crate::core::*;
use crate::platforms::douyin::DouyinPlatform;
use crate::storage::{DatabaseManager, ExtractorConfig};
use crate::browser::{BrowserAutomator, BrowserAuthResult, BrowserAuthStep};
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use serde::Serialize;

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
#[tauri::command]
pub fn get_publications(
    app: AppHandle,
    _platform: &str,
    account_id: &str,
) -> Result<Vec<PlatformPublication>, String> {
    let data_path = app.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("data"));
    let db_manager = DatabaseManager::new(data_path);
    db_manager.get_publications_by_account(account_id)
        .map_err(|e| e.to_string())
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
        description: description.to_string(),
        hashtags,
        visibility_type: 0,
        download_allowed: 0,
        timeout: 0,
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
#[tauri::command]
pub async fn start_browser_auth(_app: AppHandle, state: tauri::State<'_, AppState>, platform: &str, _chrome_path: Option<&str>) -> Result<BrowserAuthStatusResult, String> {
    eprintln!("[Command] start_browser_auth called for platform: {}", platform);

    let mut automator = state.browser_automator.lock().await;

    // 使用通用规则引擎启动授权
    automator.start_authorize(&state.db_manager, platform)
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

        match save_browser_credentials(&_app, &result, platform) {
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

        match save_browser_credentials(&app, &result, "douyin") {
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
fn save_browser_credentials(app: &AppHandle, result: &BrowserAuthResult, platform: &str) -> Result<UserAccount, String> {
    eprintln!("[Save] save_browser_credentials called");
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

    let account = UserAccount {
        id: uuid::Uuid::new_v4().to_string(),
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
