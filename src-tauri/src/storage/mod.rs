// Storage module - SQLite database operations
// 存储模块 - SQLite 数据库操作（统一账号表）

use rusqlite::{Connection, Result};
use std::path::PathBuf;
use crate::core::{
    UserAccount, PlatformType, AccountStatus, PlatformPublication,
    PublicationStatus, PublicationStats,
};

/// Database manager for SQLite operations
/// 数据库管理器 - 统一存储所有平台账号
#[derive(Clone, Debug)]
pub struct DatabaseManager {
    /// Base path for database files
    pub base_path: PathBuf,
}

/// Platform extractor configuration struct
/// 平台提取引擎配置结构
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ExtractorConfig {
    pub id: String,
    pub platform_id: String,
    pub platform_name: String,
    pub login_url: String,
    pub login_success_pattern: String,
    pub redirect_url: Option<String>,
    pub extract_rules: serde_json::Value,
    pub is_default: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl DatabaseManager {
    /// Create a new database manager
    /// 创建新的数据库管理器
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    /// Get unified database path
    /// 获取统一的数据库路径
    fn get_db_path(&self) -> PathBuf {
        self.base_path.join("matrix.db")
    }

    /// Get or create connection
    /// 获取或创建数据库连接
    fn get_connection(&self) -> Result<Connection> {
        let db_path = self.get_db_path();
        // Create parent directories if needed
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let conn = Connection::open(&db_path)?;
        self.initialize_schema(&conn)?;
        Ok(conn)
    }

    /// Initialize unified database schema
    /// 初始化统一的数据库模式
    fn initialize_schema(&self, conn: &Connection) -> Result<()> {
        // Unified accounts table - 所有平台账号统一存储
        conn.execute(r#"
            CREATE TABLE IF NOT EXISTS accounts (
                id TEXT PRIMARY KEY,
                username TEXT NOT NULL,
                nickname TEXT NOT NULL,
                avatar_url TEXT,
                platform TEXT NOT NULL,
                params TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            )
        "#, [])?;

        // Publications table - 作品发布记录
        conn.execute(r#"
            CREATE TABLE IF NOT EXISTS publications (
                id TEXT PRIMARY KEY,
                account_id TEXT NOT NULL,
                platform TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT,
                video_path TEXT NOT NULL,
                cover_path TEXT,
                status TEXT NOT NULL DEFAULT 'draft',
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                published_at TEXT,
                publish_url TEXT,
                comments TEXT DEFAULT '0',
                likes TEXT DEFAULT '0',
                favorites TEXT DEFAULT '0',
                shares TEXT DEFAULT '0',
                FOREIGN KEY (account_id) REFERENCES accounts(id)
            )
        "#, [])?;

        // Platform extractor configs table - 平台数据提取引擎配置
        conn.execute(r#"
            CREATE TABLE IF NOT EXISTS extractor_configs (
                id TEXT PRIMARY KEY,
                platform_id TEXT NOT NULL UNIQUE,
                platform_name TEXT NOT NULL,
                login_url TEXT NOT NULL,
                login_success_pattern TEXT NOT NULL,
                redirect_url TEXT,
                extract_rules TEXT NOT NULL,
                is_default INTEGER DEFAULT 0,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT DEFAULT CURRENT_TIMESTAMP
            )
        "#, [])?;

        // Initialize default configurations for supported platforms
        Self::initialize_default_configs(conn)?;

        Ok(())
    }

    // ============================================================================
    // 账号操作
    // ============================================================================

    /// Save account to database
    /// 保存账号到数据库
    pub fn save_account(&self, account: &UserAccount) -> Result<(), rusqlite::Error> {
        let conn = self.get_connection()?;

        conn.execute(r#"
            INSERT OR REPLACE INTO accounts (
                id, username, nickname, avatar_url, platform, params, status, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#, &[
            &account.id,
            &account.username,
            &account.nickname,
            &account.avatar_url,
            &format!("{:?}", account.platform),
            &account.params,
            &format!("{:?}", account.status),
            &account.created_at,
        ])?;

        Ok(())
    }

    /// Get account by ID
    /// 根据 ID 获取账号
    pub fn get_account(&self, account_id: &str) -> Result<Option<UserAccount>, rusqlite::Error> {
        let conn = self.get_connection()?;

        let mut stmt = conn.prepare("SELECT * FROM accounts WHERE id = ?")?;

        match stmt.query_row([account_id], |row| {
            Ok(UserAccount {
                id: row.get(0)?,
                username: row.get(1)?,
                nickname: row.get(2)?,
                avatar_url: row.get(3)?,
                platform: Self::parse_platform(row.get::<_, String>(4)?),
                params: row.get(5)?,
                status: Self::parse_status(row.get::<_, String>(6)?),
                created_at: row.get(7)?,
            })
        }) {
            Ok(account) => Ok(Some(account)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Get all accounts
    /// 获取所有账号
    pub fn get_all_accounts(&self) -> Result<Vec<UserAccount>, rusqlite::Error> {
        let conn = self.get_connection()?;

        let mut stmt = conn.prepare("SELECT * FROM accounts ORDER BY created_at DESC")?;
        let accounts = stmt.query_map([], |row| {
            Ok(UserAccount {
                id: row.get(0)?,
                username: row.get(1)?,
                nickname: row.get(2)?,
                avatar_url: row.get(3)?,
                platform: Self::parse_platform(row.get::<_, String>(4)?),
                params: row.get(5)?,
                status: Self::parse_status(row.get::<_, String>(6)?),
                created_at: row.get(7)?,
            })
        })?.filter_map(|r| r.ok()).collect();

        Ok(accounts)
    }

    /// Get accounts by platform
    /// 获取指定平台的所有账号
    pub fn get_accounts_by_platform(&self, platform: PlatformType) -> Result<Vec<UserAccount>, rusqlite::Error> {
        let conn = self.get_connection()?;

        let mut stmt = conn.prepare("SELECT * FROM accounts WHERE platform = ? ORDER BY created_at DESC")?;
        let platform_str = format!("{:?}", platform);
        let accounts = stmt.query_map([platform_str], |row| {
            Ok(UserAccount {
                id: row.get(0)?,
                username: row.get(1)?,
                nickname: row.get(2)?,
                avatar_url: row.get(3)?,
                platform: Self::parse_platform(row.get::<_, String>(4)?),
                params: row.get(5)?,
                status: Self::parse_status(row.get::<_, String>(6)?),
                created_at: row.get(7)?,
            })
        })?.filter_map(|r| r.ok()).collect();

        Ok(accounts)
    }

    /// Delete account
    /// 删除账号
    pub fn delete_account(&self, account_id: &str) -> Result<bool, rusqlite::Error> {
        let conn = self.get_connection()?;

        let rows = conn.execute(
            "DELETE FROM accounts WHERE id = ?",
            [account_id],
        )?;

        Ok(rows > 0)
    }

    // ============================================================================
    // 作品发布操作
    // ============================================================================

    /// Save publication
    /// 保存发布记录
    pub fn save_publication(&self, publication: &PlatformPublication) -> Result<(), rusqlite::Error> {
        let conn = self.get_connection()?;

        conn.execute(r#"
            INSERT OR REPLACE INTO publications (
                id, account_id, platform, title, description, video_path, cover_path,
                status, created_at, published_at, publish_url,
                comments, likes, favorites, shares
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#, &[
            &publication.id,
            &publication.account_id,
            &format!("{:?}", publication.platform),
            &publication.title,
            &publication.description,
            &publication.video_path,
            publication.cover_path.as_ref().unwrap_or(&String::new()),
            &format!("{:?}", publication.status),
            &publication.created_at,
            publication.published_at.as_ref().unwrap_or(&String::new()),
            publication.publish_url.as_ref().unwrap_or(&String::new()),
            &publication.stats.comments.to_string(),
            &publication.stats.likes.to_string(),
            &publication.stats.favorites.to_string(),
            &publication.stats.shares.to_string(),
        ])?;

        Ok(())
    }

    /// Get publication by ID
    /// 根据 ID 获取发布记录
    pub fn get_publication(&self, publication_id: &str) -> Result<Option<PlatformPublication>, rusqlite::Error> {
        let conn = self.get_connection()?;

        let mut stmt = conn.prepare("SELECT * FROM publications WHERE id = ?")?;

        match stmt.query_row([publication_id], |row| {
            Ok(PlatformPublication {
                id: row.get(0)?,
                account_id: row.get(1)?,
                platform: Self::parse_platform(row.get::<_, String>(2)?),
                title: row.get(3)?,
                description: row.get(4)?,
                video_path: row.get(5)?,
                cover_path: Some(row.get(6)?),
                status: Self::parse_publication_status(row.get::<_, String>(7)?),
                created_at: row.get(8)?,
                published_at: Some(row.get(9)?),
                publish_url: Some(row.get(10)?),
                stats: PublicationStats {
                    comments: row.get::<_, String>(11)?.parse().unwrap_or(0),
                    likes: row.get::<_, String>(12)?.parse().unwrap_or(0),
                    favorites: row.get::<_, String>(13)?.parse().unwrap_or(0),
                    shares: row.get::<_, String>(14)?.parse().unwrap_or(0),
                },
            })
        }) {
            Ok(publication) => Ok(Some(publication)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Get publications for an account
    /// 获取账号的所有发布记录
    pub fn get_publications_by_account(&self, account_id: &str) -> Result<Vec<PlatformPublication>, rusqlite::Error> {
        let conn = self.get_connection()?;

        let mut stmt = conn.prepare(
            "SELECT * FROM publications WHERE account_id = ? ORDER BY created_at DESC"
        )?;

        let publications = stmt.query_map([account_id], |row| {
            Ok(PlatformPublication {
                id: row.get(0)?,
                account_id: row.get(1)?,
                platform: Self::parse_platform(row.get::<_, String>(2)?),
                title: row.get(3)?,
                description: row.get(4)?,
                video_path: row.get(5)?,
                cover_path: Some(row.get(6)?),
                status: Self::parse_publication_status(row.get::<_, String>(7)?),
                created_at: row.get(8)?,
                published_at: Some(row.get(9)?),
                publish_url: Some(row.get(10)?),
                stats: PublicationStats {
                    comments: row.get::<_, String>(11)?.parse().unwrap_or(0),
                    likes: row.get::<_, String>(12)?.parse().unwrap_or(0),
                    favorites: row.get::<_, String>(13)?.parse().unwrap_or(0),
                    shares: row.get::<_, String>(14)?.parse().unwrap_or(0),
                },
            })
        })?.filter_map(|r| r.ok()).collect();

        Ok(publications)
    }

    /// Get all publications
    /// 获取所有发布记录
    pub fn get_all_publications(&self) -> Result<Vec<PlatformPublication>, rusqlite::Error> {
        let conn = self.get_connection()?;

        let mut stmt = conn.prepare("SELECT * FROM publications ORDER BY created_at DESC")?;
        let publications = stmt.query_map([], |row| {
            Ok(PlatformPublication {
                id: row.get(0)?,
                account_id: row.get(1)?,
                platform: Self::parse_platform(row.get::<_, String>(2)?),
                title: row.get(3)?,
                description: row.get(4)?,
                video_path: row.get(5)?,
                cover_path: Some(row.get(6)?),
                status: Self::parse_publication_status(row.get::<_, String>(7)?),
                created_at: row.get(8)?,
                published_at: Some(row.get(9)?),
                publish_url: Some(row.get(10)?),
                stats: PublicationStats {
                    comments: row.get::<_, String>(11)?.parse().unwrap_or(0),
                    likes: row.get::<_, String>(12)?.parse().unwrap_or(0),
                    favorites: row.get::<_, String>(13)?.parse().unwrap_or(0),
                    shares: row.get::<_, String>(14)?.parse().unwrap_or(0),
                },
            })
        })?.filter_map(|r| r.ok()).collect();

        Ok(publications)
    }

    // ============================================================================
    // 辅助方法
    // ============================================================================

    /// Parse platform string
    /// 解析平台字符串
    fn parse_platform(s: String) -> PlatformType {
        match s.to_lowercase().as_str() {
            "douyin" => PlatformType::Douyin,
            "xiaohongshu" => PlatformType::Xiaohongshu,
            "kuaishou" => PlatformType::Kuaishou,
            "bilibili" => PlatformType::Bilibili,
            _ => PlatformType::Douyin,
        }
    }

    /// Parse status string
    /// 解析状态字符串
    fn parse_status(s: String) -> AccountStatus {
        match s.to_lowercase().as_str() {
            "active" => AccountStatus::Active,
            "expired" => AccountStatus::Expired,
            _ => AccountStatus::Pending,
        }
    }

    /// Parse publication status string
    /// 解析发布状态字符串
    fn parse_publication_status(s: String) -> PublicationStatus {
        match s.to_lowercase().as_str() {
            "draft" => PublicationStatus::Draft,
            "publishing" => PublicationStatus::Publishing,
            "completed" => PublicationStatus::Completed,
            "failed" => PublicationStatus::Failed,
            _ => PublicationStatus::Draft,
        }
    }

    // ============================================================================
    // 平台提取引擎配置操作
    // ============================================================================

    /// Initialize default configurations for supported platforms
    /// 初始化支持的平台的默认配置
    fn initialize_default_configs(conn: &Connection) -> Result<()> {
        // Default Douyin configuration with proper user_info structure
        let douyin_rules = serde_json::json!({
            "user_info": {
                "nickname": "${api:/web/api/media/user/info:response:body:user:nickname}",
                "avatar_url": "${api:/web/api/media/user/info:response:body:user:avatar_thumb:url_list:0}",
                "third_id": "${api:/account/api/v1/user/account/info:response:body:user:uid}",
                "sec_uid": "${api:/web/api/media/user/info:response:body:user:sec_uid}"
            },
            "request_headers": {
                "cookie": "${api:/account/api/v1/user/account/info:request:headers:cookie}"
            },
            "local_storage": [
                "security-sdk/s_sdk_cert_key",
                "security-sdk/s_sdk_crypt_sdk",
                "security-sdk/s_sdk_pri_key",
                "security-sdk/s_sdk_pub_key"
            ],
            "cookie": {
                "source": "from_api",
                "api_path": "/account/api/v1/user/account/info",
                "header_name": "cookie"
            }
        });

        let configs = vec![
            (
                "douyin",
                "抖音",
                "https://creator.douyin.com/",
                r#"**/creator-micro/**"#,
                Some("https://creator.douyin.com/creator-micro/content/post"),
                douyin_rules,
            ),
            // Add more platform defaults as needed
        ];

        for (platform_id, platform_name, login_url, pattern, redirect_url, rules) in configs {
            conn.execute(
                r#"INSERT OR IGNORE INTO extractor_configs
                    (id, platform_id, platform_name, login_url, login_success_pattern, redirect_url, extract_rules, is_default)
                    VALUES (?, ?, ?, ?, ?, ?, ?, 1)"#,
                &[
                    &format!("config_{}", platform_id),
                    platform_id,
                    platform_name,
                    login_url,
                    pattern,
                    redirect_url.unwrap_or_default(),
                    &rules.to_string(),
                ],
            )?;
        }

        Ok(())
    }

    /// Save extractor configuration
    /// 保存提取引擎配置
    pub fn save_extractor_config(&self, config: &ExtractorConfig) -> Result<(), rusqlite::Error> {
        let conn = self.get_connection()?;

        conn.execute(r#"
            INSERT OR REPLACE INTO extractor_configs (
                id, platform_id, platform_name, login_url, login_success_pattern,
                redirect_url, extract_rules, is_default, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
        "#, &[
            &config.id,
            &config.platform_id,
            &config.platform_name,
            &config.login_url,
            &config.login_success_pattern,
            &config.redirect_url.as_ref().unwrap_or(&String::new()),
            &config.extract_rules.to_string(),
            &if config.is_default { "1".to_string() } else { "0".to_string() },
        ])?;

        Ok(())
    }

    /// Get extractor configuration by platform ID
    /// 根据平台 ID 获取提取引擎配置
    pub fn get_extractor_config(&self, platform_id: &str) -> Result<Option<ExtractorConfig>, rusqlite::Error> {
        let conn = self.get_connection()?;

        let mut stmt = conn.prepare("SELECT * FROM extractor_configs WHERE platform_id = ?")?;

        match stmt.query_row([platform_id], |row| {
            Ok(ExtractorConfig {
                id: row.get(0)?,
                platform_id: row.get(1)?,
                platform_name: row.get(2)?,
                login_url: row.get(3)?,
                login_success_pattern: row.get(4)?,
                redirect_url: Some(row.get(5)?),
                extract_rules: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or(serde_json::json!({})),
                is_default: row.get::<_, i32>(7)? == 1,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        }) {
            Ok(config) => Ok(Some(config)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Get all extractor configurations
    /// 获取所有提取引擎配置
    pub fn get_all_extractor_configs(&self) -> Result<Vec<ExtractorConfig>, rusqlite::Error> {
        let conn = self.get_connection()?;

        let mut stmt = conn.prepare("SELECT * FROM extractor_configs ORDER BY platform_name")?;
        let configs = stmt.query_map([], |row| {
            let rules_str: String = row.get(6)?;
            let rules: serde_json::Value = serde_json::from_str(&rules_str).unwrap_or(serde_json::json!({}));

            Ok(ExtractorConfig {
                id: row.get(0)?,
                platform_id: row.get(1)?,
                platform_name: row.get(2)?,
                login_url: row.get(3)?,
                login_success_pattern: row.get(4)?,
                redirect_url: Some(row.get(5)?),
                extract_rules: rules,
                is_default: row.get::<_, i32>(7)? == 1,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?.filter_map(|r| r.ok()).collect();

        Ok(configs)
    }

    /// Delete extractor configuration
    /// 删除提取引擎配置
    pub fn delete_extractor_config(&self, platform_id: &str) -> Result<bool, rusqlite::Error> {
        let conn = self.get_connection()?;

        // Prevent deleting default configurations
        let is_default = conn.query_row(
            "SELECT is_default FROM extractor_configs WHERE platform_id = ?",
            [platform_id],
            |row| row.get::<_, i32>(0)
        ).unwrap_or(0) == 1;

        if is_default {
            return Err(rusqlite::Error::IntegralValueOutOfRange(
                0,
                0
            ));
        }

        let rows = conn.execute(
            "DELETE FROM extractor_configs WHERE platform_id = ?",
            [platform_id],
        )?;

        Ok(rows > 0)
    }
}
