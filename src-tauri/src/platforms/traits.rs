//! 发布策略 traits 定义
//!
//! 定义各平台发布策略的通用接口
//! 遵循策略模式，支持不同平台（抖音、快手、小红书等）的视频发布

use crate::core::{PlatformError, PublishResult, PublishRequest};

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
