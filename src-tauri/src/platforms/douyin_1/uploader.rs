// 抖音视频上传器
// Douyin Video Uploader
//
// 本模块负责将视频文件上传到抖音VOD（Video on Demand）服务
// 支持小文件直接上传和大文件分片上传
//
// 上传流程：
// 1. 调用 ApplyUploadInner API 获取上传地址和凭证
// 2. 生成V4签名
// 3. 上传视频（直传或分片）
// 4. 调用 CommitUploadInner API 提交上传
//
// 参考 Python 实现: uploader.py

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use crate::core::PlatformError;
use super::client::{UploadAuth, BdTicketConfig};
use super::signature_v4::SignatureV4;

// ============================================================================
// 常量定义
// ============================================================================

/// VOD服务基础URL
const VIDEO_UPLOAD_URL: &str = "https://vod.bytedanceapi.com/";

/// 小文件阈值（5MB）
const VIDEO_MAX_SIZE: u64 = 5 * 1024 * 1024;

/// 并发上传限制
const CONCURRENT_LIMIT: usize = 2;

// ============================================================================
// 视频上传器
// ============================================================================

/// 视频上传器
/// 处理抖音VOD服务的视频上传
#[derive(Debug, Clone)]
pub struct VideoUploader {
    /// 上传认证信息
    upload_auth: UploadAuth,
    /// 第三方ID
    third_id: String,
    /// User-Agent
    user_agent: String,
    /// BD Ticket配置（预留用于后续鉴权）
    _bd_ticket_config: BdTicketConfig,
    /// HTTP客户端
    http_client: reqwest::Client,
}

impl VideoUploader {
    /// 创建新的视频上传器
    pub fn new(upload_auth: UploadAuth, third_id: String, user_agent: String, bd_ticket_config: BdTicketConfig) -> Self {
        Self {
            upload_auth,
            third_id,
            user_agent,
            _bd_ticket_config: bd_ticket_config,
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
        }
    }

    /// 上传视频文件
    ///
    /// # 参数
    /// * `video_path` - 视频文件路径
    ///
    /// # 返回
    /// 视频ID (video_id)
    pub async fn upload_video(&self, video_path: &Path) -> Result<String, PlatformError> {
        // 验证文件
        if !video_path.exists() {
            return Err(PlatformError::InvalidInput(format!("视频文件不存在: {:?}", video_path)));
        }

        let file_size = std::fs::metadata(video_path)?
            .len();

        if file_size == 0 {
            return Err(PlatformError::InvalidInput("视频文件为空".to_string()));
        }

        // 获取上传地址和凭证
        let (upload_url, upload_authorization, video_id, session_key) =
            self.get_apply_upload_inner(file_size).await?;

        // 根据文件大小选择上传方式
        if file_size <= VIDEO_MAX_SIZE {
            // 小文件直接上传
            self.upload_little_content(&upload_url, video_path, &upload_authorization).await?;
        } else {
            // 大文件分片上传
            self.upload_big_content(&upload_url, video_path, &upload_authorization).await?;
        }

        // 提交上传
        self.get_commit_upload_inner(&session_key).await?;

        Ok(video_id)
    }

    /// 申请上传，获取上传地址和凭证
    async fn get_apply_upload_inner(&self, file_size: u64) -> Result<(String, String, String, String), PlatformError> {
        // 生成随机参数s（16进制字符串）
        let random_num = rand::random::<u32>();
        let s = format!("{:x}", random_num);

        // 构建请求参数
        let mut params = HashMap::new();
        params.insert("Action".to_string(), "ApplyUploadInner".to_string());
        params.insert("Version".to_string(), "2020-11-19".to_string());
        params.insert("SpaceName".to_string(), "aweme".to_string());
        params.insert("FileType".to_string(), "video".to_string());
        params.insert("IsInner".to_string(), "1".to_string());
        params.insert("FileSize".to_string(), file_size.to_string());
        params.insert("app_id".to_string(), "2906".to_string());
        params.insert("user_id".to_string(), self.third_id.clone());
        params.insert("s".to_string(), s);

        let url = format!("{}?Action=ApplyUploadInner&Version=2020-11-19", VIDEO_UPLOAD_URL);

        let mut headers = HashMap::new();
        headers.insert("User-Agent".to_string(), self.user_agent.clone());
        headers.insert("Referer".to_string(), "https://creator.douyin.com/".to_string());
        headers.insert("Sec-Gpc".to_string(), "1".to_string());

        // 构建凭证信息
        let mut credentials = HashMap::new();
        credentials.insert("ak".to_string(), self.upload_auth.access_key_id.clone());
        credentials.insert("sk".to_string(), self.upload_auth.secret_access_key.clone());
        credentials.insert("st".to_string(), self.upload_auth.session_token.clone());
        credentials.insert("region".to_string(), "cn-north-1".to_string());
        credentials.insert("service".to_string(), "vod".to_string());

        // 使用V4签名对请求进行签名
        let mut signer = SignatureV4::new();
        let signed_headers = signer.sign_request_headers("GET", &url, &params, &headers, &[], &credentials)
            .map_err(|e| PlatformError::VideoUploadFailed(format!("签名失败: {}", e)))?;

        // 发送请求
        let response = self.http_client
            .get(url)
            .query(&params)
            .headers(self.convert_headers(&signed_headers))
            .send()
            .await
            .map_err(|e| PlatformError::NetworkError(format!("获取上传凭证失败: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(PlatformError::VideoUploadFailed(
                format!("HTTP {}: {}", status, error_text)
            ));
        }

        let data: serde_json::Value = response.json().await
            .map_err(|e| PlatformError::NetworkError(format!("解析响应失败: {}", e)))?;

        // 检查错误
        if let Some(error) = data.get("ResponseMetadata").and_then(|m| m.get("Error")) {
            let error_msg = error.get("Message").map(|m| m.as_str().unwrap_or("")).unwrap_or("获取失败");
            return Err(PlatformError::VideoUploadFailed(format!("{} [上传凭证]", error_msg)));
        }

        // 提取上传信息
        let upload_result = data.get("Result").ok_or(PlatformError::VideoUploadFailed("无效的响应格式".to_string()))?;
        let inner_upload = upload_result.get("InnerUploadAddress").ok_or(PlatformError::VideoUploadFailed("获取InnerUploadAddress失败".to_string()))?;
        let upload_nodes = inner_upload.get("UploadNodes").and_then(|v| v.as_array()).ok_or(PlatformError::VideoUploadFailed("获取UploadNodes失败".to_string()))?;

        if upload_nodes.is_empty() {
            return Err(PlatformError::VideoUploadFailed("上传节点为空".to_string()));
        }

        let node = &upload_nodes[0];
        let upload_host = node.get("UploadHost").and_then(|v| v.as_str()).ok_or(PlatformError::VideoUploadFailed("获取UploadHost失败".to_string()))?;
        let store_infos = node.get("StoreInfos").and_then(|v| v.as_array()).ok_or(PlatformError::VideoUploadFailed("获取StoreInfos失败".to_string()))?;

        if store_infos.is_empty() {
            return Err(PlatformError::VideoUploadFailed("存储信息为空".to_string()));
        }

        let store_info = &store_infos[0];
        let store_uri = store_info.get("StoreUri").and_then(|v| v.as_str()).ok_or(PlatformError::VideoUploadFailed("获取StoreUri失败".to_string()))?;
        let upload_authorization = store_info.get("Auth").and_then(|v| v.as_str()).ok_or(PlatformError::VideoUploadFailed("获取Auth失败".to_string()))?.to_string();
        let video_id = node.get("Vid").and_then(|v| v.as_str()).ok_or(PlatformError::VideoUploadFailed("获取Vid失败".to_string()))?.to_string();
        let session_key = node.get("SessionKey").and_then(|v| v.as_str()).ok_or(PlatformError::VideoUploadFailed("获取SessionKey失败".to_string()))?.to_string();

        let upload_url = format!("https://{}/upload/v1/{}", upload_host, store_uri);

        Ok((upload_url, upload_authorization, video_id, session_key))
    }

    /// 上传小文件（小于5MB）
    async fn upload_little_content(&self, upload_url: &str, video_path: &Path, upload_authorization: &str) -> Result<(), PlatformError> {
        let max_retries = 3;

        for attempt in 0..max_retries {
            match self.upload_little_content_once(upload_url, video_path, upload_authorization).await {
                Ok(()) => return Ok(()),
                Err(e) if attempt < max_retries - 1 => {
                    tokio::time::sleep(Duration::from_millis(800)).await;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }

        Err(PlatformError::VideoUploadFailed("单文件上传失败".to_string()))
    }

    async fn upload_little_content_once(&self, upload_url: &str, video_path: &Path, upload_authorization: &str) -> Result<(), PlatformError> {
        // 读取文件内容
        let mut file = File::open(video_path)?;
        let mut content = Vec::new();
        file.read_to_end(&mut content)?;

        // 计算CRC32校验和
        let crc32 = Self::calculate_crc32(&content);

        let headers = reqwest::header::HeaderMap::new();
        let mut headers = headers;

        headers.insert("Authorization", upload_authorization.parse().unwrap());
        headers.insert("Content-Length", content.len().to_string().parse().unwrap());
        headers.insert("Content-CRC32", crc32.parse().unwrap());
        headers.insert("Referer", "https://creator.douyin.com/".parse().unwrap());
        headers.insert("User-Agent", self.user_agent.parse().unwrap());
        headers.insert("X-Logical-Part-Mode", "logical_part".parse().unwrap());
        headers.insert("X-Storage-U", self.third_id.parse().unwrap());

        let response = self.http_client
            .post(upload_url)
            .headers(headers)
            .body(content)
            .send()
            .await
            .map_err(|e| PlatformError::NetworkError(format!("上传失败: {}", e)))?;

        if !response.status().is_success() {
            return Err(PlatformError::VideoUploadFailed(
                format!("HTTP {}", response.status())
            ));
        }

        Ok(())
    }

    /// 上传大文件（大于5MB，分片上传）
    async fn upload_big_content(&self, upload_url: &str, video_path: &Path, upload_authorization: &str) -> Result<(), PlatformError> {
        let file_size = std::fs::metadata(video_path)?.len();

        // 初始化分片上传
        let upload_id = self.init_part_upload(upload_url, upload_authorization).await?;
        // 克隆一份用于循环中
        let upload_id_for_tasks = upload_id.clone();

        // 计算分片数量
        let last_chunk_size = file_size % VIDEO_MAX_SIZE;
        let actual_chunk_count = ((file_size + VIDEO_MAX_SIZE - 1) / VIDEO_MAX_SIZE) as usize;

        let actual_chunk_count = if last_chunk_size > 0 {
            actual_chunk_count - 1
        } else {
            actual_chunk_count
        };

        // 上传所有分片（可以并发上传，但受CONCURRENT_LIMIT限制）
        let mut uploaded_chunks = Vec::new();

        // 使用信号量限制并发（使用Arc包装）
        let semaphore = Arc::new(tokio::sync::Semaphore::new(CONCURRENT_LIMIT));
        let mut tasks = Vec::new();

        for i in 0..actual_chunk_count {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let upload_url = upload_url.to_string();
            let upload_authorization = upload_authorization.to_string();
            let video_path = video_path.to_path_buf();
            let third_id = self.third_id.clone();
            let user_agent = self.user_agent.clone();
            let upload_id_for_tasks = upload_id_for_tasks.clone(); // 每次循环克隆

            let task = tokio::spawn(async move {
                let _permit = permit;
                Self::upload_chunk(&video_path, &upload_url, &upload_id_for_tasks, actual_chunk_count, &upload_authorization, i, &third_id, &user_agent).await
            });

            tasks.push(task);
        }

        for task in tasks {
            let result = task.await.map_err(|e| PlatformError::VideoUploadFailed(format!("任务执行失败: {}", e)))?;
            uploaded_chunks.push(result?);
        }

        // 完成分片上传
        self.finish_part_upload(upload_url, &upload_id, upload_authorization, &uploaded_chunks).await?;

        Ok(())
    }

    /// 上传单个分片
    async fn upload_chunk(
        video_path: &Path,
        upload_url: &str,
        upload_id: &str,
        actual_chunk_count: usize,
        upload_authorization: &str,
        index: usize,
        third_id: &str,
        user_agent: &str,
    ) -> Result<ChunkInfo, PlatformError> {
        let max_retries = 3;

        for attempt in 0..max_retries {
            match Self::upload_chunk_once(video_path, upload_url, upload_id, actual_chunk_count, upload_authorization, index, third_id, user_agent).await {
                Ok(info) => return Ok(info),
                Err(e) if attempt < max_retries - 1 => {
                    tokio::time::sleep(Duration::from_millis(800)).await;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }

        Err(PlatformError::VideoUploadFailed("上传分片失败".to_string()))
    }

    async fn upload_chunk_once(
        video_path: &Path,
        upload_url: &str,
        upload_id: &str,
        actual_chunk_count: usize,
        upload_authorization: &str,
        index: usize,
        third_id: &str,
        user_agent: &str,
    ) -> Result<ChunkInfo, PlatformError> {
        let file_size = std::fs::metadata(video_path)?.len();
        let start = (index as u64) * VIDEO_MAX_SIZE;

        // 计算分片大小
        let chunk_size = if index == actual_chunk_count - 1 && actual_chunk_count != (file_size / VIDEO_MAX_SIZE) as usize {
            file_size - start
        } else {
            std::cmp::min(start + VIDEO_MAX_SIZE, file_size) - start
        };

        // 读取分片内容
        let mut file = File::open(video_path)?;
        file.seek(std::io::SeekFrom::Start(start))?;
        let mut content = vec![0u8; chunk_size as usize];
        file.read_exact(&mut content)?;

        // 计算CRC32
        let crc32 = Self::calculate_crc32(&content);

        let mut params = HashMap::new();
        params.insert("phase".to_string(), "transfer".to_string());
        params.insert("part_number".to_string(), (index + 1).to_string());
        params.insert("part_offset".to_string(), start.to_string());
        params.insert("uploadid".to_string(), upload_id.to_string());

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Authorization", upload_authorization.parse().unwrap());
        headers.insert("Content-Type", "application/octet-stream".parse().unwrap());
        headers.insert("Content-Length", chunk_size.to_string().parse().unwrap());
        headers.insert("Content-CRC32", crc32.parse().unwrap());
        headers.insert("X-Storage-U", third_id.parse().unwrap());
        headers.insert("Referer", "https://creator.douyin.com/".parse().unwrap());
        headers.insert("User-Agent", user_agent.parse().unwrap());
        headers.insert("X-Logical-Part-Mode", "logical_part".parse().unwrap());
        headers.insert("X-Storage-Mode", "gateway".parse().unwrap());

        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap();

        let response = http_client
            .post(upload_url)
            .query(&params)
            .headers(headers)
            .body(content)
            .send()
            .await
            .map_err(|e| PlatformError::NetworkError(format!("上传分片失败: {}", e)))?;

        if !response.status().is_success() {
            return Err(PlatformError::VideoUploadFailed(
                format!("HTTP {}", response.status())
            ));
        }

        Ok(ChunkInfo {
            part_number: index + 1,
            crc32,
        })
    }

    /// 初始化分片上传
    async fn init_part_upload(&self, upload_url: &str, upload_authorization: &str) -> Result<String, PlatformError> {
        let mut params = HashMap::new();
        params.insert("uploadmode".to_string(), "part".to_string());
        params.insert("phase".to_string(), "init".to_string());

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Authorization", upload_authorization.parse().unwrap());
        headers.insert("Referer", "https://creator.douyin.com/".parse().unwrap());
        headers.insert("Sec-Gpc", "1".parse().unwrap());
        headers.insert("User-Agent", self.user_agent.parse().unwrap());
        headers.insert("X-Logical-Part-Mode", "logical_part".parse().unwrap());
        headers.insert("X-Storage-Mode", "gateway".parse().unwrap());
        headers.insert("X-Storage-U", self.third_id.parse().unwrap());

        let response = self.http_client
            .post(upload_url)
            .query(&params)
            .headers(headers)
            .send()
            .await
            .map_err(|e| PlatformError::NetworkError(format!("初始化分片上传失败: {}", e)))?;

        if !response.status().is_success() {
            return Err(PlatformError::VideoUploadFailed(
                format!("HTTP {}", response.status())
            ));
        }

        let data: serde_json::Value = response.json().await
            .map_err(|e| PlatformError::NetworkError(format!("解析响应失败: {}", e)))?;

        let upload_id = data.get("data")
            .and_then(|d| d.get("uploadid"))
            .and_then(|v| v.as_str())
            .ok_or(PlatformError::VideoUploadFailed("初始化分片上传失败".to_string()))?
            .to_string();

        Ok(upload_id)
    }

    /// 完成分片上传
    async fn finish_part_upload(
        &self,
        upload_url: &str,
        upload_id: &str,
        upload_authorization: &str,
        uploaded_chunks: &[ChunkInfo],
    ) -> Result<(), PlatformError> {
        // 按partNumber排序
        let mut sorted_chunks = uploaded_chunks.to_vec();
        sorted_chunks.sort_by_key(|c| c.part_number);

        // 构建partInfo字符串：partNumber:crc32,partNumber:crc32,...
        let part_info: Vec<String> = sorted_chunks.iter()
            .map(|c| format!("{}:{}", c.part_number, c.crc32))
            .collect();
        let part_info_str = part_info.join(",");

        let mut params = HashMap::new();
        params.insert("uploadmode".to_string(), "part".to_string());
        params.insert("phase".to_string(), "finish".to_string());
        params.insert("uploadid".to_string(), upload_id.to_string());

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Authorization", upload_authorization.parse().unwrap());
        headers.insert("Content-Type", "text/plain;charset=UTF-8".parse().unwrap());
        headers.insert("Referer", "https://creator.douyin.com/".parse().unwrap());
        headers.insert("User-Agent", self.user_agent.parse().unwrap());
        headers.insert("X-Logical-Part-Mode", "logical_part".parse().unwrap());
        headers.insert("X-Storage-Mode", "gateway".parse().unwrap());

        let response = self.http_client
            .post(upload_url)
            .query(&params)
            .headers(headers)
            .body(part_info_str)
            .send()
            .await
            .map_err(|e| PlatformError::NetworkError(format!("完成分片上传失败: {}", e)))?;

        if !response.status().is_success() {
            return Err(PlatformError::VideoUploadFailed(
                format!("HTTP {}", response.status())
            ));
        }

        Ok(())
    }

    /// 提交上传完成
    async fn get_commit_upload_inner(&self, session_key: &str) -> Result<(), PlatformError> {
        let mut params = HashMap::new();
        params.insert("Action".to_string(), "CommitUploadInner".to_string());
        params.insert("Version".to_string(), "2020-11-19".to_string());
        params.insert("SpaceName".to_string(), "aweme".to_string());
        params.insert("app_id".to_string(), "2906".to_string());
        params.insert("user_id".to_string(), self.third_id.clone());

        let body = serde_json::json!({
            "SessionKey": session_key,
            "Functions": [
                {"name": "GetMeta"},
                {
                    "name": "Snapshot",
                    "input": {"SnapshotTime": 0}
                }
            ]
        });

        let url = format!("{}?Action=CommitUploadInner&Version=2020-11-19", VIDEO_UPLOAD_URL);
        let body_bytes = body.to_string().into_bytes();

        let mut headers = HashMap::new();
        headers.insert("User-Agent".to_string(), self.user_agent.clone());
        headers.insert("Referer".to_string(), "https://creator.douyin.com/".to_string());
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        // 构建凭证信息
        let mut credentials = HashMap::new();
        credentials.insert("ak".to_string(), self.upload_auth.access_key_id.clone());
        credentials.insert("sk".to_string(), self.upload_auth.secret_access_key.clone());
        credentials.insert("st".to_string(), self.upload_auth.session_token.clone());
        credentials.insert("region".to_string(), "cn-north-1".to_string());
        credentials.insert("service".to_string(), "vod".to_string());

        // 使用V4签名对请求进行签名
        let mut signer = SignatureV4::new();
        let signed_headers = signer.sign_request_headers("POST", &url, &params, &headers, &body_bytes, &credentials)
            .map_err(|e| PlatformError::VideoUploadFailed(format!("签名失败: {}", e)))?;

        let response = self.http_client
            .post(url)
            .query(&params)
            .headers(self.convert_headers(&signed_headers))
            .body(body_bytes)
            .send()
            .await
            .map_err(|e| PlatformError::NetworkError(format!("提交上传失败: {}", e)))?;

        if !response.status().is_success() {
            return Err(PlatformError::VideoUploadFailed(
                format!("HTTP {}", response.status())
            ));
        }

        Ok(())
    }

    /// 计算CRC32校验和
    fn calculate_crc32(data: &[u8]) -> String {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(data);
        format!("{:08x}", hasher.finalize())
    }

    /// 转换HashMap到reqwest HeaderMap
    fn convert_headers(&self, headers: &HashMap<String, String>) -> reqwest::header::HeaderMap {
        let mut result = reqwest::header::HeaderMap::new();
        for (key, value) in headers {
            if let (Ok(header_name), Ok(header_value)) = (
                reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                value.parse::<reqwest::header::HeaderValue>(),
            ) {
                result.insert(header_name, header_value);
            }
        }
        result
    }
}

// ============================================================================
// 辅助类型
// ============================================================================

/// 分片信息
#[derive(Debug, Clone)]
struct ChunkInfo {
    part_number: usize,
    crc32: String,
}
