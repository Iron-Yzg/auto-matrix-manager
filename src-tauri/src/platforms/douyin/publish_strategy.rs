//! 抖音视频发布策略
//!
//! 实现策略模式，支持将视频发布到抖音平台
//!
//! # 发布流程（9步）
//!
//! 1. **参数校验** - 检查视频路径和账号参数
//! 2. **解析账号参数** - 从数据库params字段解析抖音账号信息
//! 3. **创建客户端** - 创建DouyinClient实例
//! 4. **上传视频** - 获取上传配置 → V4签名上传 → 提交完成
//! 5. **获取BD凭证** - 调用BD Ticket API获取安全凭证
//! 6. **获取CSRF Token** - 从响应头获取CSRF Token
//! 7. **处理文案和话题** - 处理标题、描述、话题标签
//! 8. **构建发布数据** - 组装发布请求数据
//! 9. **发布视频** - 调用发布接口
//!
//! # 使用示例
//!
//! ```rust
//! use crate::platforms::douyin::publish_strategy::{DouyinPublishStrategy, PublishRequest};
//! use crate::platforms::PublishStrategyFactory;
//! use crate::core::PlatformType;
//!
//! // 创建发布请求
//! let request = PublishRequest {
//!     video_path: "/path/to/video.mp4".into(),
//!     title: "视频标题".to_string(),
//!     description: Some("视频描述".to_string()),
//!     hashtag_names: vec!["话题1".to_string(), "话题2".to_string()],
//!     params: r#"{"third_id":"123","third_param":{"cookie":"xxx"}}"#.to_string(),
//!     // ...其他字段
//! };
//!
//! // 使用工厂获取发布策略
//! let strategy = PublishStrategyFactory::get_service(PlatformType::Douyin);
//! if let Some(s) = strategy {
//!     let result = s.publish(request).await;
//! }
//! ```
//!
//! # 与Java代码对照
//!
//! 本模块完全对应Java中的 `DouyinPublishStrategy.java`

use crate::core::{PlatformError, PublishResult};
use crate::platforms::PublishStrategy;
use crate::platforms::douyin::account_params::AccountParams;
use crate::platforms::douyin::douyin_client::DouyinClient;
use crate::platforms::douyin::utils::{
    calculate_timing, format_poi_anchor_content, generate_creation_id, get_string_length, strip_html_tags,
    to_json_string,
};
use crate::platforms::douyin::video_uploader::VideoUploader;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 平台类型标识
/// 1 = 抖音
pub const PLATFORM_TYPE_DOUYIN: i64 = 1;

/// 发布请求
///
/// 包含视频发布所需的所有信息
#[derive(Debug, Clone)]
pub struct PublishRequest {
    /// 账号ID
    pub account_id: String,
    /// 视频文件路径
    pub video_path: PathBuf,
    /// 封面图片路径
    pub cover_path: Option<PathBuf>,
    /// 视频标题
    pub title: String,
    /// 视频描述（文案）
    pub description: Option<String>,
    /// 话题标签列表
    pub hashtag_names: Vec<String>,
    /// 记录ID（用于回调）
    pub record_id: Option<String>,
    /// 第三方用户ID
    pub third_id: Option<String>,
    /// 账号参数JSON（数据库中的params字段）
    pub params: String,
    /// 是否允许下载
    pub download_allowed: Option<i32>,
    /// 可见性类型
    pub visibility_type: Option<i32>,
    /// 超时时间（秒）
    pub timeout: Option<i64>,
    /// 发送时间（Unix时间戳）
    pub send_time: Option<i64>,
    /// 音乐信息
    pub music_info: Option<HashMap<String, String>>,
    /// POI信息
    pub poi_id: Option<String>,
    pub poi_name: Option<String>,
    /// 锚点信息
    pub anchor: Option<HashMap<String, Value>>,
    /// 额外信息
    pub extra_info: Option<HashMap<String, Value>>,
}

/// 抖音视频发布策略
///
/// 实现 `PublishStrategy` 接口，提供抖音视频发布功能
///
/// # 与Java代码对照
///
/// - `DouyinPublishStrategy.java` (588行)
#[derive(Debug, Clone)]
pub struct DouyinPublishStrategy {
    // 客户端缓存（用于复用）
    client_cache: Arc<Mutex<HashMap<String, DouyinClient>>>,
}

impl DouyinPublishStrategy {
    /// 创建新的发布策略实例
    pub fn new() -> Self {
        Self {
            client_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 获取或创建客户端
    fn get_or_create_client(&self, params: &AccountParams) -> DouyinClient {
        let cookie = params.get_cookie();
        let user_agent = params.get_user_agent();
        let third_id = params.get_third_id();
        let local_data = params.get_local_data();

        DouyinClient::new(cookie, user_agent, third_id, local_data)
    }
}

#[async_trait::async_trait]
impl PublishStrategy for DouyinPublishStrategy {
    /// 发布视频到抖音平台
    ///
    /// # 参数
    ///
    /// * `request` - 发布请求
    ///
    /// # 返回
    ///
    /// 发布结果
    ///
    /// # 错误
    ///
    /// 如果发布失败，返回错误信息
    async fn publish(&self, request: PublishRequest) -> Result<PublishResult, PlatformError> {
        // ========== 步骤1: 参数校验 ==========
        tracing::info!("[Publish] ====== 步骤1: 参数校验 ======");

        if request.video_path.as_os_str().is_empty() {
            return Err(PlatformError::InvalidInput("视频路径不能为空".to_string()));
        }

        if request.params.is_empty() {
            return Err(PlatformError::InvalidInput("账号参数不能为空".to_string()));
        }

        // ========== 步骤2: 解析抖音账号参数 ==========
        tracing::info!("[Publish] ====== 步骤2: 解析抖音账号参数 ======");

        let account_params = AccountParams::from_json(&request.params);

        let third_id = request.third_id.clone().unwrap_or_else(|| account_params.get_third_id());

        if third_id.is_empty() {
            tracing::error!("[Publish] third_id为空，无法发布视频");
            tracing::error!("[Publish] 请求中的third_id: {:?}", request.third_id);
            tracing::error!("[Publish] 解析AccountParams结果: third_id={:?}, cookie={}...",
                account_params.third_id,
                account_params.get_cookie().len()
            );
            return Err(PlatformError::InvalidInput("thirdId不能为空".to_string()));
        }

        let cookie = account_params.get_cookie();
        let user_agent = account_params.get_user_agent();
        let local_data = account_params.get_local_data();


        // ========== 步骤3: 创建客户端 ==========
        tracing::info!("[Publish] ====== 步骤3: 创建客户端 ======");

        let mut client = DouyinClient::new(cookie, user_agent, third_id.clone(), local_data);

        // ========== 步骤4: 申请上传地址和凭证 (V4签名) ==========
        tracing::info!("[Publish] ====== 步骤4: 申请上传地址和凭证 ======");

        // 验证视频文件
        if !request.video_path.exists() {
            return Err(PlatformError::InvalidInput(format!(
                "视频文件不存在: {}",
                request.video_path.display()
            )));
        }

        // 上传视频
        let video_id = self.upload_video(&mut client, &request.video_path).await?;

        tracing::info!("[Publish] 视频上传成功, videoId: {}", video_id);

        // ========== 步骤5: 获取BD凭证 ==========
        tracing::info!("[Publish] ====== 步骤5: 获取BD凭证 ======");

        let bd_ticket = client
            .get_header_ticket_key("video")
            .await
            .map_err(|e| PlatformError::VideoUploadFailed(e))?;

        tracing::info!("[Publish] BD凭证获取成功, 包含 {} 个字段", bd_ticket.len());

        // ========== 步骤6: 获取CSRF Token ==========
        tracing::info!("[Publish] ====== 步骤6: 获取CSRF Token ======");

        let csrf_token = client
            .get_csrf_token("/web/api/media/aweme/create_v2/")
            .await
            .map_err(|e| PlatformError::AuthenticationFailed(e))?;

        tracing::info!("[Publish] CSRF Token获取成功:{}", &csrf_token);

        // ========== 步骤7: 处理文案和话题标签 ==========
        tracing::info!("[Publish] ====== 步骤7: 处理文案和话题标签 ======");

        let caption_result = self.process_caption_and_hashtags(
            &mut client,
            &request.title,
            request.description.as_deref().unwrap_or(""),
            &request.hashtag_names,
        ).await;

        tracing::info!("[Publish] 文案处理完成, 标题长度: {}, 话题数: {}",
            caption_result.item_title.len(), caption_result.challenges.len());

        // ========== 步骤8: 构建发布数据 ==========
        tracing::info!("[Publish] ====== 步骤8: 构建发布数据 ======");

        let publish_data = self.build_publish_data(caption_result, &video_id, &request);

        // ========== 步骤9: 发布视频 ==========
        tracing::info!("[Publish] ====== 步骤9: 发布视频到抖音 ======");

        let post_result = client
            .get_public_video_v2(publish_data, Some(csrf_token), Some(bd_ticket))
            .await
            .map_err(|e| PlatformError::PublicationFailed(e))?;

        // 构建返回结果
        let timing = self.get_timing_from_publish_data(&post_result);
        let item_id = self.get_item_id_from_result(&post_result);

        tracing::info!("抖音视频发布成功, itemId: {}", item_id);

        Ok(PublishResult {
            success: true,
            publication_id: request.record_id.clone().unwrap_or_default(),
            item_id: Some(item_id),
            error_message: None,
        })
    }

    /// 获取平台类型
    fn get_platform_type(&self) -> i64 {
        PLATFORM_TYPE_DOUYIN
    }
}

impl DouyinPublishStrategy {
    /// 上传视频（包含步骤4-6）
    ///
    /// 步骤4: 获取上传配置
    /// 步骤5: 申请上传地址和凭证（V4签名）
    /// 步骤6: 上传视频内容
    /// 步骤7: 提交上传完成（V4签名）
    async fn upload_video(&self, client: &mut DouyinClient, video_path: &PathBuf) -> Result<String, PlatformError> {
        // 步骤4: 获取上传配置
        tracing::info!("[Upload] ====== 步骤4: 获取上传配置 ======");
        let upload_options = client
            .get_upload_options()
            .await
            .map_err(|e| PlatformError::VideoUploadFailed(e))?;

        let auth = upload_options
            .get("auth")
            .cloned()
            .ok_or_else(|| PlatformError::VideoUploadFailed("获取上传配置失败：auth为空".to_string()))?;

        tracing::debug!("[Upload] auth类型: {:?}", auth);

        // 将auth转换为HashMap
        let upload_auth: HashMap<String, Value> = match auth {
            Value::Object(map) => {
                tracing::debug!("[Upload] auth是JSON对象，包含 {} 个字段", map.len());
                map
                .iter()
                .filter(|(_, v)| !v.is_null())
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
            }
            Value::String(s) => {
                // 如果是字符串，尝试解析JSON
                tracing::debug!("[Upload] auth是字符串: {}...", &s[..s.len().min(200)]);
                match serde_json::from_str(&s) {
                    Ok(parsed) => {
                        tracing::debug!("[Upload] auth字符串解析成功");
                        parsed
                    }
                    Err(e) => {
                        tracing::warn!("[Upload] auth字符串解析失败: {}，使用空HashMap", e);
                        HashMap::new()
                    }
                }
            }
            _ => {
                tracing::warn!("[Upload] auth类型未知: {:?}，使用空HashMap", auth);
                HashMap::new()
            }
        };

        tracing::debug!("[Upload] upload_auth字段数量: {}", upload_auth.len());
        // 调试：打印upload_auth中的key
        tracing::debug!("[Upload] upload_auth keys: {:?}", upload_auth.keys().map(|k| k.as_str()).collect::<Vec<_>>());

        if upload_auth.is_empty() {
            return Err(PlatformError::VideoUploadFailed(
                "获取上传配置失败：auth为空".to_string(),
            ));
        }

        tracing::info!("[Upload] 上传配置获取成功");

        // 步骤5-7: 通过上传器完成视频上传（V4签名上传）
        tracing::info!("[Upload] ====== 步骤5-7: V4签名上传视频到VOD ======");
        let video_path_str = video_path.to_string_lossy().to_string();
        let mut uploader = VideoUploader::new(upload_auth, client.third_id.clone(), client.user_agent.clone());

        let video_id = uploader
            .upload_video(&video_path_str)
            .await
            .map_err(|e| PlatformError::VideoUploadFailed(e))?;

        Ok(video_id)
    }

    /// 处理文案和话题标签
    ///
    /// # 参数
    ///
    /// * `client` - 抖音客户端
    /// * `title` - 视频标题
    /// * `description` - 视频描述
    /// * `hashtag_names` - 话题标签列表
    ///
    /// # 返回
    ///
    /// 处理结果
    async fn process_caption_and_hashtags(
        &self,
        client: &mut DouyinClient,
        title: &str,
        description: &str,
        hashtag_names: &[String],
    ) -> CaptionAndHashtagsResult {
        let mut result = CaptionAndHashtagsResult::default();

        // 处理话题列表
        let mut challenges: Vec<HashMap<String, String>> = Vec::new();
        for hashtag_name in hashtag_names {
            match self.search_challenge(client, hashtag_name).await {
                Ok(challenge) => challenges.push(challenge),
                Err(_) => {
                    // 失败时使用后备方案
                    let mut fallback = HashMap::new();
                    fallback.insert("hashtag_name".to_string(), hashtag_name.clone());
                    fallback.insert("hashtag_id".to_string(), "0".to_string());
                    challenges.push(fallback);
                }
            }
        }

        // 构建话题字符串（#话题1 #话题2）
        let hashtags_str = self.build_hashtags_string(&challenges);

        // item_title: 标题
        let item_title = title.trim().to_string();

        // 清理描述
        let desc = strip_html_tags(description).trim().to_string();

        // caption: 描述 + 话题
        let caption_text = if !desc.is_empty() && !hashtags_str.is_empty() {
            format!("{} {}", desc, hashtags_str)
        } else if !desc.is_empty() {
            desc.clone()
        } else if !hashtags_str.is_empty() {
            hashtags_str.clone()
        } else {
            String::new()
        };

        // text: 标题 + 描述 + 话题
        let text = if !desc.is_empty() && !hashtags_str.is_empty() {
            format!("{} {} {}", item_title, desc, hashtags_str)
        } else if !desc.is_empty() {
            format!("{} {}", item_title, desc)
        } else if !hashtags_str.is_empty() {
            format!("{} {}", item_title, hashtags_str)
        } else {
            item_title.clone()
        };

        // 计算textExtra
        let text_start = if item_title.is_empty() { 0 } else { item_title.len() + 1 };
        let text_extra = self.build_text_extra(&challenges, text_start);

        result.item_title = item_title;
        result.caption = caption_text;
        result.text = text;
        result.challenges = challenges;
        result.text_extra = text_extra;

        result
    }

    /// 根据话题列表构建字符串（#话题1 #话题2）
    fn build_hashtags_string(&self, challenges: &[HashMap<String, String>]) -> String {
        if challenges.is_empty() {
            return String::new();
        }

        challenges
            .iter()
            .enumerate()
            .map(|(i, c)| {
                if i > 0 {
                    format!(" #{}", c.get("hashtag_name").unwrap_or(&String::new()))
                } else {
                    format!("#{}", c.get("hashtag_name").unwrap_or(&String::new()))
                }
            })
            .collect()
    }

    /// 搜索话题
    async fn search_challenge(&self, client: &mut DouyinClient, keyword: &str) -> Result<HashMap<String, String>, String> {
        let response = client.search_challenge_sug(keyword).await;

        let empty_vec: Vec<Value> = Vec::new();
        let sug_list = response
            .get("sug_list")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);

        let mut hashtag_id = String::new();
        if !sug_list.is_empty() {
            if let Some(first_item) = sug_list.first() {
                hashtag_id = first_item
                    .get("cid")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
            }
        }

        let mut challenge = HashMap::new();
        challenge.insert("hashtag_name".to_string(), keyword.to_string());
        challenge.insert("hashtag_id".to_string(), hashtag_id);

        Ok(challenge)
    }

    /// 构建话题位置信息
    ///
    /// # 参数
    ///
    /// * `challenges` - 话题列表
    /// * `text_start` - 话题在text中的起始位置
    fn build_text_extra(&self, challenges: &[HashMap<String, String>], text_start: usize) -> Vec<HashMap<String, Value>> {
        let mut text_extra: Vec<HashMap<String, Value>> = Vec::new();

        if challenges.is_empty() {
            return text_extra;
        }

        let mut caption_current_pos = 0;

        for challenge in challenges {
            let hashtag_text = format!("#{}", challenge.get("hashtag_name").unwrap_or(&String::new()));
            let hashtag_len = get_string_length(&hashtag_text);

            // text中的位置
            let start = text_start + caption_current_pos;
            let end = start + hashtag_len;

            // caption中的位置
            let caption_start = caption_current_pos;
            let caption_end = caption_current_pos + hashtag_len;

            let mut extra: HashMap<String, Value> = HashMap::new();
            extra.insert("start".to_string(), Value::Number(serde_json::Number::from(start)));
            extra.insert("end".to_string(), Value::Number(serde_json::Number::from(end)));
            extra.insert("type".to_string(), Value::Number(serde_json::Number::from(1)));
            extra.insert(
                "hashtag_name".to_string(),
                Value::String(challenge.get("hashtag_name").unwrap_or(&String::new()).clone()),
            );
            extra.insert(
                "hashtag_id".to_string(),
                Value::String(challenge.get("hashtag_id").unwrap_or(&String::new()).clone()),
            );
            extra.insert("user_id".to_string(), Value::String(String::new()));
            extra.insert("caption_start".to_string(), Value::Number(serde_json::Number::from(caption_start)));
            extra.insert("caption_end".to_string(), Value::Number(serde_json::Number::from(caption_end)));

            text_extra.push(extra);
            caption_current_pos = caption_end;
        }

        text_extra
    }

    /// 构建发布数据
    ///
    /// # 参数
    ///
    /// * `caption_result` - 文案处理结果
    /// * `video_id` - 视频ID
    /// * `request` - 发布请求
    ///
    /// # 返回
    ///
    /// 发布数据HashMap
    fn build_publish_data(
        &self,
        caption_result: CaptionAndHashtagsResult,
        video_id: &str,
        request: &PublishRequest,
    ) -> HashMap<String, Value> {
        let creation_id = generate_creation_id();

        let mut music_id = String::new();
        let mut music_end_time = String::new();
        if let Some(music_info) = &request.music_info {
            music_id = music_info.get("music_id").cloned().unwrap_or_default();
            music_end_time = music_info.get("music_end_time").cloned().unwrap_or_default();
        }

        let mut common_data: HashMap<String, Value> = HashMap::new();
        common_data.insert("text".to_string(), Value::String(caption_result.text.trim().to_string()));
        common_data.insert("caption".to_string(), Value::String(caption_result.caption.trim().to_string()));
        common_data.insert("item_title".to_string(), Value::String(caption_result.item_title.clone()));
        common_data.insert("activity".to_string(), Value::String("[]".to_string()));
        common_data.insert("text_extra".to_string(), Value::String(to_json_string(&caption_result.text_extra)));
        common_data.insert("challenges".to_string(), Value::String("[]".to_string()));
        common_data.insert("mentions".to_string(), Value::String("[]".to_string()));
        common_data.insert(
            "hashtag_source".to_string(),
            Value::String(if caption_result.challenges.is_empty() {
                String::new()
            } else {
                "search/search".to_string()
            }),
        );
        common_data.insert("hot_sentence".to_string(), Value::String(String::new()));
        common_data.insert(
            "download".to_string(),
            Value::Number(serde_json::Number::from(request.download_allowed.unwrap_or(1))),
        );
        common_data.insert(
            "visibility_type".to_string(),
            Value::Number(serde_json::Number::from(request.visibility_type.unwrap_or(0))),
        );
        common_data.insert("timing".to_string(), Value::Number(serde_json::Number::from(0)));
        common_data.insert("creation_id".to_string(), Value::String(creation_id));
        common_data.insert("media_type".to_string(), Value::Number(serde_json::Number::from(4)));
        common_data.insert("video_id".to_string(), Value::String(video_id.to_string()));
        common_data.insert("source_info".to_string(), Value::String("{}".to_string()));

        if !music_id.is_empty() {
            common_data.insert("music_source".to_string(), Value::Number(serde_json::Number::from(1)));
            common_data.insert("music_id".to_string(), Value::String(music_id));
            common_data.insert("music_end_time".to_string(), Value::String(music_end_time));
        } else {
            common_data.insert("music_source".to_string(), Value::Number(serde_json::Number::from(0)));
            common_data.insert("music_id".to_string(), Value::Null);
        }

        if let Some(timeout) = request.timeout {
            if timeout > 0 {
                let timing = calculate_timing(timeout, request.send_time.unwrap_or(0));
                common_data.insert("timing".to_string(), Value::Number(serde_json::Number::from(timing)));
            }
        }

        let anchor_data = self.build_anchor_data(request);

        let mut item_data: HashMap<String, Value> = HashMap::new();
        item_data.insert("common".to_string(), Value::Object(serde_json::Map::from_iter(common_data)));
        item_data.insert("anchor".to_string(), Value::Object(serde_json::Map::from_iter(anchor_data)));
        item_data.insert("mix".to_string(), Value::Object(serde_json::Map::new()));
        item_data.insert("sync".to_string(), self.create_sync_data());
        item_data.insert("open_platform".to_string(), Value::Object(serde_json::Map::new()));
        item_data.insert("assistant".to_string(), self.create_assistant_data());
        item_data.insert("declare".to_string(), self.create_declare_data(request.extra_info.as_ref()));

        let mut version2_data: HashMap<String, Value> = HashMap::new();
        version2_data.insert("item".to_string(), Value::Object(serde_json::Map::from_iter(item_data)));

        if let Some(timeout) = request.timeout {
            if timeout > 0 {
                version2_data.insert(
                    "timeOut".to_string(),
                    Value::Number(serde_json::Number::from(timeout)),
                );
                version2_data.insert(
                    "sendTime".to_string(),
                    Value::Number(serde_json::Number::from(request.send_time.unwrap_or(0))),
                );
            }
        }

        version2_data
    }

    /// 构建anchor数据
    fn build_anchor_data(&self, request: &PublishRequest) -> HashMap<String, Value> {
        let mut anchor_data: HashMap<String, Value> = HashMap::new();

        // 处理POI
        if let Some(poi_id) = &request.poi_id {
            if !poi_id.is_empty() && poi_id != "null" {
                anchor_data.insert("poi_id".to_string(), Value::String(poi_id.clone()));
                anchor_data.insert(
                    "poi_name".to_string(),
                    Value::String(request.poi_name.clone().unwrap_or_default()),
                );
                anchor_data.insert("anchor_content".to_string(), Value::String(format_poi_anchor_content()));
            }
        }

        // 处理锚点中的POI
        if let Some(anchor) = &request.anchor {
            if let Some(poi_component) = anchor.get("poi_component") {
                if let Some(poi_obj) = poi_component.as_object() {
                    let poi_id = poi_obj.get("poi_id").and_then(|v| v.as_str()).unwrap_or("");
                    let poi_name = poi_obj.get("poi_name").and_then(|v| v.as_str()).unwrap_or("");

                    if !poi_id.is_empty() && poi_id != "null" {
                        anchor_data.insert("poi_id".to_string(), Value::String(poi_id.to_string()));
                        anchor_data.insert("poi_name".to_string(), Value::String(poi_name.to_string()));
                        anchor_data.insert("anchor_content".to_string(), Value::String(format_poi_anchor_content()));
                    }
                }
            }

            // 处理购物车
            if let Some(shop_cart) = anchor.get("shop_cart") {
                if let Some(shop_obj) = shop_cart.as_object() {
                    let shop_draft_id = shop_obj.get("shop_draft_id").and_then(|v| v.as_str()).unwrap_or("");
                    if !shop_draft_id.is_empty() && shop_draft_id != "null" {
                        anchor_data.insert("shop_draft_id".to_string(), Value::String(shop_draft_id.to_string()));
                    }
                }
            }
        }

        anchor_data
    }

    /// 创建同步数据
    fn create_sync_data(&self) -> Value {
        let mut sync: HashMap<String, Value> = HashMap::new();
        sync.insert("should_sync".to_string(), Value::Bool(false));
        sync.insert("sync_to_toutiao".to_string(), Value::Number(serde_json::Number::from(0)));
        Value::Object(serde_json::Map::from_iter(sync))
    }

    /// 创建助手数据
    fn create_assistant_data(&self) -> Value {
        let mut assistant: HashMap<String, Value> = HashMap::new();
        assistant.insert("is_preview".to_string(), Value::Number(serde_json::Number::from(0)));
        assistant.insert("is_post_assistant".to_string(), Value::Number(serde_json::Number::from(1)));
        Value::Object(serde_json::Map::from_iter(assistant))
    }

    /// 创建声明数据
    fn create_declare_data(&self, extra_info: Option<&HashMap<String, Value>>) -> Value {
        let mut declare: HashMap<String, Value> = HashMap::new();

        if let Some(info) = extra_info {
            if let Some(self_declaration) = info.get("self_declaration") {
                declare.insert(
                    "user_declare_info".to_string(),
                    Value::String(to_json_string(self_declaration)),
                );
            } else {
                declare.insert("user_declare_info".to_string(), Value::String("{}".to_string()));
            }
        } else {
            declare.insert("user_declare_info".to_string(), Value::String("{}".to_string()));
        }

        Value::Object(serde_json::Map::from_iter(declare))
    }

    /// 从发布数据获取timing
    fn get_timing_from_publish_data(&self, publish_data: &Value) -> i64 {
        if let Some(item) = publish_data.get("item") {
            if let Some(common) = item.get("common") {
                if let Some(timing) = common.get("timing") {
                    return timing.as_i64().unwrap_or(0);
                }
            }
        }
        0
    }

    /// 从结果获取item_id
    fn get_item_id_from_result(&self, post_result: &Value) -> String {
        post_result
            .get("aweme")
            .and_then(|v| v.get("item_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    }
}

/// 内部类：处理文案和话题的结果
#[derive(Debug, Default, Clone)]
struct CaptionAndHashtagsResult {
    /// 完整文本（标题+描述+话题）
    text: String,
    /// 描述+话题
    caption: String,
    /// 标题
    item_title: String,
    /// 话题列表
    challenges: Vec<HashMap<String, String>>,
    /// 话题位置信息
    text_extra: Vec<HashMap<String, Value>>,
}

impl Default for DouyinPublishStrategy {
    fn default() -> Self {
        Self::new()
    }
}
