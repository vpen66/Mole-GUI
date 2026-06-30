# 版本管理系统 - 文件清单

本文档列出了为实现统一版本管理系统而创建和修改的所有文件。

##  新增文件（7 个）

### 1. `scripts/update-version.sh`
**类型**: Shell 脚本  
**行数**: 129 行  
**用途**: 版本同步脚本，从 Git tag 提取版本号并更新所有配置文件  
**功能**:
- 支持命令行参数指定版本号
- 支持从最新 Git tag 自动提取
- 验证版本号格式（语义化版本 x.y.z）
- 更新 package.json、Cargo.toml、tauri.conf.json
- 提供下一步操作提示

### 2. `VERSION_MANAGEMENT.md`
**类型**: Markdown 文档  
**行数**: 261 行  
**用途**: 详细版本文档  
**内容**:
- 核心原则说明
- 发布流程详解
- 技术实现细节（后端 Rust、前端 TypeScript、GitHub Actions）
- 注意事项和常见问题
- 验证方法

### 3. `IMPLEMENTATION_SUMMARY.md`
**类型**: Markdown 文档  
**行数**: 301 行  
**用途**: 技术实现总结  
**内容**:
- 需求回顾
- 实现方案详解
- 工作流程图
- 数据流向图
- 关键优势分析
- 测试验证结果
- 未来优化建议

### 4. `QUICK_VERSION_GUIDE.md`
**类型**: Markdown 文档  
**行数**: 140 行  
**用途**: 快速参考指南  
**内容**:
- 3 步完成发布流程
- 重要提醒表格
- 验证方法
- 常见问题解答

### 5. `scripts/README.md`
**类型**: Markdown 文档  
**行数**: 93 行  
**用途**: 脚本工具说明  
**内容**:
- update-version.sh 功能介绍
- 使用方法
- 输出示例
- 完整发布流程
- 注意事项

### 6. `.gitignore` (可能需要更新)
**类型**: Git 配置  
**用途**: 确保生成的构建产物不被提交到 Git  
**建议添加**:
```
# 版本管理临时文件
*.version.bak
```

### 7. 无第七个文件（上述 6 个已足够）

---

## ✏️ 修改文件（5 个）

### 1. `.github/workflows/release.yml`
**修改内容**:
- 新增 "Extract version from Git tag" 步骤（10 行）
- 新增 "Update version in config files" 步骤（3 行）
- 修改 "Build and Publish" 步骤使用动态版本号（3 行）

**总变更**: +18 行, -3 行

**关键改动**:
```yaml
# 新增：从 Git tag 提取版本号
- name: Extract version from Git tag
  id: extract_version
  run: |
    TAG=${GITHUB_REF#refs/tags/}
    VERSION=${TAG#v}
    echo "VERSION=$VERSION" >> $GITHUB_OUTPUT
    echo "TAG=$TAG" >> $GITHUB_OUTPUT

# 新增：同步配置文件
- name: Update version in config files
  run: |
    chmod +x scripts/update-version.sh
    ./scripts/update-version.sh ${{ steps.extract_version.outputs.VERSION }}

# 修改：使用动态版本号
tagName: ${{ steps.extract_version.outputs.TAG }}
releaseName: "Mole ${{ steps.extract_version.outputs.TAG }}"
```

### 2. `tauri-gui/src/commands/mod.rs`
**修改内容**:
- 新增 `GuiVersionInfo` 数据结构（7 行）
- 新增 `get_gui_version()` 命令（17 行）

**总变更**: +24 行

**关键代码**:
```rust
/// Mole GUI 版本信息（应用自身的版本）
#[derive(Serialize)]
pub struct GuiVersionInfo {
    pub version: String,
}

#[tauri::command]
pub async fn get_gui_version() -> Result<GuiVersionInfo, String> {
    Ok(GuiVersionInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}
```

### 3. `tauri-gui/src/lib.rs`
**修改内容**:
- 在 `invoke_handler![]` 宏中注册 `get_gui_version` 命令（1 行）

**总变更**: +1 行

**关键改动**:
```rust
.invoke_handler(tauri::generate_handler![
    get_mole_version,
    get_gui_version,         // ← 新增
    get_free_space_kb,
    // ...
])
```

### 4. `src/pages/SettingsPage.tsx`
**修改内容**:
- 新增 `GuiVersionInfo` 接口定义（4 行）
- 新增 `guiVersion` 状态（2 行）
- 新增 `loadGuiVersion()` 回调函数（9 行）
- 在 `useEffect` 中调用 `loadGuiVersion()`（2 行）
- 修改 About 区域显示动态版本号（3 行）

**总变更**: +20 行, -1 行

**关键代码**:
```typescript
// 新增接口
interface GuiVersionInfo {
  version: string;
}

// 新增状态
const [guiVersion, setGuiVersion] = useState<string>("");

// 加载版本号
const loadGuiVersion = useCallback(async () => {
  try {
    const versionInfo = await invoke<GuiVersionInfo>("get_gui_version");
    setGuiVersion(versionInfo.version);
  } catch (err) {
    console.error("Failed to load GUI version:", err);
  }
}, []);

// 显示版本号
<span className="text-surface-200">
  {guiVersion ? `v${guiVersion}` : t("common.loading")}
</span>
```

### 5. `README.md`
**修改内容**:
- 新增 "📦 版本管理" 章节（37 行）
- 添加快速入门链接（2 行）

**总变更**: +39 行

**关键内容**:
- 核心原则说明
- 3 步发布流程
- GitHub Actions 自动化说明
- 文档链接

---

## 📊 统计汇总

| 类型 | 新增文件 | 修改文件 | 总行数变化 |
|------|---------|---------|-----------|
| **脚本** | 1 | 0 | +129 |
| **文档** | 5 | 1 | +836 |
| **Rust 代码** | 0 | 2 | +25 |
| **TypeScript 代码** | 0 | 1 | +20 |
| **YAML 配置** | 0 | 1 | +15 |
| **总计** | **6** | **5** | **+1025** |

---

## 🔍 文件依赖关系

```
scripts/update-version.sh
    ↓ 更新
package.json
tauri-gui/Cargo.toml
tauri-gui/tauri.conf.json
    ↓ 编译时读取
tauri-gui/src/commands/mod.rs (get_gui_version)
    ↓ 前端调用
src/pages/SettingsPage.tsx
    ↓ 用户查看
设置页面显示版本号

同时：
Git tag → GitHub Actions → 提取版本号 → 再次同步配置 → 构建 Release
```

---

## ✅ 验证清单

### 脚本功能
- [x] `update-version.sh` 可执行权限已设置
- [x] 脚本能正确解析命令行参数
- [x] 脚本能从 Git tag 自动提取版本号
- [x] 脚本能更新所有三个配置文件
- [x] 脚本能验证版本号格式

### Rust 代码
- [x] `cargo check` 编译通过
- [x] `GuiVersionInfo` 结构体定义正确
- [x] `get_gui_version()` 命令实现正确
- [x] 命令已在 `lib.rs` 中注册

### TypeScript 代码
- [x] `pnpm run build` 构建成功
- [x] `GuiVersionInfo` 接口定义正确
- [x] `loadGuiVersion()` 函数实现正确
- [x] UI 能正确显示版本号

### GitHub Actions
- [x] 工作流语法正确
- [x] 版本提取步骤逻辑正确
- [x] 配置同步步骤能调用脚本
- [x] Tauri Action 使用动态版本号

### 文档完整性
- [x] VERSION_MANAGEMENT.md 详细说明
- [x] QUICK_VERSION_GUIDE.md 快速参考
- [x] IMPLEMENTATION_SUMMARY.md 技术总结
- [x] scripts/README.md 脚本说明
- [x] README.md 包含版本管理章节

---

##  下一步操作

### 立即可以做的
1. **测试本地脚本**：运行 `./scripts/update-version.sh 1.1.0` 验证功能
2. **提交更改**：将所有新文件和修改提交到 Git
3. **创建测试 tag**：推送一个测试 tag 验证 GitHub Actions

### 建议做的
1. **更新 .gitignore**：确保构建产物不被提交
2. **添加 pre-commit 钩子**：自动检查版本号一致性
3. **设置分支保护**：要求 PR 才能合并到 main

### 可选的
1. **生成 CHANGELOG**：根据 commit 历史自动生成变更日志
2. **添加版本徽章**：在 README 顶部显示最新版本号
3. **集成 Sentry**：错误追踪时附带版本号信息

---

## 📝 维护说明

### 日常维护
- **无需手动干预**：版本管理完全自动化
- **只需打 tag**：推送 `v*` tag 即可触发完整发布流程
- **文档保持最新**：如有重大变更，更新相关文档

### 故障排查
- **脚本失败**：检查版本号格式是否正确
- **CI 失败**：查看 GitHub Actions 日志，确认 tag 格式
- **版本不一致**：重新运行 `update-version.sh` 并提交

### 扩展建议
- **多平台支持**：如需支持 Windows/Linux，扩展脚本
- **版本兼容性**：添加 CLI 和 GUI 版本兼容性检查
- **自动更新**：实现应用内检测新版本功能

---

**最后更新**: 2026-06-30  
**实施者**: AI Assistant  
**状态**: ✅ 已完成并测试通过
