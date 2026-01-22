/**
 * 通用数据提取引擎脚本
 * 根据配置规则从 API 响应中提取数据
 * 用法: node generic_extractor.js <platform_id> [output_file] [--config <config_path>]
 */

const { chromium } = require('playwright');
const fs = require('fs');
const path = require('path');

// 通用的登录提示
const GENERIC_LOGIN_TIP = `
    <div style="
        position: fixed;
        top: 20px;
        left: 50%;
        transform: translateX(-50%);
        background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
        color: white;
        padding: 16px 24px;
        border-radius: 12px;
        font-size: 14px;
        font-weight: 600;
        box-shadow: 0 10px 40px rgba(102, 126, 234, 0.4);
        z-index: 99999999;
        display: flex;
        align-items: center;
        gap: 10px;
        font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    ">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
        </svg>
        <span>请使用手机App扫码登录，登录成功后页面将自动跳转</span>
    </div>
`;

// 日志控制 - info始终输出，debug需要DEBUG=1
const ENABLE_DEBUG_LOG = process.env.DEBUG === '1';

function log(...args) {
    if (ENABLE_DEBUG_LOG) console.error('[DEBUG]', ...args);
}

// info 日志始终输出，方便调试
function info(...args) {
    console.error('[INFO]', ...args);
}

function error(...args) {
    console.error('[ERROR]', ...args);
}

// 规则解析器
const RuleParser = {
    // 解析规则字符串
    parse(rule) {
        if (rule.startsWith('${api:')) {
            return this.parseApiRule(rule);
        } else if (rule.startsWith('${localStorage:')) {
            return this.parseStorageRule(rule);
        }
        return { type: 'fixed', value: rule };
    },

    // 解析 API 规则：${api:/path/to/api:type:path:to:field}
    parseApiRule(rule) {
        const content = rule.slice(6, -1); // 去掉 ${api: 和 }
        const parts = content.split(':');

        if (parts.length < 3) {
            return { type: 'fixed', value: rule };
        }

        return {
            type: 'api',
            apiPath: parts[0],
            requestType: parts[1],  // request 或 response
            extractType: parts[2],  // headers 或 body
            fieldPath: parts.slice(3).join(':')
        };
    },

    // 解析 localStorage 规则
    parseStorageRule(rule) {
        const key = rule.slice(14, -1); // 去掉 ${localStorage: 和 }
        return { type: 'localStorage', key };
    }
};

// 从 JSON 中按路径提取值
function extractJsonPath(json, pathStr) {
    const parts = pathStr.split(':');
    let current = json;

    for (const part of parts) {
        if (current === null || current === undefined) return '';
        if (typeof current === 'object') {
            if (Array.isArray(current)) {
                const idx = parseInt(part);
                if (isNaN(idx) || idx >= current.length) return '';
                current = current[idx];
            } else {
                current = current[part];
            }
        } else {
            return '';
        }
    }

    if (typeof current === 'string') return current;
    if (typeof current === 'number') return current.toString();
    if (typeof current === 'boolean') return current.toString();
    if (current === null || current === undefined) return '';
    return JSON.stringify(current);
}

// 评估规则并提取值
function evaluateRule(rule, apiData, requestHeaders) {
    const parsed = RuleParser.parse(rule);

    switch (parsed.type) {
        case 'fixed':
            return parsed.value;

        case 'api':
            // 查找匹配的 API 数据
            for (const [url, data] of Object.entries(apiData)) {
                if (url.includes(parsed.apiPath)) {
                    if (parsed.requestType === 'request' && parsed.extractType === 'headers') {
                        // 从请求头提取（HTTP头不区分大小写）
                        if (parsed.fieldPath) {
                            const lowerField = parsed.fieldPath.toLowerCase();
                            for (const [headerName, headerValue] of Object.entries(data.requestHeaders)) {
                                if (headerName.toLowerCase() === lowerField) {
                                    return headerValue;
                                }
                            }
                            return '';
                        }
                        return JSON.stringify(data.requestHeaders);
                    } else if (parsed.requestType === 'response' && parsed.extractType === 'body') {
                        // 从响应体提取
                        if (parsed.fieldPath && data.responseBody) {
                            return extractJsonPath(data.responseBody, parsed.fieldPath);
                        }
                    }
                }
            }
            return '';

        case 'localStorage':
            // localStorage 提取需要在浏览器上下文中执行
            return '';

        default:
            return '';
    }
}

// 从规则中提取 API 路径
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

// 匹配操作符执行
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
            info(`[警告] 未知的操作符: ${operator}，使用默认值(Eq)`);
            return actualStr === expectedStr;
    }
}

async function main() {
    const args = process.argv.slice(2);
    let platformId;
    let outputFile;
    let customConfigPath = null;

    // 解析参数
    for (let i = 0; i < args.length; i++) {
        const arg = args[i];
        if (arg === '--config' && i + 1 < args.length) {
            customConfigPath = args[++i];
        } else if (!arg.startsWith('--')) {
            if (!platformId) {
                platformId = arg;
            } else {
                outputFile = arg;
            }
        }
    }

    if (!platformId) {
        console.error('用法: node generic_extractor.js <platform_id> [output_file] [--config <config_path>]');
        process.exit(1);
    }

    // 加载平台配置：优先从环境变量读取，其次从文件读取
    let config;
    const ammConfig = process.env.AMM_CONFIG;
    info(`AMM_CONFIG 环境变量存在: ${!!ammConfig}, 长度: ${ammConfig ? ammConfig.length : 0}`);
    if (ammConfig) {
        try {
            config = JSON.parse(ammConfig);
            info(`========== 收到的配置 ==========`);
            info(`platform_id: ${config.platform_id}`);
            info(`platform_name: ${config.platform_name}`);
            info(`login_url: ${config.login_url}`);
            info(`login_success_mode: ${config.login_success_mode || 'url_match'}`);
            if (config.login_success_mode === 'api_match') {
                info(`login_success_api_rule: ${config.login_success_api_rule || '(未配置)'}`);
                info(`login_success_api_operator: ${config.login_success_api_operator || 'Eq'}`);
                info(`login_success_api_value: ${config.login_success_api_value || '(未配置)'}`);
            }
            info(`login_success_pattern: ${config.login_success_pattern}`);
            info(`=================================`);
        } catch (e) {
            error(`解析环境变量配置失败: ${e.message}`);
            info(`AMM_CONFIG 内容前500字符: ${ammConfig.substring(0, 500)}`);
            process.exit(1);
        }
    } else {
        // 回退到从文件读取
        let configPath;
        if (customConfigPath) {
            configPath = customConfigPath;
        } else {
            configPath = path.join(__dirname, 'configs', `${platformId}.json`);
        }

        if (fs.existsSync(configPath)) {
            try {
                config = JSON.parse(fs.readFileSync(configPath, 'utf8'));
                info(`从文件加载配置: ${config.platform_name || platformId}`);
            } catch (e) {
                error(`加载配置文件失败: ${e.message}`);
                process.exit(1);
            }
        } else {
            error(`配置文件不存在: ${configPath}`);
            process.exit(1);
        }
    }

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

        // 存储捕获的 API 数据
        const capturedApiData = {};

        // 解析提取规则，获取需要监听的 API 路径
        const rules = config.extract_rules || {};
        const userInfoRules = rules.user_info || {};  // 使用 snake_case
        const headerRules = rules.request_headers || {};  // 使用 snake_case

        const localStorageKeys = rules.local_storage || [];
        // 收集所有需要捕获的 API 路径
        const apiPaths = new Set();
        info('');
        info('========== 解析 API 路径 ==========');
        info('userInfo 规则解析:');
        for (const [key, rule] of Object.entries(userInfoRules)) {
            info(`  ${key}: "${rule}"`);
            const parsed = RuleParser.parse(rule);
            if (parsed.type === 'api') {
                apiPaths.add(parsed.apiPath);
                // info(`    -> ✅ 匹配 API 路径: "${parsed.apiPath}"`);
            } else {
                // info(`    -> ❌ 非 API 规则 (type=${parsed.type})`);
            }
        }
        info('');
        info('requestHeaders 规则解析:');
        for (const [key, rule] of Object.entries(headerRules)) {
            info(`  ${key}: "${rule}"`);
            const parsed = RuleParser.parse(rule);
            if (parsed.type === 'api') {
                apiPaths.add(parsed.apiPath);
                // info(`    -> ✅ 匹配 API 路径: "${parsed.apiPath}"`);
            } else {
                // info(`    -> ❌ 非 API 规则 (type=${parsed.type})`);
            }
        }
        info('');
        info(`==================================`);
        info(`需要监听的 API 路径列表: ${Array.from(apiPaths).join(', ')}`);
        info(`共 ${apiPaths.size} 个 API 路径`);
        info('');

        // 登录成功状态
        let loginSuccess = false;

        // 监听 API 响应
        info('========== 开始监听 API 响应 ==========');

        // 处理 API 匹配模式的登录成功检测
        const isApiMatchMode = config.login_success_mode === 'api_match';
        const loginApiRule = config.login_success_api_rule;
        const loginApiPath = isApiMatchMode ? extractApiPath(loginApiRule) : null;
        const loginOperator = config.login_success_api_operator || 'Eq';
        const loginExpectedValue = config.login_success_api_value || '';

        if (isApiMatchMode) {
            info(`[API匹配模式] 登录成功检测配置:`);
            info(`  API规则: ${loginApiRule}`);
            info(`  API路径: ${loginApiPath}`);
            info(`  操作符: ${loginOperator}`);
            info(`  期望值: ${loginExpectedValue}`);
        }

        page.on('response', async (response) => {
            const url = response.url();
            const status = response.status();

            // 检查是否是需要捕获的 API（用于提取数据）
            let matchedPath = null;
            for (const apiPath of apiPaths) {
                if (url.includes(apiPath)) {
                    matchedPath = apiPath;
                    info(`[API 匹配成功] ✅ URL="${url}" 匹配规则="${apiPath}"`);
                    break;
                }
            }

            if (matchedPath) {
                log(`[捕获 API] ${url}`);
                try {
                    const body = await response.text();
                    capturedApiData[url] = {
                        requestHeaders: response.request().headers(),
                        responseBody: JSON.parse(body)
                    };
                    info(`[API 捕获成功] ${url}`);
                    info(`  已捕获 ${Object.keys(capturedApiData).length} 个接口`);
                } catch (e) {
                    error(`解析 API 响应失败: ${e.message}`);
                }
            }
        });

        // 导航到登录页
        info(`导航到: ${config.login_url}`);
        await page.goto(config.login_url, { waitUntil: 'domcontentloaded', timeout: 30000 });

        // 显示通用的登录提示
        await page.evaluate((tip) => {
            const existing = document.getElementById('amm-tip-overlay');
            if (existing) existing.remove();

            const div = document.createElement('div');
            div.id = 'amm-tip-overlay';
            div.innerHTML = tip;
            document.body.insertBefore(div, document.body.firstChild);
        }, GENERIC_LOGIN_TIP);

        // 等待登录成功
        info('等待登录...');

        if (isApiMatchMode) {
            // API 匹配模式：使用 waitForResponse 等待登录 API
            info('[等待登录] 使用 API 匹配模式，等待检测到登录成功...');
            logSync(`[TRACE] 开始等待 API: ${loginApiPath}`);

            try {
                const response = await page.waitForResponse(
                    response => {
                        const url = response.url();
                        const status = response.status();
                        // 检查 URL 匹配和状态码
                        return url.includes(loginApiPath) && status >= 200 && status < 300;
                    },
                    { timeout: 0 } // 无限等待
                );

                logSync('[TRACE] 收到登录 API 响应');
                info(`[API登录检测] 收到响应: ${response.url()} (status: ${response.status()})`);

                // 检查 content-type
                const contentType = response.headers()['content-type'] || '';
                if (contentType.includes('application/json')) {
                    const body = await response.json();
                    const bodyStr = JSON.stringify(body);
                    info(`[API登录检测] 响应内容: ${bodyStr.substring(0, 500)}${bodyStr.length > 500 ? '...' : ''}`);

                    // 使用 RuleParser 解析规则并提取值
                    const extractedValue = evaluateRule(loginApiRule, { [response.url()]: { url: response.url(), requestHeaders: response.request().headers(), responseBody: body } }, {});

                    info(`[API登录检测] 提取的值: "${extractedValue}"`);
                    info(`[API登录检测] 比对: "${extractedValue}" ${loginOperator} "${loginExpectedValue}"`);

                    if (matchValue(extractedValue, loginOperator, loginExpectedValue)) {
                        info(`[API登录检测] ✅ 匹配成功! 检测到登录成功!`);
                        logSync('[TRACE] API 匹配成功');
                    } else {
                        info(`[API登录检测] ❌ 匹配未通过，API 返回了响应但内容不匹配`);
                        logSync('[TRACE] API 响应内容不匹配');
                    }
                } else {
                    info(`[API登录检测] 跳过: content-type 不是 JSON (${contentType})`);
                }
            } catch (e) {
                error(`[等待登录] 等待 API 响应失败: ${e.message}`);
            }
        } else {
            // URL 匹配模式：等待 URL 变化
            if (config.login_success_pattern) {
                await page.waitForURL(config.login_success_pattern, { timeout: 0 });
            } else {
                // 默认等待 creator-micro 页面
                await page.waitForURL('**/creator-micro/**', { timeout: 0 });
            }
            info('✅ 登录成功 (URL 匹配)');
        }

        // 移除提示
        await page.evaluate(() => {
            const tip = document.getElementById('amm-tip-overlay');
            if (tip) tip.remove();
        });

        // 等待页面加载
        await page.waitForLoadState('networkidle');

        // 获取 cookies
        const cookies = await page.context().cookies();
        const cookieString = cookies.map(c => c.name + '=' + c.value).join('; ');

        // 获取 localStorage (使用已解析的 localStorageKeys)
        const localStorageItems = await page.evaluate((keys) => {
            const items = [];
            for (const key of keys) {
                const value = localStorage.getItem(key);
                if (value) items.push({ key, value });
            }
            return items;
        }, localStorageKeys);

        // 构建 requestHeaders（用于 cookie 提取）
        const requestHeaders = capturedApiData[Object.keys(capturedApiData)[0]]?.requestHeaders || {};

        // 使用规则提取数据
        const extractionRules = config.extract_rules || {};
        const extractedData = {};

        info('--- 提取用户信息 (user_info) ---');
        // 提取用户信息
        if (extractionRules.user_info) {
            for (const [key, rule] of Object.entries(extractionRules.user_info)) {
                extractedData[key] = evaluateRule(rule, capturedApiData, requestHeaders);
                // info(`  ${key}: "${rule}"`);
                // info(`    -> 提取结果: "${extractedData[key]}"`);
                // if (extractedData[key] === '') {
                //     info(`    -> ⚠️ 结果为空! 检查 API 数据是否已捕获`);
                // }
            }
        }

        info('--- 提取请求头 (requestHeaders) ---');
        // 提取请求头
        if (extractionRules.request_headers) {
            for (const [key, rule] of Object.entries(extractionRules.request_headers)) {
                extractedData[key] = evaluateRule(rule, capturedApiData, requestHeaders);
                // info(`  ${key}: "${rule}"`);
                // info(`    -> 提取结果: "${extractedData[key]}"`);
            }
        }


        info('--- 提取cookie ---');
        // 处理 cookie 提取规则
        let cookie = cookieString;
        if (extractionRules.cookie) {
            const cookieRule = extractionRules.cookie;
            if (cookieRule.source === 'from_api' && cookieRule.apiPath) {
                for (const [url, data] of Object.entries(capturedApiData)) {
                    if (url.includes(cookieRule.apiPath)) {
                        const headerName = cookieRule.headerName || 'cookie';
                        cookie = data.requestHeaders[headerName] || cookieString;
                        break;
                    }
                }
            }
        }

        // 打印提取结果
        info('');
        info('========== 提取结果汇总 ==========');
        info(`capturedApiData 接口数: ${Object.keys(capturedApiData).length}`);
        for (const [url, data] of Object.entries(capturedApiData)) {
            info(`  ${url}`);
        }
        info('');
        info('extractedData 空字段检查:');
        for (const [key, value] of Object.entries(extractedData)) {
            if (value === '') {
                info(`  ⚠️ ${key} 为空`);
            } else {
                info(`  ✅ ${key}: ${value.substring(0, 50)}${value.length > 50 ? '...' : ''}`);
            }
        }
        info('');
        info(`localStorageItems长度: ${localStorageItems.length}`);
        info(`cookie 长度: ${cookie.length}`);
        info(`cookie 前100字符: ${cookie.substring(0, 100)}...`);
        info('===================================');

        // 构建结果 - 保持配置结构，只替换规则
        const result = {
            step: 'completed',
            message: `授权成功！账号: ${extractedData.nickname || config.platform_name + '用户'}`,
            url: page.url(),
            // cookie - 优先从 API 提取，失败则回退到浏览器 cookie
            cookie: (() => {
                const cookieRule = extractionRules.cookie;
                // 如果没有配置规则，使用浏览器 cookie
                if (!cookieRule) {
                    return cookieString;
                }
                if (typeof cookieRule === 'string') return cookieRule;
                if (cookieRule.source === 'from_api') {
                    for (const [url, data] of Object.entries(capturedApiData)) {
                        if (cookieRule.apiPath && url.includes(cookieRule.apiPath)) {
                            const headerName = cookieRule.headerName || 'cookie';
                            const apiCookie = data.requestHeaders[headerName];
                            if (apiCookie) {
                                return apiCookie;
                            }
                        }
                    }
                }
                return cookieString;
            })(),
            // local_storage - 保持配置结构
            local_storage: localStorageItems,
            // request_headers - 保持配置结构，替换规则
            request_headers: (() => {
                const headers = {};
                for (const [key, rule] of Object.entries(extractionRules.request_headers || {})) {
                    if (typeof rule === 'string' && rule.startsWith('${api:')) {
                        headers[key] = evaluateRule(rule, capturedApiData, requestHeaders);
                    } else {
                        headers[key] = rule;
                    }
                }
                return headers;
            })(),
            // user_info - 保持配置结构，替换规则
            user_info: (() => {
                const userInfo = {};
                for (const [key, rule] of Object.entries(extractionRules.user_info || {})) {
                    if (typeof rule === 'string' && rule.startsWith('${api:')) {
                        userInfo[key] = evaluateRule(rule, capturedApiData, requestHeaders);
                    } else {
                        userInfo[key] = rule;
                    }
                }
                return userInfo;
            })(),
        };

        if (outputFile) {
            fs.writeFileSync(outputFile, JSON.stringify(result, null, 2));
            info(`结果已保存到: ${outputFile}`);
        }

        // 输出结果 - 使用标记格式以便Rust解析
        console.log('RESULT_JSON_START');
        console.log(JSON.stringify(result));
        console.log('RESULT_JSON_END');
        info(`授权完成: ${result.user_info.nickname}`);

    } catch (err) {
        error(`错误: ${err.message}`);
        console.log('RESULT_JSON_START');
        console.log(JSON.stringify({
            step: 'failed',
            message: err.message || '操作被取消'
        }));
        console.log('RESULT_JSON_END');
    } finally {
        if (browser) {
            await browser.close();
        }
    }
}

main().catch(err => {
    console.error('Fatal error:', err);
    process.exit(1);
});
