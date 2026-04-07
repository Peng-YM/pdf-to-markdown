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
curl -fsSL https://raw.githubusercontent.com/pengym/pdf-extractor/main/install.sh | bash
```

或者使用 wget:

```bash
wget -qO- https://raw.githubusercontent.com/pengym/pdf-extractor/main/install.sh | bash
```

### 方法 2: 从 GitHub Release 下载

从 [GitHub Releases](https://github.com/pengym/pdf-extractor/releases) 下载对应平台的二进制文件。

### 方法 3: 从源码构建

```bash
git clone https://github.com/pengym/pdf-extractor.git
cd pdf-extractor/pdf-to-markdown
cargo build --release
```

## 使用

### 基本使用

```bash
# 使用默认提供商 (PaddleOCR)
export PADDLE_OCR_API_KEY="your_api_key"
pdf-to-markdown parse document.pdf

# 或使用智谱 AI
export ZHIPU_API_KEY="your_api_key"
pdf-to-markdown parse --provider zhipu/lite document.pdf
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
# 获取元数据并以人类可读格式显示
pdf-to-markdown metadata document.pdf

# 输出 JSON 格式
pdf-to-markdown metadata document.pdf --json

# 保存到文件
pdf-to-markdown metadata document.pdf -o metadata.json
```

#### `parse` - 解析 PDF 为 Markdown

```bash
# 基本使用
pdf-to-markdown parse document.pdf

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

## 开发

### 构建

```bash
make build
```

### 安装到本地

```bash
make install
```

### 发布构建

```bash
make release
```

### 代码检查

```bash
# 格式化
make fmt

# Clippy 检查
make clippy

# Cargo check
make check

# 运行测试
make test
```

## 架构设计

本项目采用模块化架构，便于扩展新的服务提供商：

```
pdf-to-markdown/
├── src/
│   ├── lib.rs                    # 库入口
│   ├── main.rs                   # CLI 入口
│   ├── error.rs                  # 错误处理
│   ├── converter.rs              # 转换协调器
│   ├── utils.rs                  # 工具函数
│   └── provider/                 # 提供商模块
│       ├── mod.rs                # 工厂模式
│       ├── traits.rs             # 核心 trait 定义
│       ├── zhipu/                # 智谱 AI 实现
│       └── paddleocr/            # PaddleOCR 实现
```

### 核心 Trait

```rust
#[async_trait::async_trait]
pub trait DocumentProvider: Send + Sync {
    fn name(&self) -> &'static str;
    
    async fn parse_document(
        &self,
        file_path: &Path,
        config: &dyn ProviderConfig,
        progress_cb: Box<dyn FnMut(ProgressUpdate) + Send>,
    ) -> Result<ParseResult>;
}
```

### 支持的提供商

- **PaddleOCR** - 默认提供商
- **智谱 AI (Zhipu)** - 支持 lite/expert/prime 三种模式

## 扩展新的提供商

要添加新的提供商（如阿里云、百度），只需：

1. 在 `src/provider/` 下创建新目录
2. 实现 `DocumentProvider` trait
3. 在 `ProviderType` 枚举中添加新类型
4. 在 `create_provider()` 工厂函数中添加对应逻辑

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
