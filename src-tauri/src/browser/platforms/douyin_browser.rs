// Douyin Browser Implementation
// 抖音浏览器自动化实现

use crate::browser::{BrowserAuthResult, BrowserAuthStep};
use headless_chrome::{Browser, LaunchOptions};
use std::path::PathBuf;
use std::sync::Arc;

/// 抖音浏览器实现 - 使用统一的授权类型
pub struct DouyinBrowser {
    browser: Option<Browser>,
    tab: Option<Arc<headless_chrome::Tab>>,
    result: BrowserAuthResult,
    /// 自定义 Chrome 浏览器路径
    chrome_path: Option<PathBuf>,
}

impl DouyinBrowser {
    pub fn new() -> Self {
        Self {
            browser: None,
            tab: None,
            result: BrowserAuthResult::default(),
            chrome_path: None,
        }
    }

    /// 设置 Chrome 浏览器路径
    pub fn with_chrome_path(path: PathBuf) -> Self {
        Self {
            browser: None,
            tab: None,
            result: BrowserAuthResult::default(),
            chrome_path: Some(path),
        }
    }

    /// 注入 CDC 提示浮层
    fn inject_tip_overlay(&self) {
        let tip_script = r#"
            (function() {
                console.log('[AMM] Starting tip injection...');

                // 移除已存在的提示
                const existing = document.getElementById('amm-tip-overlay');
                if (existing) {
                    console.log('[AMM] Removing existing tip overlay');
                    existing.remove();
                }

                // 检查 body 是否存在
                if (!document.body) {
                    console.log('[AMM] document.body does not exist yet');
                    return { success: false, error: 'body not exists' };
                }

                console.log('[AMM] Creating tip overlay...');

                // 创建提示浮层
                const tip = document.createElement('div');
                tip.id = 'amm-tip-overlay';
                tip.innerHTML = `
                    <div style="
                        position: fixed !important;
                        top: 20px !important;
                        left: 50% !important;
                        transform: translateX(-50%) !important;
                        background: linear-gradient(135deg, #ff9500 0%, #ff6b00 100%) !important;
                        color: white !important;
                        padding: 20px 28px !important;
                        border-radius: 12px !important;
                        font-size: 15px !important;
                        font-weight: 600 !important;
                        box-shadow: 0 10px 40px rgba(255, 149, 0, 0.4) !important;
                        z-index: 99999999 !important;
                        display: flex !important;
                        flex-direction: column !important;
                        align-items: center !important;
                        gap: 12px !important;
                        max-width: 420px !important;
                        font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif !important;
                    ">
                        <div style="display: flex !important; align-items: center !important; gap: 10px !important;">
                            <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" style="display: block !important;">
                                <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" fill="none" stroke="currentColor"></path>
                                <path d="M12 8v4" fill="none" stroke="currentColor"></path>
                                <path d="M12 16h.01" fill="none" stroke="currentColor"></path>
                            </svg>
                            <span style="display: block !important;">授权助手正在运行</span>
                        </div>
                        <span style="text-align: center !important; opacity: 0.95 !important; font-weight: 400 !important; line-height: 1.5 !important; display: block !important;">
                            请使用抖音App扫码登录<br>登录成功后页面将自动跳转
                        </span>
                        <div id="amm-login-status" style="
                            font-size: 12px !important;
                            padding: 6px 12px !important;
                            background: rgba(255,255,255,0.2) !important;
                            border-radius: 20px !important;
                            font-weight: 400 !important;
                            display: block !important;
                        ">等待扫码登录...</div>
                    </div>
                    <style>
                        @keyframes amm-slide-down {
                            from { opacity: 0 !important; transform: translateX(-50%) translateY(-30px) !important; }
                            to { opacity: 1 !important; transform: translateX(-50%) translateY(0) !important; }
                        }
                        @keyframes amm-pulse {
                            0%, 100% { opacity: 1 !important; }
                            50% { opacity: 0.7 !important; }
                        }
                        #amm-login-status {
                            animation: amm-pulse 2s infinite !important;
                        }
                        #amm-tip-overlay {
                            animation: amm-slide-down 0.4s ease-out !important;
                        }
                    </style>
                `;

                // 使用 prepend 确保在 body 最前面
                document.body.insertBefore(tip, document.body.firstChild);

                console.log('[AMM] Tip overlay inserted successfully');

                // 验证 tip 是否存在
                const tipEl = document.getElementById('amm-tip-overlay');
                if (tipEl) {
                    const style = window.getComputedStyle(tipEl);
                    return {
                        success: true,
                        display: style.display,
                        visibility: style.visibility,
                        zIndex: style.zIndex
                    };
                }
                return { success: false, error: 'tip element not found after insert' };
            })();
        "#;

        if let Some(tab) = &self.tab {
            match tab.evaluate(tip_script, true) {  // true = return_by_value
                Ok(result) => {
                    eprintln!("[DouyinBrowser] Tip injection result: {:?}", result);
                },
                Err(e) => eprintln!("[DouyinBrowser] Failed to inject tip overlay: {}", e),
            }
        }
    }

    /// 启动浏览器
    fn launch_browser(&self) -> Result<Browser, String> {
        eprintln!("[DouyinBrowser] Launch options: path={:?}", self.chrome_path);

        // 构建启动选项 - 设置 headless=false 以显示浏览器窗口
        let launch_options = LaunchOptions::default_builder()
            .headless(false)  // 显示窗口而不是 headless 模式
            .window_size(Some((1280, 800)))  // 设置窗口大小
            .path(self.chrome_path.clone())
            .build()
            .map_err(|e| format!("构建启动选项失败: {}", e))?;

        // 启动浏览器
        Browser::new(launch_options)
            .map_err(|e| format!("启动浏览器失败: {}", e))
    }

    /// 启动抖音授权流程
    pub async fn start_authorize(&mut self) -> Result<BrowserAuthResult, String> {
        self.result.step = BrowserAuthStep::LaunchingBrowser;
        self.result.message = "正在启动浏览器...".to_string();
        eprintln!("[DouyinBrowser] Launching browser...");

        // 启动浏览器
        self.browser = match self.launch_browser() {
            Ok(b) => {
                eprintln!("[DouyinBrowser] Browser launched successfully");
                Some(b)
            }
            Err(e) => {
                let err_msg = format!("无法启动浏览器: {}", e);
                eprintln!("[DouyinBrowser] Failed to launch browser: {}", e);
                self.result.step = BrowserAuthStep::Failed(err_msg.clone());
                self.result.message = err_msg.clone();
                self.result.error = Some(err_msg.clone());
                return Err(err_msg);
            }
        };

        // 获取初始标签页（而不是创建新标签页）
        let browser_ref = self.browser.as_ref().expect("Browser should be Some");
        let initial_tab = browser_ref
            .new_tab()
            .map_err(|e| format!("创建标签页失败: {}", e))?;
        self.tab = Some(initial_tab);
        eprintln!("[DouyinBrowser] Got initial tab");

        self.result.step = BrowserAuthStep::OpeningLoginPage;
        self.result.message = "正在打开登录页面...".to_string();
        eprintln!("[DouyinBrowser] Navigating to https://creator.douyin.com/...");

        // 导航到抖音创作者中心
        self.tab.as_ref().unwrap().navigate_to("https://creator.douyin.com/")
            .map_err(|e| format!("导航失败: {}", e))?;

        // 等待页面加载
        self.tab.as_ref().unwrap().wait_until_navigated()
            .map_err(|e| format!("等待导航失败: {}", e))?;

        eprintln!("[DouyinBrowser] Page navigated, waiting for content to load...");

        // 等待页面完全加载（抖音创作者中心需要加载动态内容）
        std::thread::sleep(std::time::Duration::from_secs(3));

        // 检查页面是否加载了主要内容
        let body_check = self.tab.as_ref().unwrap().evaluate(
            r#"(function() {
                const body = document.body;
                return {
                    hasContent: body && body.children.length > 0,
                    childCount: body ? body.children.length : 0,
                    readyState: document.readyState
                };
            })()"#,
            true  // return_by_value: true 执行函数并返回值
        );
        eprintln!("[DouyinBrowser] Page body check: {:?}", body_check);

        // 注入提示浮层
        eprintln!("[DouyinBrowser] Injecting tip overlay...");
        self.inject_tip_overlay();

        // 获取当前 URL
        let current_url = self.tab.as_ref().unwrap().get_url();
        self.result.current_url = current_url.clone();
        eprintln!("[DouyinBrowser] Current URL after navigation: {}", current_url);

        self.result.step = BrowserAuthStep::WaitingForLogin;
        self.result.message = "请扫码登录，页面将自动跳转".to_string();
        self.result.need_poll = true;

        Ok(self.result.clone())
    }

    /// 检查登录状态并提取凭证
    pub async fn check_and_extract(&mut self) -> Result<bool, String> {
        let tab = match self.tab.as_ref() {
            Some(t) => t,
            None => {
                eprintln!("[DouyinBrowser] Tab is None, browser may have been closed");
                return Ok(false);
            }
        };

        // 获取当前 URL
        let current_url = tab.get_url();
        self.result.current_url = current_url.clone();
        eprintln!("[DouyinBrowser] Checking URL: {}", current_url);

        // 检测是否登录成功 - 抖音创作者中心登录后会跳转到以下 URL 模式
        let is_logged_in = Self::is_douyin_logged_in(&current_url);
        eprintln!("[DouyinBrowser] Is logged in: {}", is_logged_in);

        if !is_logged_in {
            // 仍在登录页面
            self.result.step = BrowserAuthStep::WaitingForLogin;
            self.result.message = "请扫码登录...".to_string();
            self.result.need_poll = true;
            return Ok(true);
        }

        // 登录成功
        eprintln!("[DouyinBrowser] Detected login success!");

        self.result.step = BrowserAuthStep::LoginDetected;
        self.result.message = "检测到登录成功，正在跳转...".to_string();

        // 跳转到上传页面
        self.result.step = BrowserAuthStep::NavigatingToUpload;
        self.result.message = "正在跳转到上传页面...".to_string();
        eprintln!("[DouyinBrowser] Navigating to upload page...");

        tab.navigate_to("https://creator.douyin.com/creator-micro/content/post/video")
            .map_err(|e| format!("导航到上传页失败: {}", e))?;

        tab.wait_until_navigated()
            .map_err(|e| format!("等待上传页导航失败: {}", e))?;

        // 等待页面加载
        std::thread::sleep(std::time::Duration::from_secs(3));

        let upload_url = tab.get_url();
        eprintln!("[DouyinBrowser] Upload page URL: {}", upload_url);

        // 提取凭证
        self.result.step = BrowserAuthStep::ExtractingCredentials;
        self.result.message = "正在提取凭证...".to_string();
        eprintln!("[DouyinBrowser] Extracting credentials...");

        // 克隆 tab 引用以解决生命周期问题
        let tab_for_extract = self.tab.clone().unwrap();
        self.extract_credentials(tab_for_extract.as_ref()).await?;

        eprintln!("[DouyinBrowser] Cookie: {} bytes", self.result.cookie.len());
        eprintln!("[DouyinBrowser] LocalStorage: {} bytes", self.result.local_storage.len());
        eprintln!("[DouyinBrowser] Nickname: {}", self.result.nickname);

        // 关闭浏览器
        self.result.step = BrowserAuthStep::ClosingBrowser;
        self.result.message = "正在关闭浏览器...".to_string();
        self.close_browser().await;

        self.result.step = BrowserAuthStep::Completed;
        self.result.message = format!("授权完成！账号: {}", self.result.nickname);
        self.result.need_poll = false;

        eprintln!("[DouyinBrowser] Authorization completed successfully");

        return Ok(false);
    }

    /// 检测抖音是否已登录
    fn is_douyin_logged_in(url: &str) -> bool {
        // 抖音创作者中心登录成功后会跳转到以下 URL 模式
        // https://creator.douyin.com/creator-micro/home
        // https://creator.douyin.com/creator-micro/
        // 也可能是带有 query 参数的 URL
        eprintln!("[DouyinBrowser] Checking if logged in, URL: {}", url);

        if url.contains("creator.douyin.com/creator-micro") {
            // 已经在创作者中心
            return true;
        }

        if url.contains("passport.douyin.com") && url.contains("login") {
            // 抖音 passport 登录页面 - 未登录
            return false;
        }

        if url.contains("creator.douyin.com") {
            // 创作者中心其他页面 - 可能已登录
            return true;
        }

        false
    }

    /// 提取 Cookie 和 LocalStorage
    async fn extract_credentials(&mut self, tab: &headless_chrome::Tab) -> Result<(), String> {
        eprintln!("[DouyinBrowser] Starting credential extraction...");

        // 提取 Cookie
        eprintln!("[DouyinBrowser] Extracting cookies...");
        if let Ok(cookies) = tab.get_cookies() {
            let cookie_parts: Vec<String> = cookies.iter()
                .map(|c| format!("{}={}", c.name, c.value))
                .collect();
            self.result.cookie = cookie_parts.join("; ");
            eprintln!("[DouyinBrowser] Extracted {} cookies", cookies.len());
        } else {
            eprintln!("[DouyinBrowser] Failed to get cookies");
        }

        // 提取 LocalStorage
        eprintln!("[DouyinBrowser] Extracting localStorage...");
        let ls_script = r#"
            (function() {
                try {
                    const items = {};
                    for (let i = 0; i < localStorage.length; i++) {
                        const key = localStorage.key(i);
                        items[key] = localStorage.getItem(key);
                    }
                    return JSON.stringify(items);
                } catch (e) {
                    return JSON.stringify({error: e.message});
                }
            })()
        "#;

        if let Ok(ls_result) = tab.evaluate(ls_script, true) {
            if let Some(ls_value) = ls_result.value {
                if let Some(s) = ls_value.as_str() {
                    self.result.local_storage = s.to_string();
                    eprintln!("[DouyinBrowser] LocalStorage extracted successfully");
                }
            }
        }

        // 提取用户昵称 - 尝试多个可能的 key
        eprintln!("[DouyinBrowser] Extracting nickname...");
        let nickname_script = r#"
            (function() {
                // 尝试多个可能的昵称 key
                const keys = ['nickname', 'user_nickname', 'user_name', 'username', 'dy_nickname'];
                for (const key of keys) {
                    const value = localStorage.getItem(key);
                    if (value && value.trim() !== '') {
                        return value;
                    }
                }
                // 如果都没找到，返回空字符串
                return '';
            })()
        "#;

        if let Ok(nick_result) = tab.evaluate(nickname_script, true) {
            if let Some(nick_value) = nick_result.value {
                if let Some(s) = nick_value.as_str() {
                    if !s.is_empty() {
                        self.result.nickname = s.to_string();
                        eprintln!("[DouyinBrowser] Nickname extracted: {}", s);
                    }
                }
            }
        }

        if self.result.nickname.is_empty() {
            self.result.nickname = "抖音用户".to_string();
            eprintln!("[DouyinBrowser] Using default nickname");
        }

        // 提取头像
        eprintln!("[DouyinBrowser] Extracting avatar...");
        let avatar_script = r#"
            (function() {
                const keys = ['avatar_url', 'user_avatar', 'user_avatar_url', 'dy_avatar'];
                for (const key of keys) {
                    const value = localStorage.getItem(key);
                    if (value && value.trim() !== '') {
                        return value;
                    }
                }
                return '';
            })()
        "#;

        if let Ok(avatar_result) = tab.evaluate(avatar_script, true) {
            if let Some(avatar_value) = avatar_result.value {
                if let Some(s) = avatar_value.as_str() {
                    if !s.is_empty() {
                        self.result.avatar_url = s.to_string();
                        eprintln!("[DouyinBrowser] Avatar extracted");
                    }
                }
            }
        }

        eprintln!("[DouyinBrowser] Credential extraction completed");
        Ok(())
    }

    /// 关闭浏览器
    async fn close_browser(&mut self) {
        eprintln!("[DouyinBrowser] Closing browser...");

        // 清除引用，Browser 的 drop 会关闭浏览器
        self.tab = None;
        self.browser = None;

        eprintln!("[DouyinBrowser] Browser closed");
    }

    /// 取消授权
    pub async fn cancel(&mut self) {
        self.close_browser().await;
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

impl Default for DouyinBrowser {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// PlatformBrowser Trait Implementation
// ============================================================================

#[async_trait::async_trait]
impl crate::browser::PlatformBrowser for DouyinBrowser {
    fn platform_id(&self) -> &str {
        "douyin"
    }

    async fn authorize(&mut self) -> Result<BrowserAuthResult, String> {
        self.start_authorize().await
    }
}
