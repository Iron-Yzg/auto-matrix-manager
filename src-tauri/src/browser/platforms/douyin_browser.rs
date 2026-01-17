// Douyin Browser Implementation - 使用 JavaScript 直接调用 API
// 参考 Python douyin_account_bind.py 实现

use crate::browser::{BrowserAuthResult, BrowserAuthStep};
use headless_chrome::{Browser, LaunchOptions, Tab};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

/// API 响应数据
#[derive(Debug, Clone, Default)]
struct ApiResponseData {
    user_info: Option<serde_json::Value>,
    account_info: Option<serde_json::Value>,
    request_headers: Option<HashMap<String, String>>,
}

/// 抖音浏览器实现
pub struct DouyinBrowser {
    browser: Option<Browser>,
    tab: Option<Arc<Tab>>,
    result: BrowserAuthResult,
    chrome_path: Option<PathBuf>,
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

    pub fn with_chrome_path(path: PathBuf) -> Self {
        Self {
            browser: None,
            tab: None,
            result: BrowserAuthResult::default(),
            chrome_path: Some(path),
            timeout_seconds: 120,
        }
    }

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

    /// 启用网络监听
    fn enable_network_events(&self, tab: &Tab) {
        eprintln!("[DouyinBrowser] Network events ready");
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

        if let Some(user_info) = &api_data.user_info {
            eprintln!("[DouyinBrowser] Parsing user_info: {}", user_info);

            if let Some(user) = user_info.get("user") {
                if let Some(u) = user.get("uid").and_then(|v| v.as_str()) {
                    uid = u.to_string();
                }
                if let Some(nick) = user.get("nickname").and_then(|v| v.as_str()) {
                    nickname = nick.to_string();
                }
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

        if let Some(account_info) = &api_data.account_info {
            eprintln!("[DouyinBrowser] Parsing account_info: {}", account_info);

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

    /// 关闭浏览器
    fn close_browser(&mut self) {
        eprintln!("[DouyinBrowser] Closing browser...");
        self.tab = None;
        self.browser = None;
        eprintln!("[DouyinBrowser] Browser closed");
    }

    /// 注入请求头拦截器，获取原始请求头信息
    fn inject_headers_interceptor(&self, tab: &Tab) {
        eprintln!("[DouyinBrowser] Injecting headers interceptor...");

        let interceptor_script = r#"
            (function() {
                // 初始化存储
                window.__DY_HEADERS__ = null;
                window.__DY_USER_INFO__ = null;

                // 拦截 XMLHttpRequest
                const originalXHROpen = XMLHttpRequest.prototype.open;
                const originalXHRSend = XMLHttpRequest.prototype.send;
                const originalSetRequestHeader = XMLHttpRequest.prototype.setRequestHeader;

                XMLHttpRequest.prototype.open = function(method, url, ...rest) {
                    this._url = url;
                    this._method = method;
                    this._headers = {};
                    return originalXHROpen.apply(this, [method, url, ...rest]);
                };

                XMLHttpRequest.prototype.setRequestHeader = function(name, value) {
                    if (!this._headers) this._headers = {};
                    this._headers[name] = value;
                    return originalSetRequestHeader.apply(this, [name, value]);
                };

                XMLHttpRequest.prototype.send = function(...args) {
                    this.addEventListener('load', () => {
                        const url = this._url;
                        if (typeof url !== 'string') return;

                        // 捕获账号请求头
                        if (url.includes('/account/api/v1/user/account/info')) {
                            try {
                                window.__DY_HEADERS__ = this._headers || {};
                                console.log('[Headers] Captured account headers');
                            } catch (e) {
                                console.error('[Headers] Error:', e);
                            }
                        }

                        // 捕获用户信息响应
                        if (url.includes('/web/api/media/user/info')) {
                            try {
                                const data = JSON.parse(this.responseText);
                                window.__DY_USER_INFO__ = data;
                                console.log('[Headers] Captured user info');
                            } catch (e) {
                                console.error('[Headers] User info error:', e);
                            }
                        }
                    });

                    return originalXHRSend.apply(this, args);
                };

                console.log('[Headers] Interceptor installed');
            })()
        "#;

        if let Err(e) = tab.evaluate(interceptor_script, true) {
            eprintln!("[DouyinBrowser] Failed to inject headers interceptor: {}", e);
        }
    }

    /// 触发页面发送原始 API 请求（通过刷新页面触发）
    fn trigger_api_requests(&self, tab: &Tab) {
        eprintln!("[DouyinBrowser] Triggering API requests...");

        let trigger_script = r#"
            (function() {
                // 查找页面中可能触发 API 请求的元素并点击
                const buttons = document.querySelectorAll('button, a, [onclick]');
                for (let btn of buttons) {
                    const text = btn.innerText || btn.textContent || '';
                    if (text.includes('首页') || text.includes('主页') || text.includes('我的')) {
                        try {
                            btn.click();
                            console.log('[Trigger] Clicked:', text);
                        } catch(e) {}
                    }
                }
                return {clicked: buttons.length};
            })()
        "#;

        let _ = tab.evaluate(trigger_script, true);

        // 等待 API 请求
        std::thread::sleep(Duration::from_secs(2));
    }

    /// 获取拦截的请求头
    fn get_captured_headers(&self, tab: &Tab) -> Option<HashMap<String, String>> {
        let script = r#"
            (function() {
                if (window.__DY_HEADERS__) {
                    return JSON.stringify(window.__DY_HEADERS__);
                }
                return null;
            })()
        "#;

        match tab.evaluate(script, true) {
            Ok(result) => {
                if let Some(value) = result.value {
                    if let Some(s) = value.as_str() {
                        eprintln!("[DouyinBrowser] Captured headers: {}", s);
                        return serde_json::from_str(s).ok();
                    }
                }
            }
            Err(e) => {
                eprintln!("[DouyinBrowser] Get headers error: {}", e);
            }
        }
        None
    }

    /// 获取拦截的用户信息
    fn get_captured_user_info(&self, tab: &Tab) -> Option<serde_json::Value> {
        let script = r#"
            (function() {
                if (window.__DY_USER_INFO__) {
                    return JSON.stringify(window.__DY_USER_INFO__);
                }
                return null;
            })()
        "#;

        match tab.evaluate(script, true) {
            Ok(result) => {
                if let Some(value) = result.value {
                    if let Some(s) = value.as_str() {
                        eprintln!("[DouyinBrowser] Captured user info: {}", s);
                        return serde_json::from_str(s).ok();
                    }
                }
            }
            Err(e) => {
                eprintln!("[DouyinBrowser] Get user info error: {}", e);
            }
        }
        None
    }

    /// 使用 fetch 直接调用 API 获取用户信息
    fn fetch_user_info_directly(&self, tab: &Tab) -> Option<serde_json::Value> {
        eprintln!("[DouyinBrowser] Attempting to fetch user info directly via fetch...");

        let fetch_script = r#"
            (async function() {
                try {
                    const response = await fetch('https://creator.douyin.com/web/api/media/user/info', {
                        method: 'GET',
                        credentials: 'include'
                    });
                    const data = await response.json();
                    return JSON.stringify(data);
                } catch (e) {
                    return JSON.stringify({error: e.message});
                }
            })()
        "#;

        match tab.evaluate(fetch_script, true) {
            Ok(result) => {
                if let Some(value) = result.value {
                    if let Some(s) = value.as_str() {
                        eprintln!("[DouyinBrowser] Direct fetch result: {}", s);
                        return serde_json::from_str(s).ok();
                    }
                }
            }
            Err(e) => {
                eprintln!("[DouyinBrowser] Direct fetch error: {}", e);
            }
        }
        None
    }

    /// 尝试从页面 DOM 或全局变量获取用户信息
    fn extract_user_info_from_page(&self, tab: &Tab) -> Option<serde_json::Value> {
        eprintln!("[DouyinBrowser] Trying to extract user info from page...");

        let extract_script = r#"
            (function() {
                // 尝试从各种可能的全局变量获取用户信息
                const sources = [
                    window.__INITIAL_STATE__,
                    window.__NUXT__,
                    window.__DATA__,
                    window._INITIAL_DATA_,
                    window.USER_INFO,
                    window.userInfo,
                    window.__dubbo__,
                    window.__apollo__,
                ];

                for (let i = 0; i < sources.length; i++) {
                    const source = sources[i];
                    if (source) {
                        const str = JSON.stringify(source);
                        if (str.includes('uid') || str.includes('nickname') || str.includes('avatar')) {
                            return str;
                        }
                    }
                }

                // 尝试从 script 标签中提取
                const scripts = document.querySelectorAll('script');
                for (let script of scripts) {
                    const content = script.textContent || '';
                    if (content.includes('uid') && content.includes('nickname')) {
                        try {
                            // 尝试提取 JSON 对象
                            const match = content.match(/\{["']?uid["']?:[^}]+\}/);
                            if (match) {
                                return match[0];
                            }
                        } catch (e) {}
                    }
                }

                return JSON.stringify({found: false});
            })()
        "#;

        match tab.evaluate(extract_script, true) {
            Ok(result) => {
                if let Some(value) = result.value {
                    if let Some(s) = value.as_str() {
                        eprintln!("[DouyinBrowser] Page extraction result: {}", s);
                        return serde_json::from_str(s).ok();
                    }
                }
            }
            Err(e) => {
                eprintln!("[DouyinBrowser] Page extraction error: {}", e);
            }
        }
        None
    }

    /// 启动抖音授权流程
    pub async fn start_authorize(&mut self) -> Result<BrowserAuthResult, String> {
        self.result.step = BrowserAuthStep::LaunchingBrowser;
        self.result.message = "正在启动浏览器...".to_string();
        eprintln!("[DouyinBrowser] ===== Starting authorization =====");

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

        self.inject_tip_overlay(tab, "请使用抖音App扫码登录，登录成功后页面将自动跳转");

        // 步骤2: 等待登录
        self.result.step = BrowserAuthStep::WaitingForLogin;
        self.result.message = "请扫码登录...".to_string();

        eprintln!("[DouyinBrowser] Waiting for login and redirect to home page...");

        if !self.wait_for_url(tab, "/creator-micro/home", self.timeout_seconds) {
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

        self.result.current_url = tab.get_url();
        eprintln!("[DouyinBrowser] Reached creator center: {}", self.result.current_url);

        // 步骤3: 提取用户信息
        self.result.step = BrowserAuthStep::ExtractingCredentials;
        self.result.message = "正在获取用户数据...".to_string();
        self.inject_tip_overlay(tab, "正在获取用户数据，请稍候...");

        eprintln!("[DouyinBrowser] Starting data extraction...");

        // 等待页面完全加载
        std::thread::sleep(Duration::from_secs(3));

        // 方法1: 使用 XHR 拦截器获取请求头和用户信息
        eprintln!("[DouyinBrowser] Method 1: XHR interceptor...");
        self.inject_headers_interceptor(tab);

        // 刷新页面触发 API 请求
        eprintln!("[DouyinBrowser] Refreshing page to trigger API requests...");
        tab.navigate_to("https://creator.douyin.com/creator-micro/home")
            .ok();
        tab.wait_until_navigated().ok();

        // 等待 API 请求完成
        std::thread::sleep(Duration::from_secs(3));

        // 获取拦截的数据
        let captured_headers = self.get_captured_headers(tab);
        let captured_user_info = self.get_captured_user_info(tab);

        // 构建 ApiResponseData
        let mut api_response_data = ApiResponseData::default();

        if let Some(data) = captured_user_info {
            eprintln!("[DouyinBrowser] XHR interceptor got user info!");
            api_response_data.user_info = Some(data);
        }

        if let Some(headers) = captured_headers {
            eprintln!("[DouyinBrowser] XHR interceptor got headers!");
            api_response_data.request_headers = Some(headers);
        }

        // 方法2: 如果拦截器没有获取到数据，尝试直接调用 API
        if api_response_data.user_info.is_none() {
            eprintln!("[DouyinBrowser] Method 2: Direct API fetch...");
            let api_data = self.fetch_user_info_directly(tab);
            if let Some(data) = api_data {
                eprintln!("[DouyinBrowser] Direct API success!");
                api_response_data.user_info = Some(data);
            }
        }

        // 方法3: 从页面提取用户信息
        if api_response_data.user_info.is_none() {
            eprintln!("[DouyinBrowser] Method 3: Extract from page...");
            let page_data = self.extract_user_info_from_page(tab);
            if let Some(data) = page_data {
                eprintln!("[DouyinBrowser] Page extraction success!");
                if let Some(user_obj) = data.get("user").or(data.get("userInfo")) {
                    api_response_data.user_info = Some(json!({"user": user_obj}));
                } else {
                    api_response_data.user_info = Some(data);
                }
            }
        }

        // 方法4: 页面内容抓取作为后备
        if api_response_data.user_info.is_none() {
            eprintln!("[DouyinBrowser] Method 4: Scraping page content...");
            let scrape_script = r#"
                (function() {
                    const userElements = document.querySelectorAll('[class*="user"], [class*="nick"], [id*="user"]');
                    for (let el of userElements) {
                        const text = el.innerText || el.textContent;
                        if (text && text.length > 0 && text.length < 50 && !text.includes(' ')) {
                            return JSON.stringify({type: 'element', text: text});
                        }
                    }

                    const avatars = document.querySelectorAll('img[src*="avatar"], img[src*="pstatp"], img[src*="douyinv"]');
                    if (avatars.length > 0) {
                        return JSON.stringify({type: 'avatar', src: avatars[0].src});
                    }

                    return JSON.stringify({found: false});
                })()
            "#;

            if let Ok(result) = tab.evaluate(scrape_script, true) {
                if let Some(value) = result.value {
                    if let Some(s) = value.as_str() {
                        eprintln!("[DouyinBrowser] Page scrape result: {}", s);
                    }
                }
            }
        }

        // 获取用户信息
        let (uid, nickname, avatar_url) = self.extract_user_info(&api_response_data);

        eprintln!("[DouyinBrowser] User info extracted - uid: {}, nickname: {}", uid, nickname);

        // 获取 cookies
        let cookie = self.get_cookies(tab);

        // 获取 localStorage
        let local_storage_items = self.get_local_storage(tab);

        // 从拦截的请求头中提取需要的字段
        let captured_headers = api_response_data.request_headers.clone().unwrap_or_default();

        let third_param = ThirdParam {
            cookie: cookie.clone(),
            local_data: local_storage_items.clone(),
            accept: captured_headers.get("accept").cloned().unwrap_or_default(),
            referer: captured_headers.get("referer").cloned()
                .unwrap_or_else(|| "https://creator.douyin.com/creator-micro/content/post/video?enter_from=publish_page".to_string()),
            user_agent: captured_headers.get("user-agent").cloned().unwrap_or_default(),
            sec_ch_ua: captured_headers.get("sec-ch-ua").cloned().unwrap_or_default(),
            sec_fetch_dest: captured_headers.get("sec-fetch-dest").cloned().unwrap_or_default(),
            sec_fetch_mode: captured_headers.get("sec-fetch-mode").cloned().unwrap_or_default(),
            sec_fetch_site: captured_headers.get("sec-fetch-site").cloned().unwrap_or_default(),
            accept_encoding: captured_headers.get("accept-encoding").cloned().unwrap_or_default(),
            accept_language: captured_headers.get("accept-language").cloned().unwrap_or_default(),
            sec_ch_ua_mobile: captured_headers.get("sec-ch-ua-mobile").cloned().unwrap_or_default(),
            sec_ch_ua_platform: captured_headers.get("sec-ch-ua-platform").cloned().unwrap_or_default(),
            x_secsdk_csrf_token: captured_headers.get("x-secsdk-csrf-token").cloned().unwrap_or_default(),
        };

        // 序列化数据
        let _result_data = DouyinAuthResult {
            third_id: uid.parse().unwrap_or(0),
            third_param: third_param.clone(),
            third_return: api_response_data.user_info.clone().unwrap_or(serde_json::json!({})),
            all_cookies: cookie.clone(),
        };

        let _params_json = serde_json::to_string(&DouyinAuthResult {
            third_id: uid.parse().unwrap_or(0),
            third_param,
            third_return: api_response_data.user_info.clone().unwrap_or(serde_json::json!({})),
            all_cookies: cookie.clone(),
        }).map_err(|e| format!("序列化params失败: {}", e))?;

        // 更新结果
        self.result.cookie = cookie;
        self.result.local_storage = serde_json::to_string(&local_storage_items)
            .unwrap_or_default();
        self.result.nickname = nickname.clone();
        self.result.avatar_url = avatar_url.clone();
        self.result.step = BrowserAuthStep::Completed;
        self.result.message = format!("授权完成！账号: {}", nickname);
        self.result.need_poll = false;

        eprintln!("[DouyinBrowser] ===== Authorization completed =====");
        eprintln!("[DouyinBrowser] Nickname: {}", nickname);
        eprintln!("[DouyinBrowser] UID: {}", uid);
        eprintln!("[DouyinBrowser] Avatar: {}", avatar_url.clone());

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

        let current_url = tab.get_url();
        self.result.current_url = current_url.clone();
        eprintln!("[DouyinBrowser] Current URL: {}", current_url);

        self.inject_tip_overlay(&tab, &self.result.message);

        // 检查是否在创作者中心首页
        let is_home_page = current_url.contains("/creator-micro/home")
            || current_url.ends_with("/creator-micro/")
            || current_url == "https://creator.douyin.com/creator-micro";

        eprintln!("[DouyinBrowser] Is home page: {}", is_home_page);

        if is_home_page {
            eprintln!("[DouyinBrowser] On home page, extracting data...");

            self.result.step = BrowserAuthStep::ExtractingCredentials;
            self.result.message = "正在获取用户数据...".to_string();
            self.inject_tip_overlay(&tab, "正在获取用户数据，请稍候...");

            // 启用网络监听
            self.enable_network_events(&tab);

            // 等待
            std::thread::sleep(Duration::from_secs(2));

            // 获取数据
            let api_data = self.fetch_user_info_directly(&tab);
            let page_data = self.extract_user_info_from_page(&tab);

            let mut api_response_data = ApiResponseData::default();

            if let Some(data) = api_data {
                api_response_data.user_info = Some(data);
            }

            if let Some(data) = page_data {
                if api_response_data.user_info.is_none() {
                    if let Some(user_obj) = data.get("user").or(data.get("userInfo")) {
                        api_response_data.user_info = Some(json!({"user": user_obj}));
                    } else {
                        api_response_data.user_info = Some(data);
                    }
                }
            }

            let (uid, nickname, avatar_url) = self.extract_user_info(&api_response_data);
            let cookie = self.get_cookies(&tab);
            let local_storage_items = self.get_local_storage(&tab);

            self.result.cookie = cookie;
            self.result.local_storage = serde_json::to_string(&local_storage_items)
                .unwrap_or_default();
            self.result.nickname = nickname.clone();
            self.result.avatar_url = avatar_url;
            self.result.step = BrowserAuthStep::Completed;
            self.result.message = format!("授权完成！账号: {}", nickname);
            self.result.need_poll = false;

            self.close_browser();

            return Ok(false);
        }

        // 检查是否在上传页面
        if current_url.contains("/content/upload") {
            eprintln!("[DouyinBrowser] On upload page, extracting...");
            return Ok(false);
        }

        // 检查是否在创作者中心
        if current_url.contains("creator.douyin.com/creator-micro") {
            eprintln!("[DouyinBrowser] On creator center, waiting...");
            self.result.step = BrowserAuthStep::WaitingForUpload;
            self.result.message = "等待页面加载...".to_string();
            return Ok(true);
        }

        eprintln!("[DouyinBrowser] Still on login page, waiting for scan...");
        self.result.step = BrowserAuthStep::WaitingForLogin;
        self.result.message = "请扫码登录...".to_string();
        Ok(true)
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

/// 抖音授权结果数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DouyinAuthResult {
    third_id: u64,
    third_param: ThirdParam,
    third_return: serde_json::Value,
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
