//! 抖音评论提取器
//!
//! 从抖音视频提取评论数据
//!
//! # API 参考
//!
//! - API端点: `https://www.douyin.com/aweme/v1/web/comment/list/`
//! - 参数: aweme_id, cursor, count
//! - 响应: total, comments数组

use crate::core::{Comment, CommentStatus, CommentExtractResult, PlatformError, LocalDataItem};
use crate::platforms::douyin::account_params::AccountParams;
use serde_json::Value;
use uuid::Uuid;
use chrono::Local;
use rand::Rng;

/// 抖音评论API基础URL
const COMMENT_API_URL: &str = "https://www.douyin.com/aweme/v1/web/comment/list/";

/// 每页评论数
const COMMENTS_PER_PAGE: i64 = 50;

/// 最大提取评论数
const MAX_COMMENTS: i64 = 500;

/// 共享的异步HTTP客户端
static ASYNC_CLIENT: std::sync::LazyLock<reqwest::Client> = std::sync::LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to create async HTTP client")
});

/// 抖音评论提取器
///
/// 用于从抖音视频中提取评论数据
#[derive(Debug, Clone)]
pub struct DouyinCommentExtractor {
    /// 用户Cookie
    cookie: String,
    /// User-Agent
    user_agent: String,
    /// 第三方用户ID
    third_id: String,
    /// 本地数据
    local_data: Vec<LocalDataItem>,
}

impl DouyinCommentExtractor {
    /// 创建新的提取器实例
    pub fn new(cookie: String, user_agent: String, third_id: String, local_data: Vec<LocalDataItem>) -> Self {
        Self {
            cookie,
            user_agent,
            third_id,
            local_data: if local_data.is_empty() { Vec::new() } else { local_data },
        }
    }

    /// 从账号参数创建提取器
    pub fn from_params(params_json: &str) -> Result<Self, PlatformError> {
        tracing::info!("[Comment] 解析账号参数...");

        // 使用 AccountParams 解析（与发布功能一致的格式）
        let account_params = AccountParams::from_json(params_json);

        let cookie = account_params.get_cookie();
        let user_agent = account_params.get_user_agent();
        let third_id = account_params.get_third_id();

        tracing::info!("[Comment] cookie长度: {}, user_agent长度: {}, third_id: {}",
            cookie.len(), user_agent.len(), if third_id.is_empty() { "空" } else { "有值" });

        if cookie.is_empty() || user_agent.is_empty() {
            tracing::error!("[Comment] 账号参数不完整: cookie为空={}, user_agent为空={}",
                cookie.is_empty(), user_agent.is_empty());
            return Err(PlatformError::InvalidCredentials(
                "账号参数不完整，缺少cookie或user_agent".to_string()
            ));
        }

        let local_data = account_params.get_local_data();

        tracing::info!("[Comment] local_data条数: {}", local_data.len());

        Ok(Self::new(cookie, user_agent, third_id, local_data))
    }

    /// 提取视频评论
    ///
    /// # 参数
    ///
    /// * `aweme_id` - 作品ID
    /// * `max_count` - 最大提取数量（默认500，上限500）
    ///
    /// # 返回
    ///
    /// 评论提取结果
    pub async fn extract(&self, aweme_id: &str, max_count: i64) -> Result<CommentExtractResult, PlatformError> {
        let max_count = max_count.min(MAX_COMMENTS).max(1);

        tracing::info!("[Comment] 开始提取评论, aweme_id: {}, max_count: {}", aweme_id, max_count);

        let mut all_comments: Vec<Comment> = Vec::new();
        let mut cursor = 0i64;
        let mut total_in_aweme = 0i64;
        let mut consecutive_empty = 0;
        let max_empty_pages = 3; // 连续3页为空则停止

        // 分页提取评论
        while (all_comments.len() as i64) < max_count {
            // 计算本次请求的数量
            let remaining = max_count - all_comments.len() as i64;
            let count = if remaining >= COMMENTS_PER_PAGE { COMMENTS_PER_PAGE } else { remaining };

            tracing::info!("[Comment] 提取第 {} 页, cursor: {}, count: {}", consecutive_empty + 1, cursor, count);

            match self.fetch_comments_page(aweme_id, cursor, count).await {
                Ok(page_result) => {
                    // 更新总数
                    if total_in_aweme == 0 {
                        total_in_aweme = page_result.total;
                    }

                    // 解析评论
                    let page_comments = self.parse_comments(
                        aweme_id,
                        &page_result.comments,
                    );

                    if page_comments.is_empty() {
                        consecutive_empty += 1;
                        tracing::info!("[Comment] 第 {} 页无评论，连续空页: {}", cursor / COMMENTS_PER_PAGE, consecutive_empty);

                        if consecutive_empty >= max_empty_pages {
                            tracing::info!("[Comment] 连续 {} 页无评论，停止提取", consecutive_empty);
                            break;
                        }
                    } else {
                        consecutive_empty = 0;
                        let page_count = page_comments.len();
                        all_comments.extend(page_comments);
                        tracing::info!("[Comment] 提取 {} 条评论，累计: {}/{}", page_count, all_comments.len(), total_in_aweme);
                    }

                    // 检查是否还有更多评论
                    cursor += count;
                    if cursor >= page_result.total {
                        tracing::info!("[Comment] 已到达评论总数，停止提取");
                        break;
                    }
                }
                Err(e) => {
                    tracing::error!("[Comment] 提取评论失败: {}", e);
                    return Ok(CommentExtractResult {
                        success: false,
                        total_extracted: all_comments.len() as i64,
                        total_in_aweme,
                        comments: all_comments,
                        error_message: Some(format!("提取评论失败: {}", e)),
                    });
                }
            }

            // 添加小延迟避免请求过快
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        // 生成ID
        let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        for comment in &mut all_comments {
            comment.id = Uuid::new_v4().to_string();
            comment.created_at = now.clone();
        }

        Ok(CommentExtractResult {
            success: true,
            total_extracted: all_comments.len() as i64,
            total_in_aweme,
            comments: all_comments,
            error_message: None,
        })
    }

    /// 获取单页评论
    async fn fetch_comments_page(
        &self,
        aweme_id: &str,
        cursor: i64,
        count: i64,
    ) -> Result<PageResult, String> {
        tracing::info!("[Comment] fetch_comments_page: aweme_id={}, cursor={}, count={}", aweme_id, cursor, count);

        // 构建URL参数
        let webid = self.get_webid_from_cookies();
        let ms_token = self.generate_ms_token();
        let ttwid = self.get_ttwid();

        tracing::info!("[Comment] webid: {}, ttwid: {}", webid.chars().take(10).collect::<String>(), ttwid.chars().take(10).collect::<String>());

        // 构建完整URL
        let url = format!(
            "{}?device_platform=webapp&aid=6383&channel=channel_pc_web&aweme_id={}&cursor={}&count={}&item_type=0&insert_ids=&whale_cut_token=&cut_version=1&rcFT=&update_version_code=170400&pc_client_type=1&version_code=170400&version_name=17.4.0&cookie_enabled=true&screen_width=1920&screen_height=1080&browser_language=zh-CN&browser_platform=Win32&browser_name=Chrome&browser_version=123.0.0.0&browser_online=true&engine_name=Blink&engine_version=123.0.0.0&os_name=Windows&os_version=10&cpu_core_num=16&device_memory=8&platform=PC&downlink=10&effective_type=4g&round_trip_time=50&webid={}&verifyFp=verify_lwg2oa43_Ga6DRjOO_v2cd_4NL7_AHTp_qMKyKlDdoqra&fp=verify_lwg2oa43_Ga6DRjOO_v2cd_4NL7_AHTp_qMKyKlDdoqra&msToken={}",
            COMMENT_API_URL,
            aweme_id,
            cursor,
            count,
            webid,
            ms_token
        );

        // 计算a_bogus
        let a_bogus = self.calculate_a_bogus(&url);

        let final_url = format!("{}&a_bogus={}", url, a_bogus);

        tracing::info!("[Comment] 发送请求...");

        // 发送请求
        let response = ASYNC_CLIENT
            .get(&final_url)
            .header("Cookie", format!("ttwid={}", ttwid))
            .header("User-Agent", &self.user_agent)
            .header("Accept", "application/json, text/plain, */*")
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
            .header("Referer", format!("https://www.douyin.com/jieunxuan?modal_id={}", aweme_id))
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?;

        tracing::info!("[Comment] 响应状态: {}", response.status());

        let text = response
            .text()
            .await
            .map_err(|e| format!("读取响应失败: {}", e))?;

        tracing::info!("[Comment] 响应长度: {}", text.len());

        // 解析JSON
        let response_json: Value = serde_json::from_str(&text)
            .map_err(|e| format!("解析JSON失败: {}", e))?;

        // 检查状态码
        let status_code = response_json.get("status_code")
            .and_then(|v| v.as_i64())
            .unwrap_or(-1);

        tracing::info!("[Comment] status_code: {}", status_code);

        if status_code != 0 {
            let msg = response_json.get("status_msg")
                .and_then(|v| v.as_str())
                .unwrap_or("未知错误");
            let err_msg = format!("API错误: {} (code: {})", msg, status_code);
            tracing::error!("[Comment] {}", err_msg);
            return Err(err_msg);
        }

        // 提取评论
        let total = response_json.get("total")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let comments = response_json.get("comments")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        tracing::info!("[Comment] total: {}, comments_count: {}", total, comments.len());

        Ok(PageResult { total, comments })
    }

    /// 解析评论数据
    fn parse_comments(&self, aweme_id: &str, comments: &[Value]) -> Vec<Comment> {
        tracing::info!("[Comment] parse_comments: 原始评论数={}", comments.len());

        let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let mut result = Vec::new();
        let mut parse_error_count = 0;

        for c in comments.iter() {
            match self.parse_single_comment(c, aweme_id, &now) {
                Some(comment) => result.push(comment),
                None => parse_error_count += 1,
            }
        }

        tracing::info!("[Comment] parse_comments: 解析成功={}, 解析失败={}", result.len(), parse_error_count);
        result
    }

    /// 解析单条评论
    fn parse_single_comment(&self, c: &Value, aweme_id: &str, now: &str) -> Option<Comment> {
        // 提取用户信息
        let user = c.get("user")?;
        let user_id = user.get("uid")?.as_str()?.to_string();
        let user_nickname = user.get("nickname")?.as_str()?.to_string();
        let user_avatar = user.get("avatar_thumb")
            .and_then(|v| v.get("url_list"))
            .and_then(|v| v.as_array())
            .and_then(|v| v.first())
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // 提取评论内容
        let content = c.get("text")?.as_str()?.to_string();

        // 提取评论ID
        let comment_id = c.get("cid")?.as_str()?.to_string();

        // 提取统计数据
        let like_count = c.get("digg_count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let reply_count = c.get("reply_count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        // 提取评论时间
        let create_time = c.get("create_time")
            .and_then(|v| v.as_i64())
            .map(|ts| {
                chrono::DateTime::from_timestamp(ts, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| now.to_string())
            })
            .unwrap_or_else(|| now.to_string());

        Some(Comment {
            id: Uuid::new_v4().to_string(),
            account_id: String::new(), // 会在外部设置
            aweme_id: aweme_id.to_string(),
            comment_id,
            user_id,
            user_nickname,
            user_avatar,
            content,
            like_count,
            reply_count,
            create_time,
            status: CommentStatus::Completed,
            created_at: now.to_string(),
        })
    }

    /// 从Cookie中提取webid
    fn get_webid_from_cookies(&self) -> String {
        // 从Cookie中解析webid（格式类似 "webid=xxx;"）
        self.cookie.split(';')
            .filter_map(|s| {
                let s = s.trim();
                if s.starts_with("webid=") {
                    Some(s["webid=".len()..].to_string())
                } else {
                    None
                }
            })
            .next()
            .unwrap_or_else(|| Uuid::new_v4().to_string().replace('-', "").chars().take(19).collect())
    }

    /// 生成msToken
    fn generate_ms_token(&self) -> String {
        // 简化的msToken生成
        const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_";
        let mut token = String::with_capacity(84);
        let mut rng = rand::thread_rng();
        for _ in 0..84 {
            let idx = rng.gen_range(0..CHARS.len());
            token.push(CHARS[idx] as char);
        }
        token
    }

    /// 获取或生成ttwid
    fn get_ttwid(&self) -> String {
        // 从Cookie中检查是否已有ttwid
        self.cookie.split(';')
            .filter_map(|s| {
                let s = s.trim();
                if s.starts_with("ttwid=") {
                    Some(s["ttwid=".len()..].to_string())
                } else {
                    None
                }
            })
            .next()
            .unwrap_or_else(|| format!("{}", Uuid::new_v4()))
    }

    /// 计算a_bogus签名
    ///
    /// 注意：这是简化版本，实际需要根据抖音的算法实现
    /// 完整实现需要参考抖音前端的a_bogus计算逻辑
    fn calculate_a_bogus(&self, _url: &str) -> String {
        // TODO: 实现完整的a_bogus计算逻辑
        // 抖音使用WebSDK计算a_bogus，包含URL和User-Agent
        // 暂时返回一个占位符，实际使用时需要完整实现

        // 简化版本：基于URL和UA的哈希
        let combined = format!("{}{}", _url, self.user_agent);
        let hash = md5::compute(combined);
        format!("{:x}", hash)
    }
}

/// 单页评论结果
struct PageResult {
    /// 作品总评论数
    total: i64,
    /// 评论数组
    comments: Vec<Value>,
}
