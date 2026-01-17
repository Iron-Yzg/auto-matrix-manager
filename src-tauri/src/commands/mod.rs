// Commands module - Tauri commands for the application
// 命令模块 - Tauri 应用命令

use crate::core::*;
use crate::storage::DatabaseManager;
use crate::platforms::douyin::DouyinPlatform;
use crate::browser::{BrowserAutomator, BrowserAuthResult, BrowserAuthStep};
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use serde::Serialize;

// App state
// 应用状态
#[derive(Clone)]
pub struct AppState {
    pub db_manager: Arc<DatabaseManager>,
    pub douyin_platform: Arc<DouyinPlatform>,
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
    pub error: Option<String>,
}

/// 启动浏览器授权流程
#[tauri::command]
pub async fn start_browser_auth(_app: AppHandle, state: tauri::State<'_, AppState>, platform: &str, _chrome_path: Option<&str>) -> Result<BrowserAuthStatusResult, String> {
    eprintln!("[Command] start_browser_auth called for platform: {}", platform);
    let mut automator = state.browser_automator.lock().await;

    // 使用 Playwright 版本启动抖音授权
    automator.start_douyin()
        .await
        .map_err(|e| format!("启动浏览器失败: {}", e))?;

    let result = automator.get_result().clone();
    eprintln!("[Command] start_browser_auth result: step={}, need_poll={}", result.step, result.need_poll);

    // 如果在初始化阶段就完成了授权，保存到数据库
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
                    nickname: account.nickname,
                    avatar_url: account.avatar_url,
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
        let account = save_browser_credentials(&app, &result, "douyin")
            .map_err(|e| format!("保存凭证失败: {}", e))?;

        // 返回完成的账号信息
        return Ok(BrowserAuthStatusResult {
            step: "Completed".to_string(),
            message: format!("授权成功！账号: {}", account.nickname),
            current_url: result.current_url,
            screenshot: result.screenshot,
            need_poll: false,
            cookie: result.cookie,
            local_storage: result.local_storage,
            nickname: account.nickname,
            avatar_url: account.avatar_url,
            error: None,
        });
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
    let local_data_items: Vec<serde_json::Value> = if let Ok(items) = serde_json::from_str::<serde_json::Value>(&result.local_storage) {
        if let Some(obj) = items.as_object() {
            obj.iter()
                .map(|(k, v)| {
                    serde_json::json!({
                        "key": k,
                        "value": v.as_str().unwrap_or("")
                    })
                })
                .collect()
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    // 解析捕获的 request_headers
    let request_headers: serde_json::Value = serde_json::from_str(&result.request_headers)
        .unwrap_or(serde_json::json!({}));

    eprintln!("[Rust] result.request_headers: {}", result.request_headers);
    eprintln!("[Rust] parsed request_headers: {}", request_headers);

    // 检查 request_headers 是否为空
    if result.request_headers.is_empty() || result.request_headers == "{}" {
        eprintln!("[Rust WARN] request_headers is empty! No API request was captured.");
    }

    // 从 request_headers 中获取 cookie（与 Python 逻辑一致）
    let request_cookie = request_headers.get("cookie")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // 从 request_headers 中提取 third_param（与 Python 逻辑完全一致）
    let third_param = serde_json::json!({
        "accept": request_headers.get("accept").unwrap_or(&serde_json::json!("")).as_str().unwrap_or(""),
        "cookie": request_cookie,
        "referer": "https://creator.douyin.com/creator-micro/content/post/video?enter_from=publish_page",
        "local_data": local_data_items,
        "sec-ch-ua": request_headers.get("sec-ch-ua").unwrap_or(&serde_json::json!("")).as_str().unwrap_or(""),
        "user-agent": request_headers.get("user-agent").unwrap_or(&serde_json::json!("")).as_str().unwrap_or(""),
        "sec-fetch-dest": request_headers.get("sec-fetch-dest").unwrap_or(&serde_json::json!("")).as_str().unwrap_or(""),
        "sec-fetch-mode": request_headers.get("sec-fetch-mode").unwrap_or(&serde_json::json!("")).as_str().unwrap_or(""),
        "sec-fetch-site": request_headers.get("sec-fetch-site").unwrap_or(&serde_json::json!("")).as_str().unwrap_or(""),
        "accept-encoding": request_headers.get("accept-encoding").unwrap_or(&serde_json::json!("")).as_str().unwrap_or(""),
        "accept-language": request_headers.get("accept-language").unwrap_or(&serde_json::json!("")).as_str().unwrap_or(""),
        "sec-ch-ua-mobile": request_headers.get("sec-ch-ua-mobile").unwrap_or(&serde_json::json!("")).as_str().unwrap_or(""),
        "sec-ch-ua-platform": request_headers.get("sec-ch-ua-platform").unwrap_or(&serde_json::json!("")).as_str().unwrap_or(""),
        "x-secsdk-csrf-token": request_headers.get("x-secsdk-csrf-token").unwrap_or(&serde_json::json!("")).as_str().unwrap_or("")
    });

    // third_id 直接从 API 响应的 uid 获取（与 Python 逻辑一致）
    let third_id = result.uid.clone();

    // 构建 params JSON - 与 dy_account.json 结构一致，只保留 third_id 和 third_param
    let params = serde_json::json!({
        "third_id": third_id,
        "third_param": third_param
    });

    // 构建账号
    let nickname = if result.nickname.is_empty() {
        format!("{}用户", get_platform_name(platform))
    } else {
        result.nickname.clone()
    };

    let account = UserAccount {
        id: uuid::Uuid::new_v4().to_string(),
        username: nickname.clone(),
        nickname,
        avatar_url: result.avatar_url.clone(),
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
    let data_path = app.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("data"));
    let db_manager = DatabaseManager::new(data_path);
    db_manager.save_account(&account)
        .map_err(|e| e.to_string())?;

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
