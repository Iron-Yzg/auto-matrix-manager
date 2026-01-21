// Platforms module
// 平台模块
//
// 提供各平台（抖音、快手、小红书等）的发布策略实现
// 使用工厂模式支持不同平台的扩展
//
// # 模块结构
//
// - [douyin](douyin/index.html) - 抖音平台发布策略
// - [traits](traits/index.html) - 发布策略 trait 定义
// - [factory](factory/index.html) - 发布策略工厂

pub mod douyin;
pub mod traits;
pub mod factory;

// 重新导出主要类型，方便使用
pub use crate::platforms::traits::PublishStrategy;
pub use crate::platforms::factory::{PublishStrategyFactory, init_default_strategies};
