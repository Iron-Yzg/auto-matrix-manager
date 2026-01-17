// Browser Automation Module
// 浏览器自动化模块 - 使用 Playwright 实现

pub mod platforms;

pub use platforms::DouyinBrowserPlaywright;
pub use platforms::{check_playwright_env, ensure_playwright_env};

use std::fmt;
use serde::{Deserialize, Serialize};

// ============================================================================
// Browser Auth Types
// ============================================================================

/// 浏览器授权步骤
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BrowserAuthStep {
    Idle,
    LaunchingBrowser,
    OpeningLoginPage,
    WaitingForLogin,
    LoginDetected,
    NavigatingToUpload,
    WaitingForUpload,
    ExtractingCredentials,
    ClosingBrowser,
    Completed,
    Failed(String),
}

impl fmt::Display for BrowserAuthStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BrowserAuthStep::Idle => write!(f, "Idle"),
            BrowserAuthStep::LaunchingBrowser => write!(f, "LaunchingBrowser"),
            BrowserAuthStep::OpeningLoginPage => write!(f, "OpeningLoginPage"),
            BrowserAuthStep::WaitingForLogin => write!(f, "WaitingForLogin"),
            BrowserAuthStep::LoginDetected => write!(f, "LoginDetected"),
            BrowserAuthStep::NavigatingToUpload => write!(f, "NavigatingToUpload"),
            BrowserAuthStep::WaitingForUpload => write!(f, "WaitingForUpload"),
            BrowserAuthStep::ExtractingCredentials => write!(f, "ExtractingCredentials"),
            BrowserAuthStep::ClosingBrowser => write!(f, "ClosingBrowser"),
            BrowserAuthStep::Completed => write!(f, "Completed"),
            BrowserAuthStep::Failed(msg) => write!(f, "Failed({})", msg),
        }
    }
}

/// 浏览器授权结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserAuthResult {
    pub step: BrowserAuthStep,
    pub current_url: String,
    pub message: String,
    pub cookie: String,
    pub local_storage: String,
    pub nickname: String,
    pub avatar_url: String,
    pub uid: String,
    pub sec_uid: String,
    pub error: Option<String>,
    pub screenshot: Option<String>,
    pub need_poll: bool,
    pub request_headers: String, // JSON string of request headers for publishing
}

impl Default for BrowserAuthResult {
    fn default() -> Self {
        Self {
            step: BrowserAuthStep::Idle,
            current_url: String::new(),
            message: "准备开始授权".to_string(),
            cookie: String::new(),
            local_storage: String::new(),
            nickname: String::new(),
            avatar_url: String::new(),
            uid: String::new(),
            sec_uid: String::new(),
            error: None,
            screenshot: None,
            need_poll: false,
            request_headers: String::new(),
        }
    }
}

/// 浏览器自动化器（使用 Playwright）
pub struct BrowserAutomator {
    browser: Option<DouyinBrowserPlaywright>,
    result: BrowserAuthResult,
}

impl BrowserAutomator {
    pub fn new() -> Self {
        Self {
            browser: None,
            result: BrowserAuthResult::default(),
        }
    }

    /// 启动抖音授权流程
    pub async fn start_douyin(&mut self) -> Result<(), String> {
        let mut douyin_browser = DouyinBrowserPlaywright::new();
        let result = douyin_browser.start_authorize().await?;
        self.browser = Some(douyin_browser);
        self.result = result;
        Ok(())
    }

    /// 检查登录状态并提取凭证
    pub async fn check_and_extract(&mut self) -> Result<bool, String> {
        match &mut self.browser {
            Some(browser) => {
                browser.check_and_extract().await?;
                let browser_result = browser.get_result();
                self.result.step = browser_result.step.clone();
                self.result.message = browser_result.message.clone();
                self.result.current_url = browser_result.current_url.clone();
                self.result.cookie = browser_result.cookie.clone();
                self.result.local_storage = browser_result.local_storage.clone();
                self.result.nickname = browser_result.nickname.clone();
                self.result.avatar_url = browser_result.avatar_url.clone();
                self.result.uid = browser_result.uid.clone();
                self.result.sec_uid = browser_result.sec_uid.clone();
                self.result.error = browser_result.error.clone();
                self.result.need_poll = browser_result.need_poll;
                self.result.request_headers = browser_result.request_headers.clone();
                Ok(self.result.need_poll)
            }
            None => Ok(false),
        }
    }

    /// 取消授权
    pub async fn cancel(&mut self) {
        if let Some(browser) = &mut self.browser {
            browser.cancel().await;
        }
        self.result.step = BrowserAuthStep::Idle;
        self.result.message = "已取消授权".to_string();
        self.result.need_poll = false;
    }

    /// 获取授权结果
    pub fn get_result(&self) -> &BrowserAuthResult {
        &self.result
    }

    /// 获取授权结果（可修改）
    pub fn get_result_mut(&mut self) -> &mut BrowserAuthResult {
        &mut self.result
    }
}

impl Default for BrowserAutomator {
    fn default() -> Self {
        Self::new()
    }
}
