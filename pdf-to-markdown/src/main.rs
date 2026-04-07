#![allow(clippy::ptr_arg, clippy::too_many_arguments)]

use clap::Parser;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use pdf_to_markdown::error::{anyhow, Result};
use pdf_to_markdown::provider::traits::*;
use pdf_to_markdown::provider::ProviderType;
use pdf_to_markdown::utils::PdfMetadata;
use pdf_to_markdown::Converter;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex};

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
#[command(version = "0.3.0")]
#[command(after_help = "\
EXAMPLES:
    # Get PDF metadata and table of contents
    pdf-to-markdown metadata document.pdf
    
    # Get metadata as JSON
    pdf-to-markdown metadata document.pdf --json
    
    # Parse entire PDF to Markdown using default provider (paddleocr)
    pdf-to-markdown parse document.pdf
    
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
")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Parser, Debug)]
enum Commands {
    /// Get PDF metadata (title, author, table of contents, etc.)
    Metadata {
        /// Input PDF file path
        #[arg(value_name = "PDF_FILE")]
        input: PathBuf,

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
        /// Input PDF file path
        #[arg(value_name = "PDF_FILE")]
        input: PathBuf,

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

        /// PaddleOCR: Use document orientation classification
        #[arg(long)]
        use_doc_orientation_classify: bool,

        /// PaddleOCR: Use document unwarping
        #[arg(long)]
        use_doc_unwarping: bool,

        /// PaddleOCR: Use layout detection
        #[arg(long)]
        use_layout_detection: bool,

        /// PaddleOCR: Use chart recognition
        #[arg(long)]
        use_chart_recognition: bool,

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
            use_doc_orientation_classify,
            use_doc_unwarping,
            use_layout_detection,
            use_chart_recognition,
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
                use_doc_orientation_classify,
                use_doc_unwarping,
                use_layout_detection,
                use_chart_recognition,
                json,
                quiet,
                dry_run,
                overwrite,
            )
            .await
        }
    }
}

async fn handle_metadata(
    input: &PathBuf,
    output: Option<&Path>,
    json: bool,
    quiet: bool,
) -> Result<()> {
    if !input.exists() {
        if json {
            let error_json = ErrorJson {
                success: false,
                error_code: ExitCode::ResourceNotFound as i32,
                error_type: "resource_not_found".to_string(),
                message: format!("Input file does not exist: {}", input.display()),
                suggestion: Some(
                    "Check that the file path is correct and the file exists".to_string(),
                ),
            };
            println!("{}", serde_json::to_string_pretty(&error_json)?);
            std::process::exit(ExitCode::ResourceNotFound as i32);
        }
        return Err(anyhow!("Input file does not exist: {}", input.display()));
    }

    let metadata = PdfMetadata::from_pdf(input)?;

    if json {
        let json_output = serde_json::to_string_pretty(&metadata)?;
        println!("{}", json_output);
    } else if !quiet {
        println!("{}", format!("📄 Extracting metadata from: {}", input.display()).cyan());
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
    input: &PathBuf,
    output_dir: Option<&Path>,
    pages: Option<&str>,
    provider: Option<&str>,
    api_key: Option<&str>,
    use_doc_orientation_classify: bool,
    use_doc_unwarping: bool,
    use_layout_detection: bool,
    use_chart_recognition: bool,
    json: bool,
    quiet: bool,
    dry_run: bool,
    overwrite: bool,
) -> Result<()> {
    use pdf_to_markdown::provider::paddleocr::PaddleOcrConfig;

    if !input.exists() {
        if json {
            let error_json = ErrorJson {
                success: false,
                error_code: ExitCode::ResourceNotFound as i32,
                error_type: "resource_not_found".to_string(),
                message: format!("Input file does not exist: {}", input.display()),
                suggestion: Some(
                    "Check that the file path is correct and the file exists".to_string(),
                ),
            };
            println!("{}", serde_json::to_string_pretty(&error_json)?);
            std::process::exit(ExitCode::ResourceNotFound as i32);
        }
        return Err(anyhow!("Input file does not exist: {}", input.display()));
    }

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
                input_file: input.display().to_string(),
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
            println!("  {}", format!("Input: {}", input.display()).cyan());
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
        println!("{}", format!("📄 Input: {}", input.display()).cyan());
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

    let pb_clone = pb.clone();
    let result = match provider_type_clone {
        ProviderType::Zhipu(model) => {
            let config = ZhipuConfig {
                tool_type: model.as_str().to_string(),
                max_retries: 60,
                poll_interval_secs: 3,
                page_ranges,
            };
            converter
                .convert(input, &output_dir, &config, move |update| {
                    let pb = pb_clone.lock().unwrap();
                    pb.set_message(update.message);
                    if let Some(total) = update.total {
                        pb.set_length(total);
                    }
                    pb.set_position(update.current);
                })
                .await
        }
        ProviderType::PaddleOcr => {
            let config = PaddleOcrConfig {
                page_ranges,
                use_doc_orientation_classify: if use_doc_orientation_classify {
                    Some(true)
                } else {
                    None
                },
                use_doc_unwarping: if use_doc_unwarping { Some(true) } else { None },
                use_layout_detection: if use_layout_detection { Some(true) } else { None },
                use_chart_recognition: if use_chart_recognition { Some(true) } else { None },
            };
            converter
                .convert(input, &output_dir, &config, move |update| {
                    let pb = pb_clone.lock().unwrap();
                    pb.set_message(update.message);
                })
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
            input_file: input.display().to_string(),
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
