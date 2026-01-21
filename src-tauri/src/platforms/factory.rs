//! 发布策略工厂
//!
//! 用于管理不同平台的发布策略
//! 支持动态注册和获取策略
//!
//! # 使用示例
//!
//! ```rust
//! use crate::platforms::{PublishStrategyFactory, PublishStrategy};
//! use crate::core::PlatformType;
//!
//! // 获取抖音发布策略
//! let douyin_strategy = PublishStrategyFactory::get_service(PlatformType::Douyin);
//! if let Some(strategy) = douyin_strategy {
//!     let result = strategy.publish(request).await;
//! }
//! ```

use crate::core::PlatformType;
use std::sync::Arc;
use std::collections::HashMap;
use once_cell::sync::Lazy;
use crate::platforms::traits::PublishStrategy;

/// 策略注册表（线程安全）
static STRATEGY_REGISTRY: Lazy<tokio::sync::RwLock<HashMap<PlatformType, Arc<dyn PublishStrategy>>>> =
    Lazy::new(|| tokio::sync::RwLock::new(HashMap::new()));

/// 发布策略工厂
///
/// 管理所有平台的发布策略，支持：
/// - 静态注册（启动时注册）
/// - 动态注册（运行时注册）
/// - 策略获取
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
    /// 对应的发布策略实例，如果未找到返回 None
    pub async fn get_service(platform_type: PlatformType) -> Option<Arc<dyn PublishStrategy>> {
        let registry = STRATEGY_REGISTRY.read().await;
        registry.get(&platform_type).cloned()
    }

    /// 同步获取策略（不推荐在异步上下文外使用）
    #[doc(hidden)]
    pub fn get_service_sync(platform_type: PlatformType) -> Option<Arc<dyn PublishStrategy>> {
        let registry = STRATEGY_REGISTRY.blocking_read();
        registry.get(&platform_type).cloned()
    }

    /// 注册发布策略
    ///
    /// # 参数
    ///
    /// * `platform_type` - 平台类型
    /// * `strategy` - 发布策略实例
    pub async fn register(platform_type: PlatformType, strategy: Arc<dyn PublishStrategy>) {
        let mut registry = STRATEGY_REGISTRY.write().await;
        registry.insert(platform_type.clone(), strategy);
        let type_id = match platform_type {
            PlatformType::Douyin => 1,
            PlatformType::Kuaishou => 2,
            PlatformType::Xiaohongshu => 3,
            PlatformType::Bilibili => 4,
        };
        tracing::info!(
            "注册平台 {:?} 的发布策略，平台类型: {}",
            platform_type,
            type_id
        );
    }

    /// 同步注册策略
    #[doc(hidden)]
    pub fn register_sync(platform_type: PlatformType, strategy: Arc<dyn PublishStrategy>) {
        let mut registry = STRATEGY_REGISTRY.blocking_write();
        registry.insert(platform_type, strategy);
    }

    /// 获取所有支持的平台类型
    pub async fn supported_platforms() -> Vec<PlatformType> {
        let registry = STRATEGY_REGISTRY.read().await;
        registry.keys().cloned().collect()
    }

    /// 检查是否支持指定平台
    pub async fn is_supported(platform_type: PlatformType) -> bool {
        let registry = STRATEGY_REGISTRY.read().await;
        registry.contains_key(&platform_type)
    }

    /// 清除所有已注册的策略（主要用于测试）
    pub async fn clear() {
        let mut registry = STRATEGY_REGISTRY.write().await;
        registry.clear();
    }
}

/// 初始化默认策略
///
/// 在应用启动时调用，注册所有支持的平台策略
pub async fn init_default_strategies() {
    // 注册抖音策略
    let douyin_strategy: Arc<dyn PublishStrategy> = Arc::new(
        crate::platforms::douyin::DouyinPublishStrategy::new()
    );
    PublishStrategyFactory::register(PlatformType::Douyin, douyin_strategy).await;

    tracing::info!("平台策略初始化完成，支持的平台: {:?}", PublishStrategyFactory::supported_platforms().await);
}
