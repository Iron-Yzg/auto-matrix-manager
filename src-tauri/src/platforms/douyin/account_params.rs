//! 抖音账号参数结构体
//!
//! 对应数据库 `accounts` 表中 `params` 字段的JSON结构
//!
//! # JSON结构示例
//!
//! ```json
//! {
//!     "third_id": "用户ID",
//!     "third_param": {
//!         "cookie": "xxx",
//!         "user-agent": "xxx",
//!         "local_data": [{"key": "xxx", "value": "xxx"}],
//!         "x-secsdk-csrf-token": "xxx"
//!     }
//! }
//! ```
//!
//! # 使用方式
//!
//! ```rust
//! use crate::platforms::douyin::account_params::AccountParams;
//!
//! let params_json = r#"{"third_id":"123","third_param":{"cookie":"abc"}}"#;
//! let params: AccountParams = serde_json::from_str(params_json).unwrap();
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::core::LocalDataItem;

/// 抖音账号参数
///
/// 包含第三方用户ID和认证参数（cookie、user-agent等）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AccountParams {
    /// 第三方用户ID
    #[serde(rename = "third_id")]
    pub third_id: Option<String>,

    /// 第三方参数（包含cookie、user-agent等）
    #[serde(rename = "third_param")]
    pub third_param: Option<ThirdParam>,
}

/// 第三方参数详情
///
/// 包含完整的认证信息
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ThirdParam {
    /// 用户Cookie
    #[serde(rename = "cookie")]
    pub cookie: Option<String>,

    /// User-Agent
    #[serde(rename = "user-agent")]
    pub user_agent: Option<String>,

    /// X-Secsdk-CSRF-Token
    #[serde(rename = "x-secsdk-csrf-token")]
    pub x_secsdk_csrf_token: Option<String>,

    /// 本地数据（安全SDK相关）
    /// 支持两种格式：直接数组 或 序列化字符串
    #[serde(rename = "local_data")]
    pub local_data: Option<Value>,

    /// Accept
    #[serde(rename = "accept")]
    pub accept: Option<String>,

    /// Referer
    #[serde(rename = "referer")]
    pub referer: Option<String>,

    /// Accept-Language
    #[serde(rename = "accept-language")]
    pub accept_language: Option<String>,

    /// Accept-Encoding
    #[serde(rename = "accept-encoding")]
    pub accept_encoding: Option<String>,

    /// Sec-CH-UA
    #[serde(rename = "sec-ch-ua")]
    pub sec_ch_ua: Option<String>,

    /// Sec-CH-UA-Mobile
    #[serde(rename = "sec-ch-ua-mobile")]
    pub sec_ch_ua_mobile: Option<String>,

    /// Sec-CH-UA-Platform
    #[serde(rename = "sec-ch-ua-platform")]
    pub sec_ch_ua_platform: Option<String>,

    /// Sec-Fetch-Dest
    #[serde(rename = "sec-fetch-dest")]
    pub sec_fetch_dest: Option<String>,

    /// Sec-Fetch-Mode
    #[serde(rename = "sec-fetch-mode")]
    pub sec_fetch_mode: Option<String>,

    /// Sec-Fetch-Site
    #[serde(rename = "sec-fetch-site")]
    pub sec_fetch_site: Option<String>,
}

impl AccountParams {
    /// 从JSON字符串解析账号参数
    ///
    /// # 参数
    ///
    /// * `params_json` - JSON格式的参数字符串
    ///
    /// # 返回
    ///
    /// 解析后的 `AccountParams`，如果解析失败返回默认空对象
    pub fn from_json(params_json: &str) -> Self {
        if params_json.is_empty() {
            return Self::default();
        }

        match serde_json::from_str(params_json) {
            Ok(params) => params,
            Err(e) => {
                tracing::error!("[AccountParams] JSON解析失败: {}, 输入: {}", e, &params_json[..params_json.len().min(200)]);
                Self::default()
            }
        }
    }

    /// 获取Cookie
    ///
    /// 从 `third_param.cookie` 或直接获取
    pub fn get_cookie(&self) -> String {
        self.third_param
            .as_ref()
            .and_then(|tp| tp.cookie.as_ref())
            .cloned()
            .unwrap_or_default()
    }

    /// 获取User-Agent
    ///
    /// 从 `third_param.user-agent` 或直接获取
    pub fn get_user_agent(&self) -> String {
        self.third_param
            .as_ref()
            .and_then(|tp| tp.user_agent.as_ref())
            .cloned()
            .unwrap_or_default()
    }

    /// 获取第三方用户ID
    pub fn get_third_id(&self) -> String {
        self.third_id.clone().unwrap_or_default()
    }

    /// 获取本地数据
    ///
    /// 返回 `Vec<LocalDataItem>` 格式
    /// 支持两种格式：直接数组 或 序列化字符串
    pub fn get_local_data(&self) -> Vec<LocalDataItem> {
        self.third_param
            .as_ref()
            .and_then(|tp| tp.local_data.as_ref())
            .map(|value| match value {
                // 直接数组格式
                serde_json::Value::Array(arr) => arr
                    .iter()
                    .filter_map(|item| {
                        let key = item.get("key")?.as_str()?.to_string();
                        let val = item.get("value")?.as_str()?.to_string();
                        Some(LocalDataItem { key, value: val })
                    })
                    .collect(),
                // 序列化字符串格式: "[{\"key\":\"xxx\",\"value\":\"yyy\"}]"
                serde_json::Value::String(s) => {
                    if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(s) {
                        arr.iter()
                            .filter_map(|item| {
                                let key = item.get("key")?.as_str()?.to_string();
                                let val = item.get("value")?.as_str()?.to_string();
                                Some(LocalDataItem { key, value: val })
                            })
                            .collect()
                    } else {
                        tracing::warn!("[AccountParams] local_data 序列化字符串解析失败: {}", &s[..s.len().min(100)]);
                        Vec::new()
                    }
                }
                _ => {
                    tracing::warn!("[AccountParams] local_data 类型不支持: {:?}", value);
                    Vec::new()
                }
            })
            .unwrap_or_default()
    }

    /// 获取x-secsdk-csrf-token
    pub fn get_x_secsdk_csrf_token(&self) -> String {
        self.third_param
            .as_ref()
            .and_then(|tp| tp.x_secsdk_csrf_token.as_ref())
            .cloned()
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_params_from_json() {
        let json = r#"{
            "third_id": "123456",
            "third_param": {
                "cookie": "sessionid=abc123",
                "user-agent": "Mozilla/5.0",
                "local_data": [
                    {"key": "test_key", "value": "test_value"}
                ]
            }
        }"#;

        let params = AccountParams::from_json(json);

        assert_eq!(params.get_third_id(), "123456");
        assert_eq!(params.get_cookie(), "sessionid=abc123");
        assert_eq!(params.get_user_agent(), "Mozilla/5.0");
        assert_eq!(params.get_local_data().len(), 1);
    }

    #[test]
    fn test_account_params_empty() {
        let params = AccountParams::from_json("");
        assert_eq!(params.get_third_id(), "");
        assert_eq!(params.get_cookie(), "");
    }
}
