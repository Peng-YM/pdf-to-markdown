# API Key Setup

The tool requires an API key from one of two providers. PaddleOCR is the default and offers 20,000 free pages per day.

## Provider Options

| Provider | Model Specifier | Free Quota | Key Source |
|---|---|---|---|
| PaddleOCR | `paddleocr` (default) | 20,000 pages/day | https://aistudio.baidu.com/paddleocr |
| Zhipu Lite | `zhipu/lite` | Varies | https://bigmodel.cn/usercenter/proj-mgmt/apikeys |
| Zhipu Expert | `zhipu/expert` | Varies | Same as above |
| Zhipu Prime | `zhipu/prime` | Varies | Same as above |

Note: Zhipu requires real-name authentication.

## Storing API Keys (Recommended)

Use the `login` subcommand to store keys securely in the system keychain:

```bash
# Interactive: prompts for provider choice and key
pdf-to-markdown login

# Store for a specific provider (interactive key entry)
pdf-to-markdown login --provider paddleocr
pdf-to-markdown login --provider zhipu

# Non-interactive: provide both provider and key
pdf-to-markdown login --provider zhipu --api-key "your-api-key"

# List stored credentials
pdf-to-markdown login --list

# Remove a stored credential
pdf-to-markdown login --delete paddleocr
```

Once stored, you can run `pdf-to-markdown parse` without specifying `--api-key` — the tool reads from the keychain automatically.

## API Key Resolution Order

When no `--api-key` is passed, the tool checks in this order:

1. `--api-key` / `-k` flag (explicit)
2. Provider-specific env var (`PADDLE_OCR_API_KEY` or `ZHIPU_API_KEY`)
3. Generic env var (`PROVIDER_API_KEY`)
4. System keychain (set via `pdf-to-markdown login`)

This means env vars override stored credentials, and the explicit flag overrides everything.

## Login Workflow

When the user needs to set up credentials, guide them through these steps:

1. **Choose a provider** — PaddleOCR is recommended for new users (20,000 free pages/day, no real-name auth)
2. **Get the API key** — Direct the user to the provider's website (see table above) to obtain a key
3. **Store it** — Run `pdf-to-markdown login --provider paddleocr` (or `zhipu`) and paste the key when prompted
4. **Verify** — Run `pdf-to-markdown login --list` to confirm the credential is stored

If the user already has an API key, they can store it non-interactively:

```bash
pdf-to-markdown login --provider paddleocr --api-key "their-api-key"
```
