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
use reqwest;
use std::time::Duration;

/// 抖音评论API基础URL
const COMMENT_API_URL: &str = "https://www.douyin.com/aweme/v1/web/comment/list/";

/// 每页评论数
const COMMENTS_PER_PAGE: i64 = 50;

/// 最大提取评论数
const MAX_COMMENTS: i64 = 500;

/// 共享的异步HTTP客户端
static ASYNC_CLIENT: std::sync::LazyLock<reqwest::Client> = std::sync::LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to create async HTTP client")
});

/// 抖音评论提取器
#[derive(Debug, Clone)]
pub struct DouyinCommentExtractor {
    /// 用户Cookie
    cookie: String,
    /// User-Agent
    user_agent: String,
    /// 第三方用户ID
    #[allow(dead_code)]
    third_id: String,
    /// 本地数据
    #[allow(dead_code)]
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

        let account_params = AccountParams::from_json(params_json);

        let cookie = account_params.get_cookie();
        let user_agent = account_params.get_user_agent();
        let third_id = account_params.get_third_id();

        if cookie.is_empty() || user_agent.is_empty() {
            tracing::error!("[Comment] 账号参数不完整: cookie为空={}, user_agent为空={}",
                cookie.is_empty(), user_agent.is_empty());
            return Err(PlatformError::InvalidCredentials(
                "账号参数不完整，缺少cookie或user_agent".to_string()
            ));
        }

        let local_data = account_params.get_local_data();

        Ok(Self::new(cookie, user_agent, third_id, local_data))
    }

    /// 提取视频评论（单页）
    ///
    /// # 参数
    ///
    /// * `aweme_id` - 视频ID
    /// * `count` - 每次提取的数量
    /// * `cursor` - 分页游标（页码索引，0=第1页，1=第2页...）
    pub async fn extract(&self, aweme_id: &str, count: i64, cursor: i64) -> Result<CommentExtractResult, PlatformError> {
        let count = count.min(MAX_COMMENTS).max(1);
        let cursor = cursor.max(0);

        // 先从视频页面获取ttwid和webid（模拟Python的get_ttwid_webid）
        let (ttwid, webid) = match self.fetch_ttwid_webid(aweme_id).await {
            Ok((t, w)) => (t, w),
            Err(e) => {
                tracing::warn!("[Comment] 获取ttwid/webid失败: {}，使用Cookie中的值", e);
                self.parse_cookies_from_db()
            }
        };

        let ms_token = self.generate_ms_token();

        // 只提取一页，不循环
        match self.fetch_comments_page(aweme_id, cursor, count, &ttwid, &webid, &ms_token).await {
            Ok(page_result) => {
                let mut page_comments = self.parse_comments(aweme_id, &page_result.comments);

                let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                for comment in &mut page_comments {
                    comment.id = Uuid::new_v4().to_string();
                    comment.created_at = now.clone();
                }

                tracing::info!("[Comment] 提取 {} 条评论，总共 {} 条", page_comments.len(), page_result.total);

                Ok(CommentExtractResult {
                    success: true,
                    total_extracted: page_comments.len() as i64,
                    total_in_aweme: page_result.total,
                    comments: page_comments,
                    error_message: None,
                })
            }
            Err(e) => {
                tracing::error!("[Comment] 提取评论失败: {}", e);
                Ok(CommentExtractResult {
                    success: false,
                    total_extracted: 0,
                    total_in_aweme: 0,
                    comments: Vec::new(),
                    error_message: Some(format!("提取评论失败: {}", e)),
                })
            }
        }
    }

    /// 从Cookie中解析ttwid和webid
    fn parse_cookies_from_db(&self) -> (String, String) {
        let mut ttwid = String::new();
        let mut webid = String::new();

        for item in self.cookie.split(';') {
            let item = item.trim();
            if item.starts_with("ttwid=") {
                ttwid = item["ttwid=".len()..].to_string();
            } else if item.starts_with("webid=") {
                webid = item["webid=".len()..].to_string();
            }
        }

        // 如果webid不是纯数字，生成一个
        if webid.chars().any(|c| !c.is_ascii_digit()) {
            webid = self.generate_webid();
        }

        (ttwid, webid)
    }

    /// 从视频页面获取ttwid和webid（模拟Python的get_ttwid_webid）
    async fn fetch_ttwid_webid(&self, aweme_id: &str) -> Result<(String, String), String> {
        let video_url = format!("https://www.douyin.com/jingxuan?modal_id={}", aweme_id);

        let response = ASYNC_CLIENT
            .get(&video_url)
            .header("User-Agent", &self.user_agent)
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .header("Accept-Language", "zh-CN,zh;q=0.9")
            .send()
            .await
            .map_err(|e| format!("请求视频页面失败: {}", e))?;

        // 从Set-Cookie响应头中提取ttwid
        let ttwid = self.extract_cookie_from_headers(&response, "ttwid");
        

        let webid = self.extract_cookie_from_headers(&response, "webid");

        let text = response
            .text()
            .await
            .map_err(|e| format!("读取响应失败: {}", e))?;

        // 如果Cookie中没有webid，从RENDER_DATA JSON中提取
        let webid = if webid.is_empty() {
            self.extract_webid_from_html(&text)
        } else {
            webid
        };

        if webid.is_empty() {
            return Err("未找到webid".to_string());
        }

        Ok((ttwid, webid))
    }

    /// 从响应头Set-Cookie中提取指定cookie值
    fn extract_cookie_from_headers(&self, response: &reqwest::Response, name: &str) -> String {
        // 遍历所有Set-Cookie头
        for (header_name, header_value) in response.headers() {
            if header_name == "set-cookie" {
                if let Ok(value) = header_value.to_str() {
                    // Set-Cookie格式: name=value; Path=/; ...
                    if value.starts_with(&format!("{}=", name)) {
                        let after_name = &value[name.len() + 1..];
                        // 找到分号的位置（cookie值的结束）
                        if let Some(semi) = after_name.find(';') {
                            return after_name[..semi].to_string();
                        }
                        return after_name.to_string();
                    }
                }
            }
        }
        // 如果没找到，生成一个
        if name == "ttwid" {
            self.generate_ttwid()
        } else {
            String::new()
        }
    }

    /// 从HTML的RENDER_DATA中提取webid
    fn extract_webid_from_html(&self, html: &str) -> String {
        // 使用正则提取RENDER_DATA
        let re = regex::Regex::new(r#"<script id="RENDER_DATA" type="application/json">([^<]+)</script>"#)
            .unwrap_or_else(|_| regex::Regex::new(r"").unwrap());

        if let Some(caps) = re.captures(html) {
            if let Some(match_) = caps.get(1) {
                let encoded = match_.as_str();
                // URL解码
                let decoded = percent_encoding::percent_decode_str(encoded)
                    .decode_utf8_lossy()
                    .to_string();

                // 尝试提取user_unique_id
                if let Some(start) = decoded.find("\"user_unique_id\":\"") {
                    let rest = &decoded[start + "\"user_unique_id\":\"".len()..];
                    if let Some(end) = rest.find('"') {
                        return rest[..end].to_string();
                    }
                }
                // 也尝试不带引号的格式
                if let Some(start) = decoded.find("user_unique_id") {
                    let rest = &decoded[start + "user_unique_id".len()..];
                    if rest.starts_with(":\"") {
                        let rest = &rest[2..];
                        if let Some(end) = rest.find('"') {
                            return rest[..end].to_string();
                        }
                    }
                }
            }
        }
        String::new()
    }

    /// 生成ttwid
    fn generate_ttwid(&self) -> String {
        format!("{}", Uuid::new_v4())
    }

    /// 生成webid（19位数字）
    fn generate_webid(&self) -> String {
        let mut rng = rand::thread_rng();
        let mut webid = String::with_capacity(19);
        for _ in 0..19 {
            webid.push(rng.gen_range(0..10).to_string().chars().next().unwrap());
        }
        webid
    }

    /// 生成msToken（107位随机字符串，模拟Python）
    fn generate_ms_token(&self) -> String {
        const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789=";
        let mut token = String::with_capacity(107);
        let mut rng = rand::thread_rng();
        for _ in 0..107 {
            let idx = rng.gen_range(0..CHARS.len());
            token.push(CHARS[idx] as char);
        }
        token
    }

    /// 获取单页评论
    async fn fetch_comments_page(
        &self,
        aweme_id: &str,
        cursor: i64,
        count: i64,
        ttwid: &str,
        webid: &str,
        ms_token: &str,
    ) -> Result<PageResult, String> {
        tracing::info!("[Comment] fetch_comments_page: aweme_id={}, cursor={}, count={}", aweme_id, cursor, count);

        // 构建请求URL（与Python保持一致）
        let req_url = format!(
            "{}?device_platform=webapp&aid=6383&channel=channel_pc_web&aweme_id={}&cursor={}&count={}&item_type=0&insert_ids=&whale_cut_token=&cut_version=1&rcFT=&update_version_code=170400&pc_client_type=1&version_code=170400&version_name=17.4.0&cookie_enabled=true&screen_width=1920&screen_height=1080&browser_language=zh-CN&browser_platform=Win32&browser_name=Chrome&browser_version=123.0.0.0&browser_online=true&engine_name=Blink&engine_version=123.0.0.0&os_name=Windows&os_version=10&cpu_core_num=16&device_memory=8&platform=PC&downlink=10&effective_type=4g&round_trip_time=50&webid={}&verifyFp=verify_lwg2oa43_Ga6DRjOO_v2cd_4NL7_AHTp_qMKyKlDdoqra&fp=verify_lwg2oa43_Ga6DRjOO_v2cd_4NL7_AHTp_qMKyKlDdoqra&msToken={}",
            COMMENT_API_URL,
            aweme_id,
            cursor,
            count,
            webid,
            ms_token
        );

        // 计算a_bogus
        let a_bogus = self.calculate_a_bogus(&req_url);

        let final_url = format!("{}&a_bogus={}", req_url, a_bogus);

        let referer_url = format!("https://www.douyin.com/jingxuan?modal_id={}", aweme_id);

        // 发送请求（请求头与Python保持一致）
        let response = ASYNC_CLIENT
            .get(&final_url)
            .header("sec-ch-ua", "\"Google Chrome\";v=\"123\", \"Not:A-Brand\";v=\"8\", \"Chromium\";v=\"123\"")
            .header("Accept", "application/json, text/plain, */*")
            .header("sec-ch-ua-mobile", "?0")
            .header("User-Agent", &self.user_agent)
            .header("sec-ch-ua-platform", "\"Windows\"")
            .header("Sec-Fetch-Site", "same-origin")
            .header("Sec-Fetch-Mode", "cors")
            .header("Sec-Fetch-Dest", "empty")
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
            .header("Referer", &referer_url)
            .header("Cookie", format!("ttwid={};", ttwid))
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?;

        let text = response
            .text()
            .await
            .map_err(|e| format!("读取响应失败: {}", e))?;

        if text.is_empty() {
            return Err("响应为空".to_string());
        }

        // 解析JSON
        let response_json: Value = serde_json::from_str(&text)
            .map_err(|e| format!("解析JSON失败: {}", e))?;

        // 检查状态码
        let status_code = response_json.get("status_code")
            .and_then(|v| v.as_i64())
            .unwrap_or(-1);

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

        let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let mut result = Vec::new();
        let mut parse_error_count = 0;

        for c in comments.iter() {
            match self.parse_single_comment(c, aweme_id, &now) {
                Some(comment) => result.push(comment),
                None => parse_error_count += 1,
            }
        }
        result
    }

    /// 解析单条评论
    fn parse_single_comment(&self, c: &Value, aweme_id: &str, now: &str) -> Option<Comment> {
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

        let content = c.get("text")?.as_str()?.to_string();
        let comment_id = c.get("cid")?.as_str()?.to_string();

        let like_count = c.get("digg_count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let reply_count = c.get("reply_count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

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
            account_id: String::new(),
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

    /// 计算a_bogus签名
    fn calculate_a_bogus(&self, url: &str) -> String {
        let query = if let Some(pos) = url.find('?') {
            &url[pos + 1..]
        } else {
            url
        };

        // 尝试使用QuickJS
        match self.calculate_a_bogus_with_quickjs(query) {
            Ok(result) => result,
            Err(e) => {
                tracing::warn!("[Comment] a_bogus计算失败: {}，使用占位符", e);
                let combined = format!("{}{}", query, self.user_agent);
                format!("{:x}", md5::compute(combined))
            }
        }
    }

    /// 使用QuickJS计算a_bogus
    fn calculate_a_bogus_with_quickjs(&self, query: &str) -> Result<String, String> {
        let js_path = std::path::PathBuf::from("/Users/yangzhenguo/workspace/auto-matrix-manager/python/utils/a_bogus.js");

        if !js_path.exists() {
            return Err("a_bogus.js文件不存在".to_string());
        }

        let js_code = std::fs::read_to_string(&js_path)
            .map_err(|e| format!("读取文件失败: {}", e))?;

        let runtime = rquickjs::Runtime::new().map_err(|e| format!("创建JS运行时失败: {}", e))?;
        let ctx = rquickjs::Context::builder()
            .build(&runtime)
            .map_err(|e| format!("创建JS上下文失败: {}", e))?;

        ctx.with(|ctx| {
            let js_bytes: Vec<u8> = js_code.into_bytes();
            ctx.eval::<(), _>(js_bytes)
                .map_err(|e| format!("执行JS失败: {:?}", e))?;

            let func: rquickjs::Function = ctx.globals().get("generate_a_bogus")
                .map_err(|e| format!("获取函数失败: {:?}", e))?;

            let result: String = func.call((query, &self.user_agent))
                .map_err(|e| format!("调用函数失败: {:?}", e))?;

            Ok(result)
        })
    }
}

/// 单页评论结果
struct PageResult {
    total: i64,
    comments: Vec<Value>,
}
