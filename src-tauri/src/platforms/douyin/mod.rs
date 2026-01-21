//! 抖音平台模块
//!
//! 提供抖音视频发布功能的相关实现
//!
//! # 模块结构
//!
//! - [`account_params`] - 抖音账号参数结构体
//! - [`utils`] - 工具函数
//! - [`signature_v4`] - AWS Signature V4 签名
//! - [`douyin_client`] - HTTP客户端
//! - [`video_uploader`] - 视频上传器
//! - [`strategy`] - 发布策略（主入口）

use crate::core::{Platform, PlatformType, PlatformError, UserAccount, PublishRequest as CorePublishRequest};
use crate::platforms::traits::PublishStrategy;
use crate::storage::DatabaseManager;
use std::sync::Arc;

pub mod account_params;
pub mod utils;
pub mod signature_v4;
pub mod douyin_client;
pub mod video_uploader;
pub mod strategy;

// 导出主要类型
pub use self::strategy::DouyinPublishStrategy;

/// 平台类型标识
pub const PLATFORM_TYPE_DOUYIN: i64 = 1;

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

    async fn publish_video(&self, request: CorePublishRequest) -> Result<crate::core::PublishResult, PlatformError> {
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

        // 验证必要的参数
        if third_id.is_empty() {
            tracing::error!("[Publish] third_id为空，数据库中的params: {}", account.params);
            return Err(PlatformError::InvalidInput("thirdId不能为空".to_string()));
        }

        // 构建抖音特定平台数据
        let mut platform_data = serde_json::json!({
            "params": account.params,
            "third_id": third_id,
        });

        // 可选字段
        if let Some(record_id) = &request.record_id {
            platform_data["record_id"] = serde_json::json!(record_id);
        }
        if let Some(send_time) = request.send_time {
            platform_data["send_time"] = serde_json::json!(send_time);
        }
        if let Some(music) = &request.music_info {
            platform_data["music_id"] = serde_json::json!(&music.music_id);
            platform_data["music_end_time"] = serde_json::json!(&music.music_end_time);
        }
        if let Some(poi_id) = &request.poi_id {
            platform_data["poi_id"] = serde_json::json!(poi_id);
        }
        if let Some(poi_name) = &request.poi_name {
            platform_data["poi_name"] = serde_json::json!(poi_name);
        }
        if let Some(anchor) = &request.anchor {
            platform_data["anchor"] = anchor.clone();
        }
        if let Some(extra_info) = &request.extra_info {
            if let Some(decl) = &extra_info.self_declaration {
                platform_data["extra_info"] = serde_json::json!({
                    "self_declaration": decl
                });
            }
        }

        // 使用发布策略（带进度跟踪）
        let strategy = match &request.progress_info {
            Some((task_id, detail_id, account_id, app_handle)) => {
                DouyinPublishStrategy::with_progress(task_id, detail_id, account_id, app_handle)
            }
            None => DouyinPublishStrategy::new(),
        };
        tracing::info!("[Publish] 开始调用发布策略，third_id前20字符: {}...", &third_id[..third_id.len().min(20)]);

        // 构造带有平台数据的请求
        let platform_request = CorePublishRequest {
            account_id: request.account_id.clone(),
            video_path: request.video_path.clone(),
            cover_path: request.cover_path.clone(),
            title: request.title.clone(),
            description: request.description.clone(),
            hashtags: request.hashtags.clone(),
            visibility_type: request.visibility_type,
            download_allowed: request.download_allowed,
            timeout: request.timeout,
            record_id: request.record_id.clone(),
            send_time: request.send_time,
            music_info: request.music_info.clone(),
            poi_id: request.poi_id.clone(),
            poi_name: request.poi_name.clone(),
            anchor: request.anchor.clone(),
            extra_info: request.extra_info.clone(),
            platform_data: Some(platform_data),
            progress_info: None,
        };

        let result = strategy.publish(platform_request).await;

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
