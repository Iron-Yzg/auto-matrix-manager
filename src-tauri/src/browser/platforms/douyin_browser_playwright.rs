// Douyin Browser Implementation - 使用 Playwright 网络事件监听
// Playwright 可以在不注入 JS 的情况下捕获所有网络请求/响应

use crate::browser::{BrowserAuthResult, BrowserAuthStep};
use std::io::BufRead;
use std::path::PathBuf;

/// 抖音浏览器实现（Playwright 版本）
pub struct DouyinBrowserPlaywright {
    result: BrowserAuthResult,
}

impl DouyinBrowserPlaywright {
    pub fn new() -> Self {
        Self {
            result: BrowserAuthResult::default(),
        }
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

    /// 启动抖音授权流程（使用 Playwright）
    pub async fn start_authorize(&mut self) -> Result<BrowserAuthResult, String> {
        self.result.step = BrowserAuthStep::LaunchingBrowser;
        self.result.message = "正在启动浏览器...".to_string();

        // 在阻塞线程中运行 Playwright 脚本
        let result = tokio::task::spawn_blocking(move || {
            Self::run_script()
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

    /// 读取 Node.js 脚本文件
    fn read_script_file() -> Result<String, String> {
        let source_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent()
            .unwrap_or(&std::path::PathBuf::from("."))
            .to_path_buf();
        let script_path = source_dir.join("scripts").join("douyin_auth.cjs");

        if script_path.exists() {
            std::fs::read_to_string(&script_path)
                .map_err(|e| format!("读取脚本文件失败: {}", e))
        } else {
            Err(format!("脚本文件不存在: {}", script_path.display()))
        }
    }

    /// 在阻塞线程中运行 Playwright 脚本
    fn run_script() -> Result<BrowserAuthResult, String> {
        // 从文件读取 Node.js 脚本
        let script = Self::read_script_file()?;

        // 获取 Playwright 目录（使用应用数据目录）
        let playwright_dir = Self::get_playwright_dir();
        let browsers_dir = Self::get_browsers_dir();

        // 写入脚本到 Playwright 目录
        let script_path = playwright_dir.join("douyin_auth_script.js");
        if let Err(e) = std::fs::write(&script_path, &script) {
            return Err(format!("无法写入临时脚本: {}", e));
        }

        // 执行脚本 - 从 stdout 读取结果
        let mut child = std::process::Command::new("node")
            .arg(&script_path)
            .env("PLAYWRIGHT_BROWSERS_PATH", browsers_dir.to_string_lossy().as_ref())
            .current_dir(&playwright_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("无法启动脚本: {}", e))?;

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        // 收集 stdout 中的结果（用 RESULT_JSON_START/END 标记）
        let mut result_lines = Vec::new();
        let mut in_result = false;

        // 读取 stderr（打印到控制台）
        let stderr_handle = std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(line) = line {
                    eprintln!("[DouyinBrowser-Playwright] {}", line);
                }
            }
        });

        // 读取 stdout，提取结果
        let reader = std::io::BufReader::new(stdout);
        let mut in_result = false;
        for line in reader.lines() {
            let line = line.map_err(|e| format!("读取输出失败: {}", e))?;
            if line == "RESULT_JSON_START" {
                in_result = true;
                result_lines.clear();
            } else if line == "RESULT_JSON_END" {
                break;
            } else if in_result {
                result_lines.push(line);
            }
        }

        // 等待进程结束
        let status = child.wait()
            .map_err(|e| format!("等待脚本结束失败: {}", e))?;

        // 等待 stderr 线程完成
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

    /// 解析认证结果（简化版本，直接解析 Node.js 返回的格式）
    fn parse_result(content: &str) -> Result<BrowserAuthResult, String> {
        match serde_json::from_str::<serde_json::Value>(content) {
            Ok(json) => {
                let mut result = BrowserAuthResult::default();

                // 解析状态
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

                // 简化解析：Node.js 已经组装好了 third_param
                result.nickname = json.get("nickname")
                    .and_then(|n| n.as_str())
                    .unwrap_or("抖音用户")
                    .to_string();

                result.avatar_url = json.get("avatar_url")
                    .and_then(|a| a.as_str())
                    .unwrap_or("")
                    .to_string();

                result.current_url = json.get("url")
                    .and_then(|u| u.as_str())
                    .unwrap_or("")
                    .to_string();

                // 直接序列化 third_param（Node.js 已经组装好）
                result.request_headers = json.get("third_param")
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| "{}".to_string());

                // uid 即 third_id
                result.uid = json.get("third_id")
                    .and_then(|id| id.as_str())
                    .unwrap_or("")
                    .to_string();

                // cookie 从 third_param 中提取
                if let Some(cookie) = json.get("third_param")
                    .and_then(|p| p.as_object())
                    .and_then(|p| p.get("cookie"))
                    .and_then(|c| c.as_str())
                {
                    result.cookie = cookie.to_string();
                }

                // local_storage 从 third_param.local_data 转换
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

    /// 检查登录状态（轮询模式）
    pub async fn check_and_extract(&mut self) -> Result<bool, String> {
        Ok(false)
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

impl Default for DouyinBrowserPlaywright {
    fn default() -> Self {
        Self::new()
    }
}

/// 检查 Playwright 环境（返回是否需要安装）
pub fn check_playwright_env() -> (bool, bool) {
    let playwright_dir = get_playwright_dir();
    let browsers_dir = playwright_dir.join("browsers");

    // 检查 Playwright npm 包
    let playwright_installed = playwright_dir.join("node_modules").join("playwright").exists();

    // 检查 Chromium 浏览器（实际安装的是 chromium-1200 这样的目录）
    let chromium_installed = if browsers_dir.exists() {
        let entries = browsers_dir.read_dir()
            .map(|e| e.filter_map(|e| e.ok()).map(|e| e.path()).collect::<Vec<_>>());
        match entries {
            Ok(entries) => entries.iter()
                .any(|p| p.is_dir() && p.file_name()
                    .map(|n| n.to_string_lossy().starts_with("chromium-"))
                    .unwrap_or(false)),
            Err(_) => false,
        }
    } else {
        false
    };

    (playwright_installed, chromium_installed)
}

/// 安装 Playwright 环境
pub fn ensure_playwright_env() -> Result<(), String> {
    let playwright_dir = get_playwright_dir();
    let browsers_dir = playwright_dir.join("browsers");

    // 检查 Playwright npm 包
    let playwright_installed = playwright_dir.join("node_modules").join("playwright").exists();

    if !playwright_installed {
        eprintln!("[Env] 安装 Playwright npm 包...");

        // 创建目录
        let node_modules = playwright_dir.join("node_modules");
        if let Err(e) = std::fs::create_dir_all(&node_modules) {
            return Err(format!("创建目录失败: {}", e));
        }

        // 写入 package.json
        let package_json = r#"{
  "name": "auto-matrix-manager-playwright",
  "version": "1.0.0",
  "description": "Playwright for Auto Matrix Manager",
  "main": "index.js",
  "type": "commonjs",
  "dependencies": {
    "playwright": "^1.50.0"
  }
}"#;
        let package_path = playwright_dir.join("package.json");
        if let Err(e) = std::fs::write(&package_path, package_json) {
            return Err(format!("写入 package.json 失败: {}", e));
        }

        // 执行 npm install
        let output = std::process::Command::new("npm")
            .arg("install")
            .current_dir(&playwright_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .map_err(|e| format!("执行 npm install 失败: {}", e))?;

        if !output.status.success() {
            return Err("安装 Playwright 失败".to_string());
        }
        eprintln!("[Env] Playwright npm 包安装完成");
    }

    // 检查 Chromium 浏览器（实际安装的是 chromium-1200 这样的目录）
    let chromium_installed = if browsers_dir.exists() {
        let entries = browsers_dir.read_dir()
            .map(|e| e.filter_map(|e| e.ok()).map(|e| e.path()).collect::<Vec<_>>());
        match entries {
            Ok(entries) => entries.iter()
                .any(|p| p.is_dir() && p.file_name()
                    .map(|n| n.to_string_lossy().starts_with("chromium-"))
                    .unwrap_or(false)),
            Err(_) => false,
        }
    } else {
        false
    };

    if !chromium_installed {
        eprintln!("[Env] 安装 Chromium 浏览器...");

        let output = std::process::Command::new("npx")
            .arg("playwright")
            .arg("install")
            .arg("chromium")
            .env("PLAYWRIGHT_BROWSERS_PATH", browsers_dir.to_string_lossy().as_ref())
            .current_dir(&playwright_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .map_err(|e| format!("安装 Chromium 失败: {}", e))?;

        if !output.status.success() {
            return Err("安装 Chromium 失败".to_string());
        }
        eprintln!("[Env] Chromium 安装完成");
    }

    Ok(())
}

fn get_playwright_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("com.yzg.matrix")
        .join("playwright")
}
