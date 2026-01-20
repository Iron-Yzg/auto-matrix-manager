//! 抖音平台模块
//!
//! 提供抖音视频发布功能的相关实现
//! 包含：账号参数解析、HTTP客户端、签名、上传、发布策略等
//!
//! # 模块结构
//!
//! - [`account_params`] - 抖音账号参数结构体
//! - [`utils`] - 工具函数
//! - [`signature_v4`] - AWS Signature V4 签名
//! - [`douyin_client`] - HTTP客户端
//! - [`video_uploader`] - 视频上传器
//! - [`publish_strategy`] - 发布策略（主入口）

use crate::core::{Platform, PlatformType, PlatformError, PublishResult, PublishRequest as CorePublishRequest};
use crate::platforms::PublishStrategy;
use std::sync::Arc;
use crate::storage::DatabaseManager;
use crate::core::UserAccount;

/// 抖音账号参数模块
/// 对应数据库中的 params JSON 结构
pub mod account_params;

/// 工具函数模块
/// 提供字符串处理、时间计算等工具方法
pub mod utils;

/// AWS Signature Version 4 签名模块
/// 用于视频上传的请求签名
pub mod signature_v4;

/// 抖音API客户端模块
/// 负责与抖音服务器进行HTTP通信
pub mod douyin_client;

/// 视频上传器模块
/// 负责将视频文件上传到抖音VOD服务器
pub mod video_uploader;

/// 抖音发布策略模块
/// 实现策略模式，支持视频发布
pub mod publish_strategy;

/// 平台类型标识
/// 1 = 抖音
pub const PLATFORM_TYPE_DOUYIN: i64 = 1;

/// VOD API URL
const VOD_API_URL: &str = "https://vod.bytedanceapi.com/";

/// BD Ticket API URL
const BD_TICKET_API_URL: &str = "https://sssj-acibpxtpbg.cn-beijing.fcapp.run/douyin/bd-ticket-guard-client-data";

/// 视频分片大小 (5MB)
const VIDEO_MAX_SIZE: u64 = 5 * 1024 * 1024;

/// 并发上传限制
const CONCURRENT_LIMIT: usize = 2;

// 导出主要类型
pub use self::publish_strategy::{DouyinPublishStrategy, PublishRequest};

/// 抖音平台实现
///
/// 包装发布策略，提供Platform trait实现
#[derive(Debug, Clone)]
pub struct DouyinPlatform {
    /// 数据库管理器（用于获取账号信息）
    db_manager: Option<Arc<DatabaseManager>>,
}

impl DouyinPlatform {
    /// 创建新的平台实例
    pub fn new() -> Self {
        Self { db_manager: None }
    }

    /// 创建带数据库管理器的平台实例
    pub fn with_storage(db_manager: DatabaseManager) -> Self {
        Self {
            db_manager: Some(Arc::new(db_manager)),
        }
    }
}

impl Default for DouyinPlatform {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Platform for DouyinPlatform {
    fn platform_type(&self) -> PlatformType {
        PlatformType::Douyin
    }

    fn platform_name(&self) -> String {
        "抖音".to_string()
    }

    async fn authenticate_account(&self) -> Result<UserAccount, PlatformError> {
        Err(PlatformError::AuthenticationFailed(
            "浏览器认证需要使用 start_browser_auth 命令".to_string(),
        ))
    }

    async fn refresh_credentials(&self, _account_id: &str) -> Result<UserAccount, PlatformError> {
        Err(PlatformError::AuthenticationFailed(
            "请重新进行浏览器认证".to_string(),
        ))
    }

    async fn publish_video(&self, request: CorePublishRequest) -> Result<PublishResult, PlatformError> {
        tracing::info!("[Publish] 开始抖音发布流程，账号ID: {}", request.account_id);
        tracing::info!("[Publish] 视频路径: {:?}", request.video_path);
        tracing::info!("[Publish] 视频标题: {}", request.title);

        // 检查 db_manager 是否可用
        if self.db_manager.is_none() {
            tracing::error!("[Publish] db_manager 未初始化，无法获取账号参数");
            return Err(PlatformError::InvalidInput("平台未配置数据库连接".to_string()));
        }
        let db_manager = self.db_manager.as_ref().unwrap();

        // 从数据库获取账号参数
        tracing::info!("[Publish] 正在查询账号信息: {}", request.account_id);
        let account = match db_manager.get_account(&request.account_id) {
            Ok(Some(acc)) => acc,
            Ok(None) => {
                tracing::error!("[Publish] 未找到账号: {}", request.account_id);
                return Err(PlatformError::InvalidInput(
                    format!("账号不存在: {}", request.account_id)
                ));
            }
            Err(e) => {
                tracing::error!("[Publish] 查询账号失败: {:?}", e);
                return Err(PlatformError::InvalidInput(
                    format!("查询账号失败: {:?}", e)
                ));
            }
        };

        // 解析params JSON获取third_id
        let account_params = account_params::AccountParams::from_json(&account.params);

        let third_id = account_params.get_third_id();

        tracing::info!("[Publish] get_third_id()返回: {}", if third_id.is_empty() { "为空!" } else { &third_id });

        // 解析cookie和user_agent用于调试
        let cookie_len = account_params.get_cookie().len();
        let ua_len = account_params.get_user_agent().len();
        tracing::info!("[Publish] cookie长度: {}, user_agent长度: {}", cookie_len, ua_len);

        // 将 CorePublishRequest 转换为 DouyinPublishRequest
        let douyin_request = PublishRequest {
            account_id: request.account_id.clone(),
            video_path: request.video_path,
            cover_path: request.cover_path,
            title: request.title,
            description: Some(request.description),
            hashtag_names: request.hashtags,
            record_id: None,
            third_id: Some(third_id.clone()),
            params: account.params.clone(), // 从数据库获取的账号参数
            download_allowed: Some(request.download_allowed),
            visibility_type: Some(request.visibility_type),
            timeout: Some(request.timeout),
            send_time: None,
            music_info: None,
            poi_id: None,
            poi_name: None,
            anchor: None,
            extra_info: None,
        };

        // 验证必要的参数
        if third_id.is_empty() {
            tracing::error!("[Publish] third_id为空，数据库中的params: {}", account.params);
            return Err(PlatformError::InvalidInput("thirdId不能为空".to_string()));
        }

        // 使用发布策略
        let strategy = DouyinPublishStrategy::new();
        tracing::info!("[Publish] 开始调用发布策略，third_id前20字符: {}...", &third_id[..third_id.len().min(20)]);

        let result = strategy.publish(douyin_request).await;

        match &result {
            Ok(r) => tracing::info!("[Publish] 发布结果: success={}, item_id={:?}", r.success, r.item_id),
            Err(e) => tracing::error!("[Publish] 发布失败: {:?}", e),
        }
        result
    }

    async fn get_publication_status(&self, _publication_id: &str) -> Result<crate::core::PlatformPublication, PlatformError> {
        Err(PlatformError::PublicationFailed(
            "暂不支持获取发布状态".to_string(),
        ))
    }

    async fn get_account_stats(&self, _account_id: &str) -> Result<crate::core::PublicationStats, PlatformError> {
        Err(PlatformError::PublicationFailed(
            "暂不支持获取账号统计".to_string(),
        ))
    }

    fn get_credentials_from_params(&self, params: &str) -> Result<crate::core::PlatformCredentials, PlatformError> {
        use self::account_params::AccountParams;
        let account_params = AccountParams::from_json(params);

        let cookie = account_params.get_cookie();
        let user_agent = account_params.get_user_agent();
        let third_id = account_params.get_third_id();
        let local_data = account_params.get_local_data();

        if cookie.is_empty() || user_agent.is_empty() || third_id.is_empty() {
            return Err(PlatformError::InvalidCredentials(
                "账号参数不完整".to_string(),
            ));
        }

        Ok(crate::core::PlatformCredentials {
            cookie,
            user_agent,
            third_id,
            sec_uid: None,
            local_data,
        })
    }
}
