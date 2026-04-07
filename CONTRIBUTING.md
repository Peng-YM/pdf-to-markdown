# 贡献指南

感谢你对 PDF to Markdown 项目的兴趣！

## 开发

### 从源码构建

```bash
git clone https://github.com/pengym/pdf-extractor.git
cd pdf-extractor
cargo build --release
```

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
# 格式化代码
make fmt

# 检查格式化
make fmt-check

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
pdf-extractor/
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

## 提交 Pull Request

1. Fork 本仓库
2. 创建你的特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交你的更改 (`git commit -m 'Add some amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 开启一个 Pull Request

## 代码规范

- 确保运行 `make fmt` 格式化代码
- 确保通过 `make clippy` 检查
- 添加必要的测试（如果适用）
- 更新相关文档

## 许可证

通过贡献代码，你同意你的贡献将根据 MIT 许可证进行许可。
