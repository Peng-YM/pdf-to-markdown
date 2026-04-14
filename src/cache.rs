use crate::error::{anyhow, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// 环境变量名称，用于临时禁用缓存
pub const CACHE_DISABLE_ENV_VAR: &str = "PDF_TO_MARKDOWN_NO_CACHE";

/// 检查缓存是否被禁用
pub fn is_cache_disabled() -> bool {
    std::env::var(CACHE_DISABLE_ENV_VAR)
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false)
}

/// 缓存条目，存储解析结果的元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// 文件哈希或URL哈希
    pub hash: String,
    /// 原始输入（文件路径或URL）
    pub input: String,
    /// 提供者类型
    pub provider: String,
    /// 页面范围（如果有）
    pub page_ranges: Option<Vec<(u32, u32)>>,
    /// Markdown 内容
    pub markdown: String,
    /// 图片映射（相对路径 -> 内容哈希）
    pub images: HashMap<String, String>,
    /// 创建时间戳
    pub created_at: u64,
}

impl CacheEntry {
    /// 创建新的缓存条目
    pub fn new(
        hash: String,
        input: String,
        provider: String,
        page_ranges: Option<Vec<(u32, u32)>>,
        markdown: String,
        images: HashMap<String, String>,
    ) -> Self {
        Self {
            hash,
            input,
            provider,
            page_ranges,
            markdown,
            images,
            created_at: chrono::Utc::now().timestamp() as u64,
        }
    }
}

/// 缓存管理器
pub struct CacheManager {
    /// 缓存目录
    cache_dir: PathBuf,
    /// 索引文件路径
    index_path: PathBuf,
    /// 图片缓存目录
    images_dir: PathBuf,
}

impl CacheManager {
    /// 创建新的缓存管理器
    pub fn new() -> Result<Self> {
        let project_dirs = ProjectDirs::from("", "", "pdf-to-markdown")
            .ok_or_else(|| anyhow!("Failed to determine project directories"))?;

        let cache_dir = project_dirs.cache_dir().to_path_buf();
        let index_path = cache_dir.join("index.json");
        let images_dir = cache_dir.join("images");

        fs::create_dir_all(&cache_dir)?;
        fs::create_dir_all(&images_dir)?;

        Ok(Self { cache_dir, index_path, images_dir })
    }

    /// 加载缓存索引
    fn load_index(&self) -> Result<HashMap<String, CacheEntry>> {
        if !self.index_path.exists() {
            return Ok(HashMap::new());
        }

        let content = fs::read_to_string(&self.index_path)?;
        let index = serde_json::from_str(&content)?;
        Ok(index)
    }

    /// 保存缓存索引
    fn save_index(&self, index: &HashMap<String, CacheEntry>) -> Result<()> {
        let content = serde_json::to_string_pretty(index)?;
        fs::write(&self.index_path, content)?;
        Ok(())
    }

    /// 计算文件的 SHA256 哈希
    pub fn compute_file_hash(path: &Path) -> Result<String> {
        let content = fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(content);
        let hash = hasher.finalize();
        Ok(format!("{:x}", hash))
    }

    /// 计算 URL 的哈希
    pub fn compute_url_hash(url: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        let hash = hasher.finalize();
        format!("{:x}", hash)
    }

    /// 生成缓存键
    pub fn generate_cache_key(
        hash: &str,
        provider: &str,
        page_ranges: &Option<Vec<(u32, u32)>>,
    ) -> String {
        let mut key = format!("{}:{}", hash, provider);

        if let Some(ranges) = page_ranges {
            let ranges_str = ranges
                .iter()
                .map(|(start, end)| format!("{}-{}", start, end))
                .collect::<Vec<_>>()
                .join(",");
            key.push_str(&format!(":{}", ranges_str));
        }

        key
    }

    /// 从缓存中获取条目
    pub fn get(
        &self,
        hash: &str,
        provider: &str,
        page_ranges: &Option<Vec<(u32, u32)>>,
    ) -> Result<Option<CacheEntry>> {
        if is_cache_disabled() {
            return Ok(None);
        }

        let index = self.load_index()?;
        let key = Self::generate_cache_key(hash, provider, page_ranges);
        Ok(index.get(&key).cloned())
    }

    /// 保存条目到缓存
    pub fn put(
        &self,
        hash: String,
        input: String,
        provider: String,
        page_ranges: Option<Vec<(u32, u32)>>,
        markdown: String,
        images: HashMap<String, PathBuf>,
    ) -> Result<()> {
        if is_cache_disabled() {
            return Ok(());
        }

        let mut image_hashes = HashMap::new();

        // 保存图片并计算哈希
        for (name, path) in images {
            let content = fs::read(&path)?;
            let mut hasher = Sha256::new();
            hasher.update(&content);
            let image_hash = format!("{:x}", hasher.finalize());

            let image_path = self.images_dir.join(&image_hash);
            fs::write(&image_path, content)?;

            image_hashes.insert(name, image_hash);
        }

        let entry = CacheEntry::new(
            hash.clone(),
            input,
            provider.clone(),
            page_ranges.clone(),
            markdown,
            image_hashes,
        );

        let mut index = self.load_index()?;
        let key = Self::generate_cache_key(&hash, &provider, &page_ranges);
        index.insert(key, entry);
        self.save_index(&index)?;

        Ok(())
    }

    /// 从缓存恢复图片到指定目录
    pub fn restore_images(
        &self,
        images: &HashMap<String, String>,
        output_dir: &Path,
    ) -> Result<HashMap<String, PathBuf>> {
        let mut result = HashMap::new();
        let images_output_dir = output_dir.join("images");
        fs::create_dir_all(&images_output_dir)?;

        for (name, image_hash) in images {
            let source_path = self.images_dir.join(image_hash);
            if !source_path.exists() {
                continue;
            }

            let dest_path = images_output_dir.join(name);
            fs::copy(&source_path, &dest_path)?;
            result.insert(name.clone(), dest_path);
        }

        Ok(result)
    }

    /// 清除所有缓存
    pub fn clear(&self) -> Result<()> {
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)?;
            fs::create_dir_all(&self.cache_dir)?;
            fs::create_dir_all(&self.images_dir)?;
        }
        Ok(())
    }

    /// 获取缓存大小信息
    pub fn cache_size(&self) -> Result<(usize, u64)> {
        let index = self.load_index()?;
        let mut total_size = 0;

        // 计算索引文件大小
        if self.index_path.exists() {
            total_size += fs::metadata(&self.index_path)?.len();
        }

        // 计算图片大小
        if self.images_dir.exists() {
            for entry in walkdir::WalkDir::new(&self.images_dir) {
                let entry = entry?;
                if entry.file_type().is_file() {
                    total_size += entry.metadata()?.len();
                }
            }
        }

        Ok((index.len(), total_size))
    }
}
