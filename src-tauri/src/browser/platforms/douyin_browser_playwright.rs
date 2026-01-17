// Douyin Browser Implementation - 使用 Playwright 网络事件监听
// Playwright 可以在不注入 JS 的情况下捕获所有网络请求/响应
// 通过 CDP (Chrome DevTools Protocol) 实现

use crate::browser::{BrowserAuthResult, BrowserAuthStep};
use std::io::BufRead;
use std::path::PathBuf;

/// 抖音浏览器实现（Playwright 版本）
pub struct DouyinBrowserPlaywright {
    result: BrowserAuthResult,
    timeout_seconds: u32,
}

impl DouyinBrowserPlaywright {
    pub fn new() -> Self {
        Self {
            result: BrowserAuthResult::default(),
            timeout_seconds: 120,
        }
    }

    pub fn with_timeout(mut self, seconds: u32) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    /// 获取 Playwright 安装目录
    /// 使用 Tauri 应用数据目录 ~/Library/Application Support/com.yzg.matrix/playwright
    fn get_playwright_dir(&self) -> PathBuf {
        Self::get_playwright_dir_static()
    }

    /// 获取浏览器安装目录
    fn get_browsers_dir(&self) -> PathBuf {
        self.get_playwright_dir().join("browsers")
    }

    /// 确保 Playwright 已安装
    fn ensure_playwright_installed(&self) -> Result<(), String> {
        let playwright_dir = self.get_playwright_dir();
        let node_modules = playwright_dir.join("node_modules").join("playwright");

        // 如果已安装，直接返回
        if node_modules.exists() {
            eprintln!("[DouyinBrowser-Playwright] Playwright already installed at: {}", node_modules.display());
            return Ok(());
        }

        eprintln!("[DouyinBrowser-Playwright] Installing Playwright to: {}", playwright_dir.display());

        // 创建目录
        if let Err(e) = std::fs::create_dir_all(&node_modules) {
            return Err(format!("创建 Playwright 目录失败: {}", e));
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
        eprintln!("[DouyinBrowser-Playwright] Running npm install...");
        let install_output = std::process::Command::new("npm")
            .arg("install")
            .arg("--prefer-offline")
            .current_dir(&playwright_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output();

        match install_output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if !output.status.success() {
                    eprintln!("[DouyinBrowser-Playwright] npm install stdout:\n{}", stdout);
                    eprintln!("[DouyinBrowser-Playwright] npm install stderr:\n{}", stderr);
                    return Err("安装 Playwright 失败".to_string());
                }

                eprintln!("[DouyinBrowser-Playwright] Playwright installed successfully");
                Ok(())
            }
            Err(e) => Err(format!("执行 npm install 失败: {}", e))
        }
    }

    /// 确保浏览器已安装
    fn ensure_browser_installed(&self) -> Result<(), String> {
        let browsers_dir = self.get_browsers_dir();

        // 检查 chromium 是否存在
        let chromium_dir = browsers_dir.join("chromium-");
        if chromium_dir.exists() && chromium_dir.read_dir().map(|_e| _e.count()).unwrap_or(0) > 0 {
            eprintln!("[DouyinBrowser-Playwright] Chromium already installed");
            return Ok(());
        }

        eprintln!("[DouyinBrowser-Playwright] Installing Chromium browser...");

        // 使用应用目录下的 playwright 安装浏览器
        let playwright_dir = self.get_playwright_dir();
        let install_output = std::process::Command::new("npx")
            .arg("playwright")
            .arg("install")
            .arg("chromium")
            .env("PLAYWRIGHT_BROWSERS_PATH", &browsers_dir)
            .current_dir(&playwright_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output();

        match install_output {
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !output.status.success() {
                    eprintln!("[DouyinBrowser-Playwright] Browser install stderr:\n{}", stderr);
                    return Err("安装 Chromium 失败".to_string());
                }
                eprintln!("[DouyinBrowser-Playwright] Chromium installed successfully");
                Ok(())
            }
            Err(e) => Err(format!("安装浏览器失败: {}", e))
        }
    }

    /// 启动抖音授权流程（使用 Playwright）
    pub async fn start_authorize(&mut self) -> Result<BrowserAuthResult, String> {
        self.result.step = BrowserAuthStep::LaunchingBrowser;
        self.result.message = "正在启动浏览器...".to_string();
        eprintln!("[DouyinBrowser-Playwright] ===== Starting authorization with Playwright =====");

        // 确保 Playwright 已安装
        if let Err(e) = self.ensure_playwright_installed() {
            self.result.step = BrowserAuthStep::Failed(e.clone());
            self.result.message = e.clone();
            self.result.error = Some(e.clone());
            return Err(e);
        }

        // 确保浏览器已安装
        if let Err(e) = self.ensure_browser_installed() {
            self.result.step = BrowserAuthStep::Failed(e.clone());
            self.result.message = e.clone();
            self.result.error = Some(e.clone());
            return Err(e);
        }

        eprintln!("[DouyinBrowser-Playwright] Starting Playwright script...");

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

    /// 获取 Playwright 安装目录（静态方法版本，用于阻塞线程）
    /// 使用与 Tauri lib.rs 相同的路径逻辑
    fn get_playwright_dir_static() -> PathBuf {
        // 使用与 lib.rs 相同的 fallback 逻辑
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let data_path = std::path::PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("com.yzg.matrix");
        data_path.join("playwright")
    }

    /// 读取 Node.js 脚本文件
    fn read_script_file() -> Result<String, String> {
        // 尝试从源码目录读取脚本
        let source_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent()
            .unwrap_or(&std::path::PathBuf::from("."))
            .to_path_buf();
        let script_path = source_dir.join("scripts").join("douyin_auth.cjs");

        eprintln!("[DouyinBrowser-Playwright] Looking for script at: {}", script_path.display());

        if script_path.exists() {
            std::fs::read_to_string(&script_path)
                .map_err(|e| format!("读取脚本文件失败: {}", e))
        } else {
            Err(format!("脚本文件不存在: {}", script_path.display()))
        }
    }

    /// 在阻塞线程中运行 Playwright 脚本
    fn run_script() -> Result<BrowserAuthResult, String> {
        eprintln!("[DouyinBrowser-Playwright] Running Playwright script in blocking thread...");

        // 从文件读取 Node.js 脚本
        let script = Self::read_script_file()?;
        eprintln!("[DouyinBrowser-Playwright] Script loaded from file, length: {}", script.len());

        // 获取 Playwright 目录（使用应用数据目录）
        let playwright_dir = Self::get_playwright_dir_static();
        let browsers_dir = playwright_dir.join("browsers");

        let output_path = std::env::temp_dir().join("douyin_auth_result.json");

        // 写入脚本到 Playwright 目录
        let script_path = playwright_dir.join("douyin_auth_script.js");
        if let Err(e) = std::fs::write(&script_path, &script) {
            return Err(format!("无法写入临时脚本: {}", e));
        }

        // 删除旧的结果文件
        let _ = std::fs::remove_file(&output_path);

        eprintln!("[DouyinBrowser-Playwright] Script path: {}", script_path.display());
        eprintln!("[DouyinBrowser-Playwright] Output path: {}", output_path.display());
        eprintln!("[DouyinBrowser-Playwright] Browsers path: {}", browsers_dir.display());

        // 执行脚本 - 使用流式输出以便实时看到日志
        let start = std::time::Instant::now();
        let mut child = std::process::Command::new("node")
            .arg(&script_path)
            .arg(output_path.to_string_lossy().as_ref())
            .env("PLAYWRIGHT_BROWSERS_PATH", browsers_dir.to_string_lossy().as_ref())
            .current_dir(&playwright_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("无法启动脚本: {}", e))?;

        // 实时读取 stdout 和 stderr
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        // 使用线程同时读取 stdout 和 stderr
        let stdout_handle = std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(line) = line {
                    eprintln!("[DouyinBrowser-Playwright] {}", line);
                }
            }
        });

        let stderr_handle = std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(line) = line {
                    eprintln!("[DouyinBrowser-Playwright stderr] {}", line);
                }
            }
        });

        // 等待进程结束
        let status = child.wait()
            .map_err(|e| format!("等待脚本结束失败: {}", e))?;

        // 等待线程完成
        stdout_handle.join().ok();
        stderr_handle.join().ok();

        let elapsed = start.elapsed();
        eprintln!("[DouyinBrowser-Playwright] Script completed in {:.1}s", elapsed.as_secs_f64());

        if status.success() {
            if output_path.exists() {
                match std::fs::read_to_string(&output_path) {
                    Ok(content) => {
                        eprintln!("[DouyinBrowser-Playwright] Result content: {}", content);
                        Self::parse_result(&content)
                    }
                    Err(e) => Err(format!("读取结果文件失败: {}", e))
                }
            } else {
                Err("未找到结果文件".to_string())
            }
        } else {
            Err("脚本执行失败".to_string())
        }
    }

    /// 解析认证结果
    fn parse_result(content: &str) -> Result<BrowserAuthResult, String> {
        eprintln!("[DouyinBrowser-Playwright] Parsing result: {}", content);

        match serde_json::from_str::<serde_json::Value>(content) {
            Ok(json) => {
                let mut result = BrowserAuthResult::default();

                if let Some(step) = json.get("step").and_then(|s| s.as_str()) {
                    result.step = match step {
                        "completed" => BrowserAuthStep::Completed,
                        "failed" => BrowserAuthStep::Failed(
                            json.get("message").and_then(|m| m.as_str()).unwrap_or("未知错误").to_string()
                        ),
                        "waiting" => BrowserAuthStep::WaitingForLogin,
                        "extracting" => BrowserAuthStep::ExtractingCredentials,
                        _ => BrowserAuthStep::ExtractingCredentials,
                    };
                }

                result.message = json.get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("完成")
                    .to_string();

                result.cookie = json.get("cookie")
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();

                result.local_storage = json.get("local_storage")
                    .and_then(|l| l.as_str())
                    .unwrap_or("")
                    .to_string();

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

                result.need_poll = json.get("need_poll")
                    .and_then(|b| b.as_bool())
                    .unwrap_or(false);

                result.uid = json.get("uid")
                    .and_then(|u| u.as_str())
                    .unwrap_or("")
                    .to_string();

                result.sec_uid = json.get("sec_uid")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string();

                result.request_headers = json.get("request_headers")
                    .map(|h| h.to_string())
                    .unwrap_or_else(|| "{}".to_string());

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
        eprintln!("[DouyinBrowser-Playwright] check_and_extract called");

        // Playwright 模式在 start_authorize 中已完成所有工作
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
