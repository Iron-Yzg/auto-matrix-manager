/**
 * Generic Browser Extractor Script
 * 通用浏览器数据提取引擎脚本
 *
 * 功能：
 * 1. 自动登录检测（支持 URL 匹配和 API 响应匹配两种模式）
 * 2. 自动提取用户数据（Cookie、请求头、LocalStorage）
 * 3. 支持登录成功后跳转页面并重新提取数据
 */

const { chromium } = require('playwright');

/**
 * 从环境变量获取配置
 */
function getConfig() {
    const configJson = process.env.AMM_CONFIG;
    if (!configJson) {
        throw new Error('AMM_CONFIG 环境变量未设置');
    }
    try {
        return JSON.parse(configJson);
    } catch (e) {
        throw new Error(`解析配置失败: ${e.message}`);
    }
}

/**
 * 输出日志到 stderr（被 Rust 端捕获）
 */
function log(message) {
    console.error(`[GenericExtractor] ${message}`);
}

/**
 * 输出进度信息
 */
function progress(message) {
    console.error(`[Progress] ${message}`);
}

/**
 * 输出错误信息
 */
function error(message) {
    console.error(`[Error] ${message}`);
}

/**
 * 匹配操作符执行
 */
function matchValue(actual, operator, expected) {
    const actualStr = String(actual);
    const expectedStr = String(expected);

    switch (operator) {
        case 'Eq':
        case 'Equals':
        case '=':
            return actualStr === expectedStr;
        case 'Neq':
        case 'NotEquals':
        case '!=':
            return actualStr !== expectedStr;
        case 'Contains':
            return actualStr.includes(expectedStr);
        case 'NotContains':
            return !actualStr.includes(expectedStr);
        case 'StartsWith':
            return actualStr.startsWith(expectedStr);
        case 'EndsWith':
            return actualStr.endsWith(expectedStr);
        case 'Gt':
        case '>':
            return parseFloat(actualStr) > parseFloat(expectedStr);
        case 'Lt':
        case '<':
            return parseFloat(actualStr) < parseFloat(expectedStr);
        case 'Gte':
        case '>=':
            return parseFloat(actualStr) >= parseFloat(expectedStr);
        case 'Lte':
        case '<=':
            return parseFloat(actualStr) <= parseFloat(expectedStr);
        default:
            log(`未知的操作符: ${operator}，使用默认值(Eq)`);
            return actualStr === expectedStr;
    }
}

/**
 * 从规则中提取 API 路径
 * 规则格式: ${api:/path/to/api:response:body:user:uid}
 */
function extractApiPath(rule) {
    if (rule && rule.startsWith('${api:')) {
        const content = rule.slice(6, -1); // 去掉 ${api: 和 }
        const colonIndex = content.indexOf(':');
        if (colonIndex > 0) {
            return content.substring(0, colonIndex);
        }
    }
    return null;
}

/**
 * 从 JSON 中按路径提取值
 * 路径格式: body:user:uid 或 headers:cookie
 */
function extractJsonPath(json, path) {
    const parts = path.split(':');
    let current = json;

    for (const part of parts) {
        if (current === undefined || current === null) {
            return '';
        }
        if (typeof current === 'object') {
            if (Array.isArray(current)) {
                const index = parseInt(part, 10);
                if (isNaN(index) || index >= current.length) {
                    return '';
                }
                current = current[index];
            } else {
                current = current[part];
            }
        } else {
            return '';
        }
    }

    if (current === undefined || current === null) {
        return '';
    }
    if (typeof current === 'string') {
        return current;
    }
    if (typeof current === 'number') {
        return String(current);
    }
    if (typeof current === 'boolean') {
        return String(current);
    }
    return JSON.stringify(current);
}

/**
 * 评估规则并提取值
 */
function evaluateRule(rule, capturedApiData, capturedRequestHeaders) {
    if (!rule) return '';

    if (rule.startsWith('${api:')) {
        const content = rule.slice(6, -1);
        const parts = content.split(':');

        if (parts.length < 3) {
            return '';
        }

        const apiPath = parts[0];
        const requestType = parts[1]; // request 或 response
        const extractType = parts[2]; // headers 或 body
        const fieldPath = parts.length > 3 ? parts.slice(3).join(':') : '';

        // 查找匹配的 API 数据
        for (const [url, data] of Object.entries(capturedApiData)) {
            if (url.includes(apiPath)) {
                if (requestType === 'request' && extractType === 'headers') {
                    if (data.requestHeaders && data.requestHeaders[fieldPath]) {
                        return data.requestHeaders[fieldPath];
                    }
                } else if (requestType === 'response' && extractType === 'body') {
                    if (data.responseBody) {
                        return extractJsonPath(data.responseBody, fieldPath);
                    }
                }
            }
        }

        return '';
    } else if (rule.startsWith('${localStorage:')) {
        const key = rule.slice(14, -1);
        return `\${localStorage:${key}}`; // 返回占位符，由 Rust 端处理
    }

    // 固定值
    return rule;
}

/**
 * 主函数
 */
async function main() {
    let browser = null;
    let context = null;
    let page = null;

    try {
        // 获取配置
        const config = getConfig();
        log(`开始执行, platform: ${config.platform_id}, login_url: ${config.login_url}`);
        log(`login_success_mode: ${config.login_success_mode || 'url_match'}`);
        if (config.login_success_mode === 'api_match') {
            log(`login_success_api_rule: ${config.login_success_api_rule || '(未配置)'}`);
            log(`login_success_api_operator: ${config.login_success_api_operator || 'Eq'}`);
            log(`login_success_api_value: ${config.login_success_api_value || '(未配置)'}`);
        }

        // 捕获的数据
        const capturedApiData = {};
        const capturedRequestHeaders = {};
        const capturedLocalStorage = {};

        // 登录成功状态
        let loginSuccess = false;
        let loginSuccessTime = null;

        // 启动浏览器
        progress('正在启动浏览器...');
        const browsersPath = process.env.PLAYWRIGHT_BROWSERS_PATH || '';

        browser = await chromium.launch({
            headless: true,
            args: ['--no-sandbox', '--disable-setuid-sandbox']
        });

        context = await browser.newContext({
            viewport: { width: 1280, height: 800 },
            userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36'
        });

        page = await context.newPage();

        // 拦截请求头
        page.on('request', async (request) => {
            const url = request.url();
            const headers = request.headers();

            // 捕获请求头
            capturedRequestHeaders[url] = headers;
        });

        // 拦截 API 响应
        page.on('response', async (response) => {
            const url = response.url();
            const status = response.status();

            // 只处理成功的 JSON 响应
            if (status >= 200 && status < 300) {
                const contentType = response.headers()['content-type'] || '';
                if (contentType.includes('application/json')) {
                    try {
                        const body = await response.json();
                        capturedApiData[url] = {
                            url,
                            requestHeaders: capturedRequestHeaders[url] || {},
                            responseBody: body
                        };
                        log(`捕获 API 响应: ${url}`);
                    } catch (e) {
                        // 忽略解析失败的响应
                    }
                }
            }
        });

        // 监听 LocalStorage 变化
        page.on('framenavigated', async (frame) => {
            if (frame === page.mainFrame()) {
                try {
                    const localStorage = await page.evaluate(() => {
                        const items = {};
                        for (let i = 0; i < localStorage.length; i++) {
                            const key = localStorage.key(i);
                            items[key] = localStorage.getItem(key);
                        }
                        return items;
                    });
                    Object.assign(capturedLocalStorage, localStorage);
                } catch (e) {
                    // 忽略 LocalStorage 读取失败
                }
            }
        });

        // 监听 URL 变化
        page.on('framenavigated', async (frame) => {
            if (frame === page.mainFrame()) {
                const currentUrl = page.url();
                log(`导航到: ${currentUrl}`);

                // 检查登录成功（URL 匹配模式）
                if (!loginSuccess && config.login_success_mode !== 'api_match') {
                    const pattern = config.login_success_pattern;
                    if (pattern && matchUrlPattern(currentUrl, pattern)) {
                        log(`URL 匹配成功: ${currentUrl}`);
                        loginSuccess = true;
                        loginSuccessTime = new Date();
                    }
                }
            }
        });

        // 监听 API 响应变化（API 匹配模式）
        page.on('response', async (response) => {
            if (loginSuccess) return; // 已登录成功，不再检查

            if (config.login_success_mode === 'api_match') {
                const apiRule = config.login_success_api_rule;
                if (!apiRule) return;

                const apiPath = extractApiPath(apiRule);
                if (!apiPath) return;

                const url = response.url();
                const status = response.status();

                // 只处理成功的响应
                if (status < 200 || status >= 300) return;

                if (url.includes(apiPath)) {
                    log(`[API响应] ${url} (status: ${status})`);
                    const contentType = response.headers()['content-type'] || '';
                    if (contentType.includes('application/json')) {
                        try {
                            const body = await response.json();
                            const extractedValue = evaluateRule(apiRule, { [url]: { url, requestHeaders: {}, responseBody: body } }, {});
                            const operator = config.login_success_api_operator || 'Eq';
                            const expectedValue = config.login_success_api_value || '';

                            // 打印API响应的关键部分（用于调试）
                            const bodyStr = JSON.stringify(body);
                            log(`[API响应内容] ${bodyStr.substring(0, 500)}${bodyStr.length > 500 ? '...' : ''}`);

                            log(`[API匹配检查] apiPath="${apiPath}", extracted="${extractedValue}", operator=${operator}, expected="${expectedValue}"`);

                            if (matchValue(extractedValue, operator, expectedValue)) {
                                log(`[API匹配成功] 检测到登录成功!`);
                                loginSuccess = true;
                                loginSuccessTime = new Date();
                            } else {
                                log(`[API匹配未通过] extracted="${extractedValue}" ${operator} "${expectedValue}" => false`);
                            }
                        } catch (e) {
                            log(`[API错误] 解析响应失败: ${e.message}`);
                        }
                    } else {
                        log(`[API跳过] content-type 不是 JSON: ${contentType}`);
                    }
                }
            }
        });

        // 打开登录页
        progress('正在打开登录页...');
        await page.goto(config.login_url, { waitUntil: 'networkidle', timeout: 30000 });
        log(`已打开登录页: ${config.login_url}`);

        // 等待用户登录
        progress('等待登录...');

        // 等待登录成功或超时
        const loginSuccessPattern = config.login_success_pattern;
        const redirectUrl = config.redirect_url;
        const maxWaitTime = 120000; // 2 分钟超时
        const checkInterval = 1000; // 1 秒检查一次
        let waitedTime = 0;

        while (waitedTime < maxWaitTime) {
            await new Promise(resolve => setTimeout(resolve, checkInterval));
            waitedTime += checkInterval;

            const currentUrl = page.url();

            // 检查是否已登录（URL 匹配）
            if (!loginSuccess && loginSuccessPattern && matchUrlPattern(currentUrl, loginSuccessPattern)) {
                log(`检测到登录成功 (URL 匹配): ${currentUrl}`);
                loginSuccess = true;
                loginSuccessTime = new Date();
                break;
            }

            // 如果已登录且配置了跳转页，则跳转到跳转页
            if (loginSuccess && redirectUrl && !currentUrl.includes(redirectUrl.split('?')[0])) {
                progress('正在跳转到目标页面...');
                log(`跳转到: ${redirectUrl}`);

                // 清空之前捕获的数据，准备重新提取
                for (const key in capturedApiData) {
                    delete capturedApiData[key];
                }

                await page.goto(redirectUrl, { waitUntil: 'networkidle', timeout: 30000 });

                // 等待数据重新提取
                await new Promise(resolve => setTimeout(resolve, 2000));
                break;
            }

            // 显示等待状态
            if (waitedTime % 30000 === 0) {
                progress(`已等待 ${waitedTime / 1000} 秒...`);
            }
        }

        if (!loginSuccess) {
            throw new Error('登录超时，未检测到登录成功');
        }

        // 额外等待以确保数据完整
        await new Promise(resolve => setTimeout(resolve, 2000));

        // 提取最终数据
        progress('正在提取数据...');
        log(`最终 URL: ${page.url()}`);

        // 获取 Cookie
        const cookie = await page.context().cookies();
        const cookieString = cookie.map(c => `${c.name}=${c.value}`).join('; ');

        // 获取最终的 LocalStorage
        const localStorage = await page.evaluate(() => {
            const items = {};
            for (let i = 0; i < localStorage.length; i++) {
                const key = localStorage.key(i);
                items[key] = localStorage.getItem(key);
            }
            return items;
        });

        // 提取用户信息
        const extractRules = config.extract_rules || {};
        const userInfoRules = extractRules.user_info || {};
        const requestHeaderRules = extractRules.request_headers || {};

        const userInfo = {};
        for (const [key, rule] of Object.entries(userInfoRules)) {
            const value = evaluateRule(rule, capturedApiData, capturedRequestHeaders);
            if (value) {
                userInfo[key] = value;
            }
        }

        const requestHeaders = {};
        for (const [key, rule] of Object.entries(requestHeaderRules)) {
            const value = evaluateRule(rule, capturedApiData, capturedRequestHeaders);
            if (value) {
                requestHeaders[key] = value;
            }
        }

        // 构建结果
        const result = {
            success: true,
            step: 'completed',
            message: '提取完成',
            nickname: userInfo.nickname || '',
            avatarUrl: userInfo.avatar_url || '',
            thirdId: userInfo.third_id || '',
            secUid: userInfo.sec_uid || '',
            cookie: cookieString,
            localStorage: JSON.stringify(localStorage),
            requestHeaders: JSON.stringify(requestHeaders),
            currentUrl: page.url(),
            capturedApiDataCount: Object.keys(capturedApiData).length
        };

        // 输出结果
        console.log('RESULT_JSON_START');
        console.log(JSON.stringify(result));
        console.log('RESULT_JSON_END');

        log('执行完成');

    } catch (e) {
        error(`执行失败: ${e.message}`);

        // 输出错误结果
        const errorResult = {
            success: false,
            step: 'error',
            message: e.message,
            error: e.message,
            nickname: '',
            avatarUrl: '',
            thirdId: '',
            secUid: '',
            cookie: '',
            localStorage: '{}',
            requestHeaders: '{}',
            currentUrl: page ? page.url() : ''
        };

        console.log('RESULT_JSON_START');
        console.log(JSON.stringify(errorResult));
        console.log('RESULT_JSON_END');

    } finally {
        // 关闭浏览器
        if (browser) {
            await browser.close();
        }
    }
}

/**
 * 匹配 URL 模式（支持 glob 格式）
 */
function matchUrlPattern(url, pattern) {
    if (!pattern || !url) return false;

    // 精确匹配
    if (pattern === url) return true;

    // 处理 glob 模式
    // 转换 glob 模式为正则表达式
    let regexStr = pattern
        .replace(/\./g, '\\.')
        .replace(/\*/g, '.*')
        .replace(/\?/g, '.');

    // 处理 ** 通配符
    regexStr = regexStr.replace(/\.\*\*/g, '.*');
    regexStr = '^' + regexStr + '$';

    try {
        const regex = new RegExp(regexStr);
        return regex.test(url);
    } catch (e) {
        log(`正则表达式错误: ${e.message}`);
        return url.includes(pattern);
    }
}

// 运行主函数
main().catch(e => {
    console.error(`未捕获的错误: ${e.message}`);
    process.exit(1);
});
