use crate::error::Result;
use crate::provider::traits::*;
use crate::provider::ProviderType;
use crate::utils::PdfMetadata;
use std::path::Path;
use std::sync::Arc;

pub struct Converter {
    provider: Arc<dyn DocumentProvider>,
}

impl Converter {
    pub fn new(provider_type: ProviderType, api_key: String) -> Self {
        let provider = crate::provider::create_provider(provider_type, api_key);
        Self { provider }
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
