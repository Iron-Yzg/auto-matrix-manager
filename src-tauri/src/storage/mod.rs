// Storage module - SQLite database operations
// 存储模块 - SQLite 数据库操作（统一账号表）

use rusqlite::{Connection, Result};
use std::path::PathBuf;
use crate::core::{
    UserAccount, PlatformType, AccountStatus, PlatformPublication,
    PublicationStatus, PublicationStats, PublicationTask, PublicationAccountDetail,
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

        // Publication tasks table - 作品发布任务主表
        conn.execute(r#"
            CREATE TABLE IF NOT EXISTS publication_tasks (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT,
                video_path TEXT NOT NULL,
                cover_path TEXT,
                hashtags TEXT DEFAULT '[]',
                status TEXT NOT NULL DEFAULT 'draft',
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                published_at TEXT
            )
        "#, [])?;

        // Publication accounts table - 账号发布详情子表
        // 注意：title/description/hashtags 只在主表存储，子表只存储关联信息
        // 冗余 account_name 字段便于直接显示
        // 注意：移除了外键约束，允许独立管理
        conn.execute(r#"
            CREATE TABLE IF NOT EXISTS publication_accounts (
                id TEXT PRIMARY KEY,
                publication_task_id TEXT NOT NULL,
                account_id TEXT NOT NULL,
                account_name TEXT NOT NULL,
                platform TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'draft',
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                published_at TEXT,
                publish_url TEXT,
                comments INTEGER DEFAULT 0,
                likes INTEGER DEFAULT 0,
                favorites INTEGER DEFAULT 0,
                shares INTEGER DEFAULT 0
            )
        "#, [])?;

        // Create index for faster queries
        conn.execute(r#"
            CREATE INDEX IF NOT EXISTS idx_publication_accounts_task_id
            ON publication_accounts(publication_task_id)
        "#, [])?;

        // Migration: Add account_name column if not exists
        // 迁移：为 publication_accounts 表添加 account_name 列
        // SQLite doesn't support IF NOT EXISTS for ALTER TABLE, so we check first
        let table_info: Result<Vec<_>, rusqlite::Error> = conn.prepare("PRAGMA table_info(publication_accounts)")?
            .query_map([], |row| Ok(row.get::<_, String>(1)?))?
            .collect();
        if let Ok(columns) = table_info {
            if !columns.contains(&"account_name".to_string()) {
                conn.execute_batch("ALTER TABLE publication_accounts ADD COLUMN account_name TEXT NOT NULL DEFAULT ''")?;
            }
            // Migration: Update existing records with account_name from accounts table
            // 迁移：更新现有记录的 account_name 字段
            conn.execute_batch(r#"
                UPDATE publication_accounts
                SET account_name = (
                    SELECT COALESCE(nickname, username) FROM accounts
                    WHERE accounts.id = publication_accounts.account_id
                )
                WHERE account_name = '' OR account_name IS NULL
            "#)?;

            // Migration: Add message column (失败原因记录)
            if !columns.contains(&"message".to_string()) {
                conn.execute_batch("ALTER TABLE publication_accounts ADD COLUMN message TEXT DEFAULT ''")?;
            }

            // Migration: Add item_id column (发布的视频ID)
            if !columns.contains(&"item_id".to_string()) {
                conn.execute_batch("ALTER TABLE publication_accounts ADD COLUMN item_id TEXT DEFAULT ''")?;
            }
        }

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
    /// 删除账号（已移除外键约束，可直接删除）
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
    // New publication + accounts operations (主表+子表结构)
    // ============================================================================

    /// Save a publication task (main table)
    /// 保存作品发布任务主表
    pub fn save_publication_task(&self, task: &PublicationTask) -> Result<(), rusqlite::Error> {
        let conn = self.get_connection()?;

        conn.execute(r#"
            INSERT OR REPLACE INTO publication_tasks (
                id, title, description, video_path, cover_path, hashtags, status, created_at, published_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#, &[
            &task.id,
            &task.title,
            task.description.as_ref().unwrap_or(&String::new()),
            &task.video_path,
            task.cover_path.as_ref().unwrap_or(&String::new()),
            &serde_json::to_string(&task.hashtags).unwrap_or("[]".to_string()),
            &format!("{:?}", task.status),
            &task.created_at,
            task.published_at.as_ref().unwrap_or(&String::new()),
        ])?;

        Ok(())
    }

    /// Save a publication account detail (sub table)
    /// 保存作品账号详情子表
    pub fn save_publication_account_detail(&self, detail: &PublicationAccountDetail) -> Result<(), rusqlite::Error> {
        let conn = self.get_connection()?;

        conn.execute(r#"
            INSERT OR REPLACE INTO publication_accounts (
                id, publication_task_id, account_id, account_name, platform, status,
                created_at, published_at, publish_url,
                comments, likes, favorites, shares,
                message, item_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#, &[
            &detail.id,
            &detail.publication_task_id,
            &detail.account_id,
            &detail.account_name,
            &format!("{:?}", detail.platform),
            &format!("{:?}", detail.status),
            &detail.created_at,
            detail.published_at.as_ref().unwrap_or(&String::new()),
            detail.publish_url.as_ref().unwrap_or(&String::new()),
            &detail.stats.comments.to_string(),
            &detail.stats.likes.to_string(),
            &detail.stats.favorites.to_string(),
            &detail.stats.shares.to_string(),
            detail.message.as_ref().unwrap_or(&String::new()),
            detail.item_id.as_ref().unwrap_or(&String::new()),
        ])?;

        Ok(())
    }

    /// Save publication task with all account details (transaction)
    /// 保存任务和所有账号详情（事务）
    pub fn save_publication_with_accounts(
        &self,
        task: &PublicationTask,
        accounts: &[PublicationAccountDetail],
    ) -> Result<(), rusqlite::Error> {
        let mut conn = self.get_connection()?;

        // Start transaction
        let tx = conn.transaction()?;

        // Save main task (with hashtags)
        tx.execute(r#"
            INSERT OR REPLACE INTO publication_tasks (
                id, title, description, video_path, cover_path, hashtags, status, created_at, published_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#, &[
            &task.id,
            &task.title,
            task.description.as_ref().unwrap_or(&String::new()),
            &task.video_path,
            task.cover_path.as_ref().unwrap_or(&String::new()),
            &serde_json::to_string(&task.hashtags).unwrap_or("[]".to_string()),
            &format!("{:?}", task.status),
            &task.created_at,
            task.published_at.as_ref().unwrap_or(&String::new()),
        ])?;

        // Save all account details (only store account info, no title/description/hashtags)
        for detail in accounts {
            tx.execute(r#"
                INSERT OR REPLACE INTO publication_accounts (
                    id, publication_task_id, account_id, account_name, platform, status,
                    created_at, published_at, publish_url,
                    comments, likes, favorites, shares,
                    message, item_id
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#, &[
                &detail.id,
                &detail.publication_task_id,
                &detail.account_id,
                &detail.account_name,
                &format!("{:?}", detail.platform),
                &format!("{:?}", detail.status),
                &detail.created_at,
                detail.published_at.as_ref().unwrap_or(&String::new()),
                detail.publish_url.as_ref().unwrap_or(&String::new()),
                &detail.stats.comments.to_string(),
                &detail.stats.likes.to_string(),
                &detail.stats.favorites.to_string(),
                &detail.stats.shares.to_string(),
                detail.message.as_ref().unwrap_or(&String::new()),
                detail.item_id.as_ref().unwrap_or(&String::new()),
            ])?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Get publication task by ID
    /// 根据ID获取作品发布任务
    pub fn get_publication_task(&self, task_id: &str) -> Result<Option<PublicationTask>, rusqlite::Error> {
        let conn = self.get_connection()?;

        let mut stmt = conn.prepare("SELECT * FROM publication_tasks WHERE id = ?")?;

        match stmt.query_row([task_id], |row| {
            let hashtags_str: String = row.get(5)?;
            let hashtags: Vec<String> = serde_json::from_str(&hashtags_str).unwrap_or_default();
            Ok(PublicationTask {
                id: row.get(0)?,
                title: row.get(1)?,
                description: Some(row.get(2)?),
                video_path: row.get(3)?,
                cover_path: Some(row.get(4)?),
                hashtags,
                status: Self::parse_publication_status(row.get::<_, String>(6)?),
                created_at: row.get(7)?,
                published_at: Some(row.get(8)?),
            })
        }) {
            Ok(task) => Ok(Some(task)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Get all publication tasks with their account details
    /// 获取所有作品发布任务及其账号详情
    pub fn get_all_publication_tasks(&self) -> Result<Vec<crate::core::PublicationTaskWithAccounts>, rusqlite::Error> {
        let conn = self.get_connection()?;

        // Get all tasks with hashtags
        let mut task_stmt = conn.prepare("SELECT * FROM publication_tasks ORDER BY created_at DESC")?;
        let tasks: Vec<(PublicationTask, String)> = task_stmt.query_map([], |row| {
            let hashtags_str: String = row.get(5)?;
            let hashtags: Vec<String> = serde_json::from_str(&hashtags_str).unwrap_or_default();
            Ok((PublicationTask {
                id: row.get(0)?,
                title: row.get(1)?,
                description: Some(row.get(2)?),
                video_path: row.get(3)?,
                cover_path: Some(row.get(4)?),
                hashtags,
                status: Self::parse_publication_status(row.get::<_, String>(6)?),
                created_at: row.get(7)?,
                published_at: Some(row.get(8)?),
            }, hashtags_str))
        })?.filter_map(|r| r.ok()).collect();

        // Get all account details (without title/description/hashtags)
        let mut acc_stmt = conn.prepare("SELECT * FROM publication_accounts")?;
        let accounts: Vec<PublicationAccountDetail> = acc_stmt.query_map([], |row| {
            let message: String = row.get(13)?;
            let item_id: String = row.get(14)?;
            Ok(PublicationAccountDetail {
                id: row.get(0)?,
                publication_task_id: row.get(1)?,
                account_id: row.get(2)?,
                account_name: row.get(3)?,  // 冗余的账号名称
                platform: Self::parse_platform(row.get::<_, String>(4)?),
                status: Self::parse_publication_status(row.get::<_, String>(5)?),
                created_at: row.get(6)?,
                published_at: Some(row.get(7)?),
                publish_url: Some(row.get(8)?),
                stats: PublicationStats {
                    comments: row.get(9)?,
                    likes: row.get(10)?,
                    favorites: row.get(11)?,
                    shares: row.get(12)?,
                },
                message: if message.is_empty() { None } else { Some(message) },
                item_id: if item_id.is_empty() { None } else { Some(item_id) },
            })
        })?.filter_map(|r| r.ok()).collect();

        // Group accounts by task
        let mut result = Vec::new();
        for (t, hashtags_str) in tasks {
            let task_accounts: Vec<PublicationAccountDetail> = accounts.iter()
                .filter(|a| a.publication_task_id == t.id)
                .cloned()
                .collect();

            let hashtags: Vec<String> = serde_json::from_str(&hashtags_str).unwrap_or_default();

            result.push(crate::core::PublicationTaskWithAccounts {
                id: t.id,
                title: t.title,
                description: t.description.unwrap_or_default(),
                video_path: t.video_path,
                cover_path: t.cover_path.unwrap_or_default(),
                hashtags,
                status: t.status,
                created_at: t.created_at,
                published_at: t.published_at.unwrap_or_default(),
                accounts: task_accounts,
            });
        }

        Ok(result)
    }

    /// Get account detail by ID
    /// 根据ID获取账号详情
    pub fn get_publication_account_detail(&self, detail_id: &str) -> Result<Option<PublicationAccountDetail>, rusqlite::Error> {
        let conn = self.get_connection()?;

        let mut stmt = conn.prepare("SELECT * FROM publication_accounts WHERE id = ?")?;

        match stmt.query_row([detail_id], |row| {
            let message: String = row.get(13)?;
            let item_id: String = row.get(14)?;
            Ok(PublicationAccountDetail {
                id: row.get(0)?,
                publication_task_id: row.get(1)?,
                account_id: row.get(2)?,
                account_name: row.get(3)?,  // 冗余的账号名称
                platform: Self::parse_platform(row.get::<_, String>(4)?),
                status: Self::parse_publication_status(row.get::<_, String>(5)?),
                created_at: row.get(6)?,
                published_at: Some(row.get(7)?),
                publish_url: Some(row.get(8)?),
                stats: PublicationStats {
                    comments: row.get(9)?,
                    likes: row.get(10)?,
                    favorites: row.get(11)?,
                    shares: row.get(12)?,
                },
                message: if message.is_empty() { None } else { Some(message) },
                item_id: if item_id.is_empty() { None } else { Some(item_id) },
            })
        }) {
            Ok(detail) => Ok(Some(detail)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Delete publication task and all its accounts
    /// 删除作品任务及其所有账号详情
    pub fn delete_publication_task(&self, task_id: &str) -> Result<bool, rusqlite::Error> {
        let conn = self.get_connection()?;

        // Delete account details first (due to FK constraint, but CASCADE should handle it)
        conn.execute(
            "DELETE FROM publication_accounts WHERE publication_task_id = ?",
            [task_id],
        )?;

        // Delete task
        let rows = conn.execute(
            "DELETE FROM publication_tasks WHERE id = ?",
            [task_id],
        )?;

        Ok(rows > 0)
    }

    /// Update publication account detail status
    /// 更新账号发布详情状态
    pub fn update_publication_account_status(
        &self,
        detail_id: &str,
        status: PublicationStatus,
        publish_url: Option<String>,
        message: Option<String>,
        item_id: Option<String>,
    ) -> Result<(), rusqlite::Error> {
        let conn = self.get_connection()?;
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        conn.execute(r#"
            UPDATE publication_accounts
            SET status = ?, published_at = ?, publish_url = ?, message = ?, item_id = ?
            WHERE id = ?
        "#, &[
            &format!("{:?}", status),
            &now,
            publish_url.as_ref().unwrap_or(&String::new()),
            message.as_ref().unwrap_or(&String::new()),
            item_id.as_ref().unwrap_or(&String::new()),
            detail_id,
        ])?;

        Ok(())
    }

    /// Get publication task with account details and account info
    /// 获取作品任务及其详情，包含账号信息
    pub fn get_publication_task_with_accounts(&self, task_id: &str) -> Result<Option<crate::core::PublicationTaskWithAccounts>, rusqlite::Error> {
        let conn = self.get_connection()?;

        // Get the task
        let task = match self.get_publication_task(task_id)? {
            Some(t) => t,
            None => return Ok(None),
        };

        // Get account details - use explicit column names to avoid issues with column order
        let mut acc_stmt = conn.prepare("
            SELECT id, publication_task_id, account_id, account_name, platform, status,
                   created_at, published_at, publish_url, comments, likes, favorites, shares,
                   message, item_id
            FROM publication_accounts WHERE publication_task_id = ?
        ")?;
        let accounts: Vec<PublicationAccountDetail> = acc_stmt.query_map([task_id], |row| {
            let message: String = row.get(13)?;
            let item_id: String = row.get(14)?;
            Ok(PublicationAccountDetail {
                id: row.get(0)?,
                publication_task_id: row.get(1)?,
                account_id: row.get(2)?,
                account_name: row.get(3)?,  // 冗余的账号名称
                platform: Self::parse_platform(row.get::<_, String>(4)?),
                status: Self::parse_publication_status(row.get::<_, String>(5)?),
                created_at: row.get(6)?,
                published_at: Some(row.get(7)?),
                publish_url: Some(row.get(8)?),
                stats: PublicationStats {
                    comments: row.get(9)?,
                    likes: row.get(10)?,
                    favorites: row.get(11)?,
                    shares: row.get(12)?,
                },
                message: if message.is_empty() { None } else { Some(message) },
                item_id: if item_id.is_empty() { None } else { Some(item_id) },
            })
        })?.filter_map(|r| r.ok()).collect();

        Ok(Some(crate::core::PublicationTaskWithAccounts {
            id: task.id,
            title: task.title,
            description: task.description.unwrap_or_default(),
            video_path: task.video_path,
            cover_path: task.cover_path.unwrap_or_default(),
            hashtags: task.hashtags,
            status: task.status,
            created_at: task.created_at,
            published_at: task.published_at.unwrap_or_default(),
            accounts,
        }))
    }

    /// Update main task status based on all account statuses
    /// 根据所有子表状态更新主表状态
    /// 状态规则：
    /// - 如果所有子表都是 Draft -> 主表 Draft
    /// - 如果有任意子表是 Publishing -> 主表 Publishing
    /// - 如果所有子表都完成（Completed 或 Failed）：
    ///   - 如果至少有一个 Completed -> 主表 Completed
    ///   - 如果全部 Failed -> 主表 Failed
    pub fn update_task_status_from_accounts(&self, task_id: &str) -> Result<(), rusqlite::Error> {
        let conn = self.get_connection()?;

        let mut stmt = conn.prepare("SELECT status FROM publication_accounts WHERE publication_task_id = ?")?;
        let statuses: Vec<String> = stmt.query_map([task_id], |row| Ok(row.get(0)?))?
            .filter_map(|r| r.ok())
            .collect();

        if statuses.is_empty() {
            return Ok(());
        }

        let new_status = Self::calculate_task_status(&statuses);

        conn.execute(
            "UPDATE publication_tasks SET status = ? WHERE id = ?",
            [&format!("{:?}", new_status), task_id],
        )?;

        Ok(())
    }

    /// Calculate task status from account statuses
    /// 根据子表状态列表计算主表状态
    fn calculate_task_status(account_statuses: &[String]) -> PublicationStatus {
        let mut has_publishing = false;
        let mut has_completed = false;
        let mut has_failed = false;

        for status in account_statuses {
            match status.to_lowercase().as_str() {
                "draft" => {}
                "publishing" => has_publishing = true,
                "completed" => has_completed = true,
                "failed" => has_failed = true,
                _ => {}
            }
        }

        if has_publishing {
            PublicationStatus::Publishing
        } else if has_completed {
            PublicationStatus::Completed
        } else if has_failed {
            PublicationStatus::Failed
        } else {
            PublicationStatus::Draft
        }
    }

    /// Reset account status for retry (set Draft and clear message)
    /// 重置账号状态用于重发
    pub fn reset_account_for_retry(&self, detail_id: &str) -> Result<(), rusqlite::Error> {
        let conn = self.get_connection()?;

        conn.execute(r#"
            UPDATE publication_accounts
            SET status = 'draft', message = '', item_id = '', published_at = '', publish_url = ''
            WHERE id = ?
        "#, [detail_id])?;

        Ok(())
    }

    /// Get accounts that need retry (Draft or Failed status)
    /// 获取需要重发的账号列表
    pub fn get_accounts_for_retry(&self, task_id: &str) -> Result<Vec<PublicationAccountDetail>, rusqlite::Error> {
        let conn = self.get_connection()?;

        let mut stmt = conn.prepare("
            SELECT id, publication_task_id, account_id, account_name, platform, status,
                   created_at, published_at, publish_url, comments, likes, favorites, shares,
                   message, item_id
            FROM publication_accounts
            WHERE publication_task_id = ? AND status IN ('draft', 'failed')
        ")?;

        let accounts: Vec<PublicationAccountDetail> = stmt.query_map([task_id], |row| {
            let message: String = row.get(13)?;
            let item_id: String = row.get(14)?;
            Ok(PublicationAccountDetail {
                id: row.get(0)?,
                publication_task_id: row.get(1)?,
                account_id: row.get(2)?,
                account_name: row.get(3)?,
                platform: Self::parse_platform(row.get::<_, String>(4)?),
                status: Self::parse_publication_status(row.get::<_, String>(5)?),
                created_at: row.get(6)?,
                published_at: Some(row.get(7)?),
                publish_url: Some(row.get(8)?),
                stats: PublicationStats {
                    comments: row.get(9)?,
                    likes: row.get(10)?,
                    favorites: row.get(11)?,
                    shares: row.get(12)?,
                },
                message: if message.is_empty() { None } else { Some(message) },
                item_id: if item_id.is_empty() { None } else { Some(item_id) },
            })
        })?.filter_map(|r| r.ok()).collect();

        Ok(accounts)
    }

    /// Update main task status
    /// 直接更新主表状态
    pub fn update_publication_task_status(&self, task_id: &str, status: PublicationStatus) -> Result<(), rusqlite::Error> {
        let conn = self.get_connection()?;

        conn.execute(
            "UPDATE publication_tasks SET status = ? WHERE id = ?",
            [&format!("{:?}", status), task_id],
        )?;

        Ok(())
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
