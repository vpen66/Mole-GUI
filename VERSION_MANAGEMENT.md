# Mole GUI 版本管理规范

本文档说明 Mole GUI 项目的统一版本管理机制，确保 Git tag、构建产物文件名和应用内部版本号保持一致。

##  核心原则

**Git tag 是唯一真实来源（Single Source of Truth）**

所有版本号都从 Git tag 自动提取和同步，禁止手动硬编码版本号。

## 📦 版本来源

| 配置文件 | 字段 | 更新方式 |
|---------|------|---------|
| `package.json` | `version` | 通过 `scripts/update-version.sh` 自动同步 |
| `tauri-gui/Cargo.toml` | `version` | 通过 `scripts/update-version.sh` 自动同步 |
| `tauri-gui/tauri.conf.json` | `version` | 通过 `scripts/update-version.sh` 自动同步 |
| GitHub Actions | Release Tag | 从 Git tag 自动提取 |
| 应用内显示 | `get_gui_version()` | 编译时从 Cargo.toml 读取 |

## 🔄 发布流程

### 1. 本地准备新版本

```bash
# 方法一：指定版本号
./scripts/update-version.sh 1.1.0

# 方法二：从最新 Git tag 自动提取（如果已有 v1.1.0 tag）
./scripts/update-version.sh
```

脚本会自动：
- ✅ 验证版本号格式（x.y.z）
- ✅ 更新 `package.json` 的 version
- ✅ 更新 `tauri-gui/Cargo.toml` 的 version
- ✅ 更新 `tauri-gui/tauri.conf.json` 的 version
- ✅ 提供下一步操作提示

### 2. 提交更改并创建 Git tag

```bash
# 提交配置文件更改
git add package.json tauri-gui/Cargo.toml tauri-gui/tauri.conf.json
git commit -m "chore: bump version to 1.1.0"

# 创建 Git tag（必须以 v 开头）
git tag v1.1.0

# 推送代码和标签到远程仓库
git push origin main
git push origin v1.1.0
```

### 3. GitHub Actions 自动构建

当推送 `v*` 格式的 tag 时，GitHub Actions 会自动触发：

1. **提取版本号**：从 `GITHUB_REF` 中提取 tag（如 `refs/tags/v1.1.0` → `v1.1.0` → `1.1.0`）
2. **同步配置文件**：运行 `scripts/update-version.sh $VERSION` 确保所有配置文件一致
3. **构建应用**：使用 Tauri Action 构建 macOS DMG 和 APP
4. **创建 Release**：自动生成 GitHub Release，附带构建产物

最终效果：
- Release 名称：`Mole v1.1.0`
- 构建产物：`Mole_1.1.0_aarch64.dmg`、`Mole_aarch64.app.tar.gz`
- 应用内显示：设置页面显示 `v1.1.0`

## 🛠️ 技术实现细节

### 后端（Rust）

**获取 GUI 版本**：
```rust
#[tauri::command]
pub async fn get_gui_version() -> Result<GuiVersionInfo, String> {
    // env!("CARGO_PKG_VERSION") 是 Rust 编译时宏
    // 会在编译时将 Cargo.toml 中的 version 字段展开为字符串字面量
    Ok(GuiVersionInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}
```

**原理**：
- `env!` 宏在编译时读取 `Cargo.toml` 的 `version` 字段
- 由于 CI 中已通过 `update-version.sh` 同步了版本号，所以这里读取的就是 Git tag 对应的版本
- 这类似于 Java Maven/Gradle 的资源过滤机制，但更简单直接

### 前端（TypeScript）

**调用后端命令**：
```typescript
const loadGuiVersion = useCallback(async () => {
  try {
    const versionInfo = await invoke<GuiVersionInfo>("get_gui_version");
    setGuiVersion(versionInfo.version);
  } catch (err) {
    console.error("Failed to load GUI version:", err);
  }
}, []);
```

**显示版本号**：
```tsx
<span className="text-surface-200">
  {guiVersion ? `v${guiVersion}` : t("common.loading")}
</span>
```

### GitHub Actions

**提取版本号**：
```yaml
- name: Extract version from Git tag
  id: extract_version
  run: |
    # 从 Git tag 提取版本号（去掉 v 前缀）
    TAG=${GITHUB_REF#refs/tags/}
    VERSION=${TAG#v}
    echo "VERSION=$VERSION" >> $GITHUB_OUTPUT
    echo "TAG=$TAG" >> $GITHUB_OUTPUT
    echo "✅ 从 Git tag '$TAG' 提取版本号: $VERSION"
```

**同步配置文件**：
```yaml
- name: Update version in config files
  run: |
    chmod +x scripts/update-version.sh
    ./scripts/update-version.sh ${{ steps.extract_version.outputs.VERSION }}
```

**创建 Release**：
```yaml
- name: Build and Publish
  uses: tauri-apps/tauri-action@v0
  with:
    projectPath: "./tauri-gui"
    tagName: ${{ steps.extract_version.outputs.TAG }}
    releaseName: "Mole ${{ steps.extract_version.outputs.TAG }}"
```

## ⚠️ 注意事项

### 1. Tag 命名规范

- **必须**以 `v` 开头（如 `v1.0.0`、`v1.1.0-beta`）
- **推荐**遵循语义化版本（Semantic Versioning）：`MAJOR.MINOR.PATCH`
- 示例：
  - `v1.0.0` - 正式版
  - `v1.1.0` - 新功能版本
  - `v1.0.1` - 补丁版本
  - `v2.0.0-alpha` - 预发布版本

### 2. 不要手动修改版本号

 **错误做法**：
```json
// package.json
{
  "version": "1.1.0"  // ❌ 不要手动修改
}
```

✅ **正确做法**：
```bash
# 使用脚本自动同步
./scripts/update-version.sh 1.1.0
```

### 3. 本地开发与 CI 的区别

**本地开发**：
- 可以手动运行 `update-version.sh` 测试不同版本
- 不会触发 GitHub Actions（因为没有推送 tag）

**CI 环境**：
- 只有在推送 `v*` tag 时才会触发
- 会自动提取 tag 并同步所有配置
- 构建产物文件名包含版本号

### 4. 回滚或重新发布

如果需要回滚或重新发布某个版本：

```bash
# 删除本地和远程的 tag
git tag -d v1.1.0
git push origin --delete v1.1.0

# 修改代码后重新创建 tag
git tag v1.1.0
git push origin v1.1.0
```

️ **注意**：GitHub Release 一旦创建就无法完全删除，只能标记为草稿或删除资产文件。

##  验证版本一致性

### 本地验证

```bash
# 检查各个配置文件的版本号是否一致
cat package.json | grep '"version"'
cat tauri-gui/Cargo.toml | grep '^version'
cat tauri-gui/tauri.conf.json | grep '"version"'

# 应该输出相同的版本号（如 "1.1.0"）
```

### 构建后验证

```bash
# 构建应用
pnpm tauri build

# 检查生成的文件名
ls -la src-tauri/target/release/bundle/dmg/
# 应该看到类似：Mole_1.1.0_aarch64.dmg

# 启动应用，在设置页面查看版本号
# 应该显示：Mole GUI v1.1.0
```

## 📚 相关文件

- [`scripts/update-version.sh`](scripts/update-version.sh) - 版本同步脚本
- [`.github/workflows/release.yml`](.github/workflows/release.yml) - GitHub Actions 工作流
- [`tauri-gui/src/commands/mod.rs`](tauri-gui/src/commands/mod.rs) - 后端命令定义（含 `get_gui_version`）
- [`src/pages/SettingsPage.tsx`](src/pages/SettingsPage.tsx) - 前端设置页面（显示版本号）

## 💡 常见问题

### Q: 为什么不能直接在 CI 中修改配置文件？

A: 虽然可以在 CI 中动态修改，但为了保持一致性和可追溯性，我们选择在推送 tag 前先通过脚本同步配置文件。这样可以：
1. 确保本地和 CI 环境的版本号一致
2. 方便开发者在本地测试不同版本
3. 保持 Git 历史清晰（每次版本变更都有明确的 commit）

### Q: 如果我忘记运行 update-version.sh 就推送 tag 会怎样？

A: GitHub Actions 会自动运行 `update-version.sh` 同步配置文件，所以构建产物仍然是正确的。但建议还是先本地运行脚本并提交，保持 Git 历史的整洁。

### Q: 如何支持预发布版本（如 alpha、beta）？

A: 只需在 tag 中包含预发布标识符即可，例如：
```bash
./scripts/update-version.sh 2.0.0-alpha
git tag v2.0.0-alpha
git push origin v2.0.0-alpha
```

脚本会验证版本号格式，支持 `x.y.z-prerelease` 格式。

---

**最后更新**: 2026-06-30  
**维护者**: Mole GUI Team
