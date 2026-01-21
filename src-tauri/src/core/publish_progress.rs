//! Publish Progress Manager
//!
//! 负责管理发布进度并通过Tauri窗口事件推送到前端

use std::sync::{Arc, RwLock, OnceLock};
use chrono::Utc;
use crate::core::{PublishProgressEvent, ProgressStatus};
use tauri::{AppHandle, Emitter};

/// 进度事件发送器
#[derive(Clone)]
pub struct ProgressEmitter {
    // 使用 std::sync::RwLock 因为 set_app_handle 是同步调用
    app_handle: Arc<RwLock<Option<AppHandle>>>,
}

impl ProgressEmitter {
    /// 创建新的进度发射器
    pub fn new() -> Self {
        Self {
            app_handle: Arc::new(RwLock::new(None)),
        }
    }

    /// 设置 AppHandle (同步方法，使用 parkling_lot 的 fast path)
    pub fn set_app_handle(&self, app_handle: AppHandle) {
        // 使用写锁，只在设置时短暂持有
        let mut guard = self.app_handle.write().unwrap();
        *guard = Some(app_handle);
        // guard 在这里释放
    }

    /// 发送进度事件
    pub async fn emit(&self, event: &PublishProgressEvent) {
        // 克隆 AppHandle 以便在释放锁后使用
        let app_handle = {
            let guard = self.app_handle.read().unwrap();
            guard.clone()
        };

        if let Some(handle) = app_handle {
            // 使用 AppHandle 发送事件到所有窗口
            if let Err(e) = handle.emit("publish-progress", event) {
                tracing::warn!("[Progress] Failed to emit progress event: {}", e);
            } else {
                tracing::info!("[Progress] ✅ Emitted: detail_id={}, status={:?}, progress={}%, message={}",
                    event.detail_id, event.status, event.progress, event.message);
            }
        } else {
            tracing::warn!("[Progress] ⚠️ AppHandle is None, progress event not emitted: detail_id={}", event.detail_id);
        }
    }

    /// 便捷方法：发送开始事件
    pub async fn emit_starting(&self, task_id: &str, detail_id: &str, account_id: &str, platform: &str) {
        let event = PublishProgressEvent {
            task_id: task_id.to_string(),
            detail_id: detail_id.to_string(),
            account_id: account_id.to_string(),
            platform: platform.to_string(),
            status: ProgressStatus::Starting,
            message: "开始发布".to_string(),
            progress: 0,
            timestamp: Utc::now().timestamp_millis(),
        };
        self.emit(&event).await;
    }

    /// 便捷方法：发送上传视频事件
    pub async fn emit_uploading_video(&self, task_id: &str, detail_id: &str, account_id: &str, platform: &str, progress: i32) {
        let event = PublishProgressEvent {
            task_id: task_id.to_string(),
            detail_id: detail_id.to_string(),
            account_id: account_id.to_string(),
            platform: platform.to_string(),
            status: ProgressStatus::UploadingVideo,
            message: format!("上传视频中... {}%", progress),
            progress,
            timestamp: Utc::now().timestamp_millis(),
        };
        self.emit(&event).await;
    }

    /// 便捷方法：发送获取凭证事件
    pub async fn emit_getting_ticket(&self, task_id: &str, detail_id: &str, account_id: &str, platform: &str, progress: i32) {
        let event = PublishProgressEvent {
            task_id: task_id.to_string(),
            detail_id: detail_id.to_string(),
            account_id: account_id.to_string(),
            platform: platform.to_string(),
            status: ProgressStatus::GettingTicket,
            message: format!("获取发布凭证... {}%", progress),
            progress,
            timestamp: Utc::now().timestamp_millis(),
        };
        self.emit(&event).await;
    }

    /// 便捷方法：发送发布中事件
    pub async fn emit_publishing(&self, task_id: &str, detail_id: &str, account_id: &str, platform: &str, progress: i32) {
        let event = PublishProgressEvent {
            task_id: task_id.to_string(),
            detail_id: detail_id.to_string(),
            account_id: account_id.to_string(),
            platform: platform.to_string(),
            status: ProgressStatus::Publishing,
            message: format!("发布中... {}%", progress),
            progress,
            timestamp: Utc::now().timestamp_millis(),
        };
        self.emit(&event).await;
    }

    /// 便捷方法：发送完成事件
    pub async fn emit_completed(&self, task_id: &str, detail_id: &str, account_id: &str, platform: &str, message: &str) {
        let event = PublishProgressEvent {
            task_id: task_id.to_string(),
            detail_id: detail_id.to_string(),
            account_id: account_id.to_string(),
            platform: platform.to_string(),
            status: ProgressStatus::Completed,
            message: message.to_string(),
            progress: 100,
            timestamp: Utc::now().timestamp_millis(),
        };
        self.emit(&event).await;
    }

    /// 便捷方法：发送失败事件
    pub async fn emit_failed(&self, task_id: &str, detail_id: &str, account_id: &str, platform: &str, error: &str) {
        let event = PublishProgressEvent {
            task_id: task_id.to_string(),
            detail_id: detail_id.to_string(),
            account_id: account_id.to_string(),
            platform: platform.to_string(),
            status: ProgressStatus::Failed,
            message: error.to_string(),
            progress: 0,
            timestamp: Utc::now().timestamp_millis(),
        };
        self.emit(&event).await;
    }
}

impl Default for ProgressEmitter {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局进度发射器 (使用 OnceLock 延迟初始化)
static PROGRESS_EMITTER_STORAGE: OnceLock<ProgressEmitter> = OnceLock::new();

/// 获取全局进度发射器
pub fn get_progress_emitter() -> &'static ProgressEmitter {
    PROGRESS_EMITTER_STORAGE.get_or_init(|| ProgressEmitter::new())
}
