//! 发布策略 traits 定义
//!
//! 定义各平台发布策略的通用接口
//! 遵循策略模式，支持不同平台（抖音、快手、小红书等）的视频发布

use crate::core::{PlatformError, PublishResult, PublishRequest, CommentExtractResult};

/// 发布策略 trait
///
/// 所有平台的发布策略都需要实现此接口
#[async_trait::async_trait]
pub trait PublishStrategy: Send + Sync {
    /// 发布视频
    ///
    /// # 参数
    ///
    /// * `request` - 发布请求（包含视频信息和平台账号信息）
    ///
    /// # 返回
    ///
    /// 发布结果
    async fn publish(&self, request: PublishRequest)
        -> Result<PublishResult, PlatformError>;

    /// 获取平台类型
    ///
    /// # 返回
    ///
    /// 平台类型标识：1=抖音; 2=快手; 3=小红书
    fn get_platform_type(&self) -> i64;
}

/// 评论提取策略 trait
///
/// 所有平台的评论提取策略都需要实现此接口
#[async_trait::async_trait]
pub trait CommentExtractor: Send + Sync {
    /// 提取视频评论
    ///
    /// # 参数
    ///
    /// * `account_id` - 账号ID（用于获取凭证）
    /// * `aweme_id` - 作品ID
    /// * `max_count` - 最大提取评论数（默认500，上限500）
    /// * `cursor` - 分页游标（用于增量提取，从0开始）
    ///
    /// # 返回
    ///
    /// 评论提取结果
    async fn extract_comments(&self, account_id: &str, aweme_id: &str, max_count: i64, cursor: i64)
        -> Result<CommentExtractResult, PlatformError>;
}
