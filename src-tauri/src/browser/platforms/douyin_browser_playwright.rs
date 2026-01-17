// Douyin Browser Implementation - 使用 Playwright 网络事件监听
// Playwright 可以在不注入 JS 的情况下捕获所有网络请求/响应
// 通过 CDP (Chrome DevTools Protocol) 实现

use crate::browser::{BrowserAuthResult, BrowserAuthStep};
use std::path::PathBuf;

/// 抖音浏览器实现（Playwright 版本）
pub struct DouyinBrowserPlaywright {
    result: BrowserAuthResult,
    timeout_seconds: u32,
}

impl DouyinBrowserPlaywright {
    pub fn new() -> Self {
        Self {
            result: BrowserAuthResult::default(),
            timeout_seconds: 120,
        }
    }

    pub fn with_timeout(mut self, seconds: u32) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    /// 获取 Playwright 安装目录
    /// 使用 Tauri 应用数据目录 ~/Library/Application Support/com.yzg.matrix/playwright
    fn get_playwright_dir(&self) -> PathBuf {
        Self::get_playwright_dir_static()
    }

    /// 获取浏览器安装目录
    fn get_browsers_dir(&self) -> PathBuf {
        self.get_playwright_dir().join("browsers")
    }

    /// 确保 Playwright 已安装
    fn ensure_playwright_installed(&self) -> Result<(), String> {
        let playwright_dir = self.get_playwright_dir();
        let node_modules = playwright_dir.join("node_modules").join("playwright");

        // 如果已安装，直接返回
        if node_modules.exists() {
            eprintln!("[DouyinBrowser-Playwright] Playwright already installed at: {}", node_modules.display());
            return Ok(());
        }

        eprintln!("[DouyinBrowser-Playwright] Installing Playwright to: {}", playwright_dir.display());

        // 创建目录
        if let Err(e) = std::fs::create_dir_all(&node_modules) {
            return Err(format!("创建 Playwright 目录失败: {}", e));
        }

        // 写入 package.json
        let package_json = r#"{
  "name": "auto-matrix-manager-playwright",
  "version": "1.0.0",
  "description": "Playwright for Auto Matrix Manager",
  "main": "index.js",
  "type": "commonjs",
  "dependencies": {
    "playwright": "^1.50.0"
  }
}"#;
        let package_path = playwright_dir.join("package.json");
        if let Err(e) = std::fs::write(&package_path, package_json) {
            return Err(format!("写入 package.json 失败: {}", e));
        }

        // 执行 npm install
        eprintln!("[DouyinBrowser-Playwright] Running npm install...");
        let install_output = std::process::Command::new("npm")
            .arg("install")
            .arg("--prefer-offline")
            .current_dir(&playwright_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output();

        match install_output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if !output.status.success() {
                    eprintln!("[DouyinBrowser-Playwright] npm install stdout:\n{}", stdout);
                    eprintln!("[DouyinBrowser-Playwright] npm install stderr:\n{}", stderr);
                    return Err("安装 Playwright 失败".to_string());
                }

                eprintln!("[DouyinBrowser-Playwright] Playwright installed successfully");
                Ok(())
            }
            Err(e) => Err(format!("执行 npm install 失败: {}", e))
        }
    }

    /// 确保浏览器已安装
    fn ensure_browser_installed(&self) -> Result<(), String> {
        let browsers_dir = self.get_browsers_dir();

        // 检查 chromium 是否存在
        let chromium_dir = browsers_dir.join("chromium-");
        if chromium_dir.exists() && chromium_dir.read_dir().map(|_e| _e.count()).unwrap_or(0) > 0 {
            eprintln!("[DouyinBrowser-Playwright] Chromium already installed");
            return Ok(());
        }

        eprintln!("[DouyinBrowser-Playwright] Installing Chromium browser...");

        // 使用应用目录下的 playwright 安装浏览器
        let playwright_dir = self.get_playwright_dir();
        let install_output = std::process::Command::new("npx")
            .arg("playwright")
            .arg("install")
            .arg("chromium")
            .env("PLAYWRIGHT_BROWSERS_PATH", &browsers_dir)
            .current_dir(&playwright_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output();

        match install_output {
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !output.status.success() {
                    eprintln!("[DouyinBrowser-Playwright] Browser install stderr:\n{}", stderr);
                    return Err("安装 Chromium 失败".to_string());
                }
                eprintln!("[DouyinBrowser-Playwright] Chromium installed successfully");
                Ok(())
            }
            Err(e) => Err(format!("安装浏览器失败: {}", e))
        }
    }

    /// 启动抖音授权流程（使用 Playwright）
    pub async fn start_authorize(&mut self) -> Result<BrowserAuthResult, String> {
        self.result.step = BrowserAuthStep::LaunchingBrowser;
        self.result.message = "正在启动浏览器...".to_string();
        eprintln!("[DouyinBrowser-Playwright] ===== Starting authorization with Playwright =====");

        // 确保 Playwright 已安装
        if let Err(e) = self.ensure_playwright_installed() {
            self.result.step = BrowserAuthStep::Failed(e.clone());
            self.result.message = e.clone();
            self.result.error = Some(e.clone());
            return Err(e);
        }

        // 确保浏览器已安装
        if let Err(e) = self.ensure_browser_installed() {
            self.result.step = BrowserAuthStep::Failed(e.clone());
            self.result.message = e.clone();
            self.result.error = Some(e.clone());
            return Err(e);
        }

        eprintln!("[DouyinBrowser-Playwright] Starting Playwright script...");

        // 准备参数供阻塞线程使用
        let timeout_secs = self.timeout_seconds;

        // 在阻塞线程中运行 Playwright 脚本
        let result = tokio::task::spawn_blocking(move || {
            Self::run_script(timeout_secs)
        }).await;

        match result {
            Ok(Ok(auth_result)) => {
                self.result = auth_result;
                Ok(self.result.clone())
            }
            Ok(Err(e)) => {
                self.result.step = BrowserAuthStep::Failed(e.clone());
                self.result.message = e.clone();
                self.result.error = Some(e.clone());
                Err(e)
            }
            Err(e) => {
                let err_msg = format!("Playwright 任务失败: {}", e);
                self.result.step = BrowserAuthStep::Failed(err_msg.clone());
                self.result.message = err_msg.clone();
                self.result.error = Some(err_msg.clone());
                Err(err_msg)
            }
        }
    }

    /// 获取 Playwright 安装目录（静态方法版本，用于阻塞线程）
    /// 使用与 Tauri lib.rs 相同的路径逻辑
    fn get_playwright_dir_static() -> PathBuf {
        // 使用与 lib.rs 相同的 fallback 逻辑
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let data_path = std::path::PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("com.yzg.matrix");
        data_path.join("playwright")
    }

    /// 在阻塞线程中运行 Playwright 脚本
    fn run_script(timeout_secs: u32) -> Result<BrowserAuthResult, String> {
        eprintln!("[DouyinBrowser-Playwright] Running Playwright script in blocking thread...");

        // 获取 Playwright 目录（使用应用数据目录）
        let playwright_dir = Self::get_playwright_dir_static();
        let browsers_dir = playwright_dir.join("browsers");

        let output_path = std::env::temp_dir().join("douyin_auth_result.json");

        // 创建临时 Node.js 脚本
        let script = Self::create_node_script(timeout_secs);

        // 写入脚本到 Playwright 目录
        let script_path = playwright_dir.join("douyin_auth_script.js");
        if let Err(e) = std::fs::write(&script_path, &script) {
            return Err(format!("无法写入临时脚本: {}", e));
        }

        // 删除旧的结果文件
        let _ = std::fs::remove_file(&output_path);

        eprintln!("[DouyinBrowser-Playwright] Script path: {}", script_path.display());
        eprintln!("[DouyinBrowser-Playwright] Output path: {}", output_path.display());
        eprintln!("[DouyinBrowser-Playwright] Browsers path: {}", browsers_dir.display());

        // 执行脚本
        let start = std::time::Instant::now();
        let output = std::process::Command::new("node")
            .arg(&script_path)
            .arg(output_path.to_string_lossy().as_ref())
            .env("PLAYWRIGHT_BROWSERS_PATH", browsers_dir.to_string_lossy().as_ref())
            .current_dir(&playwright_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output();

        match output {
            Ok(output) => {
                let elapsed = start.elapsed();
                eprintln!("[DouyinBrowser-Playwright] Script completed in {:.1}s", elapsed.as_secs_f64());

                let stderr = String::from_utf8_lossy(&output.stderr);
                if !stderr.is_empty() {
                    eprintln!("[DouyinBrowser-Playwright] stderr:\n{}", stderr);
                }

                if output.status.success() {
                    if output_path.exists() {
                        match std::fs::read_to_string(&output_path) {
                            Ok(content) => {
                                eprintln!("[DouyinBrowser-Playwright] Result content: {}", content);
                                Self::parse_result(&content)
                            }
                            Err(e) => Err(format!("读取结果文件失败: {}", e))
                        }
                    } else {
                        Err("未找到结果文件".to_string())
                    }
                } else {
                    Err(format!("脚本执行失败: {}", stderr))
                }
            }
            Err(e) => Err(format!("无法执行脚本: {}", e))
        }
    }

    /// 解析认证结果
    fn parse_result(content: &str) -> Result<BrowserAuthResult, String> {
        eprintln!("[DouyinBrowser-Playwright] Parsing result: {}", content);

        match serde_json::from_str::<serde_json::Value>(content) {
            Ok(json) => {
                let mut result = BrowserAuthResult::default();

                if let Some(step) = json.get("step").and_then(|s| s.as_str()) {
                    result.step = match step {
                        "completed" => BrowserAuthStep::Completed,
                        "failed" => BrowserAuthStep::Failed(
                            json.get("message").and_then(|m| m.as_str()).unwrap_or("未知错误").to_string()
                        ),
                        "waiting" => BrowserAuthStep::WaitingForLogin,
                        "extracting" => BrowserAuthStep::ExtractingCredentials,
                        _ => BrowserAuthStep::ExtractingCredentials,
                    };
                }

                result.message = json.get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("完成")
                    .to_string();

                result.cookie = json.get("cookie")
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();

                result.local_storage = json.get("local_storage")
                    .and_then(|l| l.as_str())
                    .unwrap_or("")
                    .to_string();

                result.nickname = json.get("nickname")
                    .and_then(|n| n.as_str())
                    .unwrap_or("抖音用户")
                    .to_string();

                result.avatar_url = json.get("avatar_url")
                    .and_then(|a| a.as_str())
                    .unwrap_or("")
                    .to_string();

                result.current_url = json.get("url")
                    .and_then(|u| u.as_str())
                    .unwrap_or("")
                    .to_string();

                result.need_poll = json.get("need_poll")
                    .and_then(|b| b.as_bool())
                    .unwrap_or(false);

                if result.step == BrowserAuthStep::Completed {
                    Ok(result)
                } else {
                    Err(result.message.clone())
                }
            }
            Err(e) => Err(format!("解析结果失败: {}", e))
        }
    }

    /// 创建 Node.js 脚本
    fn create_node_script(_timeout_secs: u32) -> String {
        format!(r#"
const {{ chromium }} = require('playwright');
const fs = require('fs');

const OUTPUT_FILE = process.argv[2] || "";

async function main() {{
    console.log('=== Playwright Douyin Authenticator ===');
    console.log('Output file:', OUTPUT_FILE);

    let browser = null;

    try {{
        // 启动浏览器
        console.log('Launching browser...');
        browser = await chromium.launch({{
            headless: false,
            args: ['--no-sandbox', '--disable-setuid-sandbox', '--disable-dev-shm-usage']
        }});

        const context = await browser.newContext({{
            viewport: {{ width: 1280, height: 800 }},
            userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36'
        }});

        const page = await context.newPage();

        // Step 1: 导航到创作者中心
        console.log('Step 1: Navigating to https://creator.douyin.com/...');
        await page.goto('https://creator.douyin.com/', {{ waitUntil: 'domcontentloaded', timeout: 30000 }});
        console.log('[Nav] Page loaded:', page.url());

        // Step 2: 显示提示浮层
        console.log('Step 2: Showing tip overlay...');
        await page.evaluate(() => {{
            const existing = document.getElementById('amm-tip-overlay');
            if (existing) existing.remove();

            const tip = document.createElement('div');
            tip.id = 'amm-tip-overlay';
            tip.innerHTML = `
                <div style="
                    position: fixed;
                    top: 20px;
                    left: 50%;
                    transform: translateX(-50%);
                    background: linear-gradient(135deg, #ff9500 0%, #ff6b00 100%);
                    color: white;
                    padding: 16px 24px;
                    border-radius: 12px;
                    font-size: 14px;
                    font-weight: 600;
                    box-shadow: 0 10px 40px rgba(255, 149, 0, 0.4);
                    z-index: 99999999;
                    display: flex;
                    align-items: center;
                    gap: 10px;
                    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
                ">
                    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
                    </svg>
                    <span>请使用抖音App扫码登录，登录成功后页面将自动跳转</span>
                </div>
            `;
            document.body.insertBefore(tip, document.body.firstChild);
        }});

        // Step 3: 设置 CDP 监听器（在等待登录之前设置）
        console.log('Step 3: Setting up CDP listeners before waiting for login...');
        const cdpSession = await page.context().newCDPSession(page);
        await cdpSession.send('Network.enable');

        const apiResponses = {{}};

        cdpSession.on('Network.responseReceived', (params) => {{
            const url = params.response.url;
            const status = params.response.status;
            if (url.includes('/web/api/media/user/info') || url.includes('/account/api/v1/user/account/info')) {{
                console.log('[CDP Response]', status, url);
                apiResponses[url] = {{
                    requestId: params.requestId,
                    status: status,
                    url: url
                }};
            }}
        }});

        cdpSession.on('Network.requestWillBeSent', (params) => {{
            const url = params.request.url;
            if (url.includes('/web/api/media/user/info') || url.includes('/account/api/v1/user/account/info')) {{
                console.log('[CDP Request]', params.request.method, url);
            }}
        }});

        // Step 4: 无限等待用户扫码登录
        console.log('Step 4: Waiting for QR code login (no timeout)...');
        console.log('[Info] Please scan the QR code with Douyin app');

        // 使用 waitForURL 等待 URL 匹配模式
        try {{
            await page.waitForURL('**/creator-micro/**', {{ timeout: 0 }});
            console.log('[OK] Login detected! URL:', page.url());
        }} catch (e) {{
            console.log('[Timeout] Waiting for login redirect...');
            throw e;
        }}

        // 登录成功后，等待页面加载和 API 调用
        console.log('[CDP] Waiting for page load...');
        await page.waitForLoadState('load');
        console.log('[CDP] Page loaded');

        // 等待 API 调用完成
        await page.waitForTimeout(3000);
        console.log('[CDP] Waited for API calls');

        console.log('[CDP] Captured API responses:', Object.keys(apiResponses));

        // 移除提示浮层
        await page.evaluate(() => {{
            const tip = document.getElementById('amm-tip-overlay');
            if (tip) tip.remove();
        }});

        // Step 5: 从 CDP 监听的响应中提取用户信息
        console.log('Step 5: Extracting user info from CDP responses...');

        console.log('[CDP] Captured API responses:', Object.keys(apiResponses));
        console.log('[CDP] apiResponses count:', Object.keys(apiResponses).length);

        // 使用 CDP 获取响应体
        let nickname = '抖音用户';
        let avatar_url = '';

        for (const [url, info] of Object.entries(apiResponses)) {{
            console.log('[CDP] Processing URL:', url, 'requestId:', info.requestId);
            try {{
                // 使用 CDP 获取响应体
                const bodyResult = await cdpSession.send('Network.getResponseBody', {{
                    requestId: info.requestId
                }});

                console.log('[CDP] getResponseBody success, base64Encoded:', bodyResult.base64Encoded);

                let body = bodyResult.body;
                // 如果是 base64 编码的，解码
                if (bodyResult.base64Encoded) {{
                    body = Buffer.from(body, 'base64').toString('utf-8');
                }}

                const data = JSON.parse(body);
                console.log('[CDP] Response data:', JSON.stringify(data).substring(0, 300));

                if (url.includes('/web/api/media/user/info') && data?.user) {{
                    nickname = data.user.nickname || nickname;
                    if (data.user.avatar_thumb?.url_list?.[0]) {{
                        avatar_url = data.user.avatar_thumb.url_list[0];
                    }} else if (data.user.avatar_url) {{
                        avatar_url = data.user.avatar_url;
                    }}
                    console.log('[CDP] Extracted from userInfo:', nickname);
                }}

                if (url.includes('/account/api/v1/user/account/info') && data?.data?.user_info) {{
                    const user = data.data.user_info;
                    if (nickname === '抖音用户') {{
                        nickname = user.display_name || user.nickname || user.name || nickname;
                    }}
                    if (!avatar_url && user.avatar_url) {{
                        avatar_url = user.avatar_url;
                    }}
                    console.log('[CDP] Extracted from accountInfo:', nickname);
                }}
            }} catch (e) {{
                console.log('[CDP] Failed to get response body:', e.message, 'for URL:', url);
            }}
        }}

        // 方式2: 从页面全局变量中提取（备用）
        if (nickname === '抖音用户') {{
            console.log('[API] Trying window variables...');
            const pageData = await page.evaluate(() => {{
                // 尝试 __INITIAL_DATA__
                if (window.__INITIAL_DATA__) {{
                    try {{
                        const data = typeof window.__INITIAL_DATA__ === 'string'
                            ? JSON.parse(window.__INITIAL_DATA__)
                            : window.__INITIAL_DATA__;
                        if (data.user || data.userInfo || data.currentUser) {{
                            const user = data.user || data.userInfo || data.currentUser;
                            return {{ nickname: user.nickname, avatar_url: user.avatar || user.avatarUrl || user.avatar_thumb?.url_list?.[0] }};
                        }}
                    }} catch (e) {{}}
                }}
                // 尝试从 localStorage 中提取 sec_user_id
                const secUserId = localStorage.getItem('sec_user_id');
                if (secUserId) {{
                    return {{ sec_user_id: secUserId }};
                }}
                return null;
            }});

            if (pageData) {{
                if (pageData.nickname) {{
                    nickname = pageData.nickname;
                    console.log('[API] Extracted from window:', nickname);
                }}
                if (pageData.avatar_url) {{
                    avatar_url = pageData.avatar_url;
                }}
                if (pageData.sec_user_id) {{
                    console.log('[API] Found sec_user_id:', pageData.sec_user_id);
                }}
            }}
        }}

        // Step 6: 提取凭证数据
        console.log('Step 6: Extracting credentials...');

        const cookies = await page.context().cookies();
        console.log('[Data] Cookies captured:', cookies.length);

        const localStorageScript = await page.evaluate(() => {{
            const items = {{}};
            const keys = [
                'security-sdk/s_sdk_cert_key',
                'security-sdk/s_sdk_crypt_sdk',
                'security-sdk/s_sdk_pri_key',
                'security-sdk/s_sdk_pub_key',
                'security-sdk/s_sdk_server_cert_key',
                'sec_user_id',
                'sessionid'
            ];
            for (const key of keys) {{
                const value = localStorage.getItem(key);
                if (value) items[key] = value;
            }}
            return items;
        }});
        console.log('[Data] localStorage keys:', Object.keys(localStorageScript));

        // 从 localStorage 提取 sec_user_id
        if (localStorageScript['sec_user_id']) {{
            console.log('[Data] Found sec_user_id:', localStorageScript['sec_user_id']);
        }}

        // 构建结果
        const third_id = '';
        const sec_uid = localStorageScript['sec_user_id'] || '';

        console.log('[Data] Building params - third_id:', third_id, 'sec_uid:', sec_uid);

        const result = {{
            step: 'completed',
            message: '授权成功！账号: ' + nickname,
            url: page.url(),
            nickname: nickname,
            avatar_url: avatar_url,
            cookie: cookies.map(c => c.name + '=' + c.value).join('; '),
            local_storage: JSON.stringify(Object.entries(localStorageScript).map(([key, value]) => ({{ key, value }}))),
            third_id: third_id,
            sec_uid: sec_uid,
            need_poll: false
        }};

        fs.writeFileSync(OUTPUT_FILE, JSON.stringify(result, null, 2));
        console.log('[OK] Result saved to:', OUTPUT_FILE);
        console.log('[OK] Authorization completed! Nickname:', nickname);

    }} catch (error) {{
        console.error('[Error]', error.message);
        const result = {{
            step: 'failed',
            message: error.message || '操作被取消',
            need_poll: false
        }};
        fs.writeFileSync(OUTPUT_FILE, JSON.stringify(result, null, 2));
        console.log('[Error] Result saved to:', OUTPUT_FILE);
    }} finally {{
        if (browser) {{
            await browser.close();
            console.log('Browser closed');
        }}
    }}
}}

main().catch(error => {{
    console.error('Fatal error:', error);
    process.exit(1);
}});
"#
        )
    }

    /// 检查登录状态（轮询模式）
    pub async fn check_and_extract(&mut self) -> Result<bool, String> {
        eprintln!("[DouyinBrowser-Playwright] check_and_extract called");

        // Playwright 模式在 start_authorize 中已完成所有工作
        Ok(false)
    }

    /// 取消授权
    pub async fn cancel(&mut self) {
        self.result.step = BrowserAuthStep::Idle;
        self.result.message = "已取消授权".to_string();
        self.result.need_poll = false;
    }

    /// 获取授权结果
    pub fn get_result(&self) -> &BrowserAuthResult {
        &self.result
    }
}

impl Default for DouyinBrowserPlaywright {
    fn default() -> Self {
        Self::new()
    }
}
