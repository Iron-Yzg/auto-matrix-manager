// AWS Signature Version 4 签名实现
// 用于抖音视频上传的请求签名
//
// 参考 Python 实现: signature_v4.py

use hmac::{Hmac, Mac};
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use reqwest::Url;

/// V4签名器
#[derive(Debug, Clone)]
pub struct SignatureV4 {
    /// 签名密钥缓存
    cache: HashMap<String, Vec<u8>>,
    /// 缓存大小
    cache_size: usize,
}

impl SignatureV4 {
    /// 创建新的签名器
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            cache_size: 0,
        }
    }

    /// 对请求进行签名，返回签名后的请求头
    ///
    /// # 参数
    /// * `method` - HTTP方法
    /// * `url` - 请求URL
    /// * `params` - URL查询参数
    /// * `headers` - 请求头
    /// * `body` - 请求体
    /// * `credentials` - 凭证字典
    ///
    /// # 返回
    /// 包含Authorization头的请求头字典
    pub fn sign_request_headers(
        &mut self,
        method: &str,
        url: &str,
        params: &HashMap<String, String>,
        headers: &HashMap<String, String>,
        body: &[u8],
        credentials: &HashMap<String, String>,
    ) -> Result<HashMap<String, String>, String> {
        // 解析URL
        let parsed_url = Url::parse(url).map_err(|e| format!("URL解析失败: {}", e))?;
        let path = parsed_url.path();

        // 生成时间戳
        let now = chrono::Utc::now();
        let ldt = now.format("%Y%m%dT%H%M%SZ").to_string();
        let sdt = &ldt[..8]; // 短日期 YYYYMMDD

        // 使用更长生命周期的默认值
        let default_region = "cn-north-1".to_string();
        let default_service = "vod".to_string();
        let region = credentials.get("region").unwrap_or(&default_region);
        let service = credentials.get("service").unwrap_or(&default_service);
        let credential_scope = self.create_scope(sdt, region, service);

        // 添加时间戳到请求头
        let mut signed_headers = headers.clone();
        signed_headers.insert("X-Amz-Date".to_string(), ldt.clone());

        // 添加host头（AWS Signature V4要求）
        if let Some(host) = parsed_url.host_str() {
            signed_headers.insert("Host".to_string(), host.to_string());
        }

        // 如果有Session Token，添加到请求头
        if let Some(st) = credentials.get("st") {
            signed_headers.insert("X-Amz-Security-Token".to_string(), st.clone());
        }

        // 计算payload哈希
        let payload_hash = if !body.is_empty() {
            format!("{:x}", Sha256::digest(body))
        } else {
            format!("{:x}", Sha256::digest(&[]))
        };

        // 创建规范化请求
        let context = self.create_context(method, path, params, &signed_headers, &payload_hash);

        // 创建待签名字符串
        let string_to_sign = self.create_string_to_sign(&ldt, &credential_scope, &context.creq);

        // 获取签名密钥
        let secret_key = credentials.get("sk").ok_or("缺少SecretAccessKey")?;
        let signing_key = self.get_signing_key(sdt, region, service, secret_key)?;

        // 计算签名
        let signature = self.compute_signature(&string_to_sign, &signing_key);

        // 构建Authorization头
        let ak = credentials.get("ak").ok_or("缺少AccessKeyID")?;
        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            ak, credential_scope, context.headers, signature
        );

        signed_headers.insert("Authorization".to_string(), authorization);

        Ok(signed_headers)
    }

    /// 创建凭证范围
    fn create_scope(&self, short_date: &str, region: &str, service: &str) -> String {
        format!("{}/{}/{}/aws4_request", short_date, region, service)
    }

    /// 获取签名密钥（带缓存）
    fn get_signing_key(
        &mut self,
        short_date: &str,
        region: &str,
        service: &str,
        secret_key: &str,
    ) -> Result<Vec<u8>, String> {
        // 构建缓存键
        let k_secret_str = format!("AWS4{}", secret_key);
        let cache_key = format!("{}_{}_{}_{}", short_date, region, service, k_secret_str);

        if let Some(key) = self.cache.get(&cache_key) {
            return Ok(key.clone());
        }

        // 清除缓存（当达到50个条目时）
        if self.cache_size >= 50 {
            self.cache.clear();
            self.cache_size = 0;
        }

        // 计算签名密钥
        let k_secret = k_secret_str.as_bytes();

        // kDate = HMAC('AWS4' + secretKey, shortDate)
        let mut hasher = Hmac::<Sha256>::new_from_slice(k_secret).map_err(|e| format!("HMAC错误: {}", e))?;
        hasher.update(short_date.as_bytes());
        let k_date = hasher.finalize().into_bytes();

        // kRegion = HMAC(kDate, region)
        let mut hasher = Hmac::<Sha256>::new_from_slice(&k_date).map_err(|e| format!("HMAC错误: {}", e))?;
        hasher.update(region.as_bytes());
        let k_region = hasher.finalize().into_bytes();

        // kService = HMAC(kRegion, service)
        let mut hasher = Hmac::<Sha256>::new_from_slice(&k_region).map_err(|e| format!("HMAC错误: {}", e))?;
        hasher.update(service.as_bytes());
        let k_service = hasher.finalize().into_bytes();

        // kSigning = HMAC(kService, 'aws4_request')
        let mut hasher = Hmac::<Sha256>::new_from_slice(&k_service).map_err(|e| format!("HMAC错误: {}", e))?;
        hasher.update(b"aws4_request");
        let k_signing: Vec<u8> = hasher.finalize().into_bytes().to_vec();

        self.cache.insert(cache_key.clone(), k_signing.clone());
        self.cache_size += 1;

        Ok(k_signing)
    }

    /// 计算签名
    fn compute_signature(&self, string_to_sign: &str, signing_key: &[u8]) -> String {
        let mut hasher = Hmac::<Sha256>::new_from_slice(signing_key).unwrap();
        hasher.update(string_to_sign.as_bytes());
        let result = hasher.finalize().into_bytes();
        hex::encode(result)
    }

    /// 创建待签名字符串
    fn create_string_to_sign(&self, long_date: &str, credential_scope: &str, canonical_request: &str) -> String {
        let creq_hash = format!("{:x}", Sha256::digest(canonical_request.as_bytes()));
        format!("AWS4-HMAC-SHA256\n{}\n{}\n{}", long_date, credential_scope, creq_hash)
    }

    /// 创建规范化请求上下文
    fn create_context(
        &self,
        method: &str,
        path: &str,
        query: &HashMap<String, String>,
        headers: &HashMap<String, String>,
        payload: &str,
    ) -> Context {
        // 不签名的请求头黑名单
        let blacklist = [
            "cache-control", "content-type", "content-length", "expect",
            "max-forwards", "pragma", "range", "te", "if-match",
            "if-none-match", "if-modified-since", "if-unmodified-since",
            "if-range", "accept", "authorization", "proxy-authorization",
            "from", "referer", "user-agent", "proxy",
        ]
        .iter()
        .cloned()
        .collect::<std::collections::HashSet<_>>();

        // 规范化路径
        let canonical_path = self.create_canonicalized_path(path);

        // 规范化查询字符串
        let canonical_query = self.create_canonicalized_query(query);

        // 规范化请求头
        let mut canonical_headers = Vec::new();
        let mut signed_headers_list = Vec::new();

        let mut aggregate: HashMap<String, Vec<String>> = HashMap::new();
        for (key, value) in headers {
            let key_lower = key.to_lowercase();
            if !blacklist.contains(&key_lower.as_str()) {
                aggregate.entry(key_lower).or_default().push(value.clone());
            }
        }

        // 排序并规范化
        let mut keys: Vec<&String> = aggregate.keys().collect();
        keys.sort();
        for key in keys {
            let values: Vec<String> = aggregate.get(key).unwrap().iter().cloned().collect();
            let value_str = values.join(" ");
            canonical_headers.push(format!("{}:{}", key, value_str));
            signed_headers_list.push(key.clone());
        }

        let signed_headers_string = signed_headers_list.join(";");

        // 构建规范化请求
        let headers_part = if !canonical_headers.is_empty() {
            canonical_headers.join("\n") + "\n\n"
        } else {
            "\n\n".to_string()
        };

        let creq = format!(
            "{}\n{}\n{}\n{}{}\n{}",
            method,
            canonical_path,
            canonical_query,
            headers_part,
            signed_headers_string,
            payload
        );

        Context {
            creq,
            headers: signed_headers_string,
        }
    }

    /// 创建规范化路径
    fn create_canonicalized_path(&self, path: &str) -> String {
        if path.is_empty() || path == "/" {
            return "/".to_string();
        }

        // PHP实现：rawurlencode(ltrim($path, '/')) 然后替换 %2F 为 /
        let path_without_slash = path.trim_start_matches('/');
        if path_without_slash.is_empty() {
            return "/".to_string();
        }

        // URL编码
        let double_encoded = urlencoding::encode(path_without_slash);
        // 将编码后的 %2F 替换回 /
        let normalized = format!("/{}", double_encoded.replace("%2F", "/"));

        normalized
    }

    /// 创建规范化查询字符串
    fn create_canonicalized_query(&self, query: &HashMap<String, String>) -> String {
        if query.is_empty() {
            return String::new();
        }

        let mut params: Vec<_> = query.iter().collect();
        params.sort_by_key(|&(k, _)| k);

        let mut result = String::new();
        for (key, value) in params {
            if !result.is_empty() {
                result.push('&');
            }
            result.push_str(&urlencoding::encode(key));
            result.push('=');
            result.push_str(&urlencoding::encode(value));
        }

        result
    }
}

/// 上下文结构体
struct Context {
    /// 规范化请求
    creq: String,
    /// 签名的请求头
    headers: String,
}

impl Default for SignatureV4 {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_v4() {
        let mut signer = SignatureV4::new();

        let mut credentials = HashMap::new();
        credentials.insert("ak".to_string(), "test_access_key".to_string());
        credentials.insert("sk".to_string(), "test_secret_key".to_string());
        credentials.insert("st".to_string(), "test_session_token".to_string());
        credentials.insert("region".to_string(), "cn-north-1".to_string());
        credentials.insert("service".to_string(), "vod".to_string());

        let mut headers = HashMap::new();
        headers.insert("User-Agent".to_string(), "Mozilla/5.0".to_string());

        let mut params = HashMap::new();
        params.insert("Action".to_string(), "ApplyUploadInner".to_string());
        params.insert("Version".to_string(), "2020-11-19".to_string());

        let result = signer.sign_request_headers(
            "GET",
            "https://vod.bytedance.com/",
            &params,
            &headers,
            &[],
            &credentials,
        );

        assert!(result.is_ok());
        let signed_headers = result.unwrap();
        assert!(signed_headers.contains_key("Authorization"));
        assert!(signed_headers.contains_key("X-Amz-Date"));
    }
}
