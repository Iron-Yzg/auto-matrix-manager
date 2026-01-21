// Core module - Platform trait and factory

mod publish_progress;
pub use publish_progress::{ProgressEmitter, get_progress_emitter};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

/// Platform type enumeration
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlatformType {
    Douyin,
    Xiaohongshu,
    Kuaishou,
    Bilibili,
}

impl PlatformType {
    /// Get platform display name
    pub fn display_name(&self) -> String {
        match self {
            PlatformType::Douyin => "抖音",
            PlatformType::Xiaohongshu => "小红书",
            PlatformType::Kuaishou => "快手",
            PlatformType::Bilibili => "B站",
        }.to_string()
    }
}

/// Account status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AccountStatus {
    Active,
    Expired,
    Pending,
}

/// User information stored in database
/// 用户信息表结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAccount {
    pub id: String,
    pub username: String,          // 用户名
    pub nickname: String,          // 昵称
    pub avatar_url: String,        // 头像URL
    pub platform: PlatformType,    // 平台类型
    pub params: String,            // 爬取的用户参数JSON
    pub status: AccountStatus,     // 状态
    pub created_at: String,        // 保存时间
}

/// Platform credentials (retrieved from params when needed)
/// 平台凭证（从params解析得到）
#[derive(Debug, Clone)]
pub struct PlatformCredentials {
    pub cookie: String,
    pub user_agent: String,
    pub third_id: String,
    pub sec_uid: Option<String>,
    pub local_data: Vec<LocalDataItem>,
}

/// Local storage data item (for security SDK)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalDataItem {
    pub key: String,
    pub value: String,
}

/// Publication status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PublicationStatus {
    Draft,
    Publishing,
    Completed,
    Failed,
}

/// Platform publication record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformPublication {
    pub id: String,
    pub account_id: String,
    pub platform: PlatformType,
    pub title: String,
    pub description: String,
    pub video_path: String,
    pub cover_path: Option<String>,
    pub status: PublicationStatus,
    pub created_at: String,
    pub published_at: Option<String>,
    pub publish_url: Option<String>,
    pub stats: PublicationStats,
}

/// Publication statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PublicationStats {
    pub comments: i64,
    pub likes: i64,
    pub favorites: i64,
    pub shares: i64,
}

/// Publication result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishResult {
    pub success: bool,
    pub publication_id: String,
    pub item_id: Option<String>,
    pub error_message: Option<String>,
}

// ============================================================================
// Publication Task types (主表+子表结构)
// ============================================================================

/// Publication task - 主表（作品发布任务）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicationTask {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub video_path: String,
    pub cover_path: Option<String>,
    pub hashtags: Vec<String>,
    pub status: PublicationStatus,
    pub created_at: String,
    pub published_at: Option<String>,
}

/// Publication account detail - 子表（账号发布详情）
/// 注意：title/description/hashtags 只在主表存储，子表只存储关联信息和发布结果
/// 冗余 account_name 字段便于直接显示
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicationAccountDetail {
    pub id: String,
    pub publication_task_id: String,
    pub account_id: String,
    pub account_name: String,  // 冗余的账号名称
    pub platform: PlatformType,
    pub status: PublicationStatus,
    pub created_at: String,
    pub published_at: Option<String>,
    pub publish_url: Option<String>,
    pub stats: PublicationStats,
    pub message: Option<String>,  // 发布失败原因（记录到哪一步失败及错误信息）
    pub item_id: Option<String>,  // 发布成功的视频ID
}

/// Publication task with all account details - 包含账号详情的任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicationTaskWithAccounts {
    pub id: String,
    pub title: String,
    pub description: String,
    pub video_path: String,
    pub cover_path: String,
    pub hashtags: Vec<String>,
    pub status: PublicationStatus,
    pub created_at: String,
    pub published_at: String,
    pub accounts: Vec<PublicationAccountDetail>,
}

/// Platform errors
#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Video upload failed: {0}")]
    VideoUploadFailed(String),

    #[error("Publication failed: {0}")]
    PublicationFailed(String),

    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("Invalid credentials: {0}")]
    InvalidCredentials(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Browser error: {0}")]
    BrowserError(String),

    #[error("Storage error: {0}")]
    StorageError(String),
}

impl std::convert::From<rusqlite::Error> for PlatformError {
    fn from(e: rusqlite::Error) -> Self {
        PlatformError::StorageError(e.to_string())
    }
}

impl std::convert::From<std::io::Error> for PlatformError {
    fn from(e: std::io::Error) -> Self {
        PlatformError::InvalidInput(e.to_string())
    }
}

/// 音乐信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MusicInfo {
    pub music_id: String,
    pub music_end_time: String,
}

/// 额外信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtraInfo {
    pub self_declaration: Option<serde_json::Value>,
}

/// Publish request
///
/// 包含视频发布所需的公共信息
/// 各平台可通过 platform_data 字段传递平台特定的扩展数据
#[derive(Debug, Clone, Default)]
pub struct PublishRequest {
    pub account_id: String,
    pub video_path: PathBuf,
    pub cover_path: Option<PathBuf>,
    pub title: String,
    pub description: Option<String>,
    pub hashtags: Vec<String>,
    pub visibility_type: i32,
    pub download_allowed: i32,
    pub timeout: i64,
    /// 记录ID（用于回调）
    pub record_id: Option<String>,
    /// 发送时间（Unix时间戳）
    pub send_time: Option<i64>,
    /// 音乐信息
    pub music_info: Option<MusicInfo>,
    /// POI ID
    pub poi_id: Option<String>,
    pub poi_name: Option<String>,
    /// 锚点信息
    pub anchor: Option<serde_json::Value>,
    /// 额外信息
    pub extra_info: Option<ExtraInfo>,
    /// 平台特定数据（JSON格式，各平台可自定义结构）
    /// - 抖音: {"params": "...", "third_id": "...", ...}
    /// - 快手: {"access_token": "...", ...}
    /// - 小红书: {"cookie": "...", ...}
    pub platform_data: Option<serde_json::Value>,
    /// 进度跟踪信息（task_id, detail_id, account_id, app_handle 用于事件推送）
    #[doc(hidden)]
    pub progress_info: Option<(String, String, String, tauri::AppHandle)>,
}

impl PublishRequest {
    /// 从平台数据获取指定字段
    pub fn get_platform_field<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.platform_data
            .as_ref()
            .and_then(|v| v.get(key))
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// 从平台数据获取字符串字段
    pub fn get_platform_string(&self, key: &str) -> Option<String> {
        self.platform_data
            .as_ref()
            .and_then(|v| v.get(key))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// 设置平台数据
    pub fn with_platform_data(mut self, data: serde_json::Value) -> Self {
        self.platform_data = Some(data);
        self
    }
}

/// Platform trait - defines the interface for all platform implementations
#[async_trait]
pub trait Platform: Send + Sync {
    /// Get the platform type
    fn platform_type(&self) -> PlatformType;

    /// Get the platform name
    fn platform_name(&self) -> String;

    /// Authenticate account using headless browser
    async fn authenticate_account(&self) -> Result<UserAccount, PlatformError>;

    /// Refresh account credentials
    async fn refresh_credentials(&self, account_id: &str) -> Result<UserAccount, PlatformError>;

    /// Publish video to the platform
    async fn publish_video(&self, request: PublishRequest) -> Result<PublishResult, PlatformError>;

    /// Get publication status
    async fn get_publication_status(&self, publication_id: &str) -> Result<PlatformPublication, PlatformError>;

    /// Get account statistics
    async fn get_account_stats(&self, account_id: &str) -> Result<PublicationStats, PlatformError>;

    /// Get platform credentials from user params
    fn get_credentials_from_params(&self, params: &str) -> Result<PlatformCredentials, PlatformError>;
}

/// Publication progress status
/// 发布进度状态
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProgressStatus {
    #[serde(rename = "starting")]
    Starting,      // 开始
    #[serde(rename = "uploading")]
    Uploading,     // 上传中
    #[serde(rename = "uploading_video")]
    UploadingVideo, // 上传视频中
    #[serde(rename = "getting_ticket")]
    GettingTicket, // 获取凭证中
    #[serde(rename = "building_data")]
    BuildingData,  // 构建数据中
    #[serde(rename = "publishing")]
    Publishing,    // 发布中
    #[serde(rename = "completed")]
    Completed,     // 完成
    #[serde(rename = "failed")]
    Failed,        // 失败
}

/// Publication progress event
/// 发布进度事件（用于窗口间通信）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishProgressEvent {
    pub task_id: String,
    pub detail_id: String,
    pub account_id: String,
    pub platform: String,
    pub status: ProgressStatus,
    pub message: String,
    pub progress: i32,  // 0-100
    pub timestamp: i64,
}

impl UserAccount {
    /// Get platform credentials from params JSON
    /// 从params JSON解析平台凭证
    pub fn get_credentials(&self) -> Result<PlatformCredentials, PlatformError> {
        let params_value: serde_json::Value = serde_json::from_str(&self.params)
            .map_err(|e| PlatformError::InvalidCredentials(format!("解析params失败: {}", e)))?;

        let cookie = params_value.get("cookie")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let user_agent = params_value.get("user_agent")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let third_id = params_value.get("third_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let sec_uid = params_value.get("sec_uid")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let local_data: Vec<LocalDataItem> = params_value.get("local_data")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter().filter_map(|item| {
                    Some(LocalDataItem {
                        key: item.get("key")?.as_str()?.to_string(),
                        value: item.get("value")?.as_str()?.to_string(),
                    })
                }).collect()
            })
            .unwrap_or_default();

        Ok(PlatformCredentials {
            cookie,
            user_agent,
            third_id,
            sec_uid,
            local_data,
        })
    }
}
