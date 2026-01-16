// 抖音平台模块
// Douyin Platform Module
//
// 本模块提供抖音(抖音创作服务平台)的完整功能实现：
// - 账号授权认证 (Browser-based OAuth)
// - 视频上传发布 (Video Upload & Publishing)
// - 作品评论爬取 (Comment Scraping)
// - 数据统计 (Statistics)
//
// 代码分层结构：
// - client.rs: HTTP客户端，API通信
// - uploader.rs: 视频上传器，VOD服务
// - publisher.rs: 发布器，流程编排
// - auth.rs: 浏览器自动化授权
// - comments.rs: 评论获取与管理
// - signature_v4.rs: AWS V4签名算法

// 子模块
mod client;
mod uploader;
mod publisher;
mod auth;
mod comments;
mod signature_v4;

// 重新导出
pub use client::{DouyinClient, UploadOptions, UploadAuth, Challenge, HeaderTicket, BdTicketConfig};
pub use uploader::VideoUploader;
pub use publisher::DouyinPublisher;
pub use auth::{DouyinAuthenticator, AuthResult};
pub use comments::{DouyinCommentClient, Comment, CommentReply};

use async_trait::async_trait;
use crate::core::{
    Platform, PlatformType, UserAccount, PlatformCredentials,
    PlatformPublication, PublicationStatus, PublicationStats, PublishResult, PublishRequest,
    PlatformError,
};
use crate::storage::DatabaseManager;

// ============================================================================
// 抖音平台主实现
// ============================================================================

/// 抖音平台主结构体
/// 实现了Platform trait，提供统一的平台操作接口
#[derive(Debug, Clone)]
pub struct DouyinPlatform {
    /// 数据库管理器，用于账号和作品持久化
    storage: Option<DatabaseManager>,
    /// BD Ticket API配置
    bd_ticket_config: BdTicketConfig,
}

impl DouyinPlatform {
    /// 创建新的抖音平台实例（无存储）
    pub fn new() -> Self {
        Self {
            storage: None,
            bd_ticket_config: BdTicketConfig::default(),
        }
    }

    /// 使用存储创建平台实例
    pub fn with_storage(storage: DatabaseManager) -> Self {
        Self {
            storage: Some(storage),
            bd_ticket_config: BdTicketConfig::default(),
        }
    }

    /// 使用自定义BD Ticket API URL创建平台实例
    pub fn with_bd_ticket_url(storage: DatabaseManager, bd_ticket_url: String) -> Self {
        Self {
            storage: Some(storage),
            bd_ticket_config: BdTicketConfig { api_url: bd_ticket_url },
        }
    }

    /// 从存储中获取账号
    fn get_account(&self, account_id: &str) -> Result<UserAccount, PlatformError> {
        if let Some(storage) = &self.storage {
            storage.get_account(account_id)?
                .ok_or_else(|| PlatformError::AccountNotFound(account_id.to_string()))
        } else {
            Err(PlatformError::StorageError("存储未初始化".to_string()))
        }
    }

    /// 从params解析凭证
    fn get_credentials(&self, account: &UserAccount) -> Result<PlatformCredentials, PlatformError> {
        account.get_credentials()
    }

    /// 保存账号到存储
    fn save_account(&self, account: &UserAccount) -> Result<(), PlatformError> {
        if let Some(storage) = &self.storage {
            storage.save_account(account)
                .map_err(|e| PlatformError::StorageError(e.to_string()))
        } else {
            Err(PlatformError::StorageError("存储未初始化".to_string()))
        }
    }

    /// 保存作品到存储
    fn save_publication(&self, publication: &PlatformPublication) -> Result<(), PlatformError> {
        if let Some(storage) = &self.storage {
            storage.save_publication(publication)
                .map_err(|e| PlatformError::StorageError(e.to_string()))
        } else {
            Err(PlatformError::StorageError("存储未初始化".to_string()))
        }
    }
}

impl Default for DouyinPlatform {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Platform for DouyinPlatform {
    /// 获取平台类型
    fn platform_type(&self) -> PlatformType {
        PlatformType::Douyin
    }

    /// 获取平台名称
    fn platform_name(&self) -> String {
        "抖音".to_string()
    }

    /// 账号认证
    /// 使用无头浏览器打开创作服务平台，等待用户扫码/登录后获取凭证
    async fn authenticate_account(&self) -> Result<UserAccount, PlatformError> {
        let mut authenticator = DouyinAuthenticator::new();
        let account = authenticator.authenticate().await?;
        // 保存到存储
        self.save_account(&account)?;
        Ok(account)
    }

    /// 刷新账号凭证
    async fn refresh_credentials(&self, account_id: &str) -> Result<UserAccount, PlatformError> {
        let account = self.get_account(account_id)?;
        // 重新认证获取新凭证
        let mut authenticator = DouyinAuthenticator::new();
        let updated_account = authenticator.refresh(account).await?;
        // 保存更新后的账号
        self.save_account(&updated_account)?;
        Ok(updated_account)
    }

    /// 发布视频
    async fn publish_video(&self, request: PublishRequest) -> Result<PublishResult, PlatformError> {
        // 从存储获取账号
        let account = self.get_account(&request.account_id)?;

        // 从params解析凭证
        let credentials = self.get_credentials(&account)?;

        // 保存需要用于后续的字段
        let cover_path = request.cover_path.clone();
        let video_path = request.video_path.clone();
        let account_id = request.account_id.clone();
        let title = request.title.clone();
        let description = request.description.clone();

        // 创建发布器并执行发布流程
        let publisher = DouyinPublisher::new(
            account.clone(),
            credentials,
            self.bd_ticket_config.clone()
        );
        let result = publisher.publish(request).await?;

        // 保存作品记录
        let publication = PlatformPublication {
            id: result.publication_id.clone(),
            account_id,
            platform: PlatformType::Douyin,
            title,
            description,
            video_path: video_path.to_string_lossy().to_string(),
            cover_path: cover_path.map(|p| p.to_string_lossy().to_string()),
            status: PublicationStatus::Publishing,
            created_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            published_at: None,
            publish_url: None,
            stats: PublicationStats::default(),
        };
        self.save_publication(&publication)?;

        Ok(result)
    }

    /// 获取作品状态
    async fn get_publication_status(&self, publication_id: &str) -> Result<PlatformPublication, PlatformError> {
        if let Some(storage) = &self.storage {
            storage.get_publication(publication_id)?
                .ok_or_else(|| PlatformError::AccountNotFound(publication_id.to_string()))
        } else {
            Err(PlatformError::StorageError("存储未初始化".to_string()))
        }
    }

    /// 获取账号统计数据
    async fn get_account_stats(&self, _account_id: &str) -> Result<PublicationStats, PlatformError> {
        // TODO: 从抖音API获取统计数据
        Ok(PublicationStats::default())
    }

    /// 从params JSON解析平台凭证
    fn get_credentials_from_params(&self, params: &str) -> Result<PlatformCredentials, PlatformError> {
        let params_value: serde_json::Value = serde_json::from_str(params)
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

        Ok(PlatformCredentials {
            cookie,
            user_agent,
            third_id,
            sec_uid,
            local_data: vec![],
        })
    }
}

