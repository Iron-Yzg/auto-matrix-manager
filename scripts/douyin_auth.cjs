const { chromium } = require('playwright');

// 日志控制：设为 false 则只打印关键日志
const ENABLE_DEBUG_LOG = process.env.DEBUG === '1';

function log(...args) {
    if (ENABLE_DEBUG_LOG) {
        console.log('[DEBUG]', ...args);
    }
}

function info(...args) {
    console.log('[INFO]', ...args);
}

function error(...args) {
    console.error('[ERROR]', ...args);
}

async function main() {
    const OUTPUT_FILE = process.argv[2] || '';
    let browser = null;

    try {
        info('启动浏览器...');
        browser = await chromium.launch({
            headless: false,
            args: ['--no-sandbox', '--disable-setuid-sandbox', '--disable-dev-shm-usage']
        });

        const context = await browser.newContext({
            viewport: { width: 1280, height: 800 },
            userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36'
        });

        const page = await context.newPage();
        info('浏览器启动成功');

        // 存储捕获的数据
        let accountHeaders = null;  // /account/api/v1/user/account/info 的请求 headers
        let userData = null;         // /web/api/media/user/info 的响应数据

        // 监听 API 响应
        page.on('response', async (response) => {
            const url = response.url();

            // /account/api/v1/user/account/info - 提取请求 headers
            if (url.includes('/account/api/v1/user/account/info')) {
                log('捕获到 account 接口请求');
                accountHeaders = response.request().headers();
            }

            // /web/api/media/user/info - 提取用户信息
            if (url.includes('/web/api/media/user/info')) {
                log('捕获到 user 接口响应');
                try {
                    const body = await response.text();
                    const parsed = JSON.parse(body);
                    if (parsed.user) {
                        userData = parsed.user;
                    }
                } catch (e) {
                    error('解析用户数据失败: ' + e.message);
                }
            }
        });

        // 导航到创作者中心
        await page.goto('https://creator.douyin.com/', { waitUntil: 'domcontentloaded', timeout: 30000 });

        // 显示提示浮层
        info('请使用抖音 App 扫码登录...');
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

        // 等待登录成功跳转
        await page.waitForURL('**/creator-micro/**', { timeout: 0 });

        // 移除提示浮层
        await page.evaluate(() => {
            const tip = document.getElementById('amm-tip-overlay');
            if (tip) tip.remove();
        });

        info('登录成功');

        // 等待页面加载完成
        await page.waitForLoadState('networkidle');

        // 获取 cookies
        const cookies = await page.context().cookies();
        const cookieString = cookies.map(c => c.name + '=' + c.value).join('; ');

        // 获取 localStorage（与 Python 逻辑一致）
        const localStorageItems = await page.evaluate(() => {
            const keys = [
                'security-sdk/s_sdk_cert_key',
                'security-sdk/s_sdk_crypt_sdk',
                'security-sdk/s_sdk_pri_key',
                'security-sdk/s_sdk_pub_key',
                'security-sdk/s_sdk_server_cert_key',
                'security-sdk/s_sdk_sign_data_key/token',
                'security-sdk/s_sdk_sign_data_key/web_protect',
            ];
            const items = [];
            for (const key of keys) {
                const value = localStorage.getItem(key);
                if (value) {
                    items.push({ key, value });
                }
            }
            return items;
        });

        // 提取用户信息
        const uid = userData?.uid || '';
        const nickname = userData?.nickname || '抖音用户';
        const avatar_url = userData?.avatar_thumb?.url_list?.[0] || '';

        // 组装 third_param（与 Python 逻辑一致）
        const third_param = {
            'accept': accountHeaders?.accept || accountHeaders?.Accept || '',
            'cookie': cookieString,
            'referer': 'https://creator.douyin.com/creator-micro/content/post/video?enter_from=publish_page',
            'local_data': localStorageItems,
            'sec-ch-ua': accountHeaders?.['sec-ch-ua'] || accountHeaders?.['Sec-Ch-Ua'] || '',
            'user-agent': accountHeaders?.['user-agent'] || accountHeaders?.['User-Agent'] || '',
            'sec-fetch-dest': accountHeaders?.['sec-fetch-dest'] || accountHeaders?.['Sec-Fetch-Dest'] || '',
            'sec-fetch-mode': accountHeaders?.['sec-fetch-mode'] || accountHeaders?.['Sec-Fetch-Mode'] || '',
            'sec-fetch-site': accountHeaders?.['sec-fetch-site'] || accountHeaders?.['Sec-Fetch-Site'] || '',
            'accept-encoding': accountHeaders?.['accept-encoding'] || accountHeaders?.['Accept-Encoding'] || '',
            'accept-language': accountHeaders?.['accept-language'] || accountHeaders?.['Accept-Language'] || '',
            'sec-ch-ua-mobile': accountHeaders?.['sec-ch-ua-mobile'] || accountHeaders?.['Sec-Ch-Ua-Mobile'] || '',
            'sec-ch-ua-platform': accountHeaders?.['sec-ch-ua-platform'] || accountHeaders?.['Sec-Ch-Ua-Platform'] || '',
            'x-secsdk-csrf-token': accountHeaders?.['x-secsdk-csrf-token'] || accountHeaders?.['X-Secsdk-Csrf-Token'] || '',
        };

        // 最终结果（直接返回后端需要的格式）
        const result = {
            step: 'completed',
            message: '授权成功！账号: ' + nickname,
            third_id: uid,
            third_param: third_param,
            nickname: nickname,
            avatar_url: avatar_url,
            all_cookies: cookies,
            url: page.url(),
        };

        if (OUTPUT_FILE) {
            fs.writeFileSync(OUTPUT_FILE, JSON.stringify(result, null, 2));
            info('结果已保存');
        }

        // 输出到 stdout（供 Rust 读取）
        console.log('RESULT_JSON_START');
        console.log(JSON.stringify(result));
        console.log('RESULT_JSON_END');
        info('授权完成: ' + nickname);

    } catch (error) {
        error('错误: ' + error.message);
        console.log('RESULT_JSON_START');
        console.log(JSON.stringify({
            step: 'failed',
            message: error.message || '操作被取消'
        }));
        console.log('RESULT_JSON_END');
    } finally {
        if (browser) {
            await browser.close();
        }
    }
}

const fs = require('fs');
main().catch(error => {
    console.error('Fatal error:', error);
    process.exit(1);
});
