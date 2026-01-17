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

    /// 注入API拦截器
    fn inject_api_interceptor(&self) {
        let interceptor_script = r#"
            (function() {
                // 初始化数据存储
                window.__API_DATA__ = {
                    userInfo: null,
                    accountInfo: null
                };
                
                console.log('[API] Installing interceptor...');
                
                // 拦截 fetch
                const originalFetch = window.fetch;
                window.fetch = function(...args) {
                    const url = args[0];
                    console.log('[API] Fetch called:', url);
                    
                    return originalFetch.apply(this, args).then(response => {
                        const clonedResponse = response.clone();
                        
                        if (url.includes('/web/api/media/user/info')) {
                            console.log('[API] ✓ Intercepted user info API');
                            clonedResponse.json().then(data => {
                                window.__API_DATA__.userInfo = data;
                                console.log('[API] ✓ User info saved:', data);
                            }).catch(e => console.error('[API] ✗ Error parsing user info:', e));
                        } else if (url.includes('/account/api/v1/user/account/info')) {
                            console.log('[API] ✓ Intercepted account info API');
                            clonedResponse.json().then(data => {
                                window.__API_DATA__.accountInfo = data;
                                console.log('[API] ✓ Account info saved:', data);
                            }).catch(e => console.error('[API] ✗ Error parsing account info:', e));
                        }
                        
                        return response;
                    });
                };
                
                // 拦截 XMLHttpRequest
                const originalOpen = XMLHttpRequest.prototype.open;
                const originalSend = XMLHttpRequest.prototype.send;
                
                XMLHttpRequest.prototype.open = function(method, url, ...rest) {
                    this._url = url;
                    console.log('[API] XHR open:', url);
                    return originalOpen.apply(this, [method, url, ...rest]);
                };
                
                XMLHttpRequest.prototype.send = function(...args) {
                    this.addEventListener('load', function() {
                        const url = this._url;
                        
                        if (url.includes('/web/api/media/user/info')) {
                            try {
                                const data = JSON.parse(this.responseText);
                                window.__API_DATA__.userInfo = data;
                                console.log('[API] ✓ User info saved (XHR):', data);
                            } catch (e) {
                                console.error('[API] ✗ Error parsing user info (XHR):', e);
                            }
                        } else if (url.includes('/account/api/v1/user/account/info')) {
                            try {
                                const data = JSON.parse(this.responseText);
                                window.__API_DATA__.accountInfo = data;
                                console.log('[API] ✓ Account info saved (XHR):', data);
                            } catch (e) {
                                console.error('[API] ✗ Error parsing account info (XHR):', e);
                            }
                        }
                    });
                    
                    return originalSend.apply(this, args);
                };
                
                console.log('[API] ✓ Interceptor installed successfully');
                return { success: true };
            })()
        "#;

        if let Some(tab) = &self.tab {
            match tab.evaluate(interceptor_script, true) {
                Ok(_) => {
                    eprintln!("[DouyinBrowser] ✓ API interceptor injected successfully");
                },
                Err(e) => {
                    eprintln!("[DouyinBrowser] ✗ Failed to inject API interceptor: {}", e);
                }
            }
        }
    }

    /// 注入 CDC 提示浮层
    fn inject_tip_overlay(&self) {
        // 根据当前步骤生成不同的提示信息
        let tip_message = match &self.result.step {
            BrowserAuthStep::WaitingForLogin => "请使用抖音App扫码登录<br>登录成功后页面将自动跳转",
            BrowserAuthStep::LoginDetected => "检测到登录成功，正在跳转...",
            BrowserAuthStep::NavigatingToUpload => "正在跳转到上传页面...",
            BrowserAuthStep::WaitingForUpload => "正在准备提取凭证...",
            BrowserAuthStep::ExtractingCredentials => "正在提取凭证，请稍候...",
            BrowserAuthStep::ClosingBrowser => "正在关闭浏览器...",
            BrowserAuthStep::Completed => "授权完成！",
            _ => "授权助手正在运行",
        };

        let status_class = match &self.result.step {
            BrowserAuthStep::WaitingForLogin => "等待扫码登录...",
            BrowserAuthStep::ExtractingCredentials | BrowserAuthStep::WaitingForUpload => "正在加载凭证...",
            BrowserAuthStep::Completed => "授权成功！",
            _ => "处理中...",
        };

        let tip_script = format!(r#"
            (function() {{
                console.log('[AMM] Starting tip injection...');

                // 移除已存在的提示
                const existing = document.getElementById('amm-tip-overlay');
                if (existing) {{
                    console.log('[AMM] Removing existing tip overlay');
                    existing.remove();
                }}

                // 检查 body 是否存在
                if (!document.body) {{
                    console.log('[AMM] document.body does not exist yet');
                    return {{ success: false, error: 'body not exists' }};
                }}

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
                            {tip_message}
                        </span>
                        <div id="amm-login-status" style="
                            font-size: 12px !important;
                            padding: 6px 12px !important;
                            background: rgba(255,255,255,0.2) !important;
                            border-radius: 20px !important;
                            font-weight: 400 !important;
                            display: block !important;
                        ">{status_class}</div>
                    </div>
                    <style>
                        @keyframes amm-slide-down {{
                            from {{ opacity: 0 !important; transform: translateX(-50%) translateY(-30px) !important; }}
                            to {{ opacity: 1 !important; transform: translateX(-50%) translateY(0) !important; }}
                        }}
                        @keyframes amm-pulse {{
                            0%, 100% {{ opacity: 1 !important; }}
                            50% {{ opacity: 0.7 !important; }}
                        }}
                        #amm-login-status {{
                            animation: amm-pulse 2s infinite !important;
                        }}
                        #amm-tip-overlay {{
                            animation: amm-slide-down 0.4s ease-out !important;
                        }}
                    </style>
                `;

                // 使用 prepend 确保在 body 最前面
                document.body.insertBefore(tip, document.body.firstChild);

                console.log('[AMM] Tip overlay inserted successfully');

                // 验证 tip 是否存在
                const tipEl = document.getElementById('amm-tip-overlay');
                if (tipEl) {{
                    const style = window.getComputedStyle(tipEl);
                    return {{
                        success: true,
                        display: style.display,
                        visibility: style.visibility,
                        zIndex: style.zIndex
                    }};
                }}
                return {{ success: false, error: 'tip element not found after insert' }};
            }})();
        "#);

        if let Some(tab) = &self.tab {
            match tab.evaluate(&tip_script, true) {  // true = return_by_value
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

        // 获取初始标签页
        let browser_ref = self.browser.as_ref().expect("Browser should be Some");
        let initial_tab = browser_ref
            .new_tab()
            .map_err(|e| format!("创建标签页失败: {}", e))?;
        self.tab = Some(initial_tab);
        eprintln!("[DouyinBrowser] Created new tab");

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
        std::thread::sleep(std::time::Duration::from_secs(5));

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

        // 检查是否已经登录（页面可能已经自动重定向）
        eprintln!("[DouyinBrowser] Checking if already logged in...");

        // 等待可能的自动重定向，增加等待时间
        std::thread::sleep(std::time::Duration::from_secs(5));

        let updated_url = self.tab.as_ref().unwrap().get_url();
        eprintln!("[DouyinBrowser] URL after wait: {}", updated_url);

        // 检测是否已经在创作者中心
        if updated_url.contains("creator.douyin.com/creator-micro") {
            eprintln!("[DouyinBrowser] Already logged in! Detected creator-micro URL");

            // 重新注入提示浮层
            self.inject_tip_overlay();

            // 检测是否在首页
            let is_home_page = updated_url.contains("/creator-micro/home")
                || updated_url.ends_with("/creator-micro/");

            if is_home_page {
                // 在首页，先注入API拦截器，然后跳转到上传页面
                self.result.step = BrowserAuthStep::NavigatingToUpload;
                self.result.message = "检测到已登录，正在跳转到上传页面...".to_string();
                eprintln!("[DouyinBrowser] On home page, injecting API interceptor before navigation...");

                // 先注入API拦截器（在导航之前）
                self.inject_api_interceptor();

                // 立即跳转到上传页面
                match self.tab.as_ref().unwrap().navigate_to("https://creator.douyin.com/creator-micro/content/upload") {
                    Ok(_) => {
                        eprintln!("[DouyinBrowser] Navigation to upload page initiated");
                        if let Ok(_) = self.tab.as_ref().unwrap().wait_until_navigated() {
                            // 等待页面加载和API调用完成（增加等待时间）
                            eprintln!("[DouyinBrowser] Waiting for page to load and API calls to complete...");
                            std::thread::sleep(std::time::Duration::from_secs(6));
                            self.inject_tip_overlay();
                            let upload_url = self.tab.as_ref().unwrap().get_url();
                            eprintln!("[DouyinBrowser] Successfully navigated to upload page: {}", upload_url);
                            
                            // 立即开始提取凭证
                            eprintln!("[DouyinBrowser] Starting credential extraction immediately...");
                            self.result.step = BrowserAuthStep::ExtractingCredentials;
                            self.result.message = "正在提取凭证...".to_string();
                            self.result.current_url = upload_url;

                            // 立即提取凭证
                            let tab_for_extract = self.tab.clone().unwrap();
                            match self.extract_credentials(tab_for_extract.as_ref()).await {
                                Ok(_) => {
                                    eprintln!("[DouyinBrowser] Cookie: {} bytes", self.result.cookie.len());
                                    eprintln!("[DouyinBrowser] LocalStorage: {} bytes", self.result.local_storage.len());
                                    eprintln!("[DouyinBrowser] Nickname: {}", self.result.nickname);

                                    // 检查是否成功提取到凭证
                                    if self.result.cookie.is_empty() {
                                        // 凭证未完全加载，继续等待
                                        eprintln!("[DouyinBrowser] Credentials not ready, will retry in polling...");
                                        self.result.step = BrowserAuthStep::WaitingForUpload;
                                        self.result.message = "正在加载凭证，请稍候...".to_string();
                                        self.result.need_poll = true;
                                    } else {
                                        // 关闭浏览器
                                        self.result.step = BrowserAuthStep::ClosingBrowser;
                                        self.result.message = "正在关闭浏览器...".to_string();
                                        self.close_browser().await;

                                        self.result.step = BrowserAuthStep::Completed;
                                        self.result.message = format!("授权完成！账号: {}", self.result.nickname);
                                        self.result.need_poll = false;

                                        eprintln!("[DouyinBrowser] Authorization completed successfully in initialization");
                                        return Ok(self.result.clone());
                                    }
                                },
                                Err(e) => {
                                    eprintln!("[DouyinBrowser] Failed to extract credentials: {}", e);
                                    self.result.step = BrowserAuthStep::WaitingForUpload;
                                    self.result.message = "凭证提取失败，将在轮询中重试...".to_string();
                                    self.result.need_poll = true;
                                }
                            }
                        }
                    },
                    Err(e) => {
                        eprintln!("[DouyinBrowser] Failed to navigate to upload page: {}", e);
                        self.result.message = "跳转到上传页面失败，将在轮询中重试...".to_string();
                    }
                }
                self.result.need_poll = true;
            } else {
                // 已经在其他创作者中心页面（如上传页面），准备提取凭证
                self.result.step = BrowserAuthStep::WaitingForUpload;
                self.result.message = "检测到已登录，正在准备提取凭证...".to_string();
                self.result.need_poll = true;
                eprintln!("[DouyinBrowser] On creator page, will extract credentials on next poll");
            }
        } else if updated_url == "https://creator.douyin.com/" || updated_url == "https://creator.douyin.com" {
            // 在根路径，等待更长时间看是否会自动重定向
            eprintln!("[DouyinBrowser] On root path, waiting longer for potential auto-redirect...");
            
            // 等待更长时间，抖音可能需要更多时间来检查登录状态
            std::thread::sleep(std::time::Duration::from_secs(8));
            
            let final_check_url = self.tab.as_ref().unwrap().get_url();
            eprintln!("[DouyinBrowser] URL after extended wait: {}", final_check_url);
            
            if final_check_url.contains("creator.douyin.com/creator-micro") {
                // 自动重定向到了创作者中心
                eprintln!("[DouyinBrowser] Auto-redirected to creator-micro, user is logged in!");
                
                // 重新注入提示浮层
                self.inject_tip_overlay();
                
                // 检测是否在首页
                let is_home_page = final_check_url.contains("/creator-micro/home")
                    || final_check_url.ends_with("/creator-micro/");

                if is_home_page {
                    // 在首页，立即跳转到上传页面
                    self.result.step = BrowserAuthStep::NavigatingToUpload;
                    self.result.message = "检测到已登录，正在跳转到上传页面...".to_string();
                    eprintln!("[DouyinBrowser] On home page, navigating to upload page immediately...");

                    // 立即跳转到上传页面
                    match self.tab.as_ref().unwrap().navigate_to("https://creator.douyin.com/creator-micro/content/upload") {
                        Ok(_) => {
                            eprintln!("[DouyinBrowser] Navigation to upload page initiated");
                            if let Ok(_) = self.tab.as_ref().unwrap().wait_until_navigated() {
                                std::thread::sleep(std::time::Duration::from_secs(3));
                                self.inject_tip_overlay();
                                let upload_url = self.tab.as_ref().unwrap().get_url();
                                eprintln!("[DouyinBrowser] Successfully navigated to upload page: {}", upload_url);
                                self.result.step = BrowserAuthStep::WaitingForUpload;
                                self.result.message = "已跳转到上传页面，正在准备提取凭证...".to_string();
                                self.result.current_url = upload_url;
                            }
                        },
                        Err(e) => {
                            eprintln!("[DouyinBrowser] Failed to navigate to upload page: {}", e);
                            self.result.message = "跳转到上传页面失败，将在轮询中重试...".to_string();
                        }
                    }
                    self.result.need_poll = true;
                } else {
                    // 已经在其他创作者中心页面
                    self.result.step = BrowserAuthStep::WaitingForUpload;
                    self.result.message = "检测到已登录，正在准备提取凭证...".to_string();
                    self.result.need_poll = true;
                }
            } else {
                // 仍在根路径，设置为等待登录状态，让轮询来处理后续检测
                eprintln!("[DouyinBrowser] Still on root path, setting up for polling...");
                self.result.step = BrowserAuthStep::WaitingForLogin;
                self.result.message = "请扫码登录，页面将自动跳转".to_string();
                self.result.need_poll = true;
            }
        } else {
            // 未登录，等待扫码
            self.result.step = BrowserAuthStep::WaitingForLogin;
            self.result.message = "请扫码登录，页面将自动跳转".to_string();
            self.result.need_poll = true;
            eprintln!("[DouyinBrowser] Not logged in, waiting for scan");
        }

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

        // 在所有页面上注入 CDC 提示信息
        self.inject_tip_overlay();

        // 检测是否在登录页面
        let is_on_login_page = current_url.contains("passport.douyin.com")
            || current_url.contains("douyin.com/login")
            || current_url.contains("login.douyin.com");

        if is_on_login_page {
            // 仍在登录页面
            self.result.step = BrowserAuthStep::WaitingForLogin;
            self.result.message = "请扫码登录...".to_string();
            self.result.need_poll = true;
            return Ok(true);
        }

        // 检测是否在抖音创作者中心域名下（登录成功后会跳转到这个域名）
        let is_on_creator_domain = current_url.contains("creator.douyin.com/creator-micro");
        eprintln!("[DouyinBrowser] Is on creator domain: {}", is_on_creator_domain);

        if is_on_creator_domain {
            // 已经在创作者中心，说明登录成功了
            eprintln!("[DouyinBrowser] Detected login success!");

            // 检测是否在首页
            let is_home_page = current_url.contains("/creator-micro/home")
                || current_url.ends_with("/creator-micro/");

            eprintln!("[DouyinBrowser] Is on home page: {}", is_home_page);

            if is_home_page {
                // 在首页，先注入API拦截器，然后跳转到上传页面
                self.result.step = BrowserAuthStep::NavigatingToUpload;
                self.result.message = "正在跳转到上传页面...".to_string();
                eprintln!("[DouyinBrowser] Injecting API interceptor before navigation...");
                
                // 先注入API拦截器
                self.inject_api_interceptor();

                eprintln!("[DouyinBrowser] Navigating to upload page...");
                match tab.navigate_to("https://creator.douyin.com/creator-micro/content/upload") {
                    Ok(_) => {
                        eprintln!("[DouyinBrowser] Navigation initiated, waiting...");
                        
                        if let Ok(_) = tab.wait_until_navigated() {
                            // 等待页面加载和API调用完成（增加等待时间）
                            eprintln!("[DouyinBrowser] Waiting for page to load and API calls to complete...");
                            std::thread::sleep(std::time::Duration::from_secs(6));

                            // 重新注入提示信息
                            self.inject_tip_overlay();

                            let upload_url = tab.get_url();
                            eprintln!("[DouyinBrowser] Upload page URL: {}", upload_url);

                            // 检查是否成功跳转到上传页面
                            if upload_url.contains("/creator-micro/content/upload") {
                                // 成功跳转到上传页面，立即开始提取凭证
                                eprintln!("[DouyinBrowser] Successfully navigated to upload page, starting credential extraction...");
                                
                                self.result.step = BrowserAuthStep::ExtractingCredentials;
                                self.result.message = "正在提取凭证...".to_string();
                                self.result.current_url = upload_url;

                                // 立即提取凭证
                                let tab_for_extract = self.tab.clone().unwrap();
                                self.extract_credentials(tab_for_extract.as_ref()).await?;

                                eprintln!("[DouyinBrowser] Cookie: {} bytes", self.result.cookie.len());
                                eprintln!("[DouyinBrowser] LocalStorage: {} bytes", self.result.local_storage.len());
                                eprintln!("[DouyinBrowser] Nickname: {}", self.result.nickname);

                                // 检查是否成功提取到凭证
                                if self.result.cookie.is_empty() {
                                    // 凭证未完全加载，继续等待
                                    eprintln!("[DouyinBrowser] Credentials not ready, waiting...");
                                    self.result.step = BrowserAuthStep::WaitingForUpload;
                                    self.result.message = "正在加载凭证，请稍候...".to_string();
                                    self.result.need_poll = true;
                                    return Ok(true);
                                }

                                // 关闭浏览器
                                self.result.step = BrowserAuthStep::ClosingBrowser;
                                self.result.message = "正在关闭浏览器...".to_string();
                                self.close_browser().await;

                                self.result.step = BrowserAuthStep::Completed;
                                self.result.message = format!("授权完成！账号: {}", self.result.nickname);
                                self.result.need_poll = false;

                                eprintln!("[DouyinBrowser] Authorization completed successfully");
                                return Ok(false);
                            } else {
                                // 跳转可能失败，继续轮询
                                eprintln!("[DouyinBrowser] Upload page navigation may have failed, URL: {}", upload_url);
                                self.result.step = BrowserAuthStep::WaitingForUpload;
                                self.result.message = "正在准备提取凭证...".to_string();
                                self.result.need_poll = true;
                                return Ok(true);
                            }
                        } else {
                            eprintln!("[DouyinBrowser] Navigation wait failed");
                        }
                        
                        // 导航可能失败，继续轮询
                        self.result.message = "正在跳转到上传页面，请稍候...".to_string();
                        self.result.need_poll = true;
                        return Ok(true);
                    },
                    Err(e) => {
                        eprintln!("[DouyinBrowser] Navigation failed: {}", e);
                        self.result.message = "跳转失败，正在重试...".to_string();
                        self.result.need_poll = true;
                        return Ok(true);
                    }
                }
            }

            // 在上传页面或其他创作者中心页面，直接提取凭证
            eprintln!("[DouyinBrowser] On creator micro page, extracting credentials...");

            self.result.step = BrowserAuthStep::ExtractingCredentials;
            self.result.message = "正在提取凭证...".to_string();

            // 克隆 tab 引用以解决生命周期问题
            let tab_for_extract = self.tab.clone().unwrap();
            self.extract_credentials(tab_for_extract.as_ref()).await?;

            eprintln!("[DouyinBrowser] Cookie: {} bytes", self.result.cookie.len());
            eprintln!("[DouyinBrowser] LocalStorage: {} bytes", self.result.local_storage.len());
            eprintln!("[DouyinBrowser] Nickname: {}", self.result.nickname);

            // 检查是否成功提取到凭证
            if self.result.cookie.is_empty() {
                // 凭证未完全加载，继续等待
                eprintln!("[DouyinBrowser] Credentials not ready, waiting...");
                self.result.message = "正在加载凭证，请稍候...".to_string();
                self.result.need_poll = true;
                return Ok(true);
            }

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

        // 检测是否是根路径
        if current_url == "https://creator.douyin.com/" || current_url == "https://creator.douyin.com" {
            eprintln!("[DouyinBrowser] On root path, checking for login status...");

            // 等待页面加载和可能的重定向
            std::thread::sleep(std::time::Duration::from_secs(3));

            // 检查是否有自动重定向
            let new_url = tab.get_url();
            eprintln!("[DouyinBrowser] After wait, URL: {}", new_url);

            if new_url.contains("creator.douyin.com/creator-micro") {
                // 自动重定向到了创作者中心，说明已登录
                eprintln!("[DouyinBrowser] Auto-redirected to creator-micro, user is logged in!");
                
                // 重新注入提示
                self.inject_tip_overlay();
                
                self.result.step = BrowserAuthStep::NavigatingToUpload;
                self.result.message = "检测到已登录，正在跳转到上传页面...".to_string();
                self.result.need_poll = true;
                return Ok(true);
            }

            if new_url == current_url {
                // URL 没有变化，尝试主动检测登录状态
                eprintln!("[DouyinBrowser] No auto-redirect, checking login status by navigating to home...");

                // 尝试导航到创作者中心首页
                match tab.navigate_to("https://creator.douyin.com/creator-micro/home") {
                    Ok(_) => {
                        // 等待导航完成
                        if let Ok(_) = tab.wait_until_navigated() {
                            std::thread::sleep(std::time::Duration::from_secs(3));

                            let final_url = tab.get_url();
                            eprintln!("[DouyinBrowser] Final URL after navigation: {}", final_url);

                            // 重新注入提示
                            self.inject_tip_overlay();

                            // 检查是否成功到达创作者中心
                            if final_url.contains("creator.douyin.com/creator-micro") {
                                eprintln!("[DouyinBrowser] Successfully navigated to creator-micro, user is logged in!");
                                self.result.step = BrowserAuthStep::NavigatingToUpload;
                                self.result.message = "检测到已登录，正在跳转到上传页面...".to_string();
                                self.result.need_poll = true;
                                return Ok(true);
                            } else if final_url.contains("passport.douyin.com") || final_url.contains("login") {
                                eprintln!("[DouyinBrowser] Redirected to login page, user is not logged in");
                                self.result.step = BrowserAuthStep::WaitingForLogin;
                                self.result.message = "请扫码登录，页面将自动跳转".to_string();
                                self.result.need_poll = true;
                                return Ok(true);
                            }
                        }
                    },
                    Err(e) => {
                        eprintln!("[DouyinBrowser] Navigation failed: {}", e);
                    }
                }
            }
        }

        // 其他情况，继续等待登录
        self.result.step = BrowserAuthStep::WaitingForLogin;
        self.result.message = "请扫码登录...".to_string();
        self.result.need_poll = true;
        Ok(true)
    }


    /// 提取 Cookie 和 LocalStorage
    async fn extract_credentials(&mut self, tab: &headless_chrome::Tab) -> Result<(), String> {
        eprintln!("[DouyinBrowser] Starting credential extraction...");

        // API拦截器已经在导航前注入，这里只需要等待并读取数据
        eprintln!("[DouyinBrowser] Waiting for API calls to complete...");
        std::thread::sleep(std::time::Duration::from_secs(2));

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

        // 提取 LocalStorage - 需要提取特定的 security-sdk 相关项
        eprintln!("[DouyinBrowser] Extracting localStorage...");
        let ls_keys = vec![
            "security-sdk/s_sdk_cert_key",
            "security-sdk/s_sdk_crypt_sdk",
            "security-sdk/s_sdk_pri_key",
            "security-sdk/s_sdk_pub_key",
            "security-sdk/s_sdk_server_cert_key",
            "security-sdk/s_sdk_sign_data_key/token",
            "security-sdk/s_sdk_sign_data_key/web_protect",
        ];

        let mut local_data = Vec::new();
        for key in ls_keys {
            let ls_script = format!(r#"
                (function() {{
                    try {{
                        const value = localStorage.getItem('{}');
                        if (value) {{
                            return JSON.stringify({{
                                key: '{}',
                                value: value
                            }});
                        }}
                        return null;
                    }} catch (e) {{
                        return null;
                    }}
                }})()
            "#, key, key);

            if let Ok(ls_result) = tab.evaluate(&ls_script, true) {
                if let Some(ls_value) = ls_result.value {
                    if let Some(s) = ls_value.as_str() {
                        if s != "null" && !s.is_empty() {
                            local_data.push(s.to_string());
                        }
                    }
                }
            }
        }

        // 将 local_data 数组转换为 JSON 字符串
        let local_data_json = format!("[{}]", local_data.join(","));
        self.result.local_storage = local_data_json;
        eprintln!("[DouyinBrowser] LocalStorage extracted: {} items", local_data.len());

        // 从拦截的 API 数据中提取用户信息
        eprintln!("[DouyinBrowser] Extracting user info from intercepted API...");
        let api_data_script = r#"
            (function() {
                const data = window.__API_DATA__ || {};
                console.log('[API] Checking intercepted data:', data);
                return JSON.stringify(data);
            })()
        "#;

        if let Ok(api_result) = tab.evaluate(api_data_script, true) {
            if let Some(api_value) = api_result.value {
                if let Some(s) = api_value.as_str() {
                    eprintln!("[DouyinBrowser] API data string: {}", s);
                    
                    if let Ok(api_data) = serde_json::from_str::<serde_json::Value>(s) {
                        eprintln!("[DouyinBrowser] API data parsed successfully");
                        
                        // 提取用户信息
                        if let Some(user_info) = api_data.get("userInfo") {
                            if !user_info.is_null() {
                                eprintln!("[DouyinBrowser] Found userInfo in API data");
                                
                                if let Some(user) = user_info.get("user") {
                                    eprintln!("[DouyinBrowser] Found user object");
                                    
                                    if let Some(nickname) = user.get("nickname").and_then(|v| v.as_str()) {
                                        self.result.nickname = nickname.to_string();
                                        eprintln!("[DouyinBrowser] ✓ Nickname from API: {}", nickname);
                                    } else {
                                        eprintln!("[DouyinBrowser] No nickname in user object");
                                    }
                                    
                                    if let Some(avatar) = user.get("avatar_thumb")
                                        .and_then(|v| v.get("url_list"))
                                        .and_then(|v| v.get(0))
                                        .and_then(|v| v.as_str()) {
                                        self.result.avatar_url = avatar.to_string();
                                        eprintln!("[DouyinBrowser] ✓ Avatar from API");
                                    }
                                } else {
                                    eprintln!("[DouyinBrowser] No user object in userInfo");
                                }
                            } else {
                                eprintln!("[DouyinBrowser] userInfo is null");
                            }
                        } else {
                            eprintln!("[DouyinBrowser] No userInfo in API data");
                        }
                        
                        // 也检查 accountInfo
                        if let Some(account_info) = api_data.get("accountInfo") {
                            if !account_info.is_null() {
                                eprintln!("[DouyinBrowser] Found accountInfo in API data");
                                
                                // 如果userInfo没有昵称，尝试从accountInfo获取
                                if self.result.nickname.is_empty() {
                                    if let Some(nickname) = account_info.get("nickname").and_then(|v| v.as_str()) {
                                        self.result.nickname = nickname.to_string();
                                        eprintln!("[DouyinBrowser] ✓ Nickname from accountInfo: {}", nickname);
                                    }
                                }
                            }
                        }
                    } else {
                        eprintln!("[DouyinBrowser] Failed to parse API data JSON");
                    }
                } else {
                    eprintln!("[DouyinBrowser] API data is not a string");
                }
            } else {
                eprintln!("[DouyinBrowser] API result has no value");
            }
        } else {
            eprintln!("[DouyinBrowser] Failed to evaluate API data script");
        }

        // 如果没有从API获取到昵称，尝试从页面元素中获取
        if self.result.nickname.is_empty() {
            eprintln!("[DouyinBrowser] Extracting user info from page...");
            let user_info_script = r#"
                (function() {
                    try {
                        // 尝试从页面中获取用户信息
                        const avatarImg = document.querySelector('img[class*="avatar"]');
                        const nicknameEl = document.querySelector('[class*="nickname"]') || 
                                          document.querySelector('[class*="user-name"]');
                        
                        return JSON.stringify({
                            nickname: nicknameEl ? nicknameEl.textContent.trim() : '',
                            avatar: avatarImg ? avatarImg.src : ''
                        });
                    } catch (e) {
                        return JSON.stringify({nickname: '', avatar: ''});
                    }
                })()
            "#;

            if let Ok(user_result) = tab.evaluate(user_info_script, true) {
                if let Some(user_value) = user_result.value {
                    if let Some(s) = user_value.as_str() {
                        if let Ok(user_data) = serde_json::from_str::<serde_json::Value>(s) {
                            if let Some(nickname) = user_data.get("nickname").and_then(|v| v.as_str()) {
                                if !nickname.is_empty() {
                                    self.result.nickname = nickname.to_string();
                                    eprintln!("[DouyinBrowser] Nickname extracted from page: {}", nickname);
                                }
                            }
                            if let Some(avatar) = user_data.get("avatar").and_then(|v| v.as_str()) {
                                if !avatar.is_empty() && self.result.avatar_url.is_empty() {
                                    self.result.avatar_url = avatar.to_string();
                                    eprintln!("[DouyinBrowser] Avatar extracted from page");
                                }
                            }
                        }
                    }
                }
            }
        }

        if self.result.nickname.is_empty() {
            self.result.nickname = "抖音用户".to_string();
            eprintln!("[DouyinBrowser] Using default nickname");
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
