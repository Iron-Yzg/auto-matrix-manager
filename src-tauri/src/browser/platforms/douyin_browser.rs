// Douyin Browser Implementation - 重写版本
// 参考 Python douyin_account_bind.py 实现

use crate::browser::{BrowserAuthResult, BrowserAuthStep};
use headless_chrome::{Browser, LaunchOptions, Tab};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use serde::{Deserialize, Serialize};

/// API 响应数据
#[derive(Debug, Clone, Default)]
struct ApiResponseData {
    user_info: Option<serde_json::Value>,
    account_info: Option<serde_json::Value>,
}

/// 抖音浏览器实现 - 重写版本
pub struct DouyinBrowser {
    browser: Option<Browser>,
    tab: Option<Arc<Tab>>,
    result: BrowserAuthResult,
    /// 自定义 Chrome 浏览器路径
    chrome_path: Option<PathBuf>,
    /// 等待超时时间（秒）
    timeout_seconds: u32,
}

impl DouyinBrowser {
    pub fn new() -> Self {
        Self {
            browser: None,
            tab: None,
            result: BrowserAuthResult::default(),
            chrome_path: None,
            timeout_seconds: 120,
        }
    }

    /// 设置 Chrome 浏览器路径
    pub fn with_chrome_path(path: PathBuf) -> Self {
        Self {
            browser: None,
            tab: None,
            result: BrowserAuthResult::default(),
            chrome_path: Some(path),
            timeout_seconds: 120,
        }
    }

    /// 设置超时时间
    pub fn with_timeout(mut self, seconds: u32) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    /// 启动浏览器
    fn launch_browser(&self) -> Result<Browser, String> {
        eprintln!("[DouyinBrowser] Launch options: path={:?}", self.chrome_path);

        let launch_options = LaunchOptions::default_builder()
            .headless(false)
            .window_size(Some((1280, 800)))
            .path(self.chrome_path.clone())
            .build()
            .map_err(|e| format!("构建启动选项失败: {}", e))?;

        Browser::new(launch_options)
            .map_err(|e| format!("启动浏览器失败: {}", e))
    }

    /// 等待 URL 变化到指定模式
    fn wait_for_url(&self, tab: &Tab, pattern: &str, timeout_secs: u32) -> bool {
        eprintln!("[DouyinBrowser] Waiting for URL pattern: {}", pattern);
        let start = std::time::Instant::now();

        while start.elapsed().as_secs() < timeout_secs as u64 {
            let url = tab.get_url();
            eprintln!("[DouyinBrowser] Current URL: {}", url);

            if url.contains(pattern) {
                eprintln!("[DouyinBrowser] ✓ URL matched: {}", pattern);
                return true;
            }

            std::thread::sleep(Duration::from_millis(500));
        }

        eprintln!("[DouyinBrowser] ✗ URL pattern not found within timeout: {}", pattern);
        false
    }

    /// 注入 API 拦截器
    fn inject_api_interceptor(&self, tab: &Tab) -> bool {
        eprintln!("[DouyinBrowser] Injecting API interceptor...");

        let interceptor_script = r#"
            (function() {
                // 初始化数据存储
                window.__API_DATA__ = {
                    userInfo: null,
                    accountInfo: null,
                    interceptCount: 0
                };

                console.log('[API Interceptor] Installing...');

                // 拦截 fetch
                const originalFetch = window.fetch;
                window.fetch = function(...args) {
                    const url = args[0];
                    const method = args[1]?.method || 'GET';

                    console.log('[API Interceptor] Fetch:', method, url);

                    return originalFetch.apply(this, args).then(response => {
                        const clonedResponse = response.clone();

                        // 拦截用户信息 API
                        if (url.includes('/web/api/media/user/info')) {
                            console.log('[API Interceptor] ✓ Intercepted user info API');
                            window.__API_DATA__.interceptCount++;

                            clonedResponse.json().then(data => {
                                window.__API_DATA__.userInfo = data;
                                console.log('[API Interceptor] ✓ User info saved, count:', window.__API_DATA__.interceptCount);
                                console.log('[API Interceptor] User data:', JSON.stringify(data).substring(0, 200));
                            }).catch(e => console.error('[API Interceptor] ✗ Error parsing user info:', e));
                        }
                        // 拦截账号信息 API
                        else if (url.includes('/account/api/v1/user/account/info')) {
                            console.log('[API Interceptor] ✓ Intercepted account info API');
                            window.__API_DATA__.interceptCount++;

                            clonedResponse.json().then(data => {
                                window.__API_DATA__.accountInfo = data;
                                console.log('[API Interceptor] ✓ Account info saved, count:', window.__API_DATA__.interceptCount);
                                console.log('[API Interceptor] Account data:', JSON.stringify(data).substring(0, 200));
                            }).catch(e => console.error('[API Interceptor] ✗ Error parsing account info:', e));
                        }

                        return response;
                    });
                };

                // 拦截 XMLHttpRequest
                const originalXHROpen = XMLHttpRequest.prototype.open;
                const originalXHRSend = XMLHttpRequest.prototype.send;

                XMLHttpRequest.prototype.open = function(method, url, ...rest) {
                    this._url = url;
                    this._method = method;
                    console.log('[API Interceptor] XHR:', method, url);
                    return originalXHROpen.apply(this, [method, url, ...rest]);
                };

                XMLHttpRequest.prototype.send = function(...args) {
                    this.addEventListener('load', () => {
                        const url = this._url;

                        if (url.includes('/web/api/media/user/info')) {
                            try {
                                const data = JSON.parse(this.responseText);
                                window.__API_DATA__.userInfo = data;
                                window.__API_DATA__.interceptCount++;
                                console.log('[API Interceptor] ✓ User info saved (XHR), count:', window.__API_DATA__.interceptCount);
                            } catch (e) {
                                console.error('[API Interceptor] ✗ Error parsing user info (XHR):', e);
                            }
                        } else if (url.includes('/account/api/v1/user/account/info')) {
                            try {
                                const data = JSON.parse(this.responseText);
                                window.__API_DATA__.accountInfo = data;
                                window.__API_DATA__.interceptCount++;
                                console.log('[API Interceptor] ✓ Account info saved (XHR), count:', window.__API_DATA__.interceptCount);
                            } catch (e) {
                                console.error('[API Interceptor] ✗ Error parsing account info (XHR):', e);
                            }
                        }
                    });

                    return originalXHRSend.apply(this, args);
                };

                console.log('[API Interceptor] ✓ Installed successfully');
                return { success: true, count: window.__API_DATA__.interceptCount };
            })()
        "#;

        match tab.evaluate(interceptor_script, true) {
            Ok(result) => {
                eprintln!("[DouyinBrowser] API interceptor injected result: {:?}", result);
                true
            }
            Err(e) => {
                eprintln!("[DouyinBrowser] ✗ Failed to inject API interceptor: {}", e);
                false
            }
        }
    }

    /// 等待 API 数据被拦截
    fn wait_for_api_data(&self, tab: &Tab, expected_count: u32, timeout_secs: u32) -> bool {
        eprintln!("[DouyinBrowser] Waiting for API data (expected count: {})...", expected_count);

        let start = std::time::Instant::now();

        while start.elapsed().as_secs() < timeout_secs as u64 {
            let check_script = r#"
                (function() {
                    const data = window.__API_DATA__ || {interceptCount: 0};
                    return JSON.stringify({
                        count: data.interceptCount,
                        hasUserInfo: !!data.userInfo,
                        hasAccountInfo: !!data.accountInfo
                    });
                })()
            "#;

            match tab.evaluate(check_script, true) {
                Ok(result) => {
                    if let Some(value) = result.value {
                        if let Some(s) = value.as_str() {
                            if let Ok(data) = serde_json::from_str::<serde_json::Value>(s) {
                                let count = data.get("count").and_then(|v| v.as_u64()).unwrap_or(0);
                                let has_user = data.get("hasUserInfo").and_then(|v| v.as_bool()).unwrap_or(false);
                                let has_account = data.get("hasAccountInfo").and_then(|v| v.as_bool()).unwrap_or(false);

                                eprintln!("[DouyinBrowser] API data status: count={}, hasUser={}, hasAccount={}",
                                    count, has_user, has_account);

                                if count >= expected_count as u64 || (has_user && has_account) {
                                    eprintln!("[DouyinBrowser] ✓ API data received!");
                                    return true;
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[DouyinBrowser] Error checking API data: {}", e);
                }
            }

            std::thread::sleep(Duration::from_millis(500));
        }

        eprintln!("[DouyinBrowser] ✗ API data not received within timeout");
        false
    }

    /// 获取 API 拦截的数据
    fn get_api_data(&self, tab: &Tab) -> ApiResponseData {
        let script = r#"
            (function() {
                return JSON.stringify(window.__API_DATA__ || {
                    userInfo: null,
                    accountInfo: null
                });
            })()
        "#;

        let mut result = ApiResponseData::default();

        if let Ok(response) = tab.evaluate(script, true) {
            if let Some(value) = response.value {
                if let Some(s) = value.as_str() {
                    eprintln!("[DouyinBrowser] API data raw: {}", s);

                    if let Ok(data) = serde_json::from_str::<serde_json::Value>(s) {
                        result.user_info = data.get("userInfo").cloned();
                        result.account_info = data.get("accountInfo").cloned();
                    }
                }
            }
        }

        result
    }

    /// 获取 localStorage 数据
    fn get_local_storage(&self, tab: &Tab) -> Vec<LocalStorageItem> {
        let keys = vec![
            "security-sdk/s_sdk_cert_key",
            "security-sdk/s_sdk_crypt_sdk",
            "security-sdk/s_sdk_pri_key",
            "security-sdk/s_sdk_pub_key",
            "security-sdk/s_sdk_server_cert_key",
            "security-sdk/s_sdk_sign_data_key/token",
            "security-sdk/s_sdk_sign_data_key/web_protect",
        ];

        let mut items = Vec::new();

        for key in &keys {
            let script = format!(r#"
                (function() {{
                    try {{
                        const value = localStorage.getItem('{}');
                        if (value) {{
                            // security-sdk/s_sdk_cert_key 需要检查是否包含 pub.
                            if ('{}'.includes('s_sdk_cert_key') && !value.includes('pub.')) {{
                                return null;
                            }}
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
            "#, key, key, key);

            match tab.evaluate(&script, true) {
                Ok(result) => {
                    if let Some(value) = result.value {
                        if let Some(s) = value.as_str() {
                            if let Ok(item) = serde_json::from_str::<LocalStorageItem>(s) {
                                items.push(item);
                                eprintln!("[DouyinBrowser] Got localStorage: {}", key);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[DouyinBrowser] Error getting localStorage {}: {}", key, e);
                }
            }
        }

        items
    }

    /// 获取 cookies
    fn get_cookies(&self, tab: &Tab) -> String {
        match tab.get_cookies() {
            Ok(cookies) => {
                let parts: Vec<String> = cookies.iter()
                    .map(|c| format!("{}={}", c.name, c.value))
                    .collect();
                let cookie_str = parts.join("; ");
                eprintln!("[DouyinBrowser] Got {} cookies", cookies.len());
                cookie_str
            }
            Err(e) => {
                eprintln!("[DouyinBrowser] Error getting cookies: {}", e);
                String::new()
            }
        }
    }

    /// 从 API 数据中提取用户信息
    fn extract_user_info(&self, api_data: &ApiResponseData) -> (String, String, String) {
        let mut uid = String::new();
        let mut nickname = String::new();
        let mut avatar_url = String::new();

        // 从 user_info 中提取
        if let Some(user_info) = &api_data.user_info {
            eprintln!("[DouyinBrowser] Parsing user_info: {}", user_info);

            // 获取 uid
            if let Some(user) = user_info.get("user") {
                if let Some(u) = user.get("uid").and_then(|v| v.as_str()) {
                    uid = u.to_string();
                }
                // 获取昵称
                if let Some(nick) = user.get("nickname").and_then(|v| v.as_str()) {
                    nickname = nick.to_string();
                }
                // 获取头像
                if let Some(avatar) = user.get("avatar_thumb")
                    .and_then(|v| v.get("url_list"))
                    .and_then(|v| v.as_array())
                    .and_then(|v| v.first())
                    .and_then(|v| v.as_str())
                {
                    avatar_url = avatar.to_string();
                }
            }
        }

        // 从 account_info 中补充信息
        if let Some(account_info) = &api_data.account_info {
            eprintln!("[DouyinBrowser] Parsing account_info: {}", account_info);

            // 如果没有昵称，尝试从 account_info 获取
            if nickname.is_empty() {
                if let Some(display_name) = account_info.get("display_name")
                    .or_else(|| account_info.get("nickname"))
                    .and_then(|v| v.as_str())
                {
                    nickname = display_name.to_string();
                }
            }
        }

        if nickname.is_empty() {
            nickname = "抖音用户".to_string();
        }

        (uid, nickname, avatar_url)
    }

    /// 注入提示浮层
    fn inject_tip_overlay(&self, tab: &Tab, message: &str) {
        let tip_script = format!(r#"
            (function() {{
                const existing = document.getElementById('amm-tip-overlay');
                if (existing) {{
                    existing.remove();
                }}

                if (!document.body) {{
                    return {{ success: false, error: 'body not exists' }};
                }}

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
                            <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                                <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" fill="none" stroke="currentColor"></path>
                                <path d="M12 8v4" fill="none" stroke="currentColor"></path>
                                <path d="M12 16h.01" fill="none" stroke="currentColor"></path>
                            </svg>
                            <span style="display: block !important;">授权助手</span>
                        </div>
                        <span style="text-align: center !important; opacity: 0.95 !important; font-weight: 400 !important; line-height: 1.5 !important; display: block !important;">
                            {}
                        </span>
                    </div>
                `;

                document.body.insertBefore(tip, document.body.firstChild);
                console.log('[AMM] Tip overlay inserted');

                return {{ success: true }};
            }})();
        "#, message);

        if let Err(e) = tab.evaluate(&tip_script, true) {
            eprintln!("[DouyinBrowser] Failed to inject tip overlay: {}", e);
        }
    }

    /// 移除提示浮层
    fn remove_tip_overlay(&self, tab: &Tab) {
        let script = r#"
            (function() {
                const existing = document.getElementById('amm-tip-overlay');
                if (existing) {
                    existing.remove();
                    return { success: true };
                }
                return { success: false, notFound: true };
            })()
        "#;

        if let Err(e) = tab.evaluate(script, true) {
            eprintln!("[DouyinBrowser] Failed to remove tip overlay: {}", e);
        }
    }

    /// 关闭浏览器
    fn close_browser(&mut self) {
        eprintln!("[DouyinBrowser] Closing browser...");
        self.tab = None;
        self.browser = None;
        eprintln!("[DouyinBrowser] Browser closed");
    }

    /// 启动抖音授权流程 - 重写版本
    pub async fn start_authorize(&mut self) -> Result<BrowserAuthResult, String> {
        self.result.step = BrowserAuthStep::LaunchingBrowser;
        self.result.message = "正在启动浏览器...".to_string();
        eprintln!("[DouyinBrowser] ===== Starting authorization =====");

        // 启动浏览器
        self.browser = match self.launch_browser() {
            Ok(b) => {
                eprintln!("[DouyinBrowser] Browser launched successfully");
                Some(b)
            }
            Err(e) => {
                let err_msg = format!("无法启动浏览器: {}", e);
                eprintln!("[DouyinBrowser] ✗ {}", err_msg);
                self.result.step = BrowserAuthStep::Failed(err_msg.clone());
                self.result.message = err_msg.clone();
                self.result.error = Some(err_msg.clone());
                return Err(err_msg);
            }
        };

        // 创建新标签页
        let browser_ref = self.browser.as_ref().expect("Browser should exist");
        let initial_tab = browser_ref
            .new_tab()
            .map_err(|e| format!("创建标签页失败: {}", e))?;
        self.tab = Some(initial_tab);
        let tab = self.tab.as_ref().unwrap();

        eprintln!("[DouyinBrowser] Tab created");

        // 步骤1: 导航到创作者中心
        self.result.step = BrowserAuthStep::OpeningLoginPage;
        self.result.message = "正在打开创作者中心...".to_string();
        tab.navigate_to("https://creator.douyin.com/")
            .map_err(|e| format!("导航失败: {}", e))?;
        tab.wait_until_navigated()
            .map_err(|e| format!("等待导航失败: {}", e))?;

        eprintln!("[DouyinBrowser] Navigated to creator.douyin.com");
        std::thread::sleep(Duration::from_secs(2));

        // 注入提示
        self.inject_tip_overlay(tab, "请使用抖音App扫码登录，登录成功后页面将自动跳转");

        // 步骤2: 等待登录并跳转到创作者中心首页
        self.result.step = BrowserAuthStep::WaitingForLogin;
        self.result.message = "请扫码登录...".to_string();

        eprintln!("[DouyinBrowser] Waiting for login and redirect to home page...");

        // 等待 URL 变化到 creator-micro/home
        if !self.wait_for_url(tab, "/creator-micro/home", self.timeout_seconds) {
            // 超时，尝试手动检查
            let current_url = tab.get_url();
            if !current_url.contains("creator.douyin.com/creator-micro") {
                let err_msg = "等待登录超时，请确保扫码登录".to_string();
                eprintln!("[DouyinBrowser] ✗ {}", err_msg);
                self.result.step = BrowserAuthStep::Failed(err_msg.clone());
                self.result.message = err_msg.clone();
                self.result.error = Some(err_msg.clone());
                self.close_browser();
                return Err(err_msg);
            }
        }

        // 更新 URL
        self.result.current_url = tab.get_url();
        eprintln!("[DouyinBrowser] Reached creator center: {}", self.result.current_url);

        // 步骤3: 注入 API 拦截器
        self.result.step = BrowserAuthStep::LoginDetected;
        self.result.message = "登录成功，正在准备提取数据...".to_string();
        self.inject_tip_overlay(tab, "登录成功，正在准备提取数据...");

        eprintln!("[DouyinBrowser] Injecting API interceptor...");
        self.inject_api_interceptor(tab);

        // 等待一下让拦截器生效
        std::thread::sleep(Duration::from_secs(1));

        // 步骤4: 导航到上传页面触发 API 调用
        self.result.step = BrowserAuthStep::NavigatingToUpload;
        self.result.message = "正在跳转到上传页面以获取API数据...".to_string();
        self.inject_tip_overlay(tab, "正在获取用户数据，请稍候...");

        eprintln!("[DouyinBrowser] Navigating to upload page to trigger API calls...");

        // 导航到上传页面
        tab.navigate_to("https://creator.douyin.com/creator-micro/content/upload")
            .map_err(|e| format!("导航到上传页面失败: {}", e))?;
        tab.wait_until_navigated()
            .map_err(|e| format!("等待导航失败: {}", e))?;

        eprintln!("[DouyinBrowser] Navigated to upload page");

        // 等待 API 数据被拦截
        self.result.step = BrowserAuthStep::ExtractingCredentials;
        self.result.message = "正在提取用户数据...".to_string();
        self.inject_tip_overlay(tab, "正在提取用户数据...");

        // 等待 API 调用完成（需要至少2个响应）
        if !self.wait_for_api_data(tab, 2, 30u32) {
            eprintln!("[DouyinBrowser] Warning: API data not fully received, but continuing...");
        }

        // 额外等待确保数据完整
        std::thread::sleep(Duration::from_secs(2));

        // 步骤5: 提取数据
        eprintln!("[DouyinBrowser] Extracting data...");

        // 获取 API 拦截的数据
        let api_data = self.get_api_data(tab);
        let (uid, nickname, avatar_url) = self.extract_user_info(&api_data);

        eprintln!("[DouyinBrowser] User info - uid: {}, nickname: {}", uid, nickname);

        // 获取 cookies
        let cookie = self.get_cookies(tab);

        // 获取 localStorage
        let local_storage_items = self.get_local_storage(tab);

        // 构建 third_param（参考 Python 版本）
        let third_param = ThirdParam {
            cookie: cookie.clone(),
            local_data: local_storage_items.clone(),
            // 这些从 API 请求头中获取，暂时使用空字符串
            accept: "application/json".to_string(),
            referer: "https://creator.douyin.com/creator-micro/content/post/video?enter_from=publish_page".to_string(),
            user_agent: "".to_string(),
            sec_ch_ua: "".to_string(),
            sec_fetch_dest: "".to_string(),
            sec_fetch_mode: "".to_string(),
            sec_fetch_site: "".to_string(),
            accept_encoding: "".to_string(),
            accept_language: "".to_string(),
            sec_ch_ua_mobile: "".to_string(),
            sec_ch_ua_platform: "".to_string(),
            x_secsdk_csrf_token: "".to_string(),
        };

        // 构建结果
        let _result_data = DouyinAuthResult {
            third_id: uid.parse().unwrap_or(0),
            third_param,
            third_return: api_data.user_info.clone().unwrap_or(serde_json::json!({})),
            all_cookies: cookie.clone(),
        };

        // 序列化 params（用于存储）
        let _params_json = serde_json::to_string(&DouyinAuthResult {
            third_id: uid.parse().unwrap_or(0),
            third_param: ThirdParam {
                cookie: cookie.clone(),
                local_data: local_storage_items.clone(),
                accept: "application/json".to_string(),
                referer: "https://creator.douyin.com/creator-micro/content/post/video?enter_from=publish_page".to_string(),
                user_agent: "".to_string(),
                sec_ch_ua: "".to_string(),
                sec_fetch_dest: "".to_string(),
                sec_fetch_mode: "".to_string(),
                sec_fetch_site: "".to_string(),
                accept_encoding: "".to_string(),
                accept_language: "".to_string(),
                sec_ch_ua_mobile: "".to_string(),
                sec_ch_ua_platform: "".to_string(),
                x_secsdk_csrf_token: "".to_string(),
            },
            third_return: api_data.user_info.clone().unwrap_or(serde_json::json!({})),
            all_cookies: cookie.clone(),
        }).map_err(|e| format!("序列化params失败: {}", e))?;

        // 更新结果
        self.result.cookie = cookie;
        self.result.local_storage = serde_json::to_string(&local_storage_items)
            .unwrap_or_default();
        self.result.nickname = nickname.clone();
        self.result.avatar_url = avatar_url;
        self.result.step = BrowserAuthStep::Completed;
        self.result.message = format!("授权完成！账号: {}", nickname);
        self.result.need_poll = false;

        eprintln!("[DouyinBrowser] ===== Authorization completed =====");
        eprintln!("[DouyinBrowser] Nickname: {}", nickname);
        eprintln!("[DouyinBrowser] UID: {}", uid);

        // 关闭浏览器
        self.close_browser();

        Ok(self.result.clone())
    }

    /// 检查登录状态并提取凭证（轮询模式）
    pub async fn check_and_extract(&mut self) -> Result<bool, String> {
        eprintln!("[DouyinBrowser] check_and_extract called");

        let tab = match self.tab.as_ref() {
            Some(t) => t.clone(),
            None => {
                eprintln!("[DouyinBrowser] Tab is None");
                return Ok(false);
            }
        };

        // 强制刷新页面获取最新 URL（使用 navigate_to 当前 URL 来刷新）
        let current_url_for_refresh = tab.get_url();
        let _ = tab.navigate_to(&current_url_for_refresh);
        std::thread::sleep(Duration::from_millis(500));

        let current_url = tab.get_url();
        self.result.current_url = current_url.clone();
        eprintln!("[DouyinBrowser] Current URL (refreshed): {}", current_url);

        // 注入提示
        self.inject_tip_overlay(&tab, &self.result.message);

        // 精确检查是否在创作者中心首页
        let is_home_page = current_url.contains("/creator-micro/home")
            || current_url.ends_with("/creator-micro/")
            || current_url == "https://creator.douyin.com/creator-micro";

        eprintln!("[DouyinBrowser] Is home page: {}", is_home_page);

        if is_home_page {
            eprintln!("[DouyinBrowser] On home page, starting extraction...");

            // 注入 API 拦截器
            self.inject_api_interceptor(&tab);

            // 导航到上传页面触发 API
            match tab.navigate_to("https://creator.douyin.com/creator-micro/content/upload") {
                Ok(_) => {
                    tab.wait_until_navigated()
                        .map_err(|e| format!("等待导航失败: {}", e))?;
                }
                Err(e) => {
                    eprintln!("[DouyinBrowser] 导航失败: {}", e);
                }
            }

            // 等待 API 数据
            std::thread::sleep(Duration::from_secs(3));

            // 提取数据
            self.extract_and_save(&tab).await?;

            return Ok(false);
        }

        // 检查是否在上传页面
        if current_url.contains("/content/upload") {
            eprintln!("[DouyinBrowser] On upload page, extracting...");

            // 注入 API 拦截器（如果还没有）
            self.inject_api_interceptor(&tab);

            // 等待 API 数据
            std::thread::sleep(Duration::from_secs(3));

            self.extract_and_save(&tab).await?;

            return Ok(false);
        }

        // 检查是否在创作者中心（其他页面）
        if current_url.contains("creator.douyin.com/creator-micro") {
            eprintln!("[DouyinBrowser] On creator center (other page), navigating to upload...");
            self.result.step = BrowserAuthStep::NavigatingToUpload;
            self.result.message = "正在跳转到上传页面...".to_string();
            self.inject_tip_overlay(&tab, "正在跳转到上传页面...");

            tab.navigate_to("https://creator.douyin.com/creator-micro/content/upload")
                .ok();
            tab.wait_until_navigated().ok();
            std::thread::sleep(Duration::from_secs(2));

            return Ok(true);
        }

        // 仍在登录页面
        eprintln!("[DouyinBrowser] Still on login page, waiting for scan...");
        self.result.step = BrowserAuthStep::WaitingForLogin;
        self.result.message = "请扫码登录...".to_string();
        Ok(true)
    }

    /// 提取并保存数据
    async fn extract_and_save(&mut self, tab: &Tab) -> Result<(), String> {
        // 等待 API 数据
        if !self.wait_for_api_data(tab, 2, 30u32) {
            eprintln!("[DouyinBrowser] Warning: API data not fully received");
        }

        // 获取数据
        let api_data = self.get_api_data(tab);
        let (uid, nickname, avatar_url) = self.extract_user_info(&api_data);
        let cookie = self.get_cookies(tab);
        let local_storage_items = self.get_local_storage(tab);

        // 构建结果
        let result_data = DouyinAuthResult {
            third_id: uid.parse().unwrap_or(0),
            third_param: ThirdParam {
                cookie: cookie.clone(),
                local_data: local_storage_items.clone(),
                accept: "".to_string(),
                referer: "".to_string(),
                user_agent: "".to_string(),
                sec_ch_ua: "".to_string(),
                sec_fetch_dest: "".to_string(),
                sec_fetch_mode: "".to_string(),
                sec_fetch_site: "".to_string(),
                accept_encoding: "".to_string(),
                accept_language: "".to_string(),
                sec_ch_ua_mobile: "".to_string(),
                sec_ch_ua_platform: "".to_string(),
                x_secsdk_csrf_token: "".to_string(),
            },
            third_return: api_data.user_info.clone().unwrap_or(serde_json::json!({})),
            all_cookies: cookie.clone(),
        };

        let params_json = serde_json::to_string(&result_data)
            .map_err(|e| format!("序列化失败: {}", e))?;

        // 更新结果
        self.result.cookie = cookie;
        self.result.local_storage = serde_json::to_string(&local_storage_items)
            .unwrap_or_default();
        self.result.nickname = nickname.clone();
        self.result.avatar_url = avatar_url;
        self.result.step = BrowserAuthStep::Completed;
        self.result.message = format!("授权完成！账号: {}", nickname);
        self.result.need_poll = false;

        // 关闭浏览器
        self.close_browser();

        Ok(())
    }

    /// 取消授权
    pub async fn cancel(&mut self) {
        self.close_browser();
        self.result.step = BrowserAuthStep::Idle;
        self.result.message = "已取消授权".to_string();
        self.result.need_poll = false;
    }

    /// 获取授权结果
    pub fn get_result(&self) -> &BrowserAuthResult {
        &self.result
    }
}

impl Default for DouyinBrowser {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 数据结构
// ============================================================================

/// LocalStorage 项
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LocalStorageItem {
    key: String,
    value: String,
}

/// 抖音授权结果数据结构（参考 Python 版本）
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DouyinAuthResult {
    /// 用户 ID
    third_id: u64,
    /// 请求参数
    third_param: ThirdParam,
    /// 用户信息响应
    third_return: serde_json::Value,
    /// 所有 cookies
    all_cookies: String,
}

/// 请求头参数
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ThirdParam {
    #[serde(rename = "accept")]
    accept: String,
    #[serde(rename = "cookie")]
    cookie: String,
    #[serde(rename = "referer")]
    referer: String,
    #[serde(rename = "local_data")]
    local_data: Vec<LocalStorageItem>,
    #[serde(rename = "sec-ch-ua")]
    sec_ch_ua: String,
    #[serde(rename = "user-agent")]
    user_agent: String,
    #[serde(rename = "sec-fetch-dest")]
    sec_fetch_dest: String,
    #[serde(rename = "sec-fetch-mode")]
    sec_fetch_mode: String,
    #[serde(rename = "sec-fetch-site")]
    sec_fetch_site: String,
    #[serde(rename = "accept-encoding")]
    accept_encoding: String,
    #[serde(rename = "accept-language")]
    accept_language: String,
    #[serde(rename = "sec-ch-ua-mobile")]
    sec_ch_ua_mobile: String,
    #[serde(rename = "sec-ch-ua-platform")]
    sec_ch_ua_platform: String,
    #[serde(rename = "x-secsdk-csrf-token")]
    x_secsdk_csrf_token: String,
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
