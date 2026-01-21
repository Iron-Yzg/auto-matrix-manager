//! 抖音发布工具类
//!
//! 提供字符串处理、时间计算、JSON转换等工具方法
//!
//! # 主要功能
//!
//! - 去除HTML标签
//! - 获取字符串长度（考虑中文字符和Emoji）
//! - 截取字符串（考虑中文字符）
//! - 生成创建ID
//! - 计算延迟发布时间
//! - 对象转JSON字符串

use serde::Serialize;

/// 去除HTML标签
///
/// 使用html-escape去除字符串中的HTML标签
///
/// # 参数
///
/// * `input` - 输入字符串
///
/// # 返回
///
/// 去除HTML标签后的字符串
pub fn strip_html_tags(input: &str) -> String {
    if input.is_empty() {
        return String::new();
    }
    // 使用html-escape库进行HTML标签清理
    html_escape::encode_text(input).to_string()
}

/// 获取字符串长度（考虑中文字符和Emoji）
///
/// 中文字符和Emoji计算为2个长度，普通字符计算为1个长度
///
/// # 参数
///
/// * `input` - 输入字符串
///
/// # 返回
///
/// 计算后的字符串长度
pub fn get_string_length(input: &str) -> usize {
    if input.is_empty() {
        return 0;
    }
    let mut length = 0;
    for c in input.chars() {
        if is_chinese(c) || is_emoji(c) {
            length += 2;
        } else {
            length += 1;
        }
    }
    length
}

/// 截取字符串（考虑中文字符）
///
/// 按字符截取，确保不会截断中文字符或Emoji
///
/// # 参数
///
/// * `input` - 输入字符串
/// * `start` - 起始位置
/// * `max_length` - 最大长度
///
/// # 返回
///
/// 截取后的字符串
pub fn substr(input: &str, start: usize, max_length: usize) -> String {
    if input.is_empty() {
        return String::new();
    }
    if max_length == 0 {
        return String::new();
    }

    let mut current_length = 0;

    for (i, c) in input.char_indices() {
        if i < start {
            continue;
        }
        let char_length = if is_chinese(c) || is_emoji(c) { 2 } else { 1 };

        if current_length + char_length > max_length {
            break;
        }

        current_length += char_length;
    }

    // 再次遍历找到正确的end_index
    let mut count = 0;
    let mut result = String::new();
    for c in input.chars() {
        if count >= start && count < start + max_length {
            let c_len = if is_chinese(c) || is_emoji(c) { 2 } else { 1 };
            if count + c_len > start + max_length {
                break;
            }
            result.push(c);
        }
        count += if is_chinese(c) || is_emoji(c) { 2 } else { 1 };
    }

    result
}

/// 生成创建ID
///
/// 生成格式为：`时间戳 + 4位随机数`
///
/// # 返回
///
/// 生成的创建ID字符串
pub fn generate_creation_id() -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let random: u32 = rand::random();
    format!("{}{:04}", timestamp, random % 10000)
}

/// 计算延迟发布时间
///
/// 如果指定了发送时间，返回发送时间；否则返回当前时间 + timeout
///
/// # 参数
///
/// * `timeout` - 超时时间（秒）
/// * `send_time` - 发送时间（Unix时间戳，秒）
///
/// # 返回
///
/// 计算后的发布时间（Unix时间戳，秒）
pub fn calculate_timing(timeout: i64, send_time: i64) -> i64 {
    if send_time > 0 {
        return send_time;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    now + timeout
}

/// 格式化POI anchor内容
///
/// # 返回
///
/// 格式化后的POI anchor内容JSON字符串
pub fn format_poi_anchor_content() -> String {
    r#"{"is_commerce_intention":true,"recommend_poi_group":true,"primary_recommend_product_type":1}"#.to_string()
}

/// 计算超时时间
///
/// # 参数
///
/// * `timeout` - 超时时间（秒）
/// * `send_time` - 发送时间（Unix时间戳，秒）
///
/// # 返回
///
/// 计算后的超时时间
pub fn calculate_timeout(timeout: i64, send_time: i64) -> i64 {
    if send_time > 0 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        return send_time - now;
    }
    timeout
}

/// 对象转JSON字符串
///
/// 使用 `serde_json` 将对象序列化为JSON字符串
///
/// # 参数
///
/// * `obj` - 可序列化的对象
///
/// # 返回
///
/// JSON字符串
pub fn to_json_string<T: Serialize>(obj: &T) -> String {
    serde_json::to_string(obj).unwrap_or_default()
}

/// 判断是否为中文字符
///
/// 使用Unicode范围判断
fn is_chinese(c: char) -> bool {
    let code = c as u32;

    // CJK统一汉字范围
    if (0x4E00..=0x9FFF).contains(&code) {
        return true;
    }
    // CJK统一汉字扩展A
    if (0x3400..=0x4DBF).contains(&code) {
        return true;
    }
    // CJK兼容字符
    if (0xF900..=0xFAFF).contains(&code) {
        return true;
    }
    // CJK标点符号
    if (0x3000..=0x303F).contains(&code) {
        return true;
    }

    false
}

/// 判断是否为Emoji字符
fn is_emoji(c: char) -> bool {
    // 简单的Emoji判断（基于Unicode范围）
    // 范围包括：表情符号、符号文字、装饰符号等
    let code = c as u32;

    // 基础表情符号范围
    if (0x1F600..=0x1F64F).contains(&code) {
        return true;
    }
    // 装饰符号
    if (0x1F300..=0x1F5FF).contains(&code) {
        return true;
    }
    // 交通和地图符号
    if (0x1F680..=0x1F6FF).contains(&code) {
        return true;
    }
    // 杂项符号
    if (0x2600..=0x26FF).contains(&code) {
        return true;
    }
    // 箭头
    if (0x2190..=0x21FF).contains(&code) {
        return true;
    }
    // 补充字符
    if (0xFE00..=0xFE0F).contains(&code) {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_html_tags() {
        let input = "<p>Hello <b>World</b></p>";
        let result = strip_html_tags(input);
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_get_string_length() {
        let input = "你好Hello";
        let length = get_string_length(input);
        // "你"=2, "好"=2, "H"=1, "e"=1, "l"=1, "l"=1, "o"=1 = 9
        assert_eq!(length, 9);
    }

    #[test]
    fn test_substr() {
        let input = "你好Hello";
        let result = substr(input, 0, 5);
        // 最多5个长度：你好H = 2+2+1=5
        assert_eq!(result, "你好H");
    }

    #[test]
    fn test_generate_creation_id() {
        let id1 = generate_creation_id();
        let id2 = generate_creation_id();
        assert_ne!(id1, id2);
        assert!(id1.len() > 10);
    }

    #[test]
    fn test_calculate_timing() {
        // 无send_time
        let timing = calculate_timing(3600, 0);
        assert!(timing > 0);

        // 有send_time
        let timing = calculate_timing(3600, 2000000000);
        assert_eq!(timing, 2000000000);
    }
}
