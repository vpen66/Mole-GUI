# Mole GUI 脚本工具

本目录包含 Mole GUI 项目的自动化脚本工具。

## 📜 update-version.sh

版本管理脚本，用于同步所有配置文件中的版本号。

### 功能

- ✅ 从 Git tag 或命令行参数提取版本号
- ✅ 自动更新 `package.json`、`Cargo.toml`、`tauri.conf.json`
- ✅ 验证版本号格式（语义化版本 x.y.z）
- ✅ 提供下一步操作提示

### 使用方法

#### 方法一：指定版本号

```bash
./scripts/update-version.sh 1.1.0
```

#### 方法二：从最新 Git tag 自动提取

```bash
# 前提：本地已有 v1.1.0 这样的 tag
./scripts/update-version.sh
```

### 输出示例

```
[INFO] =========================================
[INFO] Mole GUI 版本管理工具
[INFO] =========================================
[INFO] 使用指定的版本号: 1.1.0
[INFO] 已更新 package.json 版本号为: 1.1.0
[INFO] 已更新 Cargo.toml 版本号为: 1.1.0
[INFO] 已更新 tauri.conf.json 版本号为: 1.1.0
[INFO] =========================================
[INFO] ✅ 所有配置文件版本号已同步为: 1.1.0
[INFO] =========================================
[INFO] 
[INFO] 下一步操作：
[INFO] 1. 提交更改: git add package.json tauri-gui/Cargo.toml tauri-gui/tauri.conf.json
[INFO] 2. 创建标签: git tag v1.1.0
[INFO] 3. 推送标签: git push origin v1.1.0
[INFO] 4. GitHub Actions 将自动构建并发布 Release
```

### 完整发布流程

```bash
# 1. 更新版本号
./scripts/update-version.sh 1.1.0

# 2. 提交配置文件更改
git add package.json tauri-gui/Cargo.toml tauri-gui/tauri.conf.json
git commit -m "chore: bump version to 1.1.0"

# 3. 创建并推送 Git tag
git tag v1.1.0
git push origin main
git push origin v1.1.0

# 4. 等待 GitHub Actions 自动构建和发布
# 访问 https://github.com/your-repo/Mole-GUI/actions 查看进度
```

### 注意事项

⚠️ **Tag 命名规范**：
- 必须以 `v` 开头（如 `v1.0.0`、`v1.1.0-beta`）
- 推荐遵循语义化版本：`MAJOR.MINOR.PATCH`

 **不要手动修改版本号**：
- 始终使用此脚本同步版本号
- 手动修改会导致版本不一致

✅ **先提交再打 tag**：
- 确保配置文件更改已提交到 Git
- 然后再创建和推送 tag

### 相关文档

- [VERSION_MANAGEMENT.md](../VERSION_MANAGEMENT.md) - 详细版本文档
- [.github/workflows/release.yml](../.github/workflows/release.yml) - GitHub Actions 工作流

---

**最后更新**: 2026-06-30
