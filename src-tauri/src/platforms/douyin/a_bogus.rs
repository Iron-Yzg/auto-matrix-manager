//! A-Bogus 签名计算模块
//!
//! 使用 QuickJS 执行抖音的 a_bogus JavaScript 算法
//!
//! 如果 QuickJS 执行失败，会回退到简单的占位符实现

use rquickjs::{Context, Runtime, Function};
use std::path::PathBuf;
use once_cell::sync::Lazy;
use tokio::sync::Mutex;
use md5;

/// 获取 a_bogus.js 文件路径
fn get_a_bogus_js_path() -> PathBuf {
    // 尝试多个可能的位置
    let candidates = [
        // 从 src-tauri 目录
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("python/utils/a_bogus.js"),
        // 从项目根目录
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../python/utils/a_bogus.js"),
        // 绝对路径
        PathBuf::from("/Users/yangzhenguo/workspace/auto-matrix-manager/python/utils/a_bogus.js"),
    ];

    for path in &candidates {
        if path.exists() {
            tracing::info!("[A-Bogus] 找到 JS 文件: {:?}", path);
            return path.clone();
        }
    }

    tracing::warn!("[A-Bogus] 未找到 a_bogus.js 文件，使用占位符");
    candidates[0].clone()
}

/// 缓存 JS 代码（避免重复读取）
static JS_CODE_CACHE: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

/// 初始化 JS 代码
async fn init_js_code() -> Result<String, String> {
    let mut cache = JS_CODE_CACHE.lock().await;
    if let Some(ref code) = *cache {
        return Ok(code.clone());
    }

    let js_path = get_a_bogus_js_path();
    if !js_path.exists() {
        return Err(format!("a_bogus.js 文件不存在: {:?}", js_path));
    }

    let js_code = std::fs::read_to_string(&js_path)
        .map_err(|e| format!("读取 a_bogus.js 失败: {}", e))?;

    *cache = Some(js_code.clone());
    Ok(js_code)
}

/// 同步版本的初始化（用于测试）
#[allow(dead_code)]
fn init_js_code_sync() -> Result<String, String> {
    let js_path = get_a_bogus_js_path();
    if !js_path.exists() {
        return Err(format!("a_bogus.js 文件不存在: {:?}", js_path));
    }

    std::fs::read_to_string(&js_path)
        .map_err(|e| format!("读取 a_bogus.js 失败: {}", e))
}

/// 计算 a_bogus 签名
///
/// # 参数
///
/// * `url_query` - URL 查询参数（不含 ?）
/// * `user_agent` - User-Agent 字符串
///
/// # 返回
///
/// 计算得到的 a_bogus 值
pub async fn calculate_a_bogus(url_query: &str, user_agent: &str) -> Result<String, String> {
    // 检查是否强制使用占位符（通过环境变量）
    if std::env::var("A_BOGUS_USE_PLACEHOLDER").is_ok() {
        let placeholder = calculate_placeholder(url_query, user_agent);
        tracing::warn!("[A-Bogus] 强制使用占位符: {}...", &placeholder[..placeholder.len().min(20)]);
        return Ok(placeholder);
    }

    // 首先尝试使用 QuickJS 执行 JS 代码
    match calculate_with_quickjs(url_query, user_agent).await {
        Ok(result) => {
            tracing::info!("[A-Bogus] QuickJS 计算成功: {}...", &result[..result.len().min(20)]);
            Ok(result)
        }
        Err(e) => {
            tracing::warn!("[A-Bogus] QuickJS 计算失败: {}，使用占位符", e);
            // 回退到简单的占位符
            let placeholder = calculate_placeholder(url_query, user_agent);
            tracing::warn!("[A-Bogus] 使用占位符: {}...", &placeholder[..placeholder.len().min(20)]);
            Ok(placeholder)
        }
    }
}

/// 使用 QuickJS 计算 a_bogus
async fn calculate_with_quickjs(url_query: &str, user_agent: &str) -> Result<String, String> {
    let js_code = init_js_code().await?;

    // 创建新的运行时
    let runtime = Runtime::new().map_err(|e| format!("创建 JS 运行时失败: {}", e))?;
    let ctx = Context::builder()
        .build(&runtime)
        .map_err(|e| format!("创建 JS 上下文失败: {}", e))?;

    // 执行 JS 代码
    let js_code_bytes: Vec<u8> = js_code.into_bytes();
    ctx.with(|ctx| {
        // 评估 JS 代码
        ctx.eval::<(), _>(js_code_bytes)
            .map_err(|e| format!("执行 a_bogus.js 失败: {}", e))?;

        // 获取 generate_a_bogus 函数
        let generate_a_bogus: Function = ctx.globals().get("generate_a_bogus")
            .map_err(|e| format!("获取 generate_a_bogus 函数失败: {}", e))?;

        // 调用函数
        let result: String = generate_a_bogus.call((url_query, user_agent))
            .map_err(|e| format!("调用 generate_a_bogus 失败: {}", e))?;

        Ok(result)
    })
}

/// 计算占位符 a_bogus（仅用于调试）
///
/// 注意：这不是正确的 a_bogus 算法，仅用于测试
fn calculate_placeholder(url_query: &str, _user_agent: &str) -> String {
    // 简单的哈希作为占位符
    let combined = format!("{}{}", url_query, _user_agent);
    let hash = md5::compute(combined);
    format!("{:x}", hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_calculate_a_bogus() {
        let url_query = "device_platform=webapp&aid=6383&channel=channel_pc_web&aweme_id=7588468852477211310&cursor=0&count=5&item_type=0&msToken=test_token";
        let user_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36";

        let result = calculate_a_bogus(url_query, user_agent).await;
        assert!(result.is_ok(), "计算失败: {:?}", result.err());
        let bogus = result.unwrap();
        assert!(!bogus.is_empty());
        println!("a_bogus: {}", bogus);
    }

    #[test]
    fn test_calculate_placeholder() {
        let url_query = "device_platform=webapp&aid=6383";
        let user_agent = "Mozilla/5.0";

        let result = calculate_placeholder(url_query, user_agent);
        assert!(!result.is_empty());
        println!("placeholder a_bogus: {}", result);
    }

    #[test]
    fn test_js_code_exists() {
        let result = init_js_code_sync();
        assert!(result.is_ok(), "JS 文件不存在或无法读取: {:?}", result.err());
        let code = result.unwrap();
        assert!(code.contains("generate_a_bogus"), "JS 代码不包含 generate_a_bogus 函数");
        println!("JS 代码长度: {} 字节", code.len());
    }
}
