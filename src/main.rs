#![allow(clippy::ptr_arg, clippy::too_many_arguments)]

use clap::Parser;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use pdf_to_markdown::cache::{CacheManager, CACHE_DISABLE_ENV_VAR};
use pdf_to_markdown::error::{anyhow, Result};
use pdf_to_markdown::provider::traits::*;
use pdf_to_markdown::provider::ProviderType;
use pdf_to_markdown::utils::{download_pdf, is_url, normalize_arxiv_url, PdfMetadata};
use pdf_to_markdown::Converter;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use tempfile::NamedTempFile;

/// Exit codes for the CLI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
enum ExitCode {
    Success = 0,
    GeneralFailure = 1,
    UsageError = 2,
    ResourceNotFound = 3,
    #[allow(dead_code)]
    PermissionDenied = 4,
    Conflict = 5,
}

#[derive(Serialize)]
struct ErrorJson {
    success: bool,
    error_code: i32,
    error_type: String,
    message: String,
    suggestion: Option<String>,
}

#[derive(Serialize)]
struct ParseResultJson {
    success: bool,
    input_file: String,
    output_dir: String,
    markdown_file: String,
    images_dir: String,
    line_count: usize,
    char_count: usize,
    image_count: usize,
    provider: String,
}

#[derive(Serialize)]
struct DryRunResultJson {
    dry_run: bool,
    input_file: String,
    output_dir: String,
    markdown_file: String,
    images_dir: String,
    provider: String,
    pages: Option<String>,
    would_create: Vec<String>,
}

#[derive(Parser, Debug)]
#[command(name = "pdf-to-markdown")]
#[command(about = "PDF to Markdown converter with progressive information disclosure")]
#[command(version = "0.4.0")]
#[command(after_help = "\
EXAMPLES:
    # Get PDF metadata and table of contents (local file)
    pdf-to-markdown metadata document.pdf
    
    # Get PDF metadata from URL
    pdf-to-markdown metadata https://example.com/document.pdf
    
    # Get PDF metadata from arxiv (auto-converts abs link to pdf)
    pdf-to-markdown metadata https://arxiv.org/abs/2301.07041
    
    # Get metadata as JSON
    pdf-to-markdown metadata document.pdf --json
    
    # Parse entire PDF to Markdown using default provider (local file)
    pdf-to-markdown parse document.pdf
    
    # Parse PDF from URL
    pdf-to-markdown parse https://example.com/document.pdf
    
    # Parse PDF from arxiv (auto-converts abs link to pdf)
    pdf-to-markdown parse https://arxiv.org/abs/2301.07041
    
    # Parse to specific output directory
    pdf-to-markdown parse document.pdf -o ./output/
    
    # Parse specific pages (e.g., pages 1-5 and 10)
    pdf-to-markdown parse document.pdf --pages 1-5,10 -o ./output/
    
    # Use Zhipu lite model
    pdf-to-markdown parse --provider zhipu/lite document.pdf -o ./output/
    
    # Use Zhipu expert model
    pdf-to-markdown parse --provider zhipu/expert document.pdf -o ./output/
    
    # Use Zhipu prime model
    pdf-to-markdown parse --provider zhipu/prime document.pdf -o ./output/
    
    # Dry run: preview what would happen
    pdf-to-markdown parse document.pdf -o ./output/ --dry-run
    
    # Output result as JSON
    pdf-to-markdown parse document.pdf -o ./output/ --json
    
    # Quiet mode: only output the markdown file path
    pdf-to-markdown parse document.pdf -o ./output/ --quiet
    
    # Overwrite existing output files
    pdf-to-markdown parse document.pdf -o ./output/ --overwrite
    
    # Disable cache temporarily
    PDF_TO_MARKDOWN_NO_CACHE=1 pdf-to-markdown parse document.pdf
    
    # Show cache status
    pdf-to-markdown cache status
    
    # Clear cache
    pdf-to-markdown cache clear
    
    # Clear cache without confirmation
    pdf-to-markdown cache clear --force
")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Parser, Debug)]
enum Commands {
    /// Get PDF metadata (title, author, table of contents, etc.)
    Metadata {
        /// Input PDF file path or URL
        #[arg(value_name = "PDF_FILE_OR_URL")]
        input: String,

        /// Output metadata to JSON file (optional)
        #[arg(short, long, value_name = "JSON_FILE")]
        output: Option<PathBuf>,

        /// Output result as JSON to stdout
        #[arg(long)]
        json: bool,

        /// Quiet mode: only output essential information
        #[arg(short, long)]
        quiet: bool,
    },

    /// Parse PDF to Markdown using AI provider
    Parse {
        /// Input PDF file path or URL
        #[arg(value_name = "PDF_FILE_OR_URL")]
        input: String,

        /// Output directory (default: current directory)
        #[arg(short, long, value_name = "OUTPUT_DIR")]
        output_dir: Option<PathBuf>,

        /// Page ranges to parse (e.g., 1-5,10,15-20)
        #[arg(long, value_name = "PAGES")]
        pages: Option<String>,

        /// Provider: paddleocr, zhipu/lite, zhipu/expert, zhipu/prime (default: paddleocr)
        #[arg(long, value_name = "PROVIDER")]
        provider: Option<String>,

        /// API Key (can also be set via ZHIPU_API_KEY, PADDLE_OCR_API_KEY, or PROVIDER_API_KEY environment variable)
        #[arg(short = 'k', long, value_name = "API_KEY")]
        api_key: Option<String>,

        /// Output result as JSON to stdout
        #[arg(long)]
        json: bool,

        /// Quiet mode: only output essential information
        #[arg(short, long)]
        quiet: bool,

        /// Dry run: preview what would happen without executing
        #[arg(long)]
        dry_run: bool,

        /// Overwrite existing output files without confirmation
        #[arg(long)]
        overwrite: bool,
    },

    /// Cache management commands
    Cache {
        #[command(subcommand)]
        cache_command: CacheCommands,
    },
}

#[derive(Parser, Debug)]
enum CacheCommands {
    /// Show cache status (number of entries, size, etc.)
    Status {
        /// Output result as JSON to stdout
        #[arg(long)]
        json: bool,
    },
    /// Clear all cache entries
    Clear {
        /// Force clear without confirmation
        #[arg(short, long)]
        force: bool,
    },
}

/// Wrapper for main to handle exit codes
#[tokio::main]
async fn main() {
    let exit_code = match run().await {
        Ok(_) => ExitCode::Success as i32,
        Err(e) => {
            eprintln!("{}", format!("Error: {}", e).red());
            ExitCode::GeneralFailure as i32
        }
    };
    std::process::exit(exit_code);
}

async fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Metadata { input, output, json, quiet } => {
            handle_metadata(&input, output.as_deref(), json, quiet).await
        }
        Commands::Parse {
            input,
            output_dir,
            pages,
            provider,
            api_key,

            json,
            quiet,
            dry_run,
            overwrite,
        } => {
            handle_parse(
                &input,
                output_dir.as_deref(),
                pages.as_deref(),
                provider.as_deref(),
                api_key.as_deref(),
                json,
                quiet,
                dry_run,
                overwrite,
            )
            .await
        }
        Commands::Cache { cache_command } => handle_cache(cache_command).await,
    }
}

/// Helper to get a PDF path, downloading from URL if needed
async fn get_pdf_path(
    input: &str,
    quiet: bool,
    json: bool,
) -> Result<(PathBuf, Option<NamedTempFile>)> {
    if is_url(input) {
        let normalized_url = normalize_arxiv_url(input);
        if !quiet && !json {
            if normalized_url != input {
                println!("{}", format!("🔄 Converting arxiv abs link: {}", input).cyan());
                println!("{}", format!("📥 Downloading from: {}", normalized_url).cyan());
            } else {
                println!("{}", format!("📥 Downloading PDF from: {}", normalized_url).cyan());
            }
        }
        let temp_file = download_pdf(&normalized_url).await?;
        let path = temp_file.path().to_path_buf();
        Ok((path, Some(temp_file)))
    } else {
        let path = PathBuf::from(input);
        if !path.exists() {
            return Err(anyhow!("Input file does not exist: {}", path.display()));
        }
        Ok((path, None))
    }
}

async fn handle_metadata(
    input: &str,
    output: Option<&Path>,
    json: bool,
    quiet: bool,
) -> Result<()> {
    let (pdf_path, _temp_file) = match get_pdf_path(input, quiet, json).await {
        Ok(p) => p,
        Err(e) => {
            if json {
                let error_json = ErrorJson {
                    success: false,
                    error_code: ExitCode::ResourceNotFound as i32,
                    error_type: "resource_not_found".to_string(),
                    message: format!("{}", e),
                    suggestion: Some("Check that the file path or URL is correct".to_string()),
                };
                println!("{}", serde_json::to_string_pretty(&error_json)?);
                std::process::exit(ExitCode::ResourceNotFound as i32);
            }
            return Err(e);
        }
    };

    let metadata = PdfMetadata::from_pdf(&pdf_path)?;

    if json {
        let json_output = serde_json::to_string_pretty(&metadata)?;
        println!("{}", json_output);
    } else if !quiet {
        println!("{}", format!("📄 Extracting metadata from: {}", input).cyan());
        println!();
        print_metadata(&metadata);
    }

    if let Some(output_path) = output {
        let json_content = serde_json::to_string_pretty(&metadata)?;
        std::fs::write(output_path, json_content)?;
        if !quiet && !json {
            println!();
            println!("{}", format!("✅ Metadata saved to: {}", output_path.display()).green());
        }
    }

    Ok(())
}

fn print_metadata(metadata: &PdfMetadata) {
    println!("{}", "📋 PDF Metadata".bold());
    println!("{}", "─".repeat(50));

    if let Some(title) = &metadata.title {
        println!("  {}: {}", "Title".bold(), title);
    }
    if let Some(author) = &metadata.author {
        println!("  {}: {}", "Author".bold(), author);
    }
    if let Some(subject) = &metadata.subject {
        println!("  {}: {}", "Subject".bold(), subject);
    }
    if let Some(keywords) = &metadata.keywords {
        println!("  {}: {}", "Keywords".bold(), keywords);
    }
    if let Some(creator) = &metadata.creator {
        println!("  {}: {}", "Creator".bold(), creator);
    }
    if let Some(producer) = &metadata.producer {
        println!("  {}: {}", "Producer".bold(), producer);
    }
    if let Some(created) = &metadata.created {
        println!("  {}: {}", "Created".bold(), created);
    }
    if let Some(modified) = &metadata.modified {
        println!("  {}: {}", "Modified".bold(), modified);
    }
    if let Some(page_count) = &metadata.page_count {
        println!("  {}: {}", "Pages".bold(), page_count);
    }

    if !metadata.table_of_contents.is_empty() {
        println!();
        println!("{}", "📑 Table of Contents".bold());
        println!("{}", "─".repeat(50));

        for entry in &metadata.table_of_contents {
            let indent = "  ".repeat(entry.level as usize);
            let page_str = entry.page.map(|p| format!(" (p. {})", p)).unwrap_or_default();
            println!("{}- {}{}", indent, entry.title, page_str);
        }
    }
}

async fn handle_parse(
    input: &str,
    output_dir: Option<&Path>,
    pages: Option<&str>,
    provider: Option<&str>,
    api_key: Option<&str>,
    json: bool,
    quiet: bool,
    dry_run: bool,
    overwrite: bool,
) -> Result<()> {
    use pdf_to_markdown::provider::paddleocr::PaddleOcrConfig;

    // For dry run, we don't need to download the file
    let (pdf_path, _temp_file) = if dry_run {
        if is_url(input) {
            // Just use a dummy path for dry run with URL
            (PathBuf::from("url.pdf"), None)
        } else {
            let path = PathBuf::from(input);
            if !path.exists() {
                if json {
                    let error_json = ErrorJson {
                        success: false,
                        error_code: ExitCode::ResourceNotFound as i32,
                        error_type: "resource_not_found".to_string(),
                        message: format!("Input file does not exist: {}", path.display()),
                        suggestion: Some(
                            "Check that the file path is correct and the file exists".to_string(),
                        ),
                    };
                    println!("{}", serde_json::to_string_pretty(&error_json)?);
                    std::process::exit(ExitCode::ResourceNotFound as i32);
                }
                return Err(anyhow!("Input file does not exist: {}", path.display()));
            }
            (path, None)
        }
    } else {
        match get_pdf_path(input, quiet, json).await {
            Ok(p) => p,
            Err(e) => {
                if json {
                    let error_json = ErrorJson {
                        success: false,
                        error_code: ExitCode::ResourceNotFound as i32,
                        error_type: "resource_not_found".to_string(),
                        message: format!("{}", e),
                        suggestion: Some("Check that the file path or URL is correct".to_string()),
                    };
                    println!("{}", serde_json::to_string_pretty(&error_json)?);
                    std::process::exit(ExitCode::ResourceNotFound as i32);
                }
                return Err(e);
            }
        }
    };

    let provider_type = if let Some(provider_str) = provider {
        ProviderType::from_str(provider_str)
            .map_err(|_| {
                if json {
                    let error_json = ErrorJson {
                        success: false,
                        error_code: ExitCode::UsageError as i32,
                        error_type: "usage_error".to_string(),
                        message: format!("Unsupported provider: {}", provider_str),
                        suggestion: Some("Supported providers: paddleocr, zhipu/lite, zhipu/expert, zhipu/prime".to_string()),
                    };
                    let _ = serde_json::to_string_pretty(&error_json).map(|s| println!("{}", s));
                    std::process::exit(ExitCode::UsageError as i32);
                }
                anyhow!(
                    "Unsupported provider: {}. Supported providers: paddleocr, zhipu/lite, zhipu/expert, zhipu/prime",
                    provider_str
                )
            })?
    } else {
        ProviderType::default()
    };

    let output_dir = output_dir.unwrap_or_else(|| Path::new(".")).to_path_buf();
    let output_md_path = output_dir.join("doc.md");
    let output_images_dir = output_dir.join("images");

    // Dry run handling
    if dry_run {
        let would_create = vec![
            output_dir.display().to_string(),
            output_md_path.display().to_string(),
            output_images_dir.display().to_string(),
        ];

        if json {
            let dry_run_result = DryRunResultJson {
                dry_run: true,
                input_file: input.to_string(),
                output_dir: output_dir.display().to_string(),
                markdown_file: output_md_path.display().to_string(),
                images_dir: output_images_dir.display().to_string(),
                provider: provider_type.as_str(),
                pages: pages.map(|s| s.to_string()),
                would_create,
            };
            println!("{}", serde_json::to_string_pretty(&dry_run_result)?);
        } else if !quiet {
            println!("{}", "📋 Dry Run: What would happen".bold());
            println!("{}", "─".repeat(50));
            println!("  {}", format!("Input: {}", input).cyan());
            println!("  {}", format!("Output dir: {}", output_dir.display()).cyan());
            println!("  {}", format!("Provider: {}", provider_type.as_str()).cyan());
            if let Some(pages) = pages {
                println!("  {}", format!("Pages: {}", pages).cyan());
            }
            println!();
            println!("  {}", "Would create:".bold());
            for path in would_create {
                println!("    - {}", path);
            }
        }
        return Ok(());
    }

    // Check for existing output without overwrite flag
    if !overwrite && (output_md_path.exists() || output_images_dir.exists()) {
        if json {
            let error_json = ErrorJson {
                success: false,
                error_code: ExitCode::Conflict as i32,
                error_type: "conflict".to_string(),
                message: "Output files already exist".to_string(),
                suggestion: Some("Use --overwrite to overwrite existing files, or choose a different output directory".to_string()),
            };
            println!("{}", serde_json::to_string_pretty(&error_json)?);
            std::process::exit(ExitCode::Conflict as i32);
        }
        return Err(anyhow!("Output files already exist. Use --overwrite to overwrite them."));
    }

    // 调试：打印环境变量读取情况
    pdf_to_markdown::debug_print!("DEBUG: Checking API keys...");
    pdf_to_markdown::debug_print!(
        "DEBUG: PADDLE_OCR_API_KEY exists? {}",
        std::env::var("PADDLE_OCR_API_KEY").is_ok()
    );
    pdf_to_markdown::debug_print!(
        "DEBUG: ZHIPU_API_KEY exists? {}",
        std::env::var("ZHIPU_API_KEY").is_ok()
    );
    pdf_to_markdown::debug_print!(
        "DEBUG: PROVIDER_API_KEY exists? {}",
        std::env::var("PROVIDER_API_KEY").is_ok()
    );

    let api_key = api_key
        .map(|s| s.to_string())
        .or_else(|| {
            match provider_type {
                ProviderType::Zhipu(_) => std::env::var("ZHIPU_API_KEY").ok(),
                ProviderType::PaddleOcr => std::env::var("PADDLE_OCR_API_KEY").ok(),
            }
        })
        .or_else(|| std::env::var("PROVIDER_API_KEY").ok())
        .ok_or_else(|| {
            if json {
                let error_json = ErrorJson {
                    success: false,
                    error_code: ExitCode::UsageError as i32,
                    error_type: "usage_error".to_string(),
                    message: "API key must be provided".to_string(),
                    suggestion: Some("Use --api-key flag or set ZHIPU_API_KEY, PADDLE_OCR_API_KEY, or PROVIDER_API_KEY environment variable".to_string()),
                };
                let _ = serde_json::to_string_pretty(&error_json).map(|s| println!("{}", s));
                std::process::exit(ExitCode::UsageError as i32);
            }
            anyhow!("API key must be provided via --api-key or provider-specific environment variable")
        })?;

    std::fs::create_dir_all(&output_dir)?;
    std::fs::create_dir_all(&output_images_dir)?;

    if !quiet && !json {
        println!("{}", format!("📄 Input: {}", input).cyan());
        println!("{}", format!("📁 Output dir: {}", output_dir.display()).cyan());

        let provider_display = match &provider_type {
            ProviderType::Zhipu(model) => format!("Zhipu ({})", model.as_str()),
            ProviderType::PaddleOcr => "PaddleOCR".to_string(),
        };
        println!("{}", format!("🔧 Provider: {}", provider_display).cyan());

        if let Some(pages) = pages {
            println!("{}", format!("📄 Pages: {}", pages).cyan());
        }
    }

    // 保存 provider_type 的克隆用于后续匹配
    let provider_type_clone = provider_type.clone();

    let converter = Converter::new(provider_type, api_key);

    let pb = Arc::new(Mutex::new(ProgressBar::new(100)));
    pb.lock().unwrap().set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {msg}")?
            .progress_chars("#>-"),
    );

    // Save provider string representation before possible move
    let provider_str = provider_type_clone.as_str();

    let page_ranges = if let Some(pages_str) = pages {
        let mut ranges = Vec::new();
        for part in pages_str.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            if part.contains('-') {
                let parts: Vec<_> = part.split('-').collect();
                if parts.len() == 2 {
                    let start = parts[0].trim().parse::<u32>()?;
                    let end = parts[1].trim().parse::<u32>()?;
                    ranges.push((start, end));
                }
            } else {
                let page = part.parse::<u32>()?;
                ranges.push((page, page));
            }
        }
        Some(ranges)
    } else {
        None
    };
    let page_ranges_clone = page_ranges.clone();

    let pb_clone = pb.clone();
    let result = match provider_type_clone {
        ProviderType::Zhipu(model) => {
            let config = ZhipuConfig {
                tool_type: model.as_str().to_string(),
                max_retries: 60,
                poll_interval_secs: 3,
                page_ranges: page_ranges_clone.clone(),
            };
            converter
                .convert_with_cache(
                    &pdf_path,
                    input,
                    &output_dir,
                    &config,
                    &provider_str,
                    page_ranges,
                    move |update| {
                        let pb = pb_clone.lock().unwrap();
                        pb.set_message(update.message);
                        if let Some(total) = update.total {
                            pb.set_length(total);
                        }
                        pb.set_position(update.current);
                    },
                )
                .await
        }
        ProviderType::PaddleOcr => {
            let config = PaddleOcrConfig {
                page_ranges: page_ranges_clone.clone(),
                // 使用默认配置：打开布局检查，关闭图片方向矫正和扭曲矫正
                ..Default::default()
            };
            converter
                .convert_with_cache(
                    &pdf_path,
                    input,
                    &output_dir,
                    &config,
                    &provider_str,
                    page_ranges,
                    move |update| {
                        let pb = pb_clone.lock().unwrap();
                        pb.set_message(update.message);
                    },
                )
                .await
        }
    }?;

    pb.lock().unwrap().finish_and_clear();

    let line_count = result.markdown.lines().count();
    let char_count = result.markdown.chars().count();
    let image_count = result.images.len();

    if json {
        let parse_result = ParseResultJson {
            success: true,
            input_file: input.to_string(),
            output_dir: output_dir.display().to_string(),
            markdown_file: output_md_path.display().to_string(),
            images_dir: output_images_dir.display().to_string(),
            line_count,
            char_count,
            image_count,
            provider: provider_str,
        };
        println!("{}", serde_json::to_string_pretty(&parse_result)?);
    } else if quiet {
        println!("{}", output_md_path.display());
    } else {
        println!("{}", "✅ Conversion completed successfully!".green());
        println!("{}", format!("✅ Markdown saved to: {}", output_md_path.display()).green());
        println!(
            "{}",
            format!("📊 Statistics: {} lines, {} characters", line_count, char_count).cyan()
        );

        if !result.images.is_empty() {
            println!(
                "{}",
                format!("🖼️  Extracted {} images to {}", image_count, output_images_dir.display())
                    .cyan()
            );
        }
    }

    Ok(())
}

async fn handle_cache(cache_command: CacheCommands) -> Result<()> {
    let cache_manager = CacheManager::new()?;

    match cache_command {
        CacheCommands::Status { json } => {
            let (entry_count, total_size) = cache_manager.cache_size()?;

            if json {
                let status = serde_json::json!({
                    "success": true,
                    "entry_count": entry_count,
                    "total_size_bytes": total_size,
                    "total_size_human": format_size(total_size),
                    "cache_disabled": std::env::var(CACHE_DISABLE_ENV_VAR).map(|v| v == "1" || v.to_lowercase() == "true").unwrap_or(false),
                });
                println!("{}", serde_json::to_string_pretty(&status)?);
            } else {
                println!("{}", "📦 Cache Status".bold());
                println!("{}", "─".repeat(50));
                println!("  {}: {}", "Entries".bold(), entry_count);
                println!("  {}: {}", "Size".bold(), format_size(total_size));
                println!(
                    "  {}: {}",
                    "Disabled".bold(),
                    if std::env::var(CACHE_DISABLE_ENV_VAR)
                        .map(|v| v == "1" || v.to_lowercase() == "true")
                        .unwrap_or(false)
                    {
                        "Yes"
                    } else {
                        "No"
                    }
                );
            }
        }
        CacheCommands::Clear { force } => {
            if !force {
                let (entry_count, total_size) = cache_manager.cache_size()?;
                println!("{}", "⚠️  Clear Cache Confirmation".yellow().bold());
                println!("{}", "─".repeat(50));
                println!("  This will delete:");
                println!("    - {} cache entries", entry_count);
                println!("    - {} of data", format_size(total_size));
                println!();

                let confirm = ask_user::ask_user(
                    "Are you sure you want to clear the cache?",
                    &["Yes, clear it", "No, cancel"],
                )?;

                if confirm != "Yes, clear it" {
                    println!("{}", "✅ Cache clear cancelled".green());
                    return Ok(());
                }
            }

            cache_manager.clear()?;
            println!("{}", "✅ Cache cleared successfully".green());
        }
    }

    Ok(())
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

// Simple helper for user confirmation
mod ask_user {
    use std::io::{self, Write};

    pub fn ask_user(question: &str, options: &[&str]) -> super::Result<String> {
        println!("{}", question);
        for (i, option) in options.iter().enumerate() {
            println!("  {}. {}", i + 1, option);
        }

        print!("Your choice: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let choice = input.trim().parse::<usize>()?;
        if choice < 1 || choice > options.len() {
            return Err(super::anyhow!("Invalid choice"));
        }

        Ok(options[choice - 1].to_string())
    }
}
