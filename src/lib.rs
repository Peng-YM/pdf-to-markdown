/// Debug 打印宏，只在 DEBUG=1 环境变量设置时打印
#[macro_export]
macro_rules! debug_print {
    ($($arg:tt)*) => {
        if std::env::var("DEBUG").map(|v| v == "1").unwrap_or(false) {
            eprintln!($($arg)*);
        }
    };
}

pub mod cache;
pub mod converter;
pub mod error;
pub mod provider;
pub mod utils;

pub use cache::{CacheManager, CACHE_DISABLE_ENV_VAR};
pub use converter::Converter;
pub use error::{anyhow, Result};
pub use provider::{DocumentProvider, ParseResult, ProgressUpdate, ProviderType, ZhipuModel};
pub use utils::{download_pdf, is_url, normalize_arxiv_url, PdfMetadata};
