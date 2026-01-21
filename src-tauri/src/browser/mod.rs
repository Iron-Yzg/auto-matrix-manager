// Browser Automation Module
// 浏览器自动化模块 - 使用通用规则引擎

pub mod data_extractor_engine;
pub mod generic_browser;
pub mod playwright_env;

pub use generic_browser::GenericBrowser;
pub use playwright_env::{check_playwright_env, ensure_playwright_env};
pub use data_extractor_engine::DataExtractorEngine;

use std::fmt;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::storage::DatabaseManager;

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
    pub third_id: String,
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
            third_id: String::new(),
            sec_uid: String::new(),
            error: None,
            screenshot: None,
            need_poll: false,
            request_headers: String::new(),
        }
    }
}

/// 浏览器自动化器（使用通用规则引擎）
pub struct BrowserAutomator {
    browser: Option<GenericBrowser>,
    result: BrowserAuthResult,
    /// 重新授权时需要更新的账号ID
    pub account_id: Option<String>,
}

impl BrowserAutomator {
    pub fn new() -> Self {
        Self {
            browser: None,
            result: BrowserAuthResult::default(),
            account_id: None,
        }
    }

    /// 启动通用授权流程
    /// 如果传入了 account_id，则在授权完成后会更新该账号而不是创建新账号
    pub async fn start_authorize(&mut self, db_manager: &Arc<DatabaseManager>, platform_id: &str, account_id: Option<&str>) -> Result<(), String> {
        // 保存需要更新的账号ID
        self.account_id = account_id.map(|s| s.to_string());

        let mut browser = GenericBrowser::new();
        browser.set_db_manager(db_manager.clone());
        let result = browser.start_authorize(platform_id).await?;
        self.browser = Some(browser);
        self.result = result;
        Ok(())
    }

    /// 检查登录状态并提取凭证
    /// GenericBrowser 同步完成授权，此方法返回当前状态
    pub async fn check_and_extract(&mut self) -> Result<bool, String> {
        match &mut self.browser {
            Some(browser) => {
                let browser_result = browser.get_result();
                self.result.step = browser_result.step.clone();
                self.result.message = browser_result.message.clone();
                self.result.current_url = browser_result.current_url.clone();
                self.result.cookie = browser_result.cookie.clone();
                self.result.local_storage = browser_result.local_storage.clone();
                self.result.nickname = browser_result.nickname.clone();
                self.result.avatar_url = browser_result.avatar_url.clone();
                self.result.third_id = browser_result.third_id.clone();
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
