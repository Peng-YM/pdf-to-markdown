# Troubleshooting

## "API key must be provided"

The tool cannot find a valid API key. Resolve by one of:

1. Run `pdf-to-markdown login` to store a key in the encrypted credentials file
2. Set a provider-specific env var: `export PADDLE_OCR_API_KEY="your-key"`, `export ZHIPU_API_KEY="your-key"`, or `export MINERU_API_KEY="your-key"`
3. Set the generic env var: `export PROVIDER_API_KEY="your-key"`
4. Pass the key explicitly: `pdf-to-markdown parse -k "your-key" document.pdf`

Check which keys are stored: `pdf-to-markdown login --list`

## "Output files already exist"

This prevents accidental overwrites. Options:

- Add `--overwrite` to replace existing files
- Use `-o ./new-dir/` to output to a different directory
- Delete the existing `doc.md` and `images/` directory first

In automated scripts, always use `--overwrite`.

## Credentials Storage

API keys are stored in an AES-256-GCM encrypted file:

- macOS: `~/Library/Application Support/pdf-to-markdown/credentials.enc`
- Linux: `~/.config/pdf-to-markdown/credentials.enc`
- Windows: `%APPDATA%\pdf-to-markdown\config\credentials.enc`

The encryption key is derived from a hardware identifier unique to each machine. If the file is corrupted or the machine changes, re-run `pdf-to-markdown login` to re-store credentials.

## "Input file does not exist"

The specified file path doesn't exist. For local files, use an absolute path or verify the relative path is correct. For URLs, check network connectivity — the tool downloads PDFs from URLs automatically.

## Conversion Produces Poor Results

1. **Try a different provider/model:**
   - `--provider mineru` (VLM) for best quality complex documents with images
   - `--provider mineru/agent` for quick results with no API key needed
   - `--provider zhipu/expert` or `--provider zhipu/prime` for complex documents
   - Zhipu expert/prime handle tables and formulas better than PaddleOCR

2. **Convert specific pages:** Use `--pages` to focus on the problematic section, then merge results.

3. **Check if the PDF is scanned:** Scanned PDFs (images of text) are harder to parse. OCR quality depends on the provider's capabilities. Zhipu Prime handles scanned documents better.

## Cache Issues

The tool caches conversion results by file hash (SHA256) + provider + page ranges. Different providers or page ranges for the same PDF produce separate cache entries. Cache is stored at:

- macOS: `~/Library/Caches/pdf-to-markdown/`
- Linux: `~/.cache/pdf-to-markdown/`
- Windows: `%LOCALAPPDATA%\pdf-to-markdown\cache\`

### Cache Seems Stale

If you've modified the PDF, the cache should automatically miss (hash changes). If you still suspect stale data:

```bash
# Check cache size and entry count
pdf-to-markdown cache status

# Clear entire cache (with confirmation)
pdf-to-markdown cache clear

# Force clear without confirmation
pdf-to-markdown cache clear --force

# Disable cache for a single run
PDF_TO_MARKDOWN_NO_CACHE=1 pdf-to-markdown parse document.pdf
```

## Slow Conversion

- Large PDFs: Use `--pages` to convert in batches
- Network issues: Providers are cloud APIs — conversion speed depends on network latency to MinerU, PaddleOCR, or Zhipu servers
- Check cache: `pdf-to-markdown cache status` — if the cache is very large, clearing it may improve index lookup speed
- Zhipu: The polling interval is 3 seconds — large documents with many pages complete in the background; use the progress bar to monitor

## Error Codes

| Exit Code | Meaning |
|---|---|
| 0 | Success |
| 1 | General failure |
| 2 | Usage error (wrong flags, invalid provider) |
| 3 | Resource not found (file missing, bad URL) |
| 4 | Permission denied (reserved) |
| 5 | Conflict (output files already exist without `--overwrite`) |

Use `--json` for structured error output including error type and actionable suggestions.
