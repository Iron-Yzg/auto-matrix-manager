// Data Extractor Engine
// 通用数据提取引擎 - 根据配置自动提取用户数据

use crate::browser::BrowserAuthResult;
use crate::storage::DatabaseManager;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// 浏览器引擎配置
pub struct DataExtractorEngine {
    /// 数据库管理器
    db_manager: Arc<DatabaseManager>,
    /// 当前使用的配置
    current_config: Option<ExtractorConfig>,
    /// 捕获的 API 响应数据
    captured_api_data: HashMap<String, ApiCaptureData>,
    /// 捕获的请求头
    captured_request_headers: HashMap<String, HashMap<String, String>>,
}

#[derive(Debug, Clone)]
/// 捕获的 API 数据
pub struct ApiCaptureData {
    pub url: String,
    pub request_headers: HashMap<String, String>,
    pub response_body: Option<Value>,
}

/// 提取结果
pub struct ExtractResult {
    pub success: bool,
    pub user_info: HashMap<String, String>,
    pub request_headers: HashMap<String, String>,
    pub local_storage: Vec<LocalStorageItem>,
    pub cookie: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalStorageItem {
    pub key: String,
    pub value: String,
}

/// 简化的配置结构
#[derive(Debug, Clone)]
pub struct ExtractorConfig {
    pub platform_id: String,
    pub platform_name: String,
    pub login_url: String,
    pub login_success_mode: String,  // "url_match" 或 "api_match"
    pub login_success_pattern: String,
    pub login_success_api_rule: Option<String>,
    pub login_success_api_operator: Option<String>,
    pub login_success_api_value: Option<String>,
    pub redirect_url: Option<String>,
    pub extract_rules: Value,
}

impl DataExtractorEngine {
    /// 创建新的引擎实例
    pub fn new(db_manager: Arc<DatabaseManager>) -> Self {
        Self {
            db_manager,
            current_config: None,
            captured_api_data: HashMap::new(),
            captured_request_headers: HashMap::new(),
        }
    }

    /// 加载平台配置
    pub async fn load_config(&mut self, platform_id: &str) -> Result<(), String> {
        let config = self.db_manager.get_extractor_config(platform_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Platform config not found: {}", platform_id))?;

        // 转换为简化结构
        self.current_config = Some(ExtractorConfig {
            platform_id: config.platform_id,
            platform_name: config.platform_name,
            login_url: config.login_url,
            login_success_mode: config.login_success_mode,
            login_success_pattern: config.login_success_pattern,
            login_success_api_rule: config.login_success_api_rule,
            login_success_api_operator: config.login_success_api_operator,
            login_success_api_value: config.login_success_api_value,
            redirect_url: config.redirect_url,
            extract_rules: config.extract_rules,
        });
        Ok(())
    }

    /// 获取当前配置
    pub fn get_config(&self) -> Option<&ExtractorConfig> {
        self.current_config.as_ref()
    }

    /// 捕获 API 请求
    pub fn capture_api_request(&mut self, url: &str, headers: &HashMap<String, String>) {
        // 查找匹配的 API 路径
        if let Some(config) = &self.current_config {
            if let Some(rules) = config.extract_rules.get("request_headers") {
                if let Some(headers_map) = rules.as_object() {
                    for (_key, rule_val) in headers_map {
                        if let Some(rule) = rule_val.as_str() {
                            if let Some(api_path) = extract_api_path(rule) {
                                if url.contains(&api_path) {
                                    self.captured_request_headers.insert(url.to_string(), headers.clone());
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// 捕获 API 响应
    pub fn capture_api_response(&mut self, url: &str, body: &Value) {
        // 查找匹配的 API 路径
        if let Some(config) = &self.current_config {
            if let Some(rules) = config.extract_rules.get("user_info") {
                if let Some(user_info_map) = rules.as_object() {
                    for (_key, rule_val) in user_info_map {
                        if let Some(rule) = rule_val.as_str() {
                            if let Some(api_path) = extract_api_path(rule) {
                                if url.contains(&api_path) {
                                    self.captured_api_data.insert(url.to_string(), ApiCaptureData {
                                        url: url.to_string(),
                                        request_headers: self.captured_request_headers.get(url).cloned().unwrap_or_default(),
                                        response_body: Some(body.clone()),
                                    });
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// 执行数据提取
    pub fn extract(&self) -> ExtractResult {
        let config = match &self.current_config {
            Some(c) => c,
            None => {
                return ExtractResult {
                    success: false,
                    user_info: HashMap::new(),
                    request_headers: HashMap::new(),
                    local_storage: Vec::new(),
                    cookie: String::new(),
                    message: "No configuration loaded".to_string(),
                };
            }
        };

        let mut user_info = HashMap::new();
        let mut request_headers = HashMap::new();

        // 提取用户信息
        if let Some(rules) = config.extract_rules.get("user_info") {
            if let Some(user_info_map) = rules.as_object() {
                for (key, rule_val) in user_info_map {
                    if let Some(rule) = rule_val.as_str() {
                        let value = evaluate_rule(rule, &self.captured_api_data, &self.captured_request_headers);
                        if !value.is_empty() {
                            user_info.insert(key.clone(), value);
                        }
                    }
                }
            }
        }

        // 提取请求头
        if let Some(rules) = config.extract_rules.get("request_headers") {
            if let Some(headers_map) = rules.as_object() {
                for (key, rule_val) in headers_map {
                    if let Some(rule) = rule_val.as_str() {
                        let value = evaluate_rule(rule, &self.captured_api_data, &self.captured_request_headers);
                        if !value.is_empty() {
                            request_headers.insert(key.clone(), value);
                        }
                    }
                }
            }
        }

        // Cookie 处理
        let cookie = if let Some(cookie_rule) = config.extract_rules.get("cookie") {
            if let Some(source) = cookie_rule.get("source").and_then(|s| s.as_str()) {
                if source == "from_browser" {
                    self.captured_request_headers.values()
                        .flat_map(|h| h.iter())
                        .find(|(k, _)| k.to_lowercase() == "cookie")
                        .map(|(_, v)| v.clone())
                        .unwrap_or_default()
                } else if source == "from_api" {
                    if let Some(api_path) = cookie_rule.get("api_path").and_then(|p| p.as_str()) {
                        if let Some(headers) = self.captured_request_headers.get(api_path) {
                            let header_name = cookie_rule.get("header_name")
                                .and_then(|h| h.as_str())
                                .unwrap_or("cookie");
                            headers.get(header_name).cloned().unwrap_or_default()
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        // 先计算 message，再移动值
        let message = if user_info.is_empty() && cookie.is_empty() {
            "No data extracted".to_string()
        } else {
            "Extraction completed".to_string()
        };

        ExtractResult {
            success: !user_info.is_empty() || !cookie.is_empty(),
            user_info,
            request_headers,
            local_storage: Vec::new(),
            cookie,
            message,
        }
    }

    /// 生成认证结果
    pub fn build_auth_result(&self, _nickname: &str, _avatar_url: &str, current_url: &str) -> BrowserAuthResult {
        let extract_result = self.extract();

        // 构建 third_param JSON
        let third_param = serde_json::json!({
            "request_headers": extract_result.request_headers,
            "local_data": [],
            "cookie": extract_result.cookie,
        });

        // 从 user_info HashMap 中提取4个字段
        let nickname = extract_result.user_info.get("nickname")
            .cloned()
            .unwrap_or_default();
        let avatar_url: String = extract_result.user_info.get("avatar_url")
            .cloned()
            .unwrap_or_default();
        let third_id = extract_result.user_info.get("third_id")
            .cloned()
            .unwrap_or_default();
        let sec_uid = extract_result.user_info.get("sec_uid")
            .cloned()
            .unwrap_or_default();

        BrowserAuthResult {
            step: if extract_result.success {
                crate::browser::BrowserAuthStep::Completed
            } else {
                crate::browser::BrowserAuthStep::Failed(extract_result.message.clone())
            },
            message: extract_result.message.clone(),
            nickname,
            avatar_url,
            third_id,
            sec_uid,
            cookie: extract_result.cookie,
            local_storage: serde_json::to_string(&extract_result.local_storage).unwrap_or_default(),
            request_headers: third_param.to_string(),
            current_url: current_url.to_string(),
            need_poll: false,
            screenshot: None,
            error: if extract_result.success { None } else { Some(extract_result.message) },
        }
    }

    /// 清空捕获的数据
    pub fn clear(&mut self) {
        self.captured_api_data.clear();
        self.captured_request_headers.clear();
    }
}

/// 从规则中提取 API 路径
fn extract_api_path(rule: &str) -> Option<String> {
    if rule.starts_with("${api:") {
        let content = rule.trim_start_matches("${api:").trim_end_matches('}');
        if let Some(first_colon) = content.find(':') {
            return Some(content[..first_colon].to_string());
        }
    }
    None
}

/// 评估规则并提取值
fn evaluate_rule(
    rule: &str,
    api_data: &HashMap<String, ApiCaptureData>,
    request_headers: &HashMap<String, HashMap<String, String>>,
) -> String {
    if rule.starts_with("${api:") {
        let content = rule.trim_start_matches("${api:").trim_end_matches('}');
        let parts: Vec<&str> = content.split(':').collect();

        if parts.len() < 3 {
            return String::new();
        }

        let api_path = parts[0];
        let request_type = parts[1];
        let extract_type = parts[2];
        let field_path = if parts.len() > 3 {
            parts[3..].join(":")
        } else {
            String::new()
        };

        // 查找匹配的 API 数据
        for (url, data) in api_data {
            if url.contains(api_path) {
                if request_type == "request" && extract_type == "headers" {
                    return data.request_headers.get(&field_path).cloned().unwrap_or_default();
                } else if request_type == "response" && extract_type == "body" {
                    if let Some(body) = &data.response_body {
                        return extract_json_path(body, &field_path);
                    }
                }
            }
        }

        // 从 request_headers 直接查找
        for headers in request_headers.values() {
            if headers.get("url").map(|u| u.contains(api_path)).unwrap_or(false) {
                return headers.get(&field_path).cloned().unwrap_or_default();
            }
        }
    } else if rule.starts_with("${localStorage:") {
        return format!("${{localStorage:{}}}", rule.trim_start_matches("${localStorage:").trim_end_matches('}'));
    }

    // 固定值
    rule.to_string()
}

/// 从 JSON 中按路径提取值
fn extract_json_path(json: &Value, path: &str) -> String {
    let parts: Vec<&str> = path.split(':').collect();
    let mut current = json.clone();

    for part in parts {
        match current {
            Value::Object(obj) => {
                if let Some(value) = obj.get(part) {
                    current = value.clone();
                } else {
                    return String::new();
                }
            }
            Value::Array(arr) => {
                if let Ok(index) = part.parse::<usize>() {
                    if index < arr.len() {
                        current = arr[index].clone();
                    } else {
                        return String::new();
                    }
                } else {
                    return String::new();
                }
            }
            _ => return String::new(),
        }
    }

    match current {
        Value::String(s) => s,
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Array(arr) => serde_json::to_string(&arr).unwrap_or_default(),
        Value::Object(obj) => serde_json::to_string(&obj).unwrap_or_default(),
        _ => String::new(),
    }
}
