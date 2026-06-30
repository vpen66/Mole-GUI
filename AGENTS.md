# AGENTS.md — Mole GUI 项目 AI 协作规范

> 本文件用于指导 AI 编程助手（Antigravity、GitHub Copilot、Cursor 等）在本项目中的行为规范、代码风格和约束条件。

---

## 项目概述

**Mole GUI** 是一个基于 **Tauri 2 + React** 的 macOS 桌面应用，为 Mole CLI 提供图形化界面。

- **后端语言**：Rust（`tauri-gui/src/`）
- **前端语言**：TypeScript + React（`src/`）
- **构建工具**：Vite 6（前端）+ Cargo（后端）
- **包管理器**：pnpm（前端）

---

## 代码注释规范

### Rust 代码（强制要求）

> [!IMPORTANT]
> 本项目的主要开发者是 Java 背景的 Rust 新手，**所有 Rust 代码必须用中文写注释**，且注释要足够详细，解释 Rust 特有概念。

**必须注释的内容：**

1. **每个函数**：用 `///` 文档注释说明用途、参数、返回值
2. **每个结构体字段**：说明字段含义和单位（如"字节"还是"KB"）
3. **Rust 特有语法**：必须加行内注释解释，并类比 Java 概念

**注释对照模板（面向 Java 开发者）：**

```rust
// Rust 概念          Java 对应概念
// Option<T>      ≈   Optional<T>
// Result<T, E>   ≈   checked Exception（但更安全）
// Vec<T>         ≈   ArrayList<T>
// &str           ≈   String（只读引用）
// String         ≈   StringBuilder（拥有所有权）
// Mutex<T>       ≈   ReentrantLock / synchronized
// Arc<T>         ≈   线程安全的共享对象引用
// async fn       ≈   返回 CompletableFuture 的方法
// tokio::spawn   ≈   new Thread().start()
// match          ≈   switch（但更强大，支持模式匹配）
// if let Some(x) ≈   if (optional.isPresent()) { T x = optional.get(); }
// ?              ≈   if (result.isError()) throw result.getError();
// .clone()       ≈   对象的深拷贝（但 Arc.clone() 只增加引用计数）
// 'static        ≈   生命周期足够长，不依赖局部变量
```

**注释示例（正确做法）：**

```rust
/// 从持久化存储中读取用户配置的 Mole CLI 路径。
///
/// 参数：app — Tauri 应用句柄（相当于 Spring 的 ApplicationContext）
/// 返回：Option<PathBuf>，Some(路径) 表示找到，None 表示未配置
pub fn get_configured_mole_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    // .ok()? 的含义：
    //   - .ok() 将 Result<T,E> 转换为 Option<T>（出错时变成 None）
    //   - ?    如果是 None，立即从函数返回 None（相当于 Java 的 if (x == null) return null）
    let store = app.store(STORE_PATH).ok()?;
    ...
}
```

### TypeScript / React 代码

- 函数和组件加 JSDoc 注释（英文或中文均可）
- 复杂业务逻辑加行内注释说明意图
- 类型定义需要注释每个字段的含义

---

## 架构约束

### 后端（Rust）

**文件职责划分（不要打破这个边界）：**

| 文件 | 职责 | 不应该包含 |
|------|------|-----------|
| `commands/mod.rs` | Tauri 命令定义、数据结构、JSON 解析 | 直接的进程操作 |
| `mole/process.rs` | 子进程启动、流式读取、超时、取消 | 业务逻辑、JSON 解析 |
| `mole/settings.rs` | 配置读写（只操作 tauri-plugin-store） | 进程操作、业务逻辑 |
| `mole/sudo.rs` | sudo 权限管理（只调用 osascript/sudo） | 其他任何逻辑 |

**添加新 Tauri 命令的步骤（必须按此顺序）：**

1. 在 `mole/process.rs` 中添加底层进程调用函数（如果需要）
2. 在 `commands/mod.rs` 中定义数据结构和 `#[tauri::command]` 函数
3. 在 `lib.rs` 的 `invoke_handler![]` 中注册命令
4. 更新前端的 TypeScript 调用代码

**全局状态管理规则：**

- 全局 `static` 变量必须用 `Mutex<T>` 或 `AtomicT` 保护
- 当前已有的全局状态：
  - `ANALYZE_TASK` —— 当前 analyze 任务 ID
  - `NEXT_REQUEST_ID` —— 请求 ID 计数器
  - `ANALYZE_CHILD_PID` —— analyze 子进程 PID
  - `CANCEL_ANALYZE` —— analyze 取消标志

> [!WARNING]
> 不要在没有锁保护的情况下读写全局变量，Rust 编译器会拒绝编译，但如果使用 unsafe 则可能造成数据竞争。

### 前端（TypeScript）

**页面文件对应关系：**

| 页面文件 | 对应 Tauri 命令 |
|---------|----------------|
| `DashboardPage.tsx` | `get_system_status`, `get_free_space_kb` |
| `CleanPage.tsx` | `clean_dry_run`, `clean_execute` |
| `PurgePage.tsx` | `purge_dry_run`, `purge_execute` |
| `UninstallPage.tsx` | `uninstall_scan_apps`, `uninstall_execute` |
| `OptimizePage.tsx` | `optimize_dry_run`, `optimize_execute` |
| `AnalyzePage.tsx` | `analyze_scan`, `cancel_analyze_scan`, `analyze_delete` |
| `HistoryPage.tsx` | `get_history` |
| `SettingsPage.tsx` | `get_mole_path_config`, `set_mole_path_config` |

**事件监听规则：**

后端通过 `window.emit()` 推送实时数据，前端使用 `listen()` 订阅。事件名称规范：

```
mole-{command_name}-event
```

例：`mole-clean_dry_run-event`、`mole-analyze_scan-event`

---

## 开发工作流

### 验证代码正确性

每次修改 Rust 代码后，必须运行：

```bash
cd tauri-gui
cargo check
```

在提交或 PR 前运行：

```bash
cd tauri-gui
cargo clippy -- -D warnings
```

### 运行开发环境

```bash
# 在项目根目录执行
pnpm tauri dev
```

### 构建生产包

```bash
pnpm tauri build
```

---

## 安全规范

> [!CAUTION]
> 以下操作涉及文件删除和系统权限，修改时需格外谨慎。

1. **路径验证**：所有来自前端的文件路径，在执行删除前必须经过 `validate_path()` 函数验证：
   - 不能为空
   - 必须是绝对路径（以 `/` 开头）
   - 不能包含 `null` 字节
   - 不能包含路径穿越（`..`）

2. **权限提升**：需要 sudo 的操作必须通过 `osascript` 弹出 macOS 原生密码对话框，**禁止**直接在终端输出密码提示符

3. **移入废纸篓**：删除用户文件时，优先通过 Finder AppleScript `move to trash`，而不是 `rm -rf`

4. **进程管理**：同一时间只允许一个 `mole analyze` 进程运行，新的扫描请求必须先杀掉旧进程

---

## 依赖版本约束

> [!NOTE]
> 以下依赖版本已经过验证，升级前需要测试。

**Rust（Cargo.toml）：**

| 依赖 | 当前版本 | 注意事项 |
|------|---------|---------|
| `tauri` | `2.x` | Tauri v1 和 v2 API 不兼容，不要降级 |
| `tokio` | `1.x` | 必须启用 `full` feature |
| `serde` | `1.x` | 必须启用 `derive` feature |
| `tauri-plugin-store` | `2.x` | 必须与 Tauri 主版本匹配 |

**Node.js（package.json）：**

| 依赖 | 当前版本 | 注意事项 |
|------|---------|---------|
| `@tauri-apps/api` | `^2.5.0` | 必须与 Rust tauri 版本匹配 |
| `react` | `^18.3.1` | 使用 React 18 并发特性 |
| `tailwindcss` | `^3.4.17` | 使用 v3，与 v4 配置不兼容 |

---

## 常见问题与解决

### 编译报错：`cannot move out of ... which is behind a shared reference`

这是 Rust 所有权问题。解决方案：在传入闭包前先 `.clone()` 需要的值。

```rust
// 错误写法
tokio::spawn(async move {
    do_something(window);  // window 被 move 了
    do_something(app);     // app 已经被 move，这里编译失败
});

// 正确写法
let window_clone = window.clone();
let app_clone = app.clone();
tokio::spawn(async move {
    do_something(window_clone);
    do_something(app_clone);
});
```

### 子进程卡死（无输出）

原因通常是 stderr 管道缓冲区满了。确保在启动子进程后立即调用 `drain_stderr(&mut child)`。

### `Mutex` 锁中毒（poisoned）

当持有锁的线程 panic 时，Mutex 会变为中毒状态。使用 `unwrap_or_else(|e| e.into_inner())` 恢复数据：

```rust
let data = MY_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
```

---

## AI 助手行为规范

1. **修改 Rust 文件时，必须同步添加中文注释**，解释修改内容和涉及的 Rust 概念
2. **不要使用 `unsafe` 代码块**，除非有充分理由并附详细注释
3. **不要引入新的全局状态**，除非与现有的 analyze 相关全局变量有相同的保护机制
4. **修改进程管理逻辑时**，确保超时和取消机制仍然正确运作
5. **每次修改后**，在 `tauri-gui/` 目录下运行 `cargo check` 验证编译
6. **前端组件修改**时，保持与对应 Tauri 命令的事件名称一致
7. **不要修改 `entitlements.plist`** 中的权限声明，除非用户明确要求
