# PDF to Markdown 转换器

<p align="center">
  <a href="https://github.com/Peng-YM/pdf-to-markdown/stargazers">
    <img src="https://img.shields.io/github/stars/Peng-YM/pdf-to-markdown?style=flat-square" alt="Stars">
  </a>
  <a href="https://github.com/Peng-YM/pdf-to-markdown/network/members">
    <img src="https://img.shields.io/github/forks/Peng-YM/pdf-to-markdown?style=flat-square" alt="Forks">
  </a>
  <a href="https://github.com/Peng-YM/pdf-to-markdown/issues">
    <img src="https://img.shields.io/github/issues/Peng-YM/pdf-to-markdown?style=flat-square" alt="Issues">
  </a>
  <a href="https://github.com/Peng-YM/pdf-to-markdown/blob/master/LICENSE">
    <img src="https://img.shields.io/github/license/Peng-YM/pdf-to-markdown?style=flat-square" alt="License">
  </a>
  <a href="https://github.com/Peng-YM/pdf-to-markdown/releases">
    <img src="https://img.shields.io/github/v/release/Peng-YM/pdf-to-markdown?style=flat-square" alt="Release">
  </a>
  <a href="https://github.com/Peng-YM/pdf-to-markdown/releases">
    <img src="https://img.shields.io/github/downloads/Peng-YM/pdf-to-markdown/total?style=flat-square" alt="Downloads">
  </a>
</p>

**pdf-to-markdown** 是一款专为 AI 智能体设计的 PDF 转 Markdown 工具。基于 MinerU、PaddleOCR 及智谱 AI 的先进文档解析能力，可准确提取文本、表格、公式、图片及文档结构，轻松应对学术论文、技术报告等复杂版式。无需本地 GPU，**无需 API Key**（MinerU Agent 开箱即用）。一条命令即可将任意 PDF 转换为整洁、结构化的 Markdown，是 AI 智能体读取和处理 PDF 的首选工具。

<p align="center">
  <img src="assets/demo.png" alt="演示" width="800" />
</p>

## 功能特性

- 多提供商支持：MinerU（VLM/Pipeline/Agent）、PaddleOCR、智谱 AI（lite/expert/prime）
- **零配置即用** — MinerU Agent 无需 API Key；智能自动检测并选择最佳可用提供商
- API Key 使用 AES-256-GCM 加密文件安全存储，从不明文保存
- 复杂元素解析：文本、图片、表格、公式等
- 结构化 JSON 输出、有意义的退出码及演练（dry-run）支持
- 支持 Linux/macOS/Windows 一键安装脚本
- 丰富的 CLI 选项，灵活配置
- 内置缓存，避免重复 API 调用
- 基于文件哈希和 URL 的重复检测

### 支持的文档元素

- 文本：段落、标题、列表及其他文本内容
- 图片：自动提取并保存文档中的图片
- 表格：智能表格识别，并转换为 Markdown 格式
- 公式：LaTeX 公式识别，支持公式编号
- 版式：自动检测文档版面结构

针对 Arxiv 论文等学术文献进行了专项优化。

## 安装

### 一键安装脚本

Linux/macOS：

```bash
curl -fsSL https://raw.githubusercontent.com/Peng-YM/pdf-to-markdown/master/install.sh | bash
```

其他安装方式请参见 [GitHub Releases](https://github.com/Peng-YM/pdf-to-markdown/releases) 或从源码构建。

### Agent 技能

针对 AI 编程智能体（Claude Code、Codex 等），可安装 Agent 技能，使智能体在需要读取 PDF 时自动调用 pdf-to-markdown：

```bash
npx skills add Peng-YM/pdf-to-markdown
```

该技能为智能体提供安装指引、登录流程、使用模式、最佳实践及故障排查，无需手动配置。

更多开发信息请参见 [CONTRIBUTING.md](./CONTRIBUTING.md)。

## API Key 配置

API Key 为**可选项** — MinerU Agent 无需任何配置即可使用。添加 API Key 可获得更高质量或更大的每日调用量。

### 安全登录（推荐）

```bash
# 交互式：选择提供商并输入 API Key
pdf-to-markdown login

# 为指定提供商存储 API Key
pdf-to-markdown login --provider paddleocr
pdf-to-markdown login --provider zhipu
pdf-to-markdown login --provider mineru

# 非交互式：同时指定提供商和 Key
pdf-to-markdown login --provider paddleocr --api-key "your_api_key"

# 列出已存储的凭据
pdf-to-markdown login --list

# 删除已存储的凭据
pdf-to-markdown login --delete paddleocr
```

API Key 存储在 AES-256-GCM 加密文件中，从不以明文保存。

### 环境变量（备选方式）

```bash
export PADDLE_OCR_API_KEY="your_api_key"
export ZHIPU_API_KEY="your_api_key"
export MINERU_API_KEY="your_api_key"  # 仅用于精准 API；Agent API 无需 Key
```

或通过 `--api-key` / `-k` 参数传入：

```bash
pdf-to-markdown parse -k "your_api_key" document.pdf
```

### MinerU
- 申请地址：https://mineru.net/apiManage/token
- 模型：VLM（推荐）、Pipeline、Agent（轻量，无需授权）
- Agent API：无需 Token，按 IP 限速，最大 10MB/20 页
- 精准 API：每日优先处理 1,000 页，ZIP 格式输出（含图片）

### PaddleOCR
- 申请地址：https://aistudio.baidu.com/paddleocr
- 免费额度：每日 20,000 页

### 智谱 AI
- 申请地址：https://bigmodel.cn/usercenter/proj-mgmt/apikeys
- 注意：需完成实名认证

## 使用方法

### 基本用法

```bash
# 将 PDF 转为 Markdown — 无需任何配置（默认使用 MinerU Agent）
pdf-to-markdown parse document.pdf

# 可选：添加 API Key 以获得更高质量或更大容量
pdf-to-markdown login

# 使用智谱 AI
pdf-to-markdown login --provider zhipu
pdf-to-markdown parse --provider zhipu/lite document.pdf

# 使用 MinerU VLM（最高质量，需要 Token）
pdf-to-markdown login --provider mineru
pdf-to-markdown parse --provider mineru document.pdf

# 使用 MinerU Agent（轻量，无需授权）
pdf-to-markdown parse --provider mineru/agent document.pdf

# 通过 URL 直接下载并转换 PDF
pdf-to-markdown parse https://example.com/document.pdf

# 使用 arxiv abs 链接（自动转换为 PDF 链接）
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

#### `metadata` - 提取 PDF 元数据

```bash
# 以可读格式提取本地文件的元数据
pdf-to-markdown metadata document.pdf

# 通过 URL 提取元数据
pdf-to-markdown metadata https://example.com/document.pdf

# 使用 arxiv abs 链接（自动转换为 PDF）
pdf-to-markdown metadata https://arxiv.org/abs/2301.07041

# 以 JSON 格式输出
pdf-to-markdown metadata document.pdf --json

# 保存到文件
pdf-to-markdown metadata document.pdf -o metadata.json
```

#### `parse` - 将 PDF 转换为 Markdown

```bash
# 基本用法（本地文件）
pdf-to-markdown parse document.pdf

# 通过 URL 下载并转换 PDF
pdf-to-markdown parse https://example.com/document.pdf

# 使用 arxiv abs 链接（自动转换为 PDF 链接）
pdf-to-markdown parse https://arxiv.org/abs/2301.07041

# 指定输出目录
pdf-to-markdown parse document.pdf -o ./output/

# 指定页面范围
pdf-to-markdown parse document.pdf --pages 1-5,10,15-20

# 使用不同的提供商
pdf-to-markdown parse --provider paddleocr document.pdf
pdf-to-markdown parse --provider zhipu/lite document.pdf
pdf-to-markdown parse --provider zhipu/expert document.pdf
pdf-to-markdown parse --provider zhipu/prime document.pdf
pdf-to-markdown parse --provider mineru document.pdf
pdf-to-markdown parse --provider mineru/pipeline document.pdf
pdf-to-markdown parse --provider mineru/agent document.pdf

# 演练模式，预览操作
pdf-to-markdown parse document.pdf --dry-run

# JSON 格式输出
pdf-to-markdown parse document.pdf --json

# 静默模式（仅输出文件路径）
pdf-to-markdown parse document.pdf --quiet

# 覆盖已有输出文件
pdf-to-markdown parse document.pdf --overwrite

# 临时禁用缓存
PDF_TO_MARKDOWN_NO_CACHE=1 pdf-to-markdown parse document.pdf
```

#### `login` - 安全存储 API Key

```bash
# 交互式：选择提供商并输入 API Key
pdf-to-markdown login

# 为指定提供商存储 API Key（交互式输入）
pdf-to-markdown login --provider paddleocr

# 非交互式存储 API Key
pdf-to-markdown login --provider zhipu --api-key "your_api_key"

# 列出已存储的凭据
pdf-to-markdown login --list

# 删除已存储的凭据
pdf-to-markdown login --delete paddleocr

# JSON 格式输出
pdf-to-markdown login --list --json
```

#### `cache` - 缓存管理

```bash
# 查看缓存状态
pdf-to-markdown cache status

# 以 JSON 格式查看缓存状态
pdf-to-markdown cache status --json

# 清除缓存（需确认）
pdf-to-markdown cache clear

# 强制清除缓存（无需确认）
pdf-to-markdown cache clear --force
```

## 缓存机制

工具会自动缓存解析结果，避免对相同 PDF 文件或 URL 重复调用 API，节省成本与时间。

### 缓存原理

- 文件哈希：以本地文件的 SHA256 哈希作为缓存键
- URL 哈希：以 URL 的 SHA256 哈希作为缓存键
- 多维缓存：缓存键包含提供商类型和页面范围，防止不同配置间的混淆
- 图片缓存：提取的图片也会被缓存，加速重复解析

### 缓存位置

缓存存储在系统标准缓存目录中：
- Linux：`~/.cache/pdf-to-markdown/`
- macOS：`~/Library/Caches/pdf-to-markdown/`
- Windows：`%LOCALAPPDATA%\\pdf-to-markdown\\cache\\`

### 临时禁用缓存

某些情况下，您可能希望绕过缓存重新解析文件：

```bash
# 方式一：设置环境变量
PDF_TO_MARKDOWN_NO_CACHE=1 pdf-to-markdown parse document.pdf

# 方式二：使用 true 值
PDF_TO_MARKDOWN_NO_CACHE=true pdf-to-markdown parse document.pdf
```

## 开发与贡献

更多开发信息、架构设计及如何扩展新提供商，请参见 [CONTRIBUTING.md](./CONTRIBUTING.md)。

## 自动化友好设计

本工具针对自动化和脚本集成进行了优化：

- 结构化输出：`--json` 标志支持 JSON 格式输出
- 有意义的退出码：0=成功，1=失败，2=用法错误，3=未找到，4=权限，5=冲突
- 演练支持：`--dry-run` 预览操作
- 静默模式：`--quiet` 适合脚本和管道
- 可操作的错误信息：包含错误类型和修复建议
- 完善的帮助文档：丰富的示例和清晰的参数说明

## 许可证

MIT
