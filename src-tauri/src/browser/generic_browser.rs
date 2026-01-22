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

        eprintln!("[GenericBrowser] start_authorize: 开始执行");

        // 在阻塞线程中运行 Playwright 脚本
        let db_manager = self.db_manager.clone();
        let platform_id = platform_id.to_string();

        // 使用超时
        let timeout_result = tokio::time::timeout(
            std::time::Duration::from_secs(60),
            tokio::task::spawn_blocking(move || {
                Self::run_script(db_manager, &platform_id)
            })
        ).await;

        // 处理超时
        let join_handle = match timeout_result {
            Ok(handle) => handle,
            Err(_) => {
                let err_msg = "脚本执行超时 (60秒)".to_string();
                eprintln!("[GenericBrowser] {}", err_msg);
                self.result.step = BrowserAuthStep::Failed(err_msg.clone());
                self.result.message = err_msg.clone();
                self.result.error = Some(err_msg.clone());
                return Err(err_msg);
            }
        };

        // 处理 join 结果
        match join_handle {
            Ok(script_result) => {
                match script_result {
                    Ok(auth_result) => {
                        eprintln!("[GenericBrowser] 脚本执行成功");
                        self.result = auth_result;
                        Ok(self.result.clone())
                    }
                    Err(e) => {
                        eprintln!("[GenericBrowser] 脚本执行错误: {}", e);
                        self.result.step = BrowserAuthStep::Failed(e.clone());
                        self.result.message = e.clone();
                        self.result.error = Some(e.clone());
                        Err(e)
                    }
                }
            }
            Err(e) => {
                let err_msg = format!("任务执行失败: {}", e);
                eprintln!("[GenericBrowser] {}", err_msg);
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
                    "login_success_mode": config.login_success_mode,
                    "login_success_pattern": config.login_success_pattern,
                    "login_success_api_rule": config.login_success_api_rule,
                    "login_success_api_operator": config.login_success_api_operator,
                    "login_success_api_value": config.login_success_api_value,
                    "redirect_url": config.redirect_url,
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
        eprintln!("[GenericBrowser] 脚本读取成功, 长度: {} bytes", script.len());

        // 获取 Playwright 目录
        let playwright_dir = Self::get_playwright_dir();
        let browsers_dir = Self::get_browsers_dir();

        eprintln!("[GenericBrowser] Playwright目录: {}", playwright_dir.display());
        eprintln!("[GenericBrowser] 浏览器目录: {}", browsers_dir.display());

        // 检查浏览器目录是否存在
        if !browsers_dir.exists() {
            return Err(format!("浏览器目录不存在: {}", browsers_dir.display()));
        }

        // 写入脚本到 Playwright 目录
        let script_path = playwright_dir.join("generic_extractor.js");
        if let Err(e) = std::fs::write(&script_path, &script) {
            return Err(format!("无法写入临时脚本: {}", e));
        }

        // 执行脚本，通过环境变量传递配置
        eprintln!("[GenericBrowser] 启动 Node.js 脚本...");

        let mut child = std::process::Command::new("node")
            .arg(&script_path)
            .arg(platform_id)
            .env("PLAYWRIGHT_BROWSERS_PATH", browsers_dir.to_string_lossy().as_ref())
            .env("AMM_CONFIG", &config_json)
            .current_dir(&playwright_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())  // stderr 直接输出，实时打印
            .spawn()
            .map_err(|e| format!("无法启动脚本: {}", e))?;

        eprintln!("[GenericBrowser] Node.js 进程已启动, pid: {}", child.id());

        let stdout = child.stdout.take().unwrap();

        // 读取 stdout，提取结果
        let reader = std::io::BufReader::new(stdout);
        let mut result_lines = Vec::new();
        let mut in_result = false;

        eprintln!("[GenericBrowser] 开始读取输出...");

        // 只读取 stdout
        let mut stdout_lines = reader.lines();
        loop {
            match stdout_lines.next() {
                Some(Ok(line)) => {
                    if line == "RESULT_JSON_START" {
                        in_result = true;
                        result_lines.clear();
                    } else if line == "RESULT_JSON_END" {
                        eprintln!("[GenericBrowser] 收到 RESULT_JSON_END");
                        break;
                    } else if in_result {
                        result_lines.push(line);
                    }
                }
                Some(Err(e)) => {
                    eprintln!("[GenericBrowser] 读取 stdout 错误: {}", e);
                    break;
                }
                None => {
                    eprintln!("[GenericBrowser] stdout 结束");
                    break;
                }
            }
        }

        eprintln!("[GenericBrowser] 等待进程结束...");
        // 等待进程结束
        let status = child.wait()
            .map_err(|e| format!("等待脚本结束失败: {}", e))?;

        eprintln!("[GenericBrowser] 进程结束, status: {}", status);

        if !status.success() {
            return Err(format!("脚本执行失败, 退出码: {:?}", status.code()));
        }

        // 解析结果
        let result_json: String = result_lines.join("\n");
        eprintln!("[GenericBrowser] 结果 JSON 长度: {} bytes", result_json.len());

        if result_json.is_empty() {
            return Err("未获取到结果".to_string());
        }

        Self::parse_result(&result_json)
    }

    /// 解析认证结果 - 直接解析 JS 返回的新格式（保持配置结构，只替换规则）
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

                result.current_url = json.get("url")
                    .and_then(|u| u.as_str())
                    .unwrap_or("")
                    .to_string();

                // 从 user_info 对象中提取4个字段
                if let Some(user_info) = json.get("user_info").and_then(|u| u.as_object()) {
                    result.nickname = user_info.get("nickname")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    result.avatar_url = user_info.get("avatar_url")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    result.third_id = user_info.get("third_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    result.sec_uid = user_info.get("sec_uid")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                }

                // cookie - 直接从顶层获取
                result.cookie = json.get("cookie")
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();

                // local_storage - 转为字符串存储
                result.local_storage = json.get("local_storage")
                    .map(|l: &serde_json::Value| l.to_string())
                    .unwrap_or_else(|| "[]".to_string());

                // request_headers - 转为字符串存储
                result.request_headers = json.get("request_headers")
                    .map(|r| r.to_string())
                    .unwrap_or_else(|| "{}".to_string());

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
