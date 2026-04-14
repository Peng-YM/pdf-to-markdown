use crate::cache::CacheManager;
use crate::error::Result;
use crate::provider::traits::*;
use crate::provider::ProviderType;
use crate::utils::PdfMetadata;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// 带缓存转换的参数
pub struct ConvertWithCacheOptions<'a> {
    pub input_path: &'a Path,
    pub input_identifier: &'a str,
    pub output_dir: &'a Path,
    pub config: &'a dyn ProviderConfig,
    pub provider_name: &'a str,
    pub page_ranges: Option<Vec<(u32, u32)>>,
}

pub struct Converter {
    provider: Arc<dyn DocumentProvider>,
    cache_manager: Option<CacheManager>,
}

impl Converter {
    pub fn new(provider_type: ProviderType, api_key: String) -> Self {
        let provider = crate::provider::create_provider(provider_type, api_key);
        let cache_manager = CacheManager::new().ok();
        Self { provider, cache_manager }
    }

    /// 带缓存的转换方法
    pub async fn convert_with_cache(
        &self,
        options: ConvertWithCacheOptions<'_>,
        progress_cb: impl FnMut(ProgressUpdate) + Send + 'static,
    ) -> Result<ParseResult> {
        let ConvertWithCacheOptions {
            input_path,
            input_identifier,
            output_dir,
            config,
            provider_name,
            page_ranges,
        } = options;
        // 首先尝试从缓存获取
        if let Some(cache_manager) = &self.cache_manager {
            // 计算哈希
            let hash = if crate::utils::is_url(input_identifier) {
                CacheManager::compute_url_hash(input_identifier)
            } else {
                CacheManager::compute_file_hash(input_path)?
            };

            // 检查缓存
            if let Some(cache_entry) = cache_manager.get(&hash, provider_name, &page_ranges)? {
                crate::debug_print!("Cache hit for: {}", input_identifier);

                // 从缓存恢复数据
                let mut result = ParseResult {
                    markdown: cache_entry.markdown,
                    images: HashMap::new(),
                    temp_dir: None,
                };

                // 恢复图片
                result.images = cache_manager.restore_images(&cache_entry.images, output_dir)?;

                // 确保输出目录存在
                std::fs::create_dir_all(output_dir)?;

                // 创建 images 目录
                let images_dir = output_dir.join("images");
                if !result.images.is_empty() {
                    std::fs::create_dir_all(&images_dir)?;
                }

                // 更新 Markdown 中的图片引用
                let mut markdown = result.markdown.clone();
                for img_name in result.images.keys() {
                    let relative_path = format!("images/{}", img_name);
                    let original_ref = img_name;
                    markdown = markdown.replace(original_ref, &relative_path);
                }

                let output_md_path = output_dir.join("doc.md");
                std::fs::write(output_md_path, &markdown)?;

                return Ok(result);
            }
        }

        // 缓存未命中或禁用，执行实际转换
        let result = self.convert(input_path, output_dir, config, progress_cb).await?;

        // 保存到缓存
        if let Some(cache_manager) = &self.cache_manager {
            let hash = if crate::utils::is_url(input_identifier) {
                CacheManager::compute_url_hash(input_identifier)
            } else {
                CacheManager::compute_file_hash(input_path)?
            };

            cache_manager.put(
                hash,
                input_identifier.to_string(),
                provider_name.to_string(),
                page_ranges,
                result.markdown.clone(),
                result.images.clone(),
            )?;
        }

        Ok(result)
    }

    pub async fn convert(
        &self,
        input_path: &Path,
        output_dir: &Path,
        config: &dyn ProviderConfig,
        progress_cb: impl FnMut(ProgressUpdate) + Send + 'static,
    ) -> Result<ParseResult> {
        let mut result =
            self.provider.parse_document(input_path, config, Box::new(progress_cb)).await?;

        if let Ok(metadata) = PdfMetadata::from_pdf(input_path) {
            let frontmatter = metadata.to_yaml_frontmatter();
            result.markdown = format!("{}{}", frontmatter, result.markdown);
        }

        // 确保输出目录存在
        std::fs::create_dir_all(output_dir)?;

        // 创建 images 目录
        let images_dir = output_dir.join("images");
        if !result.images.is_empty() {
            std::fs::create_dir_all(&images_dir)?;
        }

        // 复制图片到输出目录，并更新 Markdown 中的图片引用
        let mut markdown = result.markdown.clone();
        for (img_name, img_temp_path) in &result.images {
            let target_img_path = images_dir.join(img_name);
            std::fs::copy(img_temp_path, &target_img_path)?;

            // 更新 Markdown 中的图片引用：替换为相对路径
            let relative_path = format!("images/{}", img_name);
            let original_ref = img_name;
            markdown = markdown.replace(original_ref, &relative_path);
        }

        let output_md_path = output_dir.join("doc.md");
        std::fs::write(output_md_path, &markdown)?;

        Ok(result)
    }
}
