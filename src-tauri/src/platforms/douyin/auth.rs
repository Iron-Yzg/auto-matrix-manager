// 抖音浏览器自动化授权
// Douyin Browser Automation Authentication
//
// 本模块实现浏览器自动化授权流程
// 架构：前端使用MCP Playwright控制浏览器，后端存储凭证

use crate::core::{UserAccount, PlatformType, AccountStatus, PlatformError};

// ============================================================================
// 类型定义
// ============================================================================

/// 授权状态
#[derive(Debug, Clone, PartialEq)]
pub enum AuthState {
    /// 初始状态
    Idle,
    /// 等待用户登录
    WaitingForLogin,
    /// 登录成功，提取凭证中
    ExtractingCredentials,
    /// 授权完成
    Completed,
    /// 授权失败
    Failed(String),
}

/// 授权结果
#[derive(Debug, Clone)]
pub struct AuthResult {
    /// 账号信息
    pub account: Option<UserAccount>,
    /// 授权状态
    pub state: AuthState,
    /// 消息
    pub message: String,
}

/// 从前端传入的凭证数据
#[derive(Debug, Clone)]
pub struct CredentialsData {
    /// Cookie字符串
    pub cookie: String,
    /// User-Agent
    pub user_agent: String,
    /// localStorage JSON
    pub local_storage: String,
    /// 用户昵称
    pub nickname: String,
    /// 用户头像
    pub avatar_url: String,
}

// ============================================================================
// 认证器
// ============================================================================

/// 抖音认证器
/// 提供授权流程控制和凭证存储
#[derive(Debug, Clone)]
pub struct DouyinAuthenticator {
    /// 当前授权状态
    pub auth_state: AuthState,
    /// 登录页面URL
    login_url: String,
    /// 上传页面URL
    upload_url: String,
}

impl DouyinAuthenticator {
    /// 创建新的认证器
    pub fn new() -> Self {
        Self {
            auth_state: AuthState::Idle,
            login_url: "https://creator.douyin.com/site/login".to_string(),
            upload_url: "https://creator.douyin.com/creator-micro/content/post/video".to_string(),
        }
    }

    /// 使用已有凭证创建认证器（用于刷新凭证）
    pub fn with_credentials(_credentials: CredentialsData) -> Self {
        Self {
            auth_state: AuthState::Idle,
            login_url: "https://creator.douyin.com/site/login".to_string(),
            upload_url: "https://creator.douyin.com/creator-micro/content/post/video".to_string(),
        }
    }

    /// 开始授权流程（异步版本，用于Platform trait）
    /// 返回登录URL，前端需要打开浏览器导航到此URL
    pub fn start_auth_flow(&mut self) -> Result<String, PlatformError> {
        self.auth_state = AuthState::WaitingForLogin;
        Ok(self.login_url.clone())
    }

    /// 检查登录状态
    /// 前端调用此方法检查用户是否已完成登录
    pub fn check_login_status(&self) -> AuthResult {
        AuthResult {
            account: None,
            state: self.auth_state.clone(),
            message: "请在浏览器中完成登录".to_string(),
        }
    }

    /// 完成授权（由前端调用）
    /// 前端完成浏览器操作后，调用此方法保存凭证
    pub fn complete_auth(&mut self, credentials: CredentialsData) -> Result<UserAccount, PlatformError> {
        self.auth_state = AuthState::ExtractingCredentials;

        // 构建local_data数组
        let local_data_items: Vec<serde_json::Value> = if let Ok(items) = serde_json::from_str::<serde_json::Value>(&credentials.local_storage) {
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

        // 提取关键字段
        let third_id = extract_from_cookies(&credentials.cookie, "tt_webid");
        let sec_uid = extract_from_localstorage(&credentials.local_storage, "sec_user_id")
            .or_else(|| extract_from_localstorage(&credentials.local_storage, "sec_uid"));

        // 构建params JSON
        let params = serde_json::json!({
            "cookie": credentials.cookie,
            "user_agent": credentials.user_agent,
            "third_id": third_id,
            "sec_uid": sec_uid.unwrap_or_default(),
            "local_data": local_data_items
        });

        // 构建账号
        let account = UserAccount {
            id: uuid::Uuid::new_v4().to_string(),
            username: credentials.nickname.clone(),
            nickname: credentials.nickname,
            avatar_url: credentials.avatar_url,
            platform: PlatformType::Douyin,
            params: params.to_string(),
            status: AccountStatus::Active,
            created_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        };

        self.auth_state = AuthState::Completed;

        Ok(account)
    }

    /// 取消授权
    pub fn cancel_auth(&mut self) {
        self.auth_state = AuthState::Idle;
    }

    /// 获取登录URL
    pub fn get_login_url(&self) -> String {
        self.login_url.clone()
    }

    /// 获取上传页面URL
    pub fn get_upload_url(&self) -> String {
        self.upload_url.clone()
    }

    /// 刷新凭证（重新认证）
    pub async fn refresh(&mut self, _existing_account: UserAccount) -> Result<UserAccount, PlatformError> {
        // 重置状态并开始新流程
        self.auth_state = AuthState::Idle;
        self.start_auth_flow()?;
        Err(PlatformError::AuthenticationFailed(
            "请重新完成登录授权".to_string()
        ))
    }

    /// 完整的认证流程（异步版本）
    /// 注意：由于浏览器自动化需要在frontend使用MCP Playwright执行，
    /// 此方法主要用于向后兼容，实际浏览器控制由frontend完成
    pub async fn authenticate(&mut self) -> Result<UserAccount, PlatformError> {
        // 启动授权流程
        let _login_url = self.start_auth_flow()?;

        // 返回错误，提示需要使用frontend的MCP Playwright进行浏览器控制
        Err(PlatformError::AuthenticationFailed(
            "请使用frontend浏览器授权功能完成登录".to_string()
        ))
    }
}

/// 从Cookie字符串中提取指定名称的值
fn extract_from_cookies(cookies: &str, name: &str) -> String {
    for cookie in cookies.split(';') {
        let cookie = cookie.trim();
        if cookie.starts_with(&format!("{}=", name)) {
            if let Some(value) = cookie.split('=').nth(1) {
                return value.to_string();
            }
        }
    }
    String::new()
}

/// 从localStorage JSON中提取指定键的值
fn extract_from_localstorage(local_storage: &str, key: &str) -> Option<String> {
    if let Ok(items) = serde_json::from_str::<serde_json::Value>(local_storage) {
        if let Some(value) = items.get(key) {
            return value.as_str().map(|s| s.to_string());
        }
    }
    None
}

/// 从用户提供的Cookie字符串解析凭证（备用方案）
pub fn parse_credentials_from_cookie_string(cookie_string: &str, user_agent: &str) -> String {
    let third_id = extract_from_cookies(cookie_string, "tt_webid");

    let params = serde_json::json!({
        "cookie": cookie_string,
        "user_agent": user_agent,
        "third_id": third_id,
        "sec_uid": "",
        "local_data": []
    });

    params.to_string()
}
