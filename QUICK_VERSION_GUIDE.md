# 🚀 快速版本发布指南

## 一句话总结

**打什么 tag，就发布什么版本** —— Git tag 是唯一真实来源，所有版本号自动同步。

---

##  3 步完成发布

### 第 1 步：更新版本号（本地）

```bash
# 方式 A：指定新版本号
./scripts/update-version.sh 1.2.0

# 方式 B：从已有 tag 自动提取（如果本地已有 v1.2.0）
./scripts/update-version.sh
```

✅ 脚本会自动更新：
- `package.json` → `"version": "1.2.0"`
- `tauri-gui/Cargo.toml` → `version = "1.2.0"`
- `tauri-gui/tauri.conf.json` → `"version": "1.2.0"`

### 第 2 步：提交并创建标签

```bash
# 提交配置文件更改
git add package.json tauri-gui/Cargo.toml tauri-gui/tauri.conf.json
git commit -m "chore: bump version to 1.2.0"

# 创建 Git tag（必须以 v 开头！）
git tag v1.2.0

# 推送代码和标签
git push origin main
git push origin v1.2.0
```

### 第 3 步：等待 GitHub Actions 完成

推送 tag 后，GitHub Actions 会自动：
1. ✅ 提取版本号 `v1.2.0` → `1.2.0`
2. ✅ 再次同步所有配置文件（确保一致性）
3. ✅ 构建 macOS DMG 和 APP 包
4. ✅ 创建 GitHub Release

**最终效果**：
- **Release 名称**: `Mole v1.2.0`
- **DMG 文件名**: `Mole_1.2.0_aarch64.dmg`
- **应用内显示**: 设置页面显示 `Mole GUI v1.2.0`

---

## ️ 重要提醒

|  错误做法 | ✅ 正确做法 |
|------------|------------|
| 手动修改 `package.json` 的 version | 使用 `update-version.sh` 脚本 |
| Tag 不带 `v` 前缀（如 `1.2.0`） | Tag 必须带 `v`（如 `v1.2.0`） |
| 只推送代码不推送 tag | 同时推送代码和 tag |
| 先打 tag 再提交代码 | 先提交代码再打 tag |

---

##  验证版本一致性

### 本地验证

```bash
# 检查三个配置文件的版本号是否一致
cat package.json | grep '"version"'
cat tauri-gui/Cargo.toml | grep '^version'
cat tauri-gui/tauri.conf.json | grep '"version"'

# 应该都输出相同的版本号（如 "1.2.0"）
```

### 构建后验证

```bash
# 构建应用
pnpm tauri build

# 检查生成的文件名（应该包含版本号）
ls -la tauri-gui/target/release/bundle/dmg/
# 示例输出：Mole_1.2.0_aarch64.dmg

# 启动应用，在设置页面查看版本号
# 应该显示：Mole GUI v1.2.0
```

---

## 💡 常见问题

### Q: 我忘记运行 update-version.sh 就推送 tag 了怎么办？

A: 没关系！GitHub Actions 会自动运行该脚本同步配置文件，构建产物仍然是正确的。但建议下次还是先本地运行脚本并提交，保持 Git 历史整洁。

### Q: 如何支持预发布版本（alpha、beta）？

A: 直接在版本号中包含预发布标识符即可：

```bash
./scripts/update-version.sh 2.0.0-alpha
git tag v2.0.0-alpha
git push origin v2.0.0-alpha
```

### Q: 如何回滚或重新发布某个版本？

A: 

```bash
# 删除本地和远程的 tag
git tag -d v1.2.0
git push origin --delete v1.2.0

# 修改代码后重新创建 tag
git tag v1.2.0
git push origin v1.2.0
```

⚠️ **注意**：GitHub Release 一旦创建就无法完全删除，只能标记为草稿或删除资产文件。

---

## 📚 更多文档

- [VERSION_MANAGEMENT.md](VERSION_MANAGEMENT.md) - 详细版本文档
- [IMPLEMENTATION_SUMMARY.md](IMPLEMENTATION_SUMMARY.md) - 技术实现细节
- [scripts/README.md](scripts/README.md) - 脚本工具说明

---

**最后更新**: 2026-06-30  
**适用版本**: Mole GUI v1.0.0+
