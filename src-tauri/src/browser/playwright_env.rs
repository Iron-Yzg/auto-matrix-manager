// Playwright Environment Check
// 检查 Playwright 环境是否正确安装

use std::path::PathBuf;
use std::process::{Command, Stdio};

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
    get_playwright_dir().join("browsers")
}

/// 检查 Playwright 是否已安装
pub fn check_playwright_env() -> Result<(), String> {
    let browsers_dir = get_browsers_dir();

    // 检查目录是否存在
    if !browsers_dir.exists() {
        return Err("Playwright browsers directory not found".to_string());
    }

    // 检查是否有浏览器安装
    let entries = std::fs::read_dir(&browsers_dir)
        .map_err(|e| format!("Failed to read browsers directory: {}", e))?;

    let browser_count = entries.filter_map(|e| e.ok())
        .filter(|e| {
            e.path().is_dir()
        })
        .count();

    if browser_count == 0 {
        return Err("No browsers installed for Playwright".to_string());
    }

    Ok(())
}

/// 确保 Playwright 环境正确
pub fn ensure_playwright_env() -> Result<(), String> {
    // 首先检查
    if let Err(e) = check_playwright_env() {
        return Err(format!("Playwright environment check failed: {}", e));
    }

    // 确保目录存在
    let playwright_dir = get_playwright_dir();
    if let Err(e) = std::fs::create_dir_all(&playwright_dir) {
        return Err(format!("Failed to create Playwright directory: {}", e));
    }

    Ok(())
}

/// 检查 Node.js 是否可用
pub fn check_node_available() -> Result<(), String> {
    let output = Command::new("node")
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("Failed to run node: {}", e))?;

    if !output.status.success() {
        return Err("Node.js not available".to_string());
    }

    Ok(())
}

/// 获取 Playwright 信息
pub fn get_playwright_info() -> Result<serde_json::Value, String> {
    let playwright_dir = get_playwright_dir();
    let browsers_dir = get_browsers_dir();

    let mut info = serde_json::json!({
        "playwright_dir": playwright_dir.to_string_lossy().as_ref(),
        "browsers_dir": browsers_dir.to_string_lossy().as_ref(),
        "browsers": []
    });

    // 列出已安装的浏览器
    if browsers_dir.exists() {
        let entries = std::fs::read_dir(&browsers_dir)
            .map_err(|e| format!("Failed to read browsers directory: {}", e))?;

        let browsers: Vec<String> = entries.filter_map(|e| e.ok())
            .filter(|e| {
                e.path().is_dir()
            })
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();

        info["browsers"] = serde_json::json!(browsers);
    }

    Ok(info)
}
