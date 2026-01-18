// Generic Browser Implementation - 通用规则引擎浏览器
// 使用配置规则从数据库中提取任意平台的用户信息

use crate::browser::{BrowserAuthResult, BrowserAuthStep};
use crate::storage::DatabaseManager;
use std::io::BufRead;
use std::path::PathBuf;
use std::sync::Arc;

/// 通用浏览器实现（使用规则引擎）
pub struct GenericBrowser {
    result: BrowserAuthResult,
    db_manager: Option<Arc<DatabaseManager>>,
}

impl GenericBrowser {
    pub fn new() -> Self {
        Self {
            result: BrowserAuthResult::default(),
            db_manager: None,
        }
    }

    /// 设置数据库管理器
    pub fn set_db_manager(&mut self, db_manager: Arc<DatabaseManager>) {
        self.db_manager = Some(db_manager);
    }

    /// 获取 Playwright 目录
    fn get_playwright_dir() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        std::path::PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("com.yzg.matrix")
            .join("playwright")
    }

    /// 获取浏览器安装目录
    fn get_browsers_dir() -> PathBuf {
        Self::get_playwright_dir().join("browsers")
    }

    /// 启动通用授权流程
    pub async fn start_authorize(&mut self, platform_id: &str) -> Result<BrowserAuthResult, String> {
        self.result.step = BrowserAuthStep::LaunchingBrowser;
        self.result.message = format!("正在启动浏览器 for {}...", platform_id);

        // 在阻塞线程中运行 Playwright 脚本
        let db_manager = self.db_manager.clone();
        let platform_id = platform_id.to_string();
        let result = tokio::task::spawn_blocking(move || {
            Self::run_script(db_manager, &platform_id)
        }).await;

        match result {
            Ok(Ok(auth_result)) => {
                self.result = auth_result;
                Ok(self.result.clone())
            }
            Ok(Err(e)) => {
                self.result.step = BrowserAuthStep::Failed(e.clone());
                self.result.message = e.clone();
                self.result.error = Some(e.clone());
                Err(e)
            }
            Err(e) => {
                let err_msg = format!("Playwright 任务失败: {}", e);
                self.result.step = BrowserAuthStep::Failed(err_msg.clone());
                self.result.message = err_msg.clone();
                self.result.error = Some(err_msg.clone());
                Err(err_msg)
            }
        }
    }

    /// 读取通用脚本文件
    fn read_script_file() -> Result<String, String> {
        let source_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent()
            .unwrap_or(&std::path::PathBuf::from("."))
            .to_path_buf();
        let script_path = source_dir.join("scripts").join("generic_extractor.cjs");

        if script_path.exists() {
            std::fs::read_to_string(&script_path)
                .map_err(|e| format!("读取脚本文件失败: {}", e))
        } else {
            Err(format!("脚本文件不存在: {}", script_path.display()))
        }
    }

    /// 从数据库加载配置
    fn load_config(db_manager: &Arc<DatabaseManager>, platform_id: &str) -> Result<String, String> {
        eprintln!("[GenericBrowser] 正在查询平台配置: {}", platform_id);

        match db_manager.get_extractor_config(platform_id) {
            Ok(Some(config)) => {
                eprintln!("[GenericBrowser] 找到配置: platform_name={}, login_url={}",
                    config.platform_name, config.login_url);

                let config_obj = serde_json::json!({
                    "platform_id": config.platform_id,
                    "platform_name": config.platform_name,
                    "login_url": config.login_url,
                    "login_success_pattern": config.login_success_pattern,
                    "extract_rules": config.extract_rules
                });
                Ok(config_obj.to_string())
            }
            Ok(None) => {
                eprintln!("[GenericBrowser] 未找到平台配置: {}", platform_id);
                Err(format!("未找到平台配置: {}", platform_id))
            }
            Err(e) => {
                eprintln!("[GenericBrowser] 读取配置失败: {}", e);
                Err(format!("读取配置失败: {}", e))
            }
        }
    }

    /// 在阻塞线程中运行 Playwright 脚本
    fn run_script(db_manager: Option<Arc<DatabaseManager>>, platform_id: &str) -> Result<BrowserAuthResult, String> {
        eprintln!("[GenericBrowser] run_script called for platform: {}", platform_id);

        // 从数据库加载配置
        let config_json = if let Some(ref db) = db_manager {
            Self::load_config(db, platform_id)?
        } else {
            return Err("数据库管理器未设置".to_string());
        };

        eprintln!("[GenericBrowser] 配置 JSON 长度: {} bytes", config_json.len());

        // 从文件读取 Node.js 脚本
        let script = Self::read_script_file()?;

        // 获取 Playwright 目录
        let playwright_dir = Self::get_playwright_dir();
        let browsers_dir = Self::get_browsers_dir();

        // 写入脚本到 Playwright 目录
        let script_path = playwright_dir.join("generic_extractor.js");
        if let Err(e) = std::fs::write(&script_path, &script) {
            return Err(format!("无法写入临时脚本: {}", e));
        }

        // 执行脚本，通过环境变量传递配置
        eprintln!("[GenericBrowser] 启动 Node.js 脚本: {} {}", script_path.display(), platform_id);
        eprintln!("[GenericBrowser] PLAYWRIGHT_BROWSERS_PATH: {}", browsers_dir.to_string_lossy().as_ref());
        eprintln!("[GenericBrowser] AMM_CONFIG 长度: {}", config_json.len());

        let mut child = std::process::Command::new("node")
            .arg(&script_path)
            .arg(platform_id)
            .env("PLAYWRIGHT_BROWSERS_PATH", browsers_dir.to_string_lossy().as_ref())
            .env("AMM_CONFIG", &config_json)
            .current_dir(&playwright_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("无法启动脚本: {}", e))?;

        eprintln!("[GenericBrowser] Node.js 进程已启动, pid: {}", child.id());

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        // 使用 mpsc 通道来实时传输 stderr 日志
        let (tx, rx) = std::sync::mpsc::channel();

        // 读取 stderr 的线程
        let stderr_handle = std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stderr);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        if tx.send(line).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // 读取 stdout，提取结果，同时打印 stderr
        let reader = std::io::BufReader::new(stdout);
        let mut result_lines = Vec::new();
        let mut in_result = false;

        // 使用 select 模式同时读取 stdout 和 stderr
        let mut stdout_lines = reader.lines();
        loop {
            // 优先检查 stderr 是否有新行
            match rx.try_recv() {
                Ok(line) => {
                    println!("[Node.js] {}", line);
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    // 没有新的 stderr，尝试读取 stdout
                    match stdout_lines.next() {
                        Some(Ok(line)) => {
                            if line == "RESULT_JSON_START" {
                                in_result = true;
                                result_lines.clear();
                            } else if line == "RESULT_JSON_END" {
                                break;
                            } else if in_result {
                                result_lines.push(line);
                            }
                        }
                        Some(Err(_)) | None => {
                            // stdout 结束了，等待 stderr 线程完成
                            break;
                        }
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    // stderr 线程结束了，继续读取 stdout
                    break;
                }
            }
        }

        // 打印剩余的 stderr
        while let Ok(line) = rx.try_recv() {
            println!("[Node.js] {}", line);
        }

        // 等待进程结束
        let status = child.wait()
            .map_err(|e| format!("等待脚本结束失败: {}", e))?;

        stderr_handle.join().ok();

        if !status.success() {
            return Err("脚本执行失败".to_string());
        }

        // 解析结果
        let result_json: String = result_lines.join("\n");
        if result_json.is_empty() {
            return Err("未获取到结果".to_string());
        }

        Self::parse_result(&result_json)
    }

    /// 解析认证结果
    fn parse_result(content: &str) -> Result<BrowserAuthResult, String> {
        match serde_json::from_str::<serde_json::Value>(content) {
            Ok(json) => {
                let mut result = BrowserAuthResult::default();

                if let Some(step) = json.get("step").and_then(|s| s.as_str()) {
                    result.step = match step {
                        "completed" => BrowserAuthStep::Completed,
                        "failed" => BrowserAuthStep::Failed(
                            json.get("message").and_then(|m| m.as_str()).unwrap_or("未知错误").to_string()
                        ),
                        _ => BrowserAuthStep::Failed("未知状态".to_string()),
                    };
                }

                result.message = json.get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("完成")
                    .to_string();

                result.nickname = json.get("nickname")
                    .and_then(|n| n.as_str())
                    .unwrap_or("用户")
                    .to_string();

                result.avatar_url = json.get("avatar_url")
                    .and_then(|a| a.as_str())
                    .unwrap_or("")
                    .to_string();

                result.current_url = json.get("url")
                    .and_then(|u| u.as_str())
                    .unwrap_or("")
                    .to_string();

                result.request_headers = json.get("third_param")
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| "{}".to_string());

                result.uid = json.get("third_id")
                    .and_then(|id| id.as_str())
                    .unwrap_or("")
                    .to_string();

                if let Some(cookie) = json.get("third_param")
                    .and_then(|p| p.as_object())
                    .and_then(|p| p.get("cookie"))
                    .and_then(|c| c.as_str())
                {
                    result.cookie = cookie.to_string();
                }

                if let Some(local_data) = json.get("third_param")
                    .and_then(|p| p.as_object())
                    .and_then(|p| p.get("local_data"))
                {
                    result.local_storage = local_data.to_string();
                }

                result.need_poll = false;

                if result.step == BrowserAuthStep::Completed {
                    Ok(result)
                } else {
                    Err(result.message.clone())
                }
            }
            Err(e) => Err(format!("解析结果失败: {}", e))
        }
    }

    /// 取消授权
    pub async fn cancel(&mut self) {
        self.result.step = BrowserAuthStep::Idle;
        self.result.message = "已取消授权".to_string();
        self.result.need_poll = false;
    }

    /// 获取授权结果
    pub fn get_result(&self) -> &BrowserAuthResult {
        &self.result
    }
}

impl Default for GenericBrowser {
    fn default() -> Self {
        Self::new()
    }
}
