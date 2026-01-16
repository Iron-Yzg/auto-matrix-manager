// Platform factory - creates platform instances

use super::{Platform, PlatformType, PlatformError};
use crate::platforms::douyin::DouyinPlatform;
use std::sync::Arc;

/// Platform factory for creating platform instances
pub struct PlatformFactory;

impl PlatformFactory {
    /// Create a platform instance based on type
    pub fn create_platform(platform_type: PlatformType) -> Arc<dyn Platform> {
        match platform_type {
            PlatformType::Douyin => Arc::new(DouyinPlatform::new()),
            PlatformType::Xiaohongshu => {
                // TODO: Implement Xiaohongshu platform
                unimplemented!("Xiaohongshu platform not yet implemented")
            }
            PlatformType::Kuaishou => {
                // TODO: Implement Kuaishou platform
                unimplemented!("Kuaishou platform not yet implemented")
            }
            PlatformType::Bilibili => {
                // TODO: Implement Bilibili platform
                unimplemented!("Bilibili platform not yet implemented")
            }
        }
    }

    /// Get all supported platform types
    pub fn supported_platforms() -> Vec<PlatformType> {
        vec![
            PlatformType::Douyin,
            // PlatformType::Xiaohongshu,
            // PlatformType::Kuaishou,
            // PlatformType::Bilibili,
        ]
    }

    /// Get platform type from string
    pub fn platform_type_from_str(s: &str) -> Option<PlatformType> {
        match s.to_lowercase().as_str() {
            "douyin" => Some(PlatformType::Douyin),
            "xiaohongshu" | "xhs" => Some(PlatformType::Xiaohongshu),
            "kuaishou" | "ks" => Some(PlatformType::Kuaishou),
            "bilibili" | "bili" => Some(PlatformType::Bilibili),
            _ => None,
        }
    }
}
