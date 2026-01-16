// Browser Automation Module
// 浏览器自动化模块 - 使用 Factory 模式选择不同平台的浏览器实现

pub mod platforms;

pub use platforms::DouyinBrowser;

use std::fmt;
use serde::{Deserialize, Serialize};

// ============================================================================
// Browser Factory - 根据平台创建对应的浏览器
// ============================================================================

/// 浏览器自动化 trait
#[async_trait::async_trait]
pub trait PlatformBrowser {
    /// 启动浏览器并完成授权流程
    async fn authorize(&mut self) -> Result<BrowserAuthResult, String>;

    /// 获取平台 ID
    fn platform_id(&self) -> &str;
}

/// 通用浏览器工厂
pub struct BrowserFactory;

impl BrowserFactory {
    /// 根据平台创建对应的浏览器实例
    /// 返回 trait object，支持动态分发
    pub fn create_browser(platform: &str) -> Option<Box<dyn PlatformBrowser>> {
        match platform {
            "douyin" => Some(Box::new(DouyinBrowser::new())),
            _ => None,
        }
    }
}

// ============================================================================
// Unified Auth State - 统一的授权状态类型
// ============================================================================

/// 浏览器授权步骤（统一类型）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BrowserAuthStep {
    Idle,
    LaunchingBrowser,
    OpeningLoginPage,
    WaitingForLogin,
    LoginDetected,
    NavigatingToUpload,
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
            BrowserAuthStep::ExtractingCredentials => write!(f, "ExtractingCredentials"),
            BrowserAuthStep::ClosingBrowser => write!(f, "ClosingBrowser"),
            BrowserAuthStep::Completed => write!(f, "Completed"),
            BrowserAuthStep::Failed(msg) => write!(f, "Failed({})", msg),
        }
    }
}

/// 浏览器授权结果（统一类型）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserAuthResult {
    pub step: BrowserAuthStep,
    pub current_url: String,
    pub message: String,
    pub cookie: String,
    pub local_storage: String,
    pub nickname: String,
    pub avatar_url: String,
    pub error: Option<String>,
    pub screenshot: Option<String>,
    pub need_poll: bool,
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
            error: None,
            screenshot: None,
            need_poll: false,
        }
    }
}

// ============================================================================
// Unified Browser Automator - 统一接口
// ============================================================================

/// 统一的浏览器自动化器
/// 内部使用具体平台的浏览器实现，对外提供统一接口
pub struct BrowserAutomator {
    // 存储具体的浏览器实现
    browser: Option<BrowserWrapper>,
    result: BrowserAuthResult,
    platform_id: String,
}

enum BrowserWrapper {
    Douyin(DouyinBrowser),
}

impl BrowserAutomator {
    pub fn new() -> Self {
        Self {
            browser: None,
            result: BrowserAuthResult::default(),
            platform_id: String::new(),
        }
    }

    /// 获取平台 ID
    pub fn get_platform_id(&self) -> &str {
        &self.platform_id
    }

    /// 启动指定平台的授权流程
    pub async fn start(&mut self, platform: &str) -> Result<(), String> {
        self.start_with_chrome_path(platform, None).await
    }

    /// 启动指定平台的授权流程（带自定义 Chrome 路径）
    pub async fn start_with_chrome_path(&mut self, platform: &str, chrome_path: Option<std::path::PathBuf>) -> Result<(), String> {
        self.platform_id = platform.to_string();

        match platform {
            "douyin" => {
                let mut douyin_browser = match chrome_path {
                    Some(path) => DouyinBrowser::with_chrome_path(path),
                    None => DouyinBrowser::new(),
                };
                let result = douyin_browser.start_authorize().await?;
                self.browser = Some(BrowserWrapper::Douyin(douyin_browser));
                self.result = result; // 直接使用，因为已经是统一类型
            }
            _ => return Err(format!("不支持的平台: {}", platform)),
        }

        Ok(())
    }

    /// 检查登录状态并提取凭证
    pub async fn check_and_extract(&mut self) -> Result<bool, String> {
        let need_poll = match self.browser {
            Some(BrowserWrapper::Douyin(ref mut browser)) => {
                browser.check_and_extract().await?
            }
            None => return Ok(false),
        };

        // 更新结果
        if let Some(BrowserWrapper::Douyin(browser)) = &self.browser {
            let browser_result = browser.get_result();
            self.result.step = browser_result.step.clone();
            self.result.message = browser_result.message.clone();
            self.result.current_url = browser_result.current_url.clone();
            self.result.cookie = browser_result.cookie.clone();
            self.result.local_storage = browser_result.local_storage.clone();
            self.result.nickname = browser_result.nickname.clone();
            self.result.avatar_url = browser_result.avatar_url.clone();
            self.result.error = browser_result.error.clone();
            self.result.need_poll = browser_result.need_poll;
        }

        Ok(need_poll)
    }

    /// 取消授权
    pub async fn cancel(&mut self) {
        if let Some(BrowserWrapper::Douyin(browser)) = &mut self.browser {
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
