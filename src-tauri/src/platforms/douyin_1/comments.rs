// 抖音评论获取模块
// Douyin Comment Fetching Module
//
// 本模块负责获取和管理作品的评论数据
// 主要功能：
// - 获取作品评论列表
// - 获取评论回复
// - 统计评论数据
// - 导出评论数据
//
// API接口：
// - 获取评论: /web/api/media/comment/list/
// - 获取回复: /web/api/media/comment/reply/list/
// - 评论总数: /web/api/media/comment/count/

use std::time::Duration;
use serde::Serialize;
use crate::core::{UserAccount, PlatformError};
use super::client::BdTicketConfig;

// ============================================================================
// 类型定义
// ============================================================================

/// 单条评论
#[derive(Debug, Clone, Serialize)]
pub struct Comment {
    /// 评论ID
    pub id: String,
    /// 评论内容
    pub text: String,
    /// 评论时间
    pub create_time: String,
    /// 点赞数
    pub digg_count: i64,
    /// 回复数
    pub reply_count: i64,
    /// 用户ID
    pub user_id: String,
    /// 用户昵称
    pub user_nickname: String,
    /// 用户头像
    pub user_avatar: String,
    /// 是否是作者回复
    pub is_author: bool,
    /// 父评论ID（如果是回复）
    pub parent_id: Option<String>,
}

/// 评论回复
#[derive(Debug, Clone, Serialize)]
pub struct CommentReply {
    /// 回复ID
    pub id: String,
    /// 回复内容
    pub text: String,
    /// 回复时间
    pub create_time: String,
    /// 点赞数
    pub digg_count: i64,
    /// 用户ID
    pub user_id: String,
    /// 用户昵称
    pub user_nickname: String,
    /// 用户头像
    pub user_avatar: String,
    /// 是否是作者回复
    pub is_author: bool,
}

/// 评论列表响应
#[derive(Debug, Clone)]
pub struct CommentListResponse {
    /// 评论列表
    pub comments: Vec<Comment>,
    /// 总数
    pub total_count: i64,
    /// 是否有更多
    pub has_more: bool,
    /// 下一页游标
    pub cursor: Option<String>,
}

/// 评论统计
#[derive(Debug, Clone)]
pub struct CommentStats {
    /// 总评论数
    pub total_count: i64,
    /// 今日新增
    pub today_count: i64,
    /// 平均点赞
    pub avg_digg_count: f64,
    /// 最高点赞
    pub max_digg_count: i64,
}

// ============================================================================
// 评论客户端
// ============================================================================

/// 评论获取客户端
/// 用于获取作品的评论数据
#[derive(Debug, Clone)]
pub struct DouyinCommentClient {
    /// 账号信息
    account: Option<UserAccount>,
    /// BD Ticket配置（预留用于后续鉴权）
    _bd_ticket_config: BdTicketConfig,
    /// HTTP客户端
    http_client: reqwest::Client,
}

impl DouyinCommentClient {
    /// 创建新的评论客户端
    pub fn new(account: Option<UserAccount>, bd_ticket_config: BdTicketConfig) -> Self {
        Self {
            account,
            _bd_ticket_config: bd_ticket_config,
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
        }
    }

    /// 获取Cookie
    fn get_cookie(&self) -> String {
        self.account.as_ref()
            .and_then(|a| a.get_credentials().ok())
            .map(|c| c.cookie)
            .unwrap_or_default()
    }

    /// 获取User-Agent
    fn get_user_agent(&self) -> String {
        self.account.as_ref()
            .and_then(|a| a.get_credentials().ok())
            .map(|c| c.user_agent)
            .unwrap_or_else(|| {
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string()
            })
    }

    // -------------------------------------------------------------------------
    // API: 获取评论列表
    // -------------------------------------------------------------------------

    /// 获取作品评论列表
    ///
    /// # 参数
    /// * `aweme_id` - 作品ID（抖音视频ID）
    /// * `cursor` - 分页游标（首次传0）
    /// * `count` - 每页数量（默认20，最大100）
    ///
    /// # 返回
    /// 评论列表响应
    pub async fn get_comments(
        &self,
        aweme_id: &str,
        cursor: i64,
        count: i64,
    ) -> Result<CommentListResponse, PlatformError> {
        let response = self.http_client
            .get("https://creator.douyin.com/web/api/media/comment/list/")
            .query(&[
                ("aweme_id", aweme_id),
                ("page_size", &count.to_string()),
                ("cursor", &cursor.to_string()),
                ("type", "1"), // 1=普通评论
            ])
            .header("Cookie", self.get_cookie())
            .header("User-Agent", self.get_user_agent())
            .send()
            .await
            .map_err(|e| PlatformError::NetworkError(e.to_string()))?;

        let data: serde_json::Value = response.json().await
            .map_err(|e| PlatformError::NetworkError(e.to_string()))?;

        if data["status_code"] != 0 {
            return Err(PlatformError::NetworkError(
                data["status_msg"].as_str()
                    .unwrap_or("获取评论失败")
                    .to_string()
            ));
        }

        // 解析评论列表
        let comments = data["comments"].as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|c| self.parse_comment(c))
            .collect();

        let total_count = data["total_count"].as_i64().unwrap_or(0);
        let has_more = data["has_more"].as_bool().unwrap_or(false);
        let next_cursor = data["cursor"].as_str().map(|s| s.to_string());

        Ok(CommentListResponse {
            comments,
            total_count,
            has_more,
            cursor: next_cursor,
        })
    }

    // -------------------------------------------------------------------------
    // API: 获取评论回复
    // -------------------------------------------------------------------------

    /// 获取评论的回复列表
    ///
    /// # 参数
    /// * `aweme_id` - 作品ID
    /// * `comment_id` - 评论ID
    /// * `cursor` - 分页游标
    /// * `count` - 每页数量
    pub async fn get_replies(
        &self,
        aweme_id: &str,
        comment_id: &str,
        cursor: i64,
        count: i64,
    ) -> Result<CommentListResponse, PlatformError> {
        let response = self.http_client
            .get("https://creator.douyin.com/web/api/media/comment/reply/list/")
            .query(&[
                ("aweme_id", aweme_id),
                ("comment_id", comment_id),
                ("page_size", &count.to_string()),
                ("cursor", &cursor.to_string()),
            ])
            .header("Cookie", self.get_cookie())
            .header("User-Agent", self.get_user_agent())
            .send()
            .await
            .map_err(|e| PlatformError::NetworkError(e.to_string()))?;

        let data: serde_json::Value = response.json().await
            .map_err(|e| PlatformError::NetworkError(e.to_string()))?;

        if data["status_code"] != 0 {
            return Err(PlatformError::NetworkError(
                data["status_msg"].as_str()
                    .unwrap_or("获取回复失败")
                    .to_string()
            ));
        }

        // 解析回复列表
        let replies = data["reply_list"].as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|r| self.parse_comment(r))
            .collect();

        let total_count = data["total_count"].as_i64().unwrap_or(0);
        let has_more = data["has_more"].as_bool().unwrap_or(false);

        Ok(CommentListResponse {
            comments: replies,
            total_count,
            has_more,
            cursor: None,
        })
    }

    // -------------------------------------------------------------------------
    // API: 获取评论数
    // -------------------------------------------------------------------------

    /// 获取作品评论总数
    pub async fn get_comment_count(&self, aweme_id: &str) -> Result<CommentStats, PlatformError> {
        let response = self.http_client
            .get("https://creator.douyin.com/web/api/media/comment/count/")
            .query(&[
                ("aweme_id", aweme_id),
            ])
            .header("Cookie", self.get_cookie())
            .header("User-Agent", self.get_user_agent())
            .send()
            .await
            .map_err(|e| PlatformError::NetworkError(e.to_string()))?;

        let data: serde_json::Value = response.json().await
            .map_err(|e| PlatformError::NetworkError(e.to_string()))?;

        let total_count = data["total_count"].as_i64().unwrap_or(0);

        Ok(CommentStats {
            total_count,
            today_count: 0, // TODO: 计算今日新增
            avg_digg_count: 0.0, // TODO: 计算平均点赞
            max_digg_count: 0, // TODO: 计算最高点赞
        })
    }

    // -------------------------------------------------------------------------
    // 辅助方法
    // -------------------------------------------------------------------------

    /// 解析单条评论
    fn parse_comment(&self, c: &serde_json::Value) -> Comment {
        let user = &c["user"];
        let text_extra = &c["text_extra"];

        // 判断是否包含@作者
        let mut is_author = false;
        if let Some(te) = text_extra.as_array() {
            for item in te {
                if item["type"].as_i64() == Some(2) { // 2=@类型
                    is_author = true;
                    break;
                }
            }
        }

        Comment {
            id: c["cid"].as_str().unwrap_or("").to_string(),
            text: c["text"].as_str().unwrap_or("").to_string(),
            create_time: c["create_time"].as_str()
                .unwrap_or("")
                .to_string(),
            digg_count: c["digg_count"].as_i64().unwrap_or(0),
            reply_count: c["reply_count"].as_i64().unwrap_or(0),
            user_id: user["uid"].as_str().unwrap_or("").to_string(),
            user_nickname: user["nickname"].as_str().unwrap_or("").to_string(),
            user_avatar: user["avatar_thumb"].as_str()
                .unwrap_or(user["avatar_url"].as_str().unwrap_or(""))
                .to_string(),
            is_author,
            parent_id: c["parent_cid"].as_str().map(|s| s.to_string()),
        }
    }

    /// 获取所有评论（分页）
    pub async fn get_all_comments(
        &self,
        aweme_id: &str,
        max_count: Option<i64>,
    ) -> Result<Vec<Comment>, PlatformError> {
        let mut all_comments = Vec::new();
        let mut cursor = 0i64;
        let page_size = 100i64;

        loop {
            let response = self.get_comments(aweme_id, cursor, page_size).await?;

            all_comments.extend(response.comments);

            // 检查数量限制
            if let Some(max) = max_count {
                if all_comments.len() >= max as usize {
                    all_comments.truncate(max as usize);
                    break;
                }
            }

            // 检查是否有更多
            if !response.has_more {
                break;
            }

            cursor = response.cursor
                .and_then(|c| c.parse().ok())
                .unwrap_or(cursor + page_size);
        }

        Ok(all_comments)
    }

    /// 导出评论为JSON
    pub fn export_to_json(&self, comments: &[Comment]) -> String {
        serde_json::to_string_pretty(comments).unwrap_or_default()
    }

    /// 导出评论为CSV
    pub fn export_to_csv(&self, comments: &[Comment]) -> String {
        let mut csv = String::from("ID,内容,时间,点赞数,回复数,用户ID,用户昵称,是否作者\n");

        for c in comments {
            let escaped_text = c.text.replace('"', "\"\"");
            csv.push_str(&format!(
                "{},\"{}\",{},{},{},{},{},{}\n",
                c.id, escaped_text, c.create_time, c.digg_count,
                c.reply_count, c.user_id, c.user_nickname, c.is_author
            ));
        }

        csv
    }
}
