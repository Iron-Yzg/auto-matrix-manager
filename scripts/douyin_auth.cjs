const { chromium } = require('playwright');
const fs = require('fs');

const OUTPUT_FILE = process.argv[2] || "";

async function main() {
    console.log('=== Playwright Douyin Authenticator ===');
    console.log('Output file:', OUTPUT_FILE);

    let browser = null;

    try {
        console.log('1. 准备启动浏览器...');

        // 启动浏览器
        console.log('2. 正在启动 chromium...');
        browser = await chromium.launch({
            headless: false,
            args: ['--no-sandbox', '--disable-setuid-sandbox', '--disable-dev-shm-usage']
        });
        console.log('3. 浏览器启动成功');

        const context = await browser.newContext({
            viewport: { width: 1280, height: 800 },
            userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36'
        });
        console.log('4. Context 创建成功');

        const page = await context.newPage();
        console.log('5. Page 创建成功');

        // 获取 CDP 会话以使用 Network 事件
        const cdpSession = await page.context().newCDPSession(page);
        console.log('6. CDP Session 创建成功');

        // 用于存储目标 API 的响应数据（分开存储，与 Python 逻辑一致）
        let userPacket = null;  // /web/api/media/user/info - 返回用户信息
        let accountPacket = null;  // /account/api/v1/user/account/info - 返回账号信息

        // 用于存储目标 API 的请求 headers（分开存储，与 Python 逻辑一致）
        const userInfoHeaders = {};  // /web/api/media/user/info 的 headers
        const accountInfoHeaders = {};  // /account/api/v1/user/account/info 的 headers

        // 使用 CDP 的 Network.requestWillBeSent 事件捕获完整请求 headers
        await cdpSession.send('Network.enable', { maxResourceBufferSize: 1024 * 1024 * 5, maxTotalBufferSize: 1024 * 1024 * 10 });
        console.log('[Info] CDP Network enabled');

        cdpSession.on('Network.requestWillBeSent', (event) => {
            const url = event.request.url;
            const headers = event.request.headers || {};

            // /web/api/media/user/info 的请求
            if (url.includes('/web/api/media/user/info')) {
                console.log('[CDP] RequestWillBeSent: /web/api/media/user/info');
                userInfoHeaders[url] = {
                    accept: headers['Accept'] || headers['accept'] || '',
                    cookie: headers['Cookie'] || headers['cookie'] || '',
                    'sec-ch-ua': headers['sec-ch-ua'] || '',
                    'user-agent': headers['User-Agent'] || headers['user-agent'] || '',
                    'sec-fetch-dest': headers['sec-fetch-dest'] || '',
                    'sec-fetch-mode': headers['sec-fetch-mode'] || '',
                    'sec-fetch-site': headers['sec-fetch-site'] || '',
                    'accept-encoding': headers['accept-encoding'] || '',
                    'accept-language': headers['accept-language'] || '',
                    'sec-ch-ua-mobile': headers['sec-ch-ua-mobile'] || '',
                    'sec-ch-ua-platform': headers['sec-ch-ua-platform'] || '',
                    'x-secsdk-csrf-token': headers['X-Secsdk-Csrf-Token'] || headers['x-secsdk-csrf-token'] || '',
                    'referer': headers['Referer'] || headers['referer'] || '',
                };
                console.log('[CDP] userInfo Cookie length: ' + (userInfoHeaders[url].cookie || '').length);
            }

            // /account/api/v1/user/account/info 的请求
            if (url.includes('/account/api/v1/user/account/info')) {
                console.log('[CDP] RequestWillBeSent: /account/api/v1/user/account/info');
                accountInfoHeaders[url] = {
                    accept: headers['Accept'] || headers['accept'] || '',
                    cookie: headers['Cookie'] || headers['cookie'] || '',
                    'sec-ch-ua': headers['sec-ch-ua'] || '',
                    'user-agent': headers['User-Agent'] || headers['user-agent'] || '',
                    'sec-fetch-dest': headers['sec-fetch-dest'] || '',
                    'sec-fetch-mode': headers['sec-fetch-mode'] || '',
                    'sec-fetch-site': headers['sec-fetch-site'] || '',
                    'accept-encoding': headers['accept-encoding'] || '',
                    'accept-language': headers['accept-language'] || '',
                    'sec-ch-ua-mobile': headers['sec-ch-ua-mobile'] || '',
                    'sec-ch-ua-platform': headers['sec-ch-ua-platform'] || '',
                    'x-secsdk-csrf-token': headers['X-Secsdk-Csrf-Token'] || headers['x-secsdk-csrf-token'] || '',
                    'referer': headers['Referer'] || headers['referer'] || '',
                };
                console.log('[CDP] accountInfo Cookie length: ' + (accountInfoHeaders[url].cookie || '').length);
            }
        });

        // 监听 CDP 的 Network.responseReceived 事件
        cdpSession.on('Network.responseReceived', (event) => {
            const url = event.response.url;
            if (url.includes('/web/api/media/user/info')) {
                console.log('[CDP] ResponseReceived: /web/api/media/user/info status=' + event.response.status);
            }
            if (url.includes('/account/api/v1/user/account/info')) {
                console.log('[CDP] ResponseReceived: /account/api/v1/user/account/info status=' + event.response.status);
            }
        });

        // 使用 page.on('response') 作为后备，分开存储两个接口
        page.on('response', async (response) => {
            const url = response.url();

            // /web/api/media/user/info
            if (url.includes('/web/api/media/user/info')) {
                console.log('[Response] Got: /web/api/media/user/info');

                // 如果还没有 headers，从 response.request().headers() 获取
                if (Object.keys(userInfoHeaders).length === 0) {
                    const headers = response.request().headers();
                    userInfoHeaders[url] = {
                        accept: headers['accept'] || headers['Accept'] || '',
                        'sec-ch-ua': headers['sec-ch-ua'] || headers['Sec-Ch-Ua'] || '',
                        'user-agent': headers['user-agent'] || headers['User-Agent'] || '',
                        'sec-fetch-dest': headers['sec-fetch-dest'] || headers['Sec-Fetch-Dest'] || '',
                        'sec-fetch-mode': headers['sec-fetch-mode'] || headers['Sec-Fetch-Mode'] || '',
                        'sec-fetch-site': headers['sec-fetch-site'] || headers['Sec-Fetch-Site'] || '',
                        'accept-encoding': headers['accept-encoding'] || headers['Accept-Encoding'] || '',
                        'accept-language': headers['accept-language'] || headers['Accept-Language'] || '',
                        'sec-ch-ua-mobile': headers['sec-ch-ua-mobile'] || headers['Sec-Ch-Ua-Mobile'] || '',
                        'sec-ch-ua-platform': headers['sec-ch-ua-platform'] || headers['Sec-Ch-Ua-Platform'] || '',
                        'x-secsdk-csrf-token': headers['x-secsdk-csrf-token'] || headers['X-Secsdk-Csrf-Token'] || '',
                    };
                }

                try {
                    const body = await response.text();
                    userPacket = { url, body, status: response.status() };
                    console.log('[Response] userPacket body length: ' + body.length);
                } catch (e) {
                    console.log('[Response] Failed to get userPacket body:', e.message);
                }
            }

            // /account/api/v1/user/account/info
            if (url.includes('/account/api/v1/user/account/info')) {
                console.log('[Response] Got: /account/api/v1/user/account/info');

                // 如果还没有 headers，从 response.request().headers() 获取
                if (Object.keys(accountInfoHeaders).length === 0) {
                    const headers = response.request().headers();
                    accountInfoHeaders[url] = {
                        accept: headers['accept'] || headers['Accept'] || '',
                        'sec-ch-ua': headers['sec-ch-ua'] || headers['Sec-Ch-Ua'] || '',
                        'user-agent': headers['user-agent'] || headers['User-Agent'] || '',
                        'sec-fetch-dest': headers['sec-fetch-dest'] || headers['Sec-Fetch-Dest'] || '',
                        'sec-fetch-mode': headers['sec-fetch-mode'] || headers['Sec-Fetch-Mode'] || '',
                        'sec-fetch-site': headers['sec-fetch-site'] || headers['Sec-Fetch-Site'] || '',
                        'accept-encoding': headers['accept-encoding'] || headers['Accept-Encoding'] || '',
                        'accept-language': headers['accept-language'] || headers['Accept-Language'] || '',
                        'sec-ch-ua-mobile': headers['sec-ch-ua-mobile'] || headers['Sec-Ch-Ua-Mobile'] || '',
                        'sec-ch-ua-platform': headers['sec-ch-ua-platform'] || headers['Sec-Ch-Ua-Platform'] || '',
                        'x-secsdk-csrf-token': headers['x-secsdk-csrf-token'] || headers['X-Secsdk-Csrf-Token'] || '',
                    };
                }

                try {
                    const body = await response.text();
                    accountPacket = { url, body, status: response.status() };
                    console.log('[Response] accountPacket body length: ' + body.length);
                } catch (e) {
                    console.log('[Response] Failed to get accountPacket body:', e.message);
                }
            }
        });

        console.log('[Info] 开始导航...');

        // Step 1: 导航到创作者中心
        console.log('Step 1: Navigating to https://creator.douyin.com/...');
        await page.goto('https://creator.douyin.com/', { waitUntil: 'domcontentloaded', timeout: 30000 });
        console.log('[Nav] Page loaded:', page.url());

        // Step 2: 显示提示浮层
        console.log('Step 2: Showing tip overlay...');
        await page.evaluate(() => {
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
        });

        // Step 3: 无限等待用户扫码登录
        console.log('Step 3: Waiting for QR code login (no timeout)...');
        console.log('[Info] Please scan the QR code with Douyin app');

        // 使用 waitForURL 等待 URL 匹配模式
        try {
            await page.waitForURL('**/creator-micro/**', { timeout: 0 });
            console.log('[OK] Login detected! URL:', page.url());
        } catch (e) {
            console.log('[Timeout] Waiting for login redirect...');
            throw e;
        }

        // Step 4: 等待页面完全加载
        console.log('Step 4: Waiting for full page load...');
        await page.waitForLoadState('networkidle');
        console.log('[OK] Network idle');

        // 移除提示浮层
        await page.evaluate(() => {
            const tip = document.getElementById('amm-tip-overlay');
            if (tip) tip.remove();
        });

        // 等待一下让所有 API 调用完成
        console.log('Step 5: Waiting for API calls to complete...');
        await page.waitForTimeout(3000);

        // Step 6: 从监听的响应中提取用户信息（分开处理，与 Python 逻辑一致）
        console.log('Step 6: Extracting user info from captured API responses...');

        let nickname = '抖音用户';
        let avatar_url = '';
        let uid = '';

        // 解析 userPacket - /web/api/media/user/info
        if (userPacket) {
            try {
                const parsed = JSON.parse(userPacket.body);
                if (parsed?.user) {
                    uid = parsed.user.uid || uid;
                    nickname = parsed.user.nickname || nickname;
                    if (parsed.user.avatar_thumb?.url_list?.[0]) {
                        avatar_url = parsed.user.avatar_thumb.url_list[0];
                    }
                    console.log('[Info] userPacket: uid=' + uid + ', nickname=' + nickname);
                }
            } catch (e) {
                console.log('[Error] Parse userPacket failed:', e.message);
            }
        } else {
            console.log('[Warn] userPacket is null');
        }

        // 解析 accountPacket - /account/api/v1/user/account/info
        if (accountPacket) {
            try {
                const parsed = JSON.parse(accountPacket.body);
                if (parsed?.data?.user_info) {
                    const user = parsed.data.user_info;
                    if (nickname === '抖音用户') {
                        nickname = user.display_name || user.nickname || user.name || nickname;
                    }
                    if (!avatar_url && user.avatar_url) {
                        avatar_url = user.avatar_url;
                    }
                    console.log('[Info] accountPacket: nickname=' + nickname);
                }
            } catch (e) {
                console.log('[Error] Parse accountPacket failed:', e.message);
            }
        } else {
            console.log('[Warn] accountPacket is null');
        }

        // 检查是否获取到用户信息
        if (!nickname || nickname === '抖音用户') {
            console.log('[Warn] Could not get nickname from APIs');
        }
        if (!avatar_url) {
            console.log('[Warn] Could not get avatar_url from APIs');
        }
        if (!uid) {
            console.log('[Warn] Could not get uid from APIs');
        }

        // Step 7: 提取凭证数据
        console.log('Step 7: Extracting credentials...');

        const cookies = await page.context().cookies();
        console.log('[Info] Cookies captured:', cookies.length);

        const localStorageScript = await page.evaluate(() => {
            const items = {};
            const keys = [
                'security-sdk/s_sdk_cert_key',
                'security-sdk/s_sdk_crypt_sdk',
                'security-sdk/s_sdk_pri_key',
                'security-sdk/s_sdk_pub_key',
                'security-sdk/s_sdk_server_cert_key',
                'sec_user_id',
                'sessionid'
            ];
            for (const key of keys) {
                const value = localStorage.getItem(key);
                if (value) items[key] = value;
            }
            return items;
        });

        // 构建结果
        const sec_uid = localStorageScript['sec_user_id'] || '';

        // 提取请求 headers（用于后续发布视频）
        // 与 Python 逻辑一致：从 accountPacket（即 /account/api/v1/user/account/info 的请求）获取 headers
        let request_headers = {};
        // 优先使用 accountInfoHeaders（来自 /account/api/v1/user/account/info 的请求）
        const accountInfoUrl = Object.keys(accountInfoHeaders)[0];
        if (accountInfoUrl && accountInfoHeaders[accountInfoUrl]) {
            request_headers = accountInfoHeaders[accountInfoUrl];
        } else {
            // 如果没有，从 userInfoHeaders 获取
            const userInfoUrl = Object.keys(userInfoHeaders)[0];
            if (userInfoUrl && userInfoHeaders[userInfoUrl]) {
                request_headers = userInfoHeaders[userInfoUrl];
            }
        }

        console.log('[Info] Request headers captured for publishing');
        console.log('[Info] accept: ' + (request_headers.accept || 'empty').substring(0, 100));
        console.log('[Info] cookie length: ' + (request_headers.cookie || '').length);
        console.log('[Info] user-agent: ' + (request_headers['user-agent'] || 'empty').substring(0, 100));
        console.log('[Info] x-secsdk-csrf-token: ' + (request_headers['x-secsdk-csrf-token'] || 'empty').substring(0, 100));
        console.log('[Info] referer: ' + (request_headers.referer || 'empty').substring(0, 100));

        console.log('[Info] uid: ' + uid);
        console.log('[Info] sec_uid: ' + (sec_uid || 'empty').substring(0, 50));
        console.log('[Info] nickname: ' + nickname);

        const result = {
            step: 'completed',
            message: '授权成功！账号: ' + nickname,
            url: page.url(),
            nickname: nickname,
            avatar_url: avatar_url,
            uid: uid,
            cookie: cookies.map(c => c.name + '=' + c.value).join('; '),
            local_storage: JSON.stringify(Object.entries(localStorageScript).map(([key, value]) => ({ key, value }))),
            sec_uid: sec_uid,
            request_headers: request_headers,
            need_poll: false
        };

        fs.writeFileSync(OUTPUT_FILE, JSON.stringify(result, null, 2));
        console.log('[OK] Result saved to:', OUTPUT_FILE);
        console.log('[OK] Authorization completed! Nickname:', nickname);

    } catch (error) {
        console.error('[Error]', error.message);
        console.error('[Error] Stack:', error.stack);
        const result = {
            step: 'failed',
            message: error.message || '操作被取消',
            need_poll: false
        };
        fs.writeFileSync(OUTPUT_FILE, JSON.stringify(result, null, 2));
        console.log('[Error] Result saved to:', OUTPUT_FILE);
    } finally {
        if (browser) {
            await browser.close();
            console.log('Browser closed');
        }
    }
}

main().catch(error => {
    console.error('Fatal error:', error);
    process.exit(1);
});
