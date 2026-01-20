//! 抖音API客户端
//!
//! 负责与抖音服务器进行HTTP通信
//!
//! # 主要功能
//!
//! - 发送GET/POST请求
//! - 获取CSRF Token
//! - 获取BD Ticket Guard Client Data（安全凭证）
//! - 获取视频上传配置
//! - 搜索话题建议
//! - 发布视频（V2接口）
//!
//! # 使用示例
//!
//! ```rust
//! use crate::platforms::douyin::douyin_client::DouyinClient;
//!
//! let client = DouyinClient::new(
//!     "cookie".to_string(),
//!     "user-agent".to_string(),
//!     "third_id".to_string(),
//!     vec![],
//! );
//!
//! // 获取上传配置
//! let upload_options = client.get_upload_options();
//!
//! // 发布视频
//! let result = client.get_public_video_v2(publish_data, csrf_token, bd_ticket).await;
//! ```

use crate::core::LocalDataItem;
use serde_json::Value;
use std::collections::HashMap;

/// 基础URL
const BASE_URL: &str = "https://creator.douyin.com";

/// MSSDK URL
const MSSDK_URL: &str = "https://mssdk.bytedance.com";

/// BD Ticket API URL
const BD_TICKET_API_URL: &str = "https://sssj-acibpxtpbg.cn-beijing.fcapp.run/douyin/bd-ticket-guard-client-data";

/// 共享的异步HTTP客户端
/// 使用静态懒加载避免在async context中创建/销毁client导致panic
static ASYNC_CLIENT: std::sync::LazyLock<reqwest::Client> = std::sync::LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to create async HTTP client")
});

/// 抖音API客户端
///
/// 用于与抖音服务器进行HTTP通信
#[derive(Debug, Clone)]
pub struct DouyinClient {
    /// 用户Cookie
    pub cookie: String,
    /// User-Agent
    pub user_agent: String,
    /// 第三方用户ID
    pub third_id: String,
    /// 本地数据
    pub local_data: Vec<LocalDataItem>,
    /// CSRF Token缓存
    csrf_token_map: HashMap<String, String>,
}

impl DouyinClient {
    /// 创建新的客户端实例
    ///
    /// # 参数
    ///
    /// * `cookie` - 用户Cookie
    /// * `user_agent` - User-Agent字符串
    /// * `third_id` - 第三方用户ID
    /// * `local_data` - 本地数据列表
    pub fn new(
        cookie: String,
        user_agent: String,
        third_id: String,
        local_data: Vec<LocalDataItem>,
    ) -> Self {
        Self {
            cookie,
            user_agent,
            third_id,
            local_data: if local_data.is_empty() {
                Vec::new()
            } else {
                local_data
            },
            csrf_token_map: HashMap::new(),
        }
    }

    /// 发送HTTP请求（GET）
    ///
    /// # 参数
    ///
    /// * `endpoint` - API端点路径
    /// * `params` - 查询参数
    ///
    /// # 返回
    ///
    /// JSON响应
    pub async fn request_get(&self, endpoint: &str, params: Option<HashMap<String, String>>) -> Value {
        let url = self.build_url(BASE_URL, endpoint, params.as_ref());

        let response = ASYNC_CLIENT
            .get(&url)
            .header("Cookie", &self.cookie)
            .header("User-Agent", &self.user_agent)
            .header("Referer", BASE_URL)
            .header("Accept", "application/json, text/plain, */*")
            .header("Accept-Language", "zh-CN,zh;q=0.9")
            .send()
            .await;

        match response {
            Ok(res) => {
                let text = res.text().await.unwrap_or_default();
                serde_json::from_str(&text).unwrap_or(Value::Null)
            }
            Err(e) => {
                tracing::error!("GET请求失败: {}", e);
                Value::Null
            }
        }
    }

    /// 发送HTTP请求（POST）
    ///
    /// # 参数
    ///
    /// * `endpoint` - API端点路径
    /// * `params` - 查询参数
    /// * `json_data` - JSON请求体数据
    /// * `extra_headers` - 额外请求头
    ///
    /// # 返回
    ///
    /// JSON响应
    pub async fn request_post(
        &self,
        endpoint: &str,
        params: Option<HashMap<String, String>>,
        json_data: Option<HashMap<String, Value>>,
        extra_headers: Option<HashMap<String, String>>,
    ) -> Value {
        let url = self.build_url(BASE_URL, endpoint, params.as_ref());

        let mut request = ASYNC_CLIENT
            .post(&url)
            .header("Content-Type", "application/json; charset=utf-8");

        // 设置默认请求头
        request = request.header("Cookie", &self.cookie);
        request = request.header("User-Agent", &self.user_agent);
        request = request.header("Accept", "application/json, text/plain, */*");
        request = request.header("Accept-Language", "zh-CN,zh;q=0.9");

        // 设置extraHeaders
        let mut referer = BASE_URL.to_string();
        if let Some(headers) = extra_headers {
            for (key, value) in headers {
                if key.to_lowercase() == "referer" {
                    referer = value;
                } else {
                    request = request.header(&key, &value);
                }
            }
        }
        request = request.header("Referer", referer);

        // 设置JSON body
        if let Some(data) = json_data {
            let json_body = serde_json::to_string(&data).unwrap_or_default();
            request = request.body(json_body);
        }

        let response = request.send().await;

        match response {
            Ok(res) => {
                let text = res.text().await.unwrap_or_default();
                serde_json::from_str(&text).unwrap_or(Value::Null)
            }
            Err(e) => {
                tracing::error!("POST请求失败: {}", e);
                Value::Null
            }
        }
    }

    /// 置换msToken
    ///
    /// # 参数
    ///
    /// * `ms_token` - msToken
    /// * `ttw_id` - ttwId
    ///
    /// # 返回
    ///
    /// JSON响应
    pub async fn get_ms_token(&self, ms_token: &str, ttw_id: &str) -> Value {
        let mut params = HashMap::new();
        params.insert("ms_appid".to_string(), "2906".to_string());
        params.insert("msToken".to_string(), ms_token.to_string());

        let url = self.build_url(MSSDK_URL, "/web/common", Some(&params));

        let response = ASYNC_CLIENT
            .post(&url)
            .header("Content-Type", "text/plain;charset=UTF-8")
            .header("Cookie", format!("ttwid={}", ttw_id))
            .header("User-Agent", &self.user_agent)
            .header("referer", BASE_URL)
            .header("sec-fetch-storage-access", "active")
            .send()
            .await;

        match response {
            Ok(res) => {
                let text = res.text().await.unwrap_or_default();
                serde_json::from_str(&text).unwrap_or(Value::Null)
            }
            Err(e) => {
                tracing::error!("获取msToken失败: {}", e);
                Value::Null
            }
        }
    }

    /// 构建带参数的URL
    ///
    /// # 参数
    ///
    /// * `base_url` - 基础URL
    /// * `endpoint` - API端点
    /// * `params` - 查询参数
    ///
    /// # 返回
    ///
    /// 完整的URL字符串
    fn build_url(
        &self,
        base_url: &str,
        endpoint: &str,
        params: Option<&HashMap<String, String>>,
    ) -> String {
        let mut url = format!("{}{}", base_url, endpoint);

        if let Some(p) = params {
            if !p.is_empty() {
                url.push('?');
                let param_list: Vec<String> = p
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect();
                url.push_str(&param_list.join("&"));
            }
        }

        url
    }

    /// 获取CSRF Token
    ///
    /// 通过HEAD请求从响应头中获取x-ware-csrf-token
    ///
    /// # 参数
    ///
    /// * `url_path` - URL路径（如："/web/api/media/aweme/create_v2/"）
    ///
    /// # 返回
    ///
    /// CSRF Token字符串
    ///
    /// # 错误
    ///
    /// 如果获取失败，返回错误信息
    pub async fn get_csrf_token(&mut self, url_path: &str) -> Result<String, String> {
        // 检查缓存
        if let Some(token) = self.csrf_token_map.get(url_path) {
            return Ok(token.clone());
        }

        let full_url = format!("{}{}", BASE_URL, url_path);

        let response = ASYNC_CLIENT
            .head(&full_url)
            .header("x-secsdk-csrf-request", "1")
            .header("x-secsdk-csrf-version", "1.2.7")
            .header("Cookie", &self.cookie)
            .header("User-Agent", &self.user_agent)
            .send()
            .await;

        match response {
            Ok(res) => {
                if !res.status().is_success() {
                    return Err(format!("获取CSRF Token失败: HTTP {}", res.status()));
                }

                // 从响应头获取x-ware-csrf-token
                let x_ware_csrf_token = res
                    .headers()
                    .get("x-ware-csrf-token")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());

                match x_ware_csrf_token {
                    Some(token) => {
                        // 解析Token（格式：version,token）
                        let parsed_token = if token.contains(',') {
                            let parts: Vec<&str> = token.split(',').collect();
                            if parts.len() > 1 {
                                parts[1].to_string()
                            } else {
                                token.clone()
                            }
                        } else {
                            token.clone()
                        };

                        self.csrf_token_map.insert(url_path.to_string(), parsed_token.clone());
                        Ok(parsed_token)
                    }
                    None => Err("获取CSRF Token失败：响应头中未找到x-ware-csrf-token".to_string()),
                }
            }
            Err(e) => Err(format!("获取CSRF Token失败: {}", e)),
        }
    }

    /// 获取BD Ticket Guard Client Data（安全凭证）
    ///
    /// # 参数
    ///
    /// * `ticket_type` - 凭证类型（如："video"）
    ///
    /// # 返回
    ///
    /// 包含BD Ticket的HashMap
    ///
    /// # 错误
    ///
    /// 如果获取失败，返回错误信息
    pub async fn get_header_ticket_key(&self, ticket_type: &str) -> Result<HashMap<String, String>, String> {
        tracing::info!("[BD凭证] 开始获取BD Ticket, type: {}", ticket_type);

        // 使用serde_json::json!构建请求体（与douyin_1一致）
        let body = serde_json::json!({
            "cookies": self.cookie,
            "user_agent": self.user_agent,
            "localdata": self.local_data,
            "third_id": self.third_id,
            "type": ticket_type,
        });

        // 重试机制（最多2次）
        let max_retries = 2;
        for attempt in 0..max_retries {
            let response = ASYNC_CLIENT
                .post(BD_TICKET_API_URL)
                .header("Content-Type", "application/json")
                .timeout(std::time::Duration::from_secs(30))
                .body(serde_json::to_string(&body).unwrap_or_default())
                .send()
                .await;

            match response {
                Ok(res) => {
                    let text = res.text().await.unwrap_or_default();

                    // 解析响应JSON
                    let result: Value = serde_json::from_str(&text)
                        .map_err(|e| format!("解析BD凭证响应失败: {}", e))?;

                    let code = result.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);

                    if code == 3602 {
                        return Err("登录离线[获取抖音发布服务BD(3602)]".to_string());
                    }

                    if code != 0 && code != 200 && code != 1000 {
                        let msg = result.get("msg").and_then(|v| v.as_str()).unwrap_or("未知错误");

                        if attempt < max_retries - 1 {
                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                            continue;
                        }
                        return Err(format!("{} [获取抖音发布服务BD凭证]", msg));
                    }

                    // 提取所有bd-ticket-guard-*字段（与Java实现一致）
                    // API返回格式: {"code": 1000, "data": {"bd-ticket-guard-xxx": "..."}}
                    // 注意：部分字段是数字类型，需要转换为字符串
                    let mut result_data = HashMap::new();

                    if let Value::Object(data_map) = result.get("data").unwrap_or(&Value::Null) {
                        for (key, value) in data_map {
                            if key.starts_with("bd-ticket-guard-") {
                                let str_value = match value {
                                    Value::String(s) => s.clone(),
                                    Value::Number(n) => n.to_string(),
                                    Value::Bool(b) => b.to_string(),
                                    _ => continue,
                                };
                                result_data.insert(key.clone(), str_value);
                            }
                        }
                    }

                    if result_data.is_empty() {
                        return Err("获取BD凭证失败：未找到bd-ticket-guard相关字段".to_string());
                    }

                    tracing::info!("[BD凭证] 解析成功, 包含 {} 个字段", result_data.len());
                    return Ok(result_data);
                }
                Err(e) => {
                    if attempt < max_retries - 1 {
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        continue;
                    }
                    return Err(format!("获取抖音发布服务BD凭证失败: {}", e));
                }
            }
        }

        Err("获取抖音发布服务BD凭证失败：重试次数用尽".to_string())
    }

    /// 获取视频上传配置
    ///
    /// # 返回
    ///
    /// JSON响应，包含上传授权信息
    ///
    /// # 错误
    ///
    /// 如果获取失败，返回错误信息
    pub async fn get_upload_options(&self) -> Result<Value, String> {
        let response = self.request_get("/web/api/media/upload/auth/v5/", None).await;

        // 调试：打印完整响应
        tracing::debug!("[UploadOptions] 完整响应: {}", response);

        let status_code = response.get("status_code").and_then(|v| v.as_i64()).unwrap_or(-1);
        if status_code != 0 {
            let msg = response.get("status_msg").and_then(|v| v.as_str()).unwrap_or("获取失败");
            return Err(format!("获取上传配置失败: {}", msg));
        }

        // 调试：检查auth字段
        let auth = response.get("auth").cloned().unwrap_or(Value::Null);
        tracing::debug!("[UploadOptions] auth字段类型: {:?}", auth);

        // 如果auth是字符串，解析为JSON对象（与Java实现一致）
        if let Value::String(auth_str) = &auth {
            tracing::debug!("[UploadOptions] auth是字符串，尝试解析: {}...", &auth_str[..auth_str.len().min(200)]);
            match serde_json::from_str::<Value>(auth_str) {
                Ok(auth_json) => {
                    tracing::debug!("[UploadOptions] auth解析成功: {}", auth_json);
                    // 返回包含解析后auth的响应
                    let mut response_with_parsed_auth = response.clone();
                    response_with_parsed_auth["auth"] = auth_json;
                    return Ok(response_with_parsed_auth);
                }
                Err(e) => {
                    tracing::warn!("[UploadOptions] auth解析失败: {}，原始值: {}", e, auth_str);
                }
            }
        }

        Ok(response)
    }

    /// 搜索话题建议
    ///
    /// # 参数
    ///
    /// * `keyword` - 搜索关键词
    ///
    /// # 返回
    ///
    /// 搜索结果JSON
    pub async fn search_challenge_sug(&self, keyword: &str) -> Value {
        let mut params = HashMap::new();
        params.insert("aid".to_string(), "2906".to_string());
        params.insert("source".to_string(), "challenge_create".to_string());
        params.insert("keyword".to_string(), keyword.to_string());

        let url = self.build_url(BASE_URL, "/aweme/v1/search/challengesug/", Some(&params));

        let response = ASYNC_CLIENT
            .get(&url)
            .header("Cookie", &self.cookie)
            .header("User-Agent", &self.user_agent)
            .header("Referer", "https://creator.douyin.com/creator-micro/content/publish-media/image-text?enter_from=publish_page")
            .header("Accept", "application/json, text/plain, */*")
            .header("Accept-Language", "zh-CN,zh;q=0.9")
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await;

        match response {
            Ok(res) => {
                let text = res.text().await.unwrap_or_default();
                serde_json::from_str(&text).unwrap_or(Value::Null)
            }
            Err(e) => {
                tracing::error!("搜索话题失败: {}", e);
                Value::Null
            }
        }
    }

    /// 发布视频（V2接口）
    ///
    /// # 参数
    ///
    /// * `publish_data` - 发布数据
    /// * `csrf_token` - CSRF Token
    /// * `bd_ticket` - BD Ticket
    ///
    /// # 返回
    ///
    /// 发布结果JSON
    ///
    /// # 错误
    ///
    /// 如果发布失败，返回错误信息
    pub async fn get_public_video_v2(
        &mut self,
        publish_data: HashMap<String, Value>,
        csrf_token: Option<String>,
        bd_ticket: Option<HashMap<String, String>>,
    ) -> Result<Value, String> {
        // 获取BD Ticket
        let bd_ticket = if let Some(ticket) = bd_ticket {
            ticket
        } else {
            self.get_header_ticket_key("video").await?
        };

        // 获取CSRF Token
        let csrf_token = if let Some(token) = csrf_token {
            token
        } else {
            self.get_csrf_token("/web/api/media/aweme/create_v2/").await?
        };

        // 构建查询参数（与Java成功的请求一致）
        let mut params = HashMap::new();
        params.insert("read_aid".to_string(), "2906".to_string());
        params.insert("cookie_enabled".to_string(), "true".to_string()); // Java用"true"
        params.insert("screen_width".to_string(), "1920".to_string());
        params.insert("screen_height".to_string(), "1080".to_string());
        params.insert("browser_language".to_string(), "zh-CN".to_string());
        params.insert("browser_name".to_string(), "Mozilla".to_string()); // Java有
        params.insert("browser_online".to_string(), "true".to_string()); // Java有
        params.insert("timezone_name".to_string(), "Asia/Shanghai".to_string()); // Java有
        params.insert("aid".to_string(), "1128".to_string()); // Java有

        // 构建请求头（使用Java成功的6个BD ticket字段）
        let mut headers = HashMap::new();

        // 提取所有6个BD ticket字段（与Java一致）
        if let Some(v) = bd_ticket.get("bd-ticket-guard-client-data") {
            headers.insert("bd-ticket-guard-client-data".to_string(), v.clone());
        }
        if let Some(v) = bd_ticket.get("bd-ticket-guard-web-sign-type") {
            headers.insert("bd-ticket-guard-web-sign-type".to_string(), v.clone());
        } else {
            headers.insert("bd-ticket-guard-web-sign-type".to_string(), "1".to_string());
        }
        if let Some(v) = bd_ticket.get("bd-ticket-guard-version") {
            headers.insert("bd-ticket-guard-version".to_string(), v.clone());
        }
        if let Some(v) = bd_ticket.get("bd-ticket-guard-web-version") {
            headers.insert("bd-ticket-guard-web-version".to_string(), v.clone());
        }
        if let Some(v) = bd_ticket.get("bd-ticket-guard-ree-public-key") {
            headers.insert("bd-ticket-guard-ree-public-key".to_string(), v.clone());
        }
        if let Some(v) = bd_ticket.get("bd-ticket-guard-iteration-version") {
            headers.insert("bd-ticket-guard-iteration-version".to_string(), v.clone());
        }

        headers.insert("X-Secsdk-Csrf-Token".to_string(), csrf_token.clone());
        headers.insert(
            "Referer".to_string(),
            format!("{}/creator-micro/content/post/video?enter_from=publish_page", BASE_URL),
        );

        // ========== 打印发布API请求信息 ==========
        tracing::info!("[PublishAPI] ====== 发布API请求信息 ======");

        // 构建完整URL
        let query_parts: Vec<String> = params.iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        let full_url = format!("{}/web/api/media/aweme/create_v2/?{}",
            BASE_URL, query_parts.join("&"));
        tracing::info!("[PublishAPI] URL: {}", full_url);

        // 打印请求体
        let body_str = serde_json::to_string(&publish_data).unwrap_or_default();
        tracing::info!("[PublishAPI] Body : {}", &body_str);

        let response = self.request_post(
            "/web/api/media/aweme/create_v2/",
            Some(params),
            Some(publish_data),
            Some(headers),
        ).await;

        if response == Value::Null {
            return Err("请去账号管理列表中解除风控[抖音]".to_string());
        }

        let status_code = response.get("status_code").and_then(|v| v.as_i64()).unwrap_or(-1);
        if status_code != 0 {
            let msg = response.get("status_msg").and_then(|v| v.as_str()).unwrap_or("未知错误");
            return Err(format!("{} [视频发布V2]", msg));
        }

        let item_id = response.get("item_id").and_then(|v| v.as_str()).unwrap_or("");

        if item_id.is_empty() {
            let msg = response.get("status_msg").and_then(|v| v.as_str()).unwrap_or("未知错误");
            return Err(format!("{} [视频发布V2结果]", msg));
        }

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_url() {
        let client = DouyinClient::new(
            "cookie".to_string(),
            "user-agent".to_string(),
            "123".to_string(),
            Vec::new(),
        );

        let mut params = HashMap::new();
        params.insert("key1".to_string(), "value1".to_string());
        params.insert("key2".to_string(), "value2".to_string());

        let url = client.build_url("https://example.com", "/api/test", Some(&params));

        assert!(url.contains("https://example.com/api/test"));
        assert!(url.contains("key1=value1"));
        assert!(url.contains("key2=value2"));
    }
}
