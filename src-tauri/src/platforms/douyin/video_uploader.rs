//! 视频上传器
//!
//! 负责将视频文件上传到抖音VOD服务器
//! 使用AWS Signature Version 4签名
//!
//! # 上传流程
//!
//! 1. **申请上传** (`apply_upload_inner`)
//!    - 生成随机参数s
//!    - 构建请求参数
//!    - 使用V4签名发送请求
//!    - 解析响应，获取上传地址、凭证、videoId、sessionKey
//!
//! 2. **上传视频内容**
//!    - 小文件（<=5MB）：直接上传
//!    - 大文件（>5MB）：分片上传
//!
//! 3. **提交上传完成** (`commit_upload_inner`)
//!    - 使用V4签名提交上传完成
//!
//! # 使用示例
//!
//! ```rust
//! use crate::platforms::douyin::video_uploader::VideoUploader;
//!
//! let upload_auth: HashMap<String, Value> = ...;
//! let uploader = VideoUploader::new(upload_auth, "third_id".to_string(), "user_agent".to_string());
//! let video_id = uploader.upload_video("/path/to/video.mp4").await?;
//! ```
//!
//! # 与Java代码对照
//!
//! 本模块完全对应Java中的 `VideoUploader.java`

use crate::platforms::douyin::signature_v4::SignatureV4;
use reqwest::{Client, StatusCode};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;

/// VOD API URL
const VOD_API_URL: &str = "https://vod.bytedanceapi.com/";

/// 视频分片大小 (5MB)
const VIDEO_MAX_SIZE: u64 = 5 * 1024 * 1024;

/// 申请上传结果
#[derive(Debug, Clone)]
struct UploadApplyResult {
    /// 上传URL
    upload_url: String,
    /// 上传凭证
    upload_auth: String,
    /// 视频ID
    video_id: String,
    /// 会话密钥
    session_key: String,
}

/// 分片信息
#[derive(Debug, Clone)]
struct PartInfo {
    /// 分片编号
    part_number: i32,
    /// CRC32校验值
    crc32: String,
}

/// 视频上传器
///
/// 负责将视频文件上传到抖音VOD服务器
#[derive(Debug, Clone)]
pub struct VideoUploader {
    /// 上传授权信息
    upload_auth: HashMap<String, Value>,
    /// 第三方用户ID
    third_id: String,
    /// User-Agent
    user_agent: String,
    /// HTTP客户端
    client: Client,
}

impl VideoUploader {
    /// 创建新的上传器实例
    pub fn new(upload_auth: HashMap<String, Value>, third_id: String, user_agent: String) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            upload_auth,
            third_id,
            user_agent,
            client,
        }
    }

    /// 上传视频（主入口）
    ///
    /// # 步骤
    ///
    /// 1. 验证视频文件
    /// 2. 申请上传地址和凭证（V4签名）
    /// 3. 上传视频内容（小文件直接上传，大文件分片上传）
    /// 4. 提交上传完成（V4签名）
    pub async fn upload_video(&mut self, video_path: &str) -> Result<String, String> {
        let video_file = std::path::Path::new(video_path);

        // 验证视频文件
        tracing::info!("[UploadVideo] ====== 步骤1: 验证视频文件 ======");
        self.validate_video_file(video_file)?;

        let file_size = std::fs::metadata(video_file)
            .map_err(|e| format!("获取文件元数据失败: {}", e))?.len();

        tracing::info!("[UploadVideo] 视频文件大小: {}MB", file_size / 1024 / 1024);

        // 步骤2: 申请上传，获取上传地址和凭证
        tracing::info!("[UploadVideo] ====== 步骤2: 申请上传地址和凭证 (V4签名) ======");
        let apply_result = self.apply_upload_inner(file_size).await?;

        tracing::info!("[UploadVideo] 上传地址获取成功, videoId: {}", apply_result.video_id);

        // 步骤3: 上传视频内容
        tracing::info!("[UploadVideo] ====== 步骤3: 上传视频内容 ======");
        if file_size <= VIDEO_MAX_SIZE {
            tracing::info!("[UploadVideo] 文件小于5MB，直接上传");
            self.upload_little_content(&apply_result, video_path).await?;
        } else {
            tracing::info!("[UploadVideo] 文件大于5MB，分片上传");
            self.upload_big_content(&apply_result, video_path, file_size).await?;
        }

        // 步骤4: 提交上传完成
        tracing::info!("[UploadVideo] ====== 步骤4: 提交上传完成 (V4签名) ======");
        self.commit_upload_inner(&apply_result.session_key).await?;

        tracing::info!("[UploadVideo] 视频上传成功, videoId: {}", apply_result.video_id);
        Ok(apply_result.video_id)
    }

    /// ============ 步骤1: 申请上传 ============

    /// 申请上传，获取上传地址和凭证
    async fn apply_upload_inner(&self, file_size: u64) -> Result<UploadApplyResult, String> {
        let random_num = rand::random::<i32>();
        let s = format!("{:x}", random_num);

        // 使用HashMap<String, String>，与douyin_1的签名接口一致
        let mut params: HashMap<String, String> = HashMap::new();
        params.insert("Action".to_string(), "ApplyUploadInner".to_string());
        params.insert("Version".to_string(), "2020-11-19".to_string());
        params.insert("SpaceName".to_string(), "aweme".to_string());
        params.insert("FileType".to_string(), "video".to_string());
        params.insert("IsInner".to_string(), "1".to_string());
        params.insert("FileSize".to_string(), file_size.to_string());
        params.insert("app_id".to_string(), "2906".to_string());
        params.insert("user_id".to_string(), self.third_id.clone());
        params.insert("s".to_string(), s);

        let params_for_url = params.clone();

        let mut headers: HashMap<String, String> = HashMap::new();
        headers.insert("User-Agent".to_string(), self.user_agent.clone());
        headers.insert("Referer".to_string(), "https://creator.douyin.com/".to_string());
        headers.insert("Sec-Gpc".to_string(), "1".to_string());

        let mut signer = SignatureV4::new();
        let credentials = self.build_credentials();
        tracing::debug!("[Upload] 签名凭证 - ak: {}, sk: {}..., region: {}, service: {}",
            credentials.get("ak").map(|s| s.as_str()).unwrap_or(""),
            credentials.get("sk").map(|s| &s[..s.len().min(10)]).unwrap_or(""),
            credentials.get("region").map(|s| s.as_str()).unwrap_or(""),
            credentials.get("service").map(|s| s.as_str()).unwrap_or(""));

        // 使用douyin_1的签名接口（返回Result）
        let signed_headers = signer.sign_request_headers("GET", VOD_API_URL, &params, &headers, b"", &credentials)
            .map_err(|e| format!("签名失败: {}", e))?;

        let url = self.build_url_with_sorted_params(VOD_API_URL, params_for_url);
        tracing::debug!("[Upload] 申请上传URL: {}", url);
        tracing::debug!("[Upload] 签名头数量: {}", signed_headers.len());
        for (k, v) in &signed_headers {
            if k == "Authorization" {
                tracing::debug!("[Upload] Authorization: {}...", &v[..v.len().min(100)]);
            } else {
                tracing::debug!("[Upload] {}: {}", k, v);
            }
        }
        let response_body = self.send_signed_get_request(&url, &signed_headers).await?;

        let result: Value = serde_json::from_str(&response_body)
            .map_err(|e| format!("解析JSON失败: {}", e))?;

        self.check_error(&result)?;
        self.parse_upload_apply_result(&result)
    }

    /// 解析申请上传响应
    fn parse_upload_apply_result(&self, result: &Value) -> Result<UploadApplyResult, String> {
        tracing::debug!("[Parse] 开始解析上传申请响应");

        let upload_result = result.get("Result")
            .and_then(|v| v.as_object())
            .ok_or_else(|| {
                tracing::error!("[Parse] 未找到Result字段或不是对象，完整响应: {}", result);
                "获取上传节点失败".to_string()
            })?;

        tracing::debug!("[Parse] upload_result keys: {:?}", upload_result.keys().map(|s| s.as_str()).collect::<Vec<_>>());

        let inner_upload = upload_result.get("InnerUploadAddress")
            .and_then(|v| v.as_object())
            .ok_or_else(|| {
                tracing::error!("[Parse] 未找到InnerUploadAddress字段");
                "获取InnerUploadAddress失败".to_string()
            })?;

        tracing::debug!("[Parse] inner_upload keys: {:?}", inner_upload.keys().map(|s| s.as_str()).collect::<Vec<_>>());

        let upload_nodes = inner_upload.get("UploadNodes")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                tracing::error!("[Parse] 未找到UploadNodes字段或不是数组");
                "获取UploadNodes失败".to_string()
            })?;

        tracing::debug!("[Parse] upload_nodes数量: {}", upload_nodes.len());

        if upload_nodes.is_empty() {
            return Err("获取上传节点失败".to_string());
        }

        let upload_node = upload_nodes.first()
            .and_then(|v| v.as_object())
            .ok_or_else(|| {
                tracing::error!("[Parse] 第一个UploadNode不是对象");
                "解析上传节点失败".to_string()
            })?;

        tracing::debug!("[Parse] upload_node keys: {:?}", upload_node.keys().map(|s| s.as_str()).collect::<Vec<_>>());

        // 从 upload_node 获取 UploadHost
        let upload_host = upload_node.get("UploadHost")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                tracing::error!("[Parse] 未找到UploadHost字段");
                "获取UploadHost失败".to_string()
            })?
            .to_string();

        tracing::debug!("[Parse] upload_host: {}", upload_host);

        // 从 upload_node 获取 Vid 作为 video_id
        let video_id = upload_node.get("Vid")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                tracing::error!("[Parse] 未找到Vid字段");
                "获取VideoId失败".to_string()
            })?
            .to_string();

        tracing::debug!("[Parse] video_id: {}", video_id);

        // 从 upload_node 获取 SessionKey
        let session_key = upload_node.get("SessionKey")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                tracing::error!("[Parse] 未找到SessionKey字段");
                "获取SessionKey失败".to_string()
            })?
            .to_string();

        tracing::debug!("[Parse] session_key: {}...", &session_key[..session_key.len().min(50)]);

        // 从 upload_node 获取 StoreInfos 数组
        let store_infos = upload_node.get("StoreInfos")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                tracing::error!("[Parse] 未找到StoreInfos字段或不是数组");
                "获取StoreInfos失败".to_string()
            })?;

        tracing::debug!("[Parse] store_infos数量: {}", store_infos.len());

        if store_infos.is_empty() {
            return Err("获取存储信息失败".to_string());
        }

        let store_info = store_infos.first()
            .and_then(|v| v.as_object())
            .ok_or_else(|| {
                tracing::error!("[Parse] 第一个StoreInfo不是对象");
                "解析存储信息失败".to_string()
            })?;

        tracing::debug!("[Parse] store_info keys: {:?}", store_info.keys().map(|s| s.as_str()).collect::<Vec<_>>());

        // 从 store_info 获取 StoreUri
        let store_uri = store_info.get("StoreUri")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                tracing::error!("[Parse] 未找到StoreUri字段");
                "获取StoreUri失败".to_string()
            })?
            .to_string();

        tracing::debug!("[Parse] store_uri: {}", store_uri);

        // 从 store_info 获取 Auth
        let auth = store_info.get("Auth")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                tracing::error!("[Parse] 未找到Auth字段");
                "获取Auth失败".to_string()
            })?
            .to_string();

        tracing::debug!("[Parse] auth: {}...", &auth[..auth.len().min(100)]);

        // 组装 upload_url: "https://" + UploadHost + "/upload/v1/" + StoreUri
        let upload_url = format!("https://{}/upload/v1/{}", upload_host, store_uri);

        tracing::debug!("[Parse] upload_url: {}", upload_url);

        Ok(UploadApplyResult {
            upload_url,
            upload_auth: auth,
            video_id,
            session_key,
        })
    }

    /// 检查响应错误
    fn check_error(&self, result: &Value) -> Result<(), String> {
        if let Some(meta) = result.get("ResponseMetadata") {
            tracing::debug!("[Check] ResponseMetadata: {}", meta);
            if let Some(error) = meta.get("Error") {
                tracing::debug!("[Check] Error: {}", error);
                let code = error.get("Code").and_then(|v| v.as_str()).unwrap_or("");
                let msg = error.get("Message").and_then(|v| v.as_str()).unwrap_or("");
                if !code.is_empty() || !msg.is_empty() {
                    return Err(format!("API错误: {} - {}", code, msg));
                }
            }
        } else {
            tracing::debug!("[Check] 无ResponseMetadata字段，响应正常");
        }
        Ok(())
    }

    /// 构建V4签名凭证
    fn build_credentials(&self) -> HashMap<String, String> {
        let mut credentials = HashMap::new();

        // 调试：打印所有可用的字段
        tracing::debug!("[Creds] upload_auth keys: {:?}", self.upload_auth.keys().map(|s| s.as_str()).collect::<Vec<_>>());

        // 注意：key 名称必须与 signature_v4.rs 中的一致
        // Java 中对应: ak, sk, st, region, service
        let ak_value = self.upload_auth.get("AccessKeyID")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let sk_value = self.upload_auth.get("SecretAccessKey")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let st_value = self.upload_auth.get("SessionToken")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        tracing::debug!("[Creds] AccessKeyID found: {}, length: {}", !ak_value.is_empty(), ak_value.len());
        tracing::debug!("[Creds] SecretAccessKey found: {}, length: {}", !sk_value.is_empty(), sk_value.len());
        tracing::debug!("[Creds] SessionToken found: {}, length: {}", !st_value.is_empty(), st_value.len());

        if !ak_value.is_empty() {
            credentials.insert("ak".to_string(), ak_value.to_string());
        }
        if !sk_value.is_empty() {
            credentials.insert("sk".to_string(), sk_value.to_string());
        }
        if !st_value.is_empty() {
            credentials.insert("st".to_string(), st_value.to_string());
        }
        if let Some(signature_version) = self.upload_auth.get("SignatureVersion") {
            if let Some(sv) = signature_version.as_str() {
                credentials.insert("sv".to_string(), sv.to_string());
            }
        }

        if credentials.get("ak").map(|s| s.is_empty()).unwrap_or(true) {
            tracing::warn!("[Creds] 未找到有效的凭证，使用空默认值");
            credentials.insert("ak".to_string(), "".to_string());
            credentials.insert("sk".to_string(), "".to_string());
            credentials.insert("st".to_string(), "".to_string());
        }

        tracing::debug!("[Creds] 最终凭证keys: {:?}", credentials.keys().map(|s| s.as_str()).collect::<Vec<_>>());
        credentials
    }

    /// ============ 步骤2: 上传视频内容 ============

    /// 上传小文件（<=5MB）
    async fn upload_little_content(&self, apply_result: &UploadApplyResult, video_path: &str) -> Result<(), String> {
        let mut file = File::open(video_path)
            .map_err(|e| format!("打开文件失败: {}", e))?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| format!("读取文件失败: {}", e))?;

        let response = self.client.put(&apply_result.upload_url)
            .header("Content-Type", "video/mp4")
            .body(buffer)
            .send()
            .await
            .map_err(|e| format!("上传视频失败: {}", e))?;

        let status = response.status();
        if status != StatusCode::OK {
            let text = response.text().await.unwrap_or_default();
            return Err(format!("上传视频失败: HTTP {}, 响应: {}", status, text));
        }

        tracing::debug!("小文件上传成功");
        Ok(())
    }

    /// 上传大文件（>5MB），分片上传
    async fn upload_big_content(&self, apply_result: &UploadApplyResult, video_path: &str, file_size: u64) -> Result<(), String> {
        let part_size = VIDEO_MAX_SIZE;
        let total_parts = (file_size + part_size - 1) / part_size;

        tracing::info!("[UploadBig] ====== 开始分片上传 ======");
        tracing::info!("[UploadBig] 文件大小: {}MB, 分片大小: {}MB, 分片数: {}",
            file_size / 1024 / 1024, part_size / 1024 / 1024, total_parts);

        // 初始化上传
        tracing::info!("[UploadBig] 步骤1: 初始化分片上传");
        let init_response = self.init_multi_part(apply_result).await?;
        // 注意：Java 返回的是 {data: {uploadid: "..."}}，不是直接 {UploadId: "..."}
        let upload_id = init_response.get("data")
            .and_then(|v| v.get("uploadid"))
            .and_then(|v| v.as_str())
            .ok_or("获取UploadId失败")?
            .to_string();

        tracing::debug!("[UploadBig] upload_id: {}", upload_id);

        let mut uploaded_parts = Vec::new();
        let mut failed = false;

        // 逐个分片上传
        tracing::info!("[UploadBig] 步骤2: 逐个上传分片");
        for i in 0..total_parts {
            let part_number = i as i32 + 1;
            let offset = i * part_size;
            let current_part_size = std::cmp::min(part_size, file_size - offset);

            tracing::info!("[UploadBig] 上传分片 {}/{}, offset: {}, size: {}",
                part_number, total_parts, offset, current_part_size);

            // 读取分片数据
            let mut file = File::open(video_path)
                .map_err(|e| format!("打开文件失败: {}", e))?;

            let mut buffer = vec![0u8; current_part_size as usize];
            file.seek(std::io::SeekFrom::Start(offset))
                .map_err(|e| format!("Seek失败: {}", e))?;
            file.read_exact(&mut buffer)
                .map_err(|e| format!("读取分片失败: {}", e))?;

            // 计算分片数据的CRC32校验值
            let part_crc32 = crc32fast::hash(&buffer);

            // 上传分片（与 Java 一致）
            let part_url = format!("{}?phase=transfer&part_number={}&part_offset={}&uploadid={}",
                apply_result.upload_url, part_number, offset, upload_id);

            let response = self.client.put(&part_url)
                .header("Authorization", &apply_result.upload_auth)
                .header("Content-Type", "application/octet-stream")
                .header("Content-Length", current_part_size)
                .header("Content-CRC32", format!("{:08x}", part_crc32))
                .header("X-Storage-U", &self.third_id)
                .header("Referer", "https://creator.douyin.com/")
                .header("User-Agent", &self.user_agent)
                .header("X-Logical-Part-Mode", "logical_part")
                .header("X-Storage-Mode", "gateway")
                .body(buffer)
                .send()
                .await
                .map_err(|e| format!("上传分片失败: {}", e))?;

            if response.status() == StatusCode::OK || response.status() == StatusCode::CREATED {
                uploaded_parts.push(PartInfo {
                    part_number,
                    crc32: format!("{:x}", part_crc32),
                });
                tracing::debug!("分片 {} 上传成功", part_number);
            } else {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                tracing::error!("分片 {} 上传失败: HTTP {}, 响应: {}", part_number, status, text);
                failed = true;
            }
        }

        if failed && uploaded_parts.is_empty() {
            return Err("所有分片上传失败".to_string());
        }

        tracing::info!("[UploadBig] 分片上传完成, 成功 {}/{} 个", uploaded_parts.len(), total_parts);

        // 完成分片上传
        tracing::info!("[UploadBig] 步骤3: 完成分片上传");
        self.complete_multi_part(apply_result, &upload_id, &uploaded_parts).await?;

        Ok(())
    }

    /// 初始化分片上传
    async fn init_multi_part(&self, apply_result: &UploadApplyResult) -> Result<Value, String> {
        // 使用 query 参数，与 Java 一致
        let init_url = format!("{}?uploadmode=part&phase=init", apply_result.upload_url);

        // 构建请求头（与 Java 一致）
        let body = serde_json::json!({
            "auth": apply_result.upload_auth,
            "session_key": apply_result.session_key,
            "callback_url": ""
        });

        let response = self.client.post(&init_url)
            .header("Authorization", &apply_result.upload_auth)
            .header("Referer", "https://creator.douyin.com/")
            .header("Sec-Gpc", "1")
            .header("User-Agent", &self.user_agent)
            .header("X-Logical-Part-Mode", "logical_part")
            .header("X-Storage-Mode", "gateway")
            .header("X-Storage-U", &self.third_id)
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&body).unwrap_or_default())
            .send()
            .await
            .map_err(|e| format!("初始化分片上传失败: {}", e))?;

        let status = response.status();
        let text = response.text().await.unwrap_or_default();

        tracing::debug!("[InitMultiPart] 响应状态: {}, 响应体: {}", status, text);

        if !status.is_success() {
            return Err(format!("初始化分片上传失败: HTTP {}, 响应: {}", status, text));
        }

        if text.is_empty() {
            tracing::warn!("[InitMultiPart] 响应体为空");
            return Err("初始化分片上传响应为空".to_string());
        }

        let result: Value = serde_json::from_str(&text)
            .map_err(|e| format!("解析初始化响应失败: {}", e))?;

        Ok(result)
    }

    /// 完成分片上传
    async fn complete_multi_part(&self, apply_result: &UploadApplyResult, upload_id: &str, parts: &[PartInfo]) -> Result<(), String> {
        // 使用 query 参数，与 Java 一致
        let complete_url = format!("{}?uploadmode=part&phase=finish&uploadid={}",
            apply_result.upload_url, upload_id);

        // 按 partNumber 排序
        let mut sorted_parts = parts.to_vec();
        sorted_parts.sort_by_key(|p| p.part_number);

        // 构建 partInfo 字符串：partNumber:crc32,partNumber:crc32,...
        let part_info_str: String = sorted_parts.iter()
            .enumerate()
            .map(|(i, p)| {
                if i > 0 { format!(",{}:{}", p.part_number, p.crc32) }
                else { format!("{}:{}", p.part_number, p.crc32) }
            })
            .collect();

        tracing::debug!("[Complete] part_info_str: {}", part_info_str);

        let response = self.client.post(&complete_url)
            .header("Authorization", &apply_result.upload_auth)
            .header("Content-Type", "text/plain;charset=UTF-8")
            .header("Referer", "https://creator.douyin.com/")
            .header("User-Agent", &self.user_agent)
            .header("X-Logical-Part-Mode", "logical_part")
            .header("X-Storage-Mode", "gateway")
            .body(part_info_str)
            .send()
            .await
            .map_err(|e| format!("完成分片上传失败: {}", e))?;

        let status = response.status();
        let text = response.text().await.unwrap_or_default();

        tracing::debug!("[Complete] 响应状态: {}, 响应体: {}", status, text);

        if !status.is_success() {
            return Err(format!("完成分片上传失败: HTTP {}, 响应: {}", status, text));
        }

        tracing::info!("分片上传完成");
        Ok(())
    }

    /// ============ 步骤3: 提交上传完成 ============

    /// 提交上传完成
    async fn commit_upload_inner(&self, session_key: &str) -> Result<(), String> {
        // 使用HashMap<String, String>，与douyin_1的签名接口一致
        let mut params: HashMap<String, String> = HashMap::new();
        params.insert("Action".to_string(), "CommitUploadInner".to_string());
        params.insert("SpaceName".to_string(), "aweme".to_string());
        params.insert("Version".to_string(), "2020-11-19".to_string());
        params.insert("app_id".to_string(), "2906".to_string());
        params.insert("user_id".to_string(), self.third_id.clone());

        let params_for_url = params.clone();

        // 构建Functions - 与Java完全一致
        let functions = serde_json::json!([
            {"name": "GetMeta"},
            {"name": "Snapshot", "input": {"SnapshotTime": 0}}
        ]);

        // 构建Body - 与Java完全一致
        // 注意：Java使用"SessionKey"（大写K），不是"session_key"
        let body = serde_json::json!({
            "SessionKey": session_key,
            "Functions": functions
        });

        // 构建Headers - 与Java完全一致
        let mut headers: HashMap<String, String> = HashMap::new();
        headers.insert("User-Agent".to_string(), self.user_agent.clone());
        headers.insert("Referer".to_string(), "https://creator.douyin.com/".to_string());
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        // 注意：Java没有Sec-Gpc头

        let mut signer = SignatureV4::new();
        let credentials = self.build_credentials();
        let body_bytes = serde_json::to_string(&body).unwrap_or_default().as_bytes().to_vec();

        tracing::debug!("[Commit] Body: {}", serde_json::to_string(&body).unwrap_or_default());

        // 使用douyin_1的签名接口（返回Result）
        let signed_headers = signer.sign_request_headers("POST", VOD_API_URL, &params, &headers, &body_bytes, &credentials)
            .map_err(|e| format!("签名失败: {}", e))?;

        let url = self.build_url_with_sorted_params(VOD_API_URL, params_for_url);
        let response_body = self.send_signed_post_request(&url, &signed_headers, &body_bytes).await?;

        tracing::debug!("提交上传响应: {}", response_body);

        // 检查响应是否有错误
        let result: Value = serde_json::from_str(&response_body)
            .map_err(|e| format!("解析JSON失败: {}", e))?;
        self.check_error(&result)?;

        Ok(())
    }

    // ============ 辅助方法 ============

    /// 验证视频文件
    fn validate_video_file(&self, video_file: &Path) -> Result<(), String> {
        if !video_file.exists() {
            return Err(format!("视频文件不存在: {}", video_file.display()));
        }
        let file_size = std::fs::metadata(video_file)
            .map_err(|e| format!("获取文件元数据失败: {}", e))?.len();
        if file_size == 0 {
            return Err("视频文件为空".to_string());
        }
        Ok(())
    }

    /// 构建带排序参数的URL
    fn build_url_with_sorted_params(&self, base_url: &str, params: HashMap<String, String>) -> String {
        let mut url = base_url.to_string();
        let mut param_list: Vec<String> = params.into_iter()
            .map(|(key, value)| {
                // 使用urlencoding::encode，与douyin_1的签名实现保持一致
                format!("{}={}", urlencoding::encode(&key), urlencoding::encode(&value))
            })
            .collect();

        param_list.sort();

        if !param_list.is_empty() {
            url.push('?');
            url.push_str(&param_list.join("&"));
        }

        url
    }

    /// 发送带签名的GET请求
    async fn send_signed_get_request(&self, url: &str, signed_headers: &HashMap<String, String>) -> Result<String, String> {
        let mut request = self.client.get(url);

        for (key, value) in signed_headers {
            request = request.header(key, value);
        }

        let response = request.send()
            .await
            .map_err(|e| format!("接口请求失败: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("接口请求失败: {}, 响应码: {}", url, response.status()));
        }

        response.text()
            .await
            .map_err(|e| format!("读取响应失败: {}", e))
    }

    /// 发送带签名的POST请求
    async fn send_signed_post_request(&self, url: &str, signed_headers: &HashMap<String, String>, body_bytes: &[u8]) -> Result<String, String> {
        let mut request = self.client.post(url);

        for (key, value) in signed_headers {
            request = request.header(key, value);
        }

        request = request.body(body_bytes.to_vec());

        let response = request.send()
            .await
            .map_err(|e| format!("接口请求失败: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("接口请求失败: {}, 响应码: {}", url, response.status()));
        }

        response.text()
            .await
            .map_err(|e| format!("读取响应失败: {}", e))
    }
}
