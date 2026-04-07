# PDF to Markdown Converter

使用模块化架构的 PDF 转 Markdown 命令行工具，支持多个文档解析服务提供商。

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
│       └── zhipu/                # 智谱 AI 实现
│           ├── mod.rs
│           ├── api.rs
│           └── models.rs
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

- **智谱 AI (Zhipu)** - 默认提供商，支持 lite/expert/prime 三种模式

## 安装

```bash
git clone <repo>
cd pdf-to-markdown
cargo build --release
```

## 使用

### 基本使用

```bash
# 使用智谱 AI (lite 模式)
export ZHIPU_API_KEY="your_api_key"
./target/release/pdf-to-markdown -t lite document.pdf

# 指定输出文件
./target/release/pdf-to-markdown -t lite document.pdf -o output.md
```

### 完整选项

```bash
./target/release/pdf-to-markdown \
  --provider zhipu \
  --api-key "your_api_key" \
  --tool-type prime \
  --max-retries 60 \
  --poll-interval 3 \
  document.pdf \
  -o output.md
```

## 扩展新的提供商

要添加新的提供商（如阿里云、百度），只需：

1. 在 `src/provider/` 下创建新目录
2. 实现 `DocumentProvider` trait
3. 在 `ProviderType` 枚举中添加新类型
4. 在 `create_provider()` 工厂函数中添加对应逻辑

## 许可证

MIT
