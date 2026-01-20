// Platforms module
// 平台模块
//
// 提供各平台（抖音、快手、小红书等）的发布策略实现
// 使用工厂模式支持不同平台的扩展
//
// # 模块结构
//
// - [douyin](douyin/index.html) - 抖音平台发布策略

pub mod douyin;

use crate::core::PlatformType;
use std::sync::Arc;

/// 发布策略接口
///
/// 定义不同平台发布策略的通用接口
#[async_trait::async_trait]
pub trait PublishStrategy: Send + Sync {
    /// 发布视频
    async fn publish(&self, request: crate::platforms::douyin::publish_strategy::PublishRequest)
        -> Result<crate::core::PublishResult, crate::core::PlatformError>;
    /// 获取平台类型
    fn get_platform_type(&self) -> i64;
}

/// 发布策略工厂
///
/// 用于管理不同平台的发布策略
/// 支持动态注册和获取策略
///
/// # 使用示例
///
/// ```rust
/// use crate::platforms::{PublishStrategyFactory, PublishStrategy};
/// use crate::core::PlatformType;
///
/// // 获取抖音发布策略
/// let douyin_strategy = PublishStrategyFactory::get_service(PlatformType::Douyin);
/// if let Some(strategy) = douyin_strategy {
///     let result = strategy.publish(request).await;
/// }
/// ```
pub struct PublishStrategyFactory;

impl PublishStrategyFactory {
    /// 获取指定平台的发布策略
    ///
    /// # 参数
    ///
    /// * `platform_type` - 平台类型
    ///
    /// # 返回
    ///
    /// 对应的发布策略实例，如果未找到返回None
    pub fn get_service(platform_type: PlatformType) -> Option<Arc<dyn PublishStrategy>> {
        match platform_type {
            PlatformType::Douyin => Some(Arc::new(
                crate::platforms::douyin::publish_strategy::DouyinPublishStrategy::new(),
            ) as Arc<dyn PublishStrategy>),
            // 其他平台：预留扩展接口
            // PlatformType::Xiaohongshu => {
            //     // 小红书发布策略（待实现）
            //     None
            // }
            // PlatformType::Kuaishou => {
            //     // 快手发布策略（待实现）
            //     None
            // }
            _ => None,
        }
    }

    /// 注册发布策略（动态注册，用于后期扩展）
    ///
    /// # 参数
    ///
    /// * `platform_type` - 平台类型
    /// * `strategy` - 发布策略实例
    pub fn register(platform_type: PlatformType, strategy: Arc<dyn PublishStrategy>) {
        // TODO: 实现动态注册逻辑
        // 可以使用 RwLock<HashMap<PlatformType, Arc<dyn PublishStrategy>>> 来存储注册的策略
        tracing::info!(
            "注册平台 {:?} 的发布策略，平台类型: {}",
            platform_type,
            strategy.get_platform_type()
        );
    }

    /// 获取所有支持的平台类型
    pub fn supported_platforms() -> Vec<PlatformType> {
        vec![PlatformType::Douyin]
    }

    /// 检查是否支持指定平台
    pub fn is_supported(platform_type: PlatformType) -> bool {
        matches!(platform_type, PlatformType::Douyin)
    }
}
