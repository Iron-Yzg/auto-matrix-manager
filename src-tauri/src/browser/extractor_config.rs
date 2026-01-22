// Data Extractor Configuration
// 数据提取器配置 - 支持通用浏览器数据提取引擎

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 匹配操作符
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MatchOperator {
    /// 等于
    Eq,
    /// 不等于
    Neq,
    /// 大于
    Gt,
    /// 小于
    Lt,
    /// 大于等于
    Gte,
    /// 小于等于
    Lte,
    /// 包含
    Contains,
    /// 不包含
    NotContains,
    /// 开头是
    StartsWith,
    /// 结尾是
    EndsWith,
}

impl Default for MatchOperator {
    fn default() -> Self {
        MatchOperator::Eq
    }
}

impl MatchOperator {
    /// 执行匹配
    pub fn match_value(&self, actual: &str, expected: &str) -> bool {
        match self {
            MatchOperator::Eq => actual == expected,
            MatchOperator::Neq => actual != expected,
            MatchOperator::Gt => actual.parse::<f64>().map(|a| a > expected.parse().unwrap_or(0.0)).unwrap_or(false),
            MatchOperator::Lt => actual.parse::<f64>().map(|a| a < expected.parse().unwrap_or(0.0)).unwrap_or(false),
            MatchOperator::Gte => actual.parse::<f64>().map(|a| a >= expected.parse().unwrap_or(0.0)).unwrap_or(false),
            MatchOperator::Lte => actual.parse::<f64>().map(|a| a <= expected.parse().unwrap_or(0.0)).unwrap_or(false),
            MatchOperator::Contains => actual.contains(expected),
            MatchOperator::NotContains => !actual.contains(expected),
            MatchOperator::StartsWith => actual.starts_with(expected),
            MatchOperator::EndsWith => actual.ends_with(expected),
        }
    }
}

/// 平台数据提取配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformExtractorConfig {
    /// 平台 ID
    pub platform_id: String,
    /// 平台名称
    pub platform_name: String,
    /// 登录页 URL
    pub login_url: String,
    /// 登录成功检测模式
    pub login_success_mode: String, // "url_match" 或 "api_match"
    /// 登录成功页 URL 模式（支持 glob 格式）- 当 login_success_mode 为 url_match 时使用
    pub login_success_pattern: String,
    /// API 响应匹配规则 - 当 login_success_mode 为 api_match 时使用
    pub login_success_api_rule: Option<String>,
    /// API 响应匹配操作符 - 当 login_success_mode 为 api_match 时使用
    pub login_success_api_operator: Option<String>,
    /// API 响应匹配值 - 当 login_success_mode 为 api_match 时使用
    pub login_success_api_value: Option<String>,
    /// 登录成功后跳转页（可选）
    pub redirect_url: Option<String>,
    /// 提取规则配置
    pub extract_rules: ExtractRules,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
}

/// 提取规则配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractRules {
    /// 用户基本信息提取规则
    pub user_info: HashMap<String, String>,
    /// 请求头提取规则
    pub request_headers: HashMap<String, String>,
    /// LocalStorage 提取规则
    pub local_storage: Vec<String>,
    /// Cookie 提取规则
    pub cookie: Option<CookieRule>,
}

/// Cookie 提取规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieRule {
    /// 来源：from_browser（从浏览器获取） 或 from_api（从 API 请求头获取）
    pub source: String,
    /// API 路径（当 source 为 from_api 时需要）
    pub api_path: Option<String>,
    /// 头名称（当 source 为 from_api 时需要）
    pub header_name: Option<String>,
}

/// 默认配置
impl Default for PlatformExtractorConfig {
    fn default() -> Self {
        Self {
            platform_id: "douyin".to_string(),
            platform_name: "抖音".to_string(),
            login_url: "https://creator.douyin.com/".to_string(),
            login_success_mode: "url_match".to_string(),
            login_success_pattern: "**/creator-micro/**".to_string(),
            login_success_api_rule: None,
            login_success_api_operator: None,
            login_success_api_value: None,
            redirect_url: None,
            extract_rules: ExtractRules::default(),
            created_at: chrono::Local::now().to_string(),
            updated_at: chrono::Local::now().to_string(),
        }
    }
}

impl Default for ExtractRules {
    fn default() -> Self {
        Self {
            user_info: HashMap::from([
                ("uid".to_string(), "${api:/web/api/media/user/info:response:body:user:uid}".to_string()),
                ("nickname".to_string(), "${api:/web/api/media/user/info:response:body:user:nickname}".to_string()),
                ("avatar_url".to_string(), "${api:/web/api/media/user/info:response:body:user:avatar_thumb:url_list:0}".to_string()),
            ]),
            request_headers: HashMap::from([
                ("accept".to_string(), "${api:/account/api/v1/user/account/info:request:headers:accept}".to_string()),
                ("user-agent".to_string(), "${api:/account/api/v1/user/account/info:request:headers:user-agent}".to_string()),
                ("sec-ch-ua".to_string(), "${api:/account/api/v1/user/account/info:request:headers:sec-ch-ua}".to_string()),
                ("sec-ch-ua-mobile".to_string(), "${api:/account/api/v1/user/account/info:request:headers:sec-ch-ua-mobile}".to_string()),
                ("sec-ch-ua-platform".to_string(), "${api:/account/api/v1/user/account/info:request:headers:sec-ch-ua-platform}".to_string()),
                ("sec-fetch-dest".to_string(), "${api:/account/api/v1/user/account/info:request:headers:sec-fetch-dest}".to_string()),
                ("sec-fetch-mode".to_string(), "${api:/account/api/v1/user/account/info:request:headers:sec-fetch-mode}".to_string()),
                ("sec-fetch-site".to_string(), "${api:/account/api/v1/user/account/info:request:headers:sec-fetch-site}".to_string()),
                ("accept-encoding".to_string(), "${api:/account/api/v1/user/account/info:request:headers:accept-encoding}".to_string()),
                ("accept-language".to_string(), "${api:/account/api/v1/user/account/info:request:headers:accept-language}".to_string()),
                ("x-secsdk-csrf-token".to_string(), "${api:/account/api/v1/user/account/info:request:headers:x-secsdk-csrf-token}".to_string()),
            ]),
            local_storage: vec![
                "security-sdk/s_sdk_cert_key".to_string(),
                "security-sdk/s_sdk_crypt_sdk".to_string(),
                "security-sdk/s_sdk_pri_key".to_string(),
                "security-sdk/s_sdk_pub_key".to_string(),
                "security-sdk/s_sdk_server_cert_key".to_string(),
                "security-sdk/s_sdk_sign_data_key/token".to_string(),
                "security-sdk/s_sdk_sign_data_key/web_protect".to_string(),
            ],
            cookie: Some(CookieRule {
                source: "from_browser".to_string(),
                api_path: None,
                header_name: None,
            }),
        }
    }
}

/// 规则语法解析器
pub struct RuleParser;

/// 规则语法：
/// - `${api:/path/to/api:response:body:user:uid}` - 从 API 响应 body 提取
/// - `${api:/path/to/api:request:headers:cookie}` - 从 API 请求 headers 提取
/// - `${localStorage:key}` - 从 localStorage 提取
/// - 固定值直接返回
impl RuleParser {
    /// 解析规则字符串
    pub fn parse(rule: &str) -> Rule {
        if rule.starts_with("${api:") {
            Self::parse_api_rule(rule)
        } else if rule.starts_with("${localStorage:") {
            Self::parse_storage_rule(rule)
        } else {
            Rule::FixedValue(rule.to_string())
        }
    }

    /// 解析 API 规则：${api:/path/to/api:type:path:to:field}
    fn parse_api_rule(rule: &str) -> Rule {
        // 去掉 ${api: 和 }
        let content = rule.trim_start_matches("${api:").trim_end_matches('}');
        let parts: Vec<&str> = content.split(':').collect();

        if parts.len() < 3 {
            return Rule::FixedValue(rule.to_string());
        }

        let api_path = parts[0].to_string();
        let request_type = parts[1].to_string(); // request 或 response
        let extract_type = parts[2].to_string(); // headers 或 body

        // 提取字段路径
        let field_path = if parts.len() > 3 {
            parts[3..].join(":")
        } else {
            String::new()
        };

        Rule::ApiExtract(ApiExtractRule {
            api_path,
            request_type, // request 或 response
            extract_type, // headers 或 body
            field_path,
        })
    }

    /// 解析 localStorage 规则：${localStorage:key}
    fn parse_storage_rule(rule: &str) -> Rule {
        let key = rule.trim_start_matches("${localStorage:").trim_end_matches('}');
        Rule::LocalStorageExtract(key.to_string())
    }
}

/// 规则类型
#[derive(Debug, Clone)]
pub enum Rule {
    /// 固定值
    FixedValue(String),
    /// 从 API 提取
    ApiExtract(ApiExtractRule),
    /// 从 localStorage 提取
    LocalStorageExtract(String),
}

/// API 提取规则
#[derive(Debug, Clone)]
pub struct ApiExtractRule {
    pub api_path: String,      // API 路径
    pub request_type: String,  // request 或 response
    pub extract_type: String,  // headers 或 body
    pub field_path: String,    // 字段路径，用 : 分割
}

/// 解析后的提取结果
#[derive(Debug, Clone)]
pub struct ExtractedData {
    pub user_info: HashMap<String, String>,
    pub request_headers: HashMap<String, String>,
    pub local_storage: Vec<LocalStorageItem>,
    pub cookie: String,
}

/// LocalStorage 项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalStorageItem {
    pub key: String,
    pub value: String,
}

impl ExtractedData {
    pub fn new() -> Self {
        Self {
            user_info: HashMap::new(),
            request_headers: HashMap::new(),
            local_storage: Vec::new(),
            cookie: String::new(),
        }
    }
}

impl Default for ExtractedData {
    fn default() -> Self {
        Self::new()
    }
}

/// 构建最终结果 JSON
pub fn build_result_json(
    config: &PlatformExtractorConfig,
    extracted: &ExtractedData,
) -> serde_json::Value {
    let mut result = serde_json::json!({});

    // 构建 user_info
    let mut user_info = serde_json::json!({});
    for (key, rule_str) in &config.extract_rules.user_info {
        let value = evaluate_rule(rule_str, extracted, None);
        user_info[key] = serde_json::Value::String(value);
    }
    result["user_info"] = user_info;

    // 构建 request_headers
    let mut request_headers = serde_json::json!({});
    for (key, rule_str) in &config.extract_rules.request_headers {
        let value = evaluate_rule(rule_str, extracted, None);
        request_headers[key] = serde_json::Value::String(value);
    }
    result["request_headers"] = request_headers;

    // 构建 local_data
    let mut local_data = serde_json::json!({});
    for key in &config.extract_rules.local_storage {
        // 从 extracted 中查找对应的值
        let value = extracted.local_storage.iter()
            .find(|item| &item.key == key)
            .map(|item| item.value.clone())
            .unwrap_or_default();
        if !value.is_empty() {
            local_data[key] = serde_json::Value::String(value);
        }
    }
    result["local_data"] = local_data;

    // 构建 cookie
    result["cookie"] = serde_json::Value::String(extracted.cookie.clone());

    result
}

/// 评估规则字符串
fn evaluate_rule(rule: &str, extracted: &ExtractedData, api_response: Option<&HashMap<String, serde_json::Value>>) -> String {
    let parsed = RuleParser::parse(rule);

    match parsed {
        Rule::FixedValue(v) => v,
        Rule::ApiExtract(api_rule) => {
            // 从 extracted 的 request_headers 中查找
            if api_rule.request_type == "request" && api_rule.extract_type == "headers" {
                extracted.request_headers.get(&api_rule.field_path).cloned().unwrap_or_default()
            } else {
                // 对于 body 提取，需要从 API 响应中获取
                // 这里简化处理，实际需要存储 API 响应数据
                String::new()
            }
        }
        Rule::LocalStorageExtract(key) => {
            extracted.local_storage.iter()
                .find(|item| &item.key == &key)
                .map(|item| item.value.clone())
                .unwrap_or_default()
        }
    }
}
