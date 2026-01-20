// 抖音视频发布器
// Douyin Video Publisher
//
// 本模块负责视频发布流程的编排和执行
// 将各个步骤整合成完整的发布流程：
// 1. 获取上传凭证
// 2. 上传视频
// 3. 处理标题和话题
// 4. 调用发布API
// 5. 返回发布结果
//
// 发布流程详解：
// Step 1: 调用 /web/api/media/upload/auth/v5/ 获取上传凭证
// Step 2: 上传视频到VOD，获取video_id
// Step 3: 构建发布数据（标题、描述、话题）
// Step 4: 调用 /web/api/media/aweme/create_v2/ 发布视频
// Step 5: 返回发布结果（item_id）

use crate::core::{UserAccount, PlatformCredentials, PlatformError, PublishRequest, PublishResult};
use crate::platforms::douyin::{
    client::DouyinClient,
    uploader::VideoUploader,
    BdTicketConfig,
};

// ============================================================================
// 发布器
// ============================================================================

/// 视频发布器
/// 编排整个视频发布流程
#[derive(Debug, Clone)]
pub struct DouyinPublisher {
    /// 账号信息
    account: UserAccount,
    /// 平台凭证
    credentials: PlatformCredentials,
    /// BD Ticket配置
    bd_ticket_config: BdTicketConfig,
}

impl DouyinPublisher {
    /// 创建新的发布器
    pub fn new(account: UserAccount, credentials: PlatformCredentials, bd_ticket_config: BdTicketConfig) -> Self {
        Self {
            account,
            credentials,
            bd_ticket_config,
        }
    }

    /// 执行发布流程
    ///
    /// # 发布流程
    /// 1. 创建API客户端
    /// 2. 获取上传凭证 (get_upload_options)
    /// 3. 上传视频到VOD (upload_video)
    /// 4. 处理话题 (search_challenge)
    /// 5. 构建发布数据
    /// 6. 调用发布API (publish_video_v2)
    /// 7. 返回发布结果
    ///
    /// # 参数
    /// * `request` - 发布请求（包含视频路径、标题、描述、话题）
    ///
    /// # 返回
    /// 发布结果（publication_id, item_id）
    pub async fn publish(&self, request: PublishRequest) -> Result<PublishResult, PlatformError> {
        // Step 1: 创建API客户端
        let client = DouyinClient::new(
            self.credentials.clone(),
            self.bd_ticket_config.clone(),
        );

        // Step 2: 获取上传凭证
        let upload_options = client.get_upload_options().await?;

        // Step 3: 上传视频
        let uploader = VideoUploader::new(
            upload_options.auth,
            self.credentials.third_id.clone(),
            self.credentials.user_agent.clone(),
            self.bd_ticket_config.clone(),
        );
        let video_id = uploader.upload_video(&request.video_path).await?;

        // Step 4: 处理标题和话题
        let (caption, text_extra) = self.process_caption_and_hashtags(&client, &request).await?;

        // Step 5: 构建发布数据
        let creation_id = generate_creation_id();
        let publish_data = self.build_publish_data(&request, &video_id, &caption, &text_extra, &creation_id)?;

        // Step 6: 调用发布API
        let publish_response = client.publish_video_v2(&publish_data).await?;

        // Step 7: 返回结果
        let publication_id = uuid::Uuid::new_v4().to_string();

        Ok(PublishResult {
            success: true,
            publication_id,
            item_id: Some(publish_response.item_id),
            error_message: None,
        })
    }

    /// 处理标题和话题
    ///
    /// # 处理逻辑
    /// 1. 拼接标题和描述
    /// 2. 搜索每个话题，获取话题ID
    /// 3. 构建text_extra数组（包含话题位置和ID）
    async fn process_caption_and_hashtags(
        &self,
        client: &DouyinClient,
        request: &PublishRequest,
    ) -> Result<(String, Vec<serde_json::Value>), PlatformError> {
        // 初始标题
        let mut caption = request.description.clone();
        let mut text_extra = vec![];

        // 搜索每个话题并添加到标题
        for hashtag in &request.hashtags {
            match client.search_challenge(hashtag).await {
                Ok(challenges) => {
                    if let Some(challenge) = challenges.first() {
                        // 记录话题在标题中的位置
                        let start = caption.len();
                        caption.push_str(&format!("#{}", challenge.hashtag_name));
                        let end = caption.len();

                        // 添加话题元数据
                        text_extra.push(serde_json::json!({
                            "start": start,
                            "end": end,
                            "type": 1,
                            "hashtag_name": challenge.hashtag_name,
                            "hashtag_id": challenge.hashtag_id,
                            "user_id": "",
                        }));
                    }
                }
                Err(e) => {
                    // 记录警告但继续处理
                    println!("警告: 搜索话题 {} 失败: {}", hashtag, e);
                }
            }
        }

        Ok((caption, text_extra))
    }

    /// 构建发布数据
    fn build_publish_data(
        &self,
        request: &PublishRequest,
        video_id: &str,
        caption: &str,
        text_extra: &[serde_json::Value],
        creation_id: &str,
    ) -> Result<serde_json::Value, PlatformError> {
        // 构建完整标题（标题 + 描述 + 话题）
        let full_title = format!("{} {}", request.title, caption).trim().to_string();

        Ok(serde_json::json!({
            "item": {
                "common": {
                    "text": full_title,
                    "caption": caption,
                    "item_title": request.title,
                    "activity": "[]",
                    "text_extra": serde_json::to_string(text_extra).unwrap_or("[]".to_string()),
                    "challenges": "[]",
                    "mentions": "[]",
                    "hashtag_source": "recommend/recommend",
                    "hot_sentence": "",
                    "download": request.download_allowed,
                    "visibility_type": request.visibility_type,
                    "timing": 0,
                    "creation_id": creation_id,
                    "media_type": 4,
                    "video_id": video_id,
                    "source_info": "{}",
                },
                "anchor": {},
                "mix": {},
                "sync": {
                    "should_sync": false,
                    "sync_to_toutiao": 0,
                },
                "open_platform": {},
                "assistant": {
                    "is_preview": 0,
                    "is_post_assistant": 1,
                },
                "declare": {
                    "user_declare_info": "{}",
                },
            },
        }))
    }
}

/// 生成唯一的creation_id
fn generate_creation_id() -> String {
    format!("creation_{}", chrono::Local::now().timestamp_millis())
}
