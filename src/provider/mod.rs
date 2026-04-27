pub mod mineru;
pub mod paddleocr;
pub mod traits;
mod zhipu;

pub use mineru::{MinerUConfig, MinerUModel, MinerUProvider};
pub use paddleocr::{PaddleOcrConfig, PaddleOcrProvider};
pub use traits::*;
pub use zhipu::ZhipuProvider;

use std::str::FromStr;
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
}

impl FromStr for ZhipuModel {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "lite" => Ok(ZhipuModel::Lite),
            "expert" => Ok(ZhipuModel::Expert),
            "prime" => Ok(ZhipuModel::Prime),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ProviderType {
    Zhipu(ZhipuModel),
    #[default]
    PaddleOcr,
    MinerU(MinerUModel),
}

impl ProviderType {
    pub fn as_str(&self) -> String {
        match self {
            ProviderType::Zhipu(model) => format!("zhipu/{}", model.as_str()),
            ProviderType::PaddleOcr => "paddleocr".to_string(),
            ProviderType::MinerU(model) => format!("mineru/{}", model.as_str()),
        }
    }
}

impl FromStr for ProviderType {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let s_lower = s.to_lowercase();
        if s_lower == "paddleocr" {
            return Ok(ProviderType::PaddleOcr);
        }

        if let Some(rest) = s_lower.strip_prefix("zhipu/") {
            if let Ok(model) = ZhipuModel::from_str(rest) {
                return Ok(ProviderType::Zhipu(model));
            }
        } else if s_lower == "zhipu" {
            return Ok(ProviderType::Zhipu(ZhipuModel::Prime));
        }

        if let Some(rest) = s_lower.strip_prefix("mineru/") {
            if let Ok(model) = MinerUModel::from_str(rest) {
                return Ok(ProviderType::MinerU(model));
            }
        } else if s_lower == "mineru" {
            return Ok(ProviderType::MinerU(MinerUModel::Vlm));
        }

        Err(())
    }
}

impl FromStr for MinerUModel {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "vlm" => Ok(MinerUModel::Vlm),
            "pipeline" => Ok(MinerUModel::Pipeline),
            "agent" => Ok(MinerUModel::Agent),
            _ => Err(()),
        }
    }
}

pub fn create_provider(provider_type: ProviderType, api_key: String) -> Arc<dyn DocumentProvider> {
    match provider_type {
        ProviderType::Zhipu(_) => Arc::new(ZhipuProvider::new(api_key)),
        ProviderType::PaddleOcr => Arc::new(PaddleOcrProvider::new(api_key)),
        ProviderType::MinerU(_) => Arc::new(MinerUProvider::new(api_key)),
    }
}
