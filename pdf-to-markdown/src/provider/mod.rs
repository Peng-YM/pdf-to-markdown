pub mod traits;
mod zhipu;
pub mod paddleocr;

pub use traits::*;
pub use zhipu::ZhipuProvider;
pub use paddleocr::PaddleOcrProvider;

use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZhipuModel {
    Lite,
    Expert,
    Prime,
}

impl ZhipuModel {
    pub fn as_str(&self) -> &'static str {
        match self {
            ZhipuModel::Lite => "lite",
            ZhipuModel::Expert => "expert",
            ZhipuModel::Prime => "prime",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "lite" => Some(ZhipuModel::Lite),
            "expert" => Some(ZhipuModel::Expert),
            "prime" => Some(ZhipuModel::Prime),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderType {
    Zhipu(ZhipuModel),
    PaddleOcr,
}

impl ProviderType {
    pub fn from_str(s: &str) -> Option<Self> {
        let s_lower = s.to_lowercase();
        if s_lower == "paddleocr" {
            return Some(ProviderType::PaddleOcr);
        }
        
        if let Some(rest) = s_lower.strip_prefix("zhipu/") {
            if let Some(model) = ZhipuModel::from_str(rest) {
                return Some(ProviderType::Zhipu(model));
            }
        } else if s_lower == "zhipu" {
            return Some(ProviderType::Zhipu(ZhipuModel::Prime));
        }
        
        None
    }
    
    pub fn as_str(&self) -> String {
        match self {
            ProviderType::Zhipu(model) => format!("zhipu/{}", model.as_str()),
            ProviderType::PaddleOcr => "paddleocr".to_string(),
        }
    }
    
    pub fn default() -> Self {
        ProviderType::PaddleOcr
    }
}

pub fn create_provider(
    provider_type: ProviderType,
    api_key: String,
) -> Arc<dyn DocumentProvider> {
    match provider_type {
        ProviderType::Zhipu(_) => Arc::new(ZhipuProvider::new(api_key)),
        ProviderType::PaddleOcr => Arc::new(PaddleOcrProvider::new(api_key)),
    }
}
