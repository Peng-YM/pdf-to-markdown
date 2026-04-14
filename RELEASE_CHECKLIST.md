# 发布检查清单

## 发布前检查

- [ ] 确保所有更改已提交到 git
- [ ] 运行 Rust Formatter 检查代码格式
- [ ] 运行 Clippy 检查代码质量
- [ ] 确保所有测试通过
- [ ] 更新 Cargo.toml 中的版本号
- [ ] 更新 main.rs 中的版本号
- [ ] 更新 install.sh 中的 DEFAULT_VERSION
- [ ] 更新 README.md 中的相关内容
- [ ] 创建发布 commit

## 发布步骤

### 1. 代码质量检查

```bash
# 运行 Formatter 检查
cargo fmt -- --check

# 如果有格式问题，自动修复
cargo fmt

# 运行 Clippy 检查
cargo clippy -- -D warnings

# 确保构建成功
cargo build --release
```

### 2. 运行发布脚本

注意：`github_release.sh` 脚本会自动从 Cargo.toml 读取版本号，创建并推送 Git Tag。

```bash
# 确保脚本有执行权限
chmod +x github_release.sh

# 运行发布脚本
./github_release.sh
```

## 发布后验证

- [ ] 确认 GitHub Tag 已创建
- [ ] 确认 GitHub Release 已创建
- [ ] 确认所有平台的二进制文件已上传
- [ ] 测试下载和安装脚本
- [ ] 验证安装后的版本号正确

## 版本说明

- **主版本号** (vX.0.0): 不兼容的 API 修改
- **次版本号** (v0.X.0): 向下兼容的功能性新增
- **修订号** (v0.0.X): 向下兼容的问题修正

## 示例发布流程

```bash
# 1. 检查并格式化代码
cargo fmt
cargo clippy -- -D warnings
cargo build --release

# 2. 确保所有更改已提交
git status

# 3. 运行发布脚本（自动创建和推送 Tag）
./github_release.sh
```
