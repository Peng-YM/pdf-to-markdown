# PDF to Markdown Converter

使用模块化架构的 PDF 转 Markdown 命令行工具，支持多个文档解析服务提供商。专为 AI 代理友好设计。

## 特性

- 🚀 **多提供商支持**: PaddleOCR (默认), 智谱 AI (lite/expert/prime)
- 🤖 **AI 代理友好**: 结构化 JSON 输出、有意义的退出码、dry-run 支持
- 📦 **易于安装**: 一键安装脚本，支持 Linux/macOS/Windows
- 🔧 **配置灵活**: 丰富的 CLI 选项和配置

## 安装

### 方法 1: 一键安装脚本 (推荐)

Linux/macOS:

```bash
curl -fsSL https://raw.githubusercontent.com/Peng-YM/pdf-to-markdown/main/install.sh | bash
```

### 方法 2: 从 GitHub Release 下载

从 [GitHub Releases](https://github.com/Peng-YM/pdf-to-markdown/releases) 下载对应平台的二进制文件。

### 方法 3: 从源码构建

```bash
git clone https://github.com/Peng-YM/pdf-to-markdown.git
cd pdf-to-markdown
cargo build --release
```

更多开发相关信息请参考 [CONTRIBUTING.md](./CONTRIBUTING.md)。

## 使用

### 基本使用

```bash
# 使用默认提供商 (PaddleOCR) - 本地文件
export PADDLE_OCR_API_KEY="your_api_key"
pdf-to-markdown parse document.pdf

# 或使用智谱 AI - 本地文件
export ZHIPU_API_KEY="your_api_key"
pdf-to-markdown parse --provider zhipu/lite document.pdf

# 使用 URL 直接下载 PDF
pdf-to-markdown parse https://example.com/document.pdf

# 使用 arxiv abs 链接 (自动转换为 pdf 链接)
pdf-to-markdown parse https://arxiv.org/abs/2301.07041
```

### 完整选项

```bash
pdf-to-markdown parse \
  --provider zhipu/expert \
  --api-key "your_api_key" \
  --pages 1-5,10 \
  --output-dir ./output/ \
  --json \
  document.pdf
```

### 子命令

#### `metadata` - 获取 PDF 元数据

```bash
# 获取元数据并以人类可读格式显示 - 本地文件
pdf-to-markdown metadata document.pdf

# 获取元数据 - 使用 URL
pdf-to-markdown metadata https://example.com/document.pdf

# 获取元数据 - 使用 arxiv abs 链接 (自动转换为 pdf)
pdf-to-markdown metadata https://arxiv.org/abs/2301.07041

# 输出 JSON 格式
pdf-to-markdown metadata document.pdf --json

# 保存到文件
pdf-to-markdown metadata document.pdf -o metadata.json
```

#### `parse` - 解析 PDF 为 Markdown

```bash
# 基本使用 - 本地文件
pdf-to-markdown parse document.pdf

# 使用 URL 直接下载 PDF
pdf-to-markdown parse https://example.com/document.pdf

# 使用 arxiv abs 链接 (自动转换为 pdf 链接)
pdf-to-markdown parse https://arxiv.org/abs/2301.07041

# 指定输出目录
pdf-to-markdown parse document.pdf -o ./output/

# 指定页面范围
pdf-to-markdown parse document.pdf --pages 1-5,10,15-20

# 使用不同提供商
pdf-to-markdown parse --provider paddleocr document.pdf
pdf-to-markdown parse --provider zhipu/lite document.pdf
pdf-to-markdown parse --provider zhipu/expert document.pdf
pdf-to-markdown parse --provider zhipu/prime document.pdf

# Dry run (预览操作)
pdf-to-markdown parse document.pdf --dry-run

# JSON 输出 (AI 代理友好)
pdf-to-markdown parse document.pdf --json

# 安静模式 (只输出文件路径)
pdf-to-markdown parse document.pdf --quiet

# 覆盖已存在的输出文件
pdf-to-markdown parse document.pdf --overwrite
```

## 开发与贡献

更多开发相关信息、架构设计和如何扩展新提供商，请参考 [CONTRIBUTING.md](./CONTRIBUTING.md)。

## AI 代理友好设计

本工具专为 AI 代理使用优化：

- ✅ **结构化输出**: `--json` 标志输出 JSON 格式
- ✅ **有意义的退出码**: 0=成功, 1=失败, 2=使用错误, 3=未找到, 4=权限, 5=冲突
- ✅ **Dry-run 支持**: `--dry-run` 预览操作
- ✅ **安静模式**: `--quiet` 适合脚本和管道
- ✅ **可操作的错误**: 包含错误类型和修复建议
- ✅ **完善的帮助**: 大量示例和清晰的参数说明

## 许可证

MIT
