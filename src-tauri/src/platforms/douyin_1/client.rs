// 抖音HTTP客户端
// Douyin HTTP Client
//
// 本模块负责与抖音创作服务平台API的所有HTTP通信
// 主要功能：
// - 获取上传凭证 (Upload Options)
// - 搜索话题/标签 (Hashtag Search)
// - 获取安全票据 (Header Ticket)
// - 获取CSRF令牌 (CSRF Token)
// - 发布视频 (Publish Video)

use std::time::Duration;
use crate::core::{PlatformCredentials, PlatformError};

// ============================================================================
// BD Ticket配置（在mod.rs中也有定义）
// ============================================================================

/// BD Ticket API 配置
/// 用于获取安全验证票据
#[derive(Debug, Clone)]
pub struct BdTicketConfig {
    pub api_url: String,
}

impl Default for BdTicketConfig {
    fn default() -> Self {
        Self {
            api_url: "https://sssj-acibpxtpbg.cn-beijing.fcapp.run/douyin/bd-ticket-guard-client-data".to_string(),
        }
    }
}

// ============================================================================
// 类型定义
// ============================================================================

/// 上传选项响应
#[derive(Debug, Clone)]
pub struct UploadOptions {
    pub auth: UploadAuth,
}

/// 上传认证信息
#[derive(Debug, Clone)]
pub struct UploadAuth {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: String,
}

/// 话题/标签搜索结果
#[derive(Debug, Clone)]
pub struct Challenge {
    pub hashtag_name: String,
    pub hashtag_id: String,
}

/// 安全票据响应
#[derive(Debug, Clone)]
pub struct HeaderTicket {
    pub bd_ticket_guard_client_data: String,
    pub web_sign_type: String,
}

/// 发布API响应
#[derive(Debug, Clone)]
pub struct PublishResponse {
    pub item_id: String,
    pub status_code: i64,
}

// ============================================================================
// 抖音客户端实现
// ============================================================================

/// HTTP客户端，用于抖音API通信
#[derive(Debug, Clone)]
pub struct DouyinClient {
    /// 平台凭证
    credentials: PlatformCredentials,
    /// BD Ticket API配置
    bd_ticket_config: BdTicketConfig,
    /// HTTP客户端
    http_client: reqwest::Client,
}

impl DouyinClient {
    /// 创建新的抖音客户端
    pub fn new(credentials: PlatformCredentials, bd_ticket_config: BdTicketConfig) -> Self {
        Self {
            credentials,
            bd_ticket_config,
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
        }
    }

    /// 获取账号Cookie
    fn get_cookie(&self) -> &str {
        &self.credentials.cookie
    }

    /// 获取账号Third ID
    fn get_third_id(&self) -> &str {
        &self.credentials.third_id
    }

    /// 获取本地存储数据
    fn get_local_data(&self) -> &[crate::core::LocalDataItem] {
        &self.credentials.local_data
    }

    /// 获取User-Agent
    fn get_user_agent(&self) -> String {
        if self.credentials.user_agent.is_empty() {
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string()
        } else {
            self.credentials.user_agent.clone()
        }
    }

    // -------------------------------------------------------------------------
    // API: 获取上传选项
    // -------------------------------------------------------------------------

    /// 获取上传凭证
    /// 这是视频上传流程的第1步
    /// 向抖音API请求临时凭证（AccessKey, SecretKey, SessionToken）
    pub async fn get_upload_options(&self) -> Result<UploadOptions, PlatformError> {
        let response = self.http_client
            .get("https://creator.douyin.com/web/api/media/upload/auth/v5/")
            .header("Cookie", self.get_cookie())
            .header("User-Agent", self.get_user_agent())
            .send()
            .await
            .map_err(|e| PlatformError::NetworkError(e.to_string()))?;

        let data: serde_json::Value = response.json().await
            .map_err(|e| PlatformError::NetworkError(e.to_string()))?;

        // 检查API响应状态
        if data["status_code"] != 0 {
            return Err(PlatformError::InvalidCredentials(
                data["status_msg"].as_str()
                    .unwrap_or("获取上传选项失败")
                    .to_string()
            ));
        }

        // 解析认证数据
        let auth_str = data["auth"].as_str().unwrap_or("{}");
        let auth: serde_json::Value = serde_json::from_str(auth_str)
            .map_err(|e| PlatformError::InvalidCredentials(e.to_string()))?;

        Ok(UploadOptions {
            auth: UploadAuth {
                access_key_id: auth["AccessKeyID"].as_str().unwrap_or("").to_string(),
                secret_access_key: auth["SecretAccessKey"].as_str().unwrap_or("").to_string(),
                session_token: auth["SessionToken"].as_str().unwrap_or("").to_string(),
            },
        })
    }

    // -------------------------------------------------------------------------
    // API: 搜索话题
    // -------------------------------------------------------------------------

    /// 搜索话题/标签
    /// 用于查找有效的hashtag ID
    pub async fn search_challenge(&self, keyword: &str) -> Result<Vec<Challenge>, PlatformError> {
        let response = self.http_client
            .get("https://creator.douyin.com/aweme/v1/search/challengesug/")
            .query(&[
                ("aid", "2906"),
                ("source", "challenge_create"),
                ("keyword", keyword),
            ])
            .header("Cookie", self.get_cookie())
            .send()
            .await
            .map_err(|e| PlatformError::NetworkError(e.to_string()))?;

        let data: serde_json::Value = response.json().await
            .map_err(|e| PlatformError::NetworkError(e.to_string()))?;

        if data["status_code"] != 0 {
            return Err(PlatformError::PublicationFailed(
                data["status_msg"].as_str()
                    .unwrap_or("搜索话题失败")
                    .to_string()
            ));
        }

        // 解析话题列表
        let empty_array: Vec<serde_json::Value> = vec![];
        let sug_list = data["sug_list"].as_array().unwrap_or(&empty_array);
        let challenges: Vec<Challenge> = sug_list.iter().map(|item| Challenge {
            hashtag_name: item["keyword"].as_str().unwrap_or("").to_string(),
            hashtag_id: item["cid"].as_str().unwrap_or("").to_string(),
        }).collect();

        Ok(challenges)
    }

    // -------------------------------------------------------------------------
    // API: 获取安全票据
    // -------------------------------------------------------------------------

    /// 获取Header Ticket
    /// 用于API请求的安全验证
    pub async fn get_header_ticket_key(&self, ticket_type: &str) -> Result<HeaderTicket, PlatformError> {
        let body = serde_json::json!({
            "cookies": self.get_cookie(),
            "user_agent": self.get_user_agent(),
            "localdata": self.get_local_data(),
            "third_id": self.get_third_id(),
            "type": ticket_type,
        });

        let response = self.http_client
            .post(&self.bd_ticket_config.api_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| PlatformError::NetworkError(e.to_string()))?;

        let data: serde_json::Value = response.json().await
            .map_err(|e| PlatformError::NetworkError(e.to_string()))?;

        let code = data["code"].as_i64().unwrap_or(1);
        if code != 0 && code != 200 && code != 1000 {
            return Err(PlatformError::AuthenticationFailed(
                data["msg"].as_str()
                    .unwrap_or("获取安全票据失败")
                    .to_string()
            ));
        }

        // 提取bd-ticket-guard-client-data
        let bd_data = data["data"]["bd-ticket-guard-client-data"].as_str()
            .unwrap_or("")
            .to_string();

        Ok(HeaderTicket {
            bd_ticket_guard_client_data: bd_data,
            web_sign_type: "1".to_string(),
        })
    }

    // -------------------------------------------------------------------------
    // API: 获取CSRF Token
    // -------------------------------------------------------------------------

    /// 获取CSRF令牌
    /// POST请求需要此令牌
    pub async fn get_csrf_token(&self, url: &str) -> Result<String, PlatformError> {
        let full_url = format!("https://creator.douyin.com{}", url);

        let response = self.http_client
            .head(&full_url)
            .header("x-secsdk-csrf-request", "1")
            .header("x-secsdk-csrf-version", "1.2.7")
            .header("Cookie", self.get_cookie())
            .send()
            .await
            .map_err(|e| PlatformError::NetworkError(e.to_string()))?;

        // 获取x-ware-csrf-token头
        let token_header = response.headers()
            .get("x-ware-csrf-token")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if token_header.is_empty() {
            return Err(PlatformError::AuthenticationFailed(
                "获取CSRF令牌失败".to_string()
            ));
        }

        // 解析令牌（格式：version,token）
        let token = token_header.split(',').nth(1)
            .unwrap_or(token_header)
            .to_string();

        Ok(token)
    }

    // -------------------------------------------------------------------------
    // API: 发布视频
    // -------------------------------------------------------------------------

    /// 发布视频（create_v2 API）
    /// 这是视频发布流程的最后一步
    pub async fn publish_video_v2(&self, publish_data: &serde_json::Value) -> Result<PublishResponse, PlatformError> {
        // 获取安全票据
        let header_ticket = self.get_header_ticket_key("video").await?;
        let csrf_token = self.get_csrf_token("/web/api/media/aweme/create_v2/").await?;

        // 构建请求头
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("X-Secsdk-Csrf-Token", csrf_token.parse().unwrap());
        headers.insert("Referer", "https://creator.douyin.com/creator-micro/content/post/video?enter_from=publish_page".parse().unwrap());

        // 添加安全票据字段
        if let Some(value) = header_ticket.bd_ticket_guard_client_data.strip_prefix("result\":\"") {
            headers.insert("bd-ticket-guard-client-data", value.trim_end_matches('"').parse().unwrap());
        }
        headers.insert("bd-ticket-guard-web-sign-type", header_ticket.web_sign_type.parse().unwrap());

        // 发送发布请求
        let response = self.http_client
            .post("https://creator.douyin.com/web/api/media/aweme/create_v2/")
            .query(&[
                ("read_aid", "2906"),
                ("cookie_enabled", "1"),
                ("screen_width", "1920"),
                ("screen_height", "1080"),
                ("browser_language", "zh-CN"),
                ("timezone_name", "Asia/Shanghai"),
            ])
            .headers(headers)
            .json(publish_data)
            .send()
            .await
            .map_err(|e| PlatformError::NetworkError(e.to_string()))?;

        let data: serde_json::Value = response.json().await
            .map_err(|e| PlatformError::NetworkError(e.to_string()))?;

        if data["status_code"] != 0 {
            return Err(PlatformError::PublicationFailed(
                data["status_msg"].as_str()
                    .unwrap_or("发布视频失败")
                    .to_string()
            ));
        }

        Ok(PublishResponse {
            item_id: data["item_id"].as_str().unwrap_or("").to_string(),
            status_code: data["status_code"].as_i64().unwrap_or(0),
        })
    }
}
