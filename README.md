# 🦔 Mole GUI

> **Mole CLI 的图形化界面** —— 一个基于 Tauri 2 + React 的 macOS 系统清理工具桌面应用

Mole GUI 是 [Mole CLI](https://github.com/tw93/Mole) 的可视化客户端，将命令行清理工具包装成一个现代化的 macOS 桌面应用。用户无需记忆任何命令，即可完成磁盘清理、应用卸载、系统优化等操作。

---

## ✨ 功能特性

| 页面 | 功能 |
|------|------|
| 📊 **Dashboard** | 实时显示 CPU、内存、磁盘使用率，系统健康评分、主机信息 |
| 🧹 **Clean** | 清理用户缓存、应用缓存、日志等，支持预览（dry-run）后执行 |
| 🗑️ **Purge** | 深度清理残留文件，支持选择性删除目标 |
| 📦 **Uninstall** | 扫描已安装应用并彻底卸载（含关联文件），支持 Homebrew Cask |
| ⚡ **Optimize** | 列出并执行系统优化项（禁用不必要服务、清理 DNS 缓存等） |
| 🔍 **Analyze** | 可视化扫描磁盘大文件/大目录，支持直接移入废纸篓 |
| 📜 **History** | 查看历史操作记录（JSON 格式） |
| ⚙️ **Settings** | 配置 Mole CLI 可执行文件路径 |

---

## 🏗️ 技术栈

### 后端（Rust）
- **[Tauri 2](https://tauri.app/)** —— 跨平台桌面应用框架（类似 Electron，但内存占用极低）
- **[Tokio](https://tokio.rs/)** —— 异步运行时（用于非阻塞地执行 Mole CLI 子进程）
- **[Serde / serde_json](https://serde.rs/)** —— JSON 序列化/反序列化
- **tauri-plugin-store** —— 持久化键值存储（保存用户配置）
- **tauri-plugin-shell** —— Shell 命令执行支持

### 前端（TypeScript）
- **[React 18](https://react.dev/)** —— UI 框架
- **[Vite 6](https://vitejs.dev/)** —— 构建工具，开发服务器运行在端口 `1420`
- **[React Router v7](https://reactrouter.com/)** —— 前端路由（多页面导航）
- **[Zustand](https://zustand-demo.pmnd.rs/)** —— 轻量级状态管理
- **[Lucide React](https://lucide.dev/)** —— 图标库
- **[TailwindCSS 3](https://tailwindcss.com/)** —— 原子化 CSS 框架

---

## 📁 项目结构

```
Mole-GUI/
├── src/                        # 前端源代码（React + TypeScript）
│   ├── pages/                  # 各功能页面组件
│   │   ├── DashboardPage.tsx   # 系统状态仪表盘
│   │   ├── CleanPage.tsx       # 清理功能
│   │   ├── PurgePage.tsx       # 深度清理
│   │   ├── UninstallPage.tsx   # 应用卸载
│   │   ├── OptimizePage.tsx    # 系统优化
│   │   ├── AnalyzePage.tsx     # 磁盘分析
│   │   ├── HistoryPage.tsx     # 历史记录
│   │   └── SettingsPage.tsx    # 设置页面
│   ├── components/             # 可复用 UI 组件
│   ├── hooks/                  # 自定义 React Hooks
│   ├── lib/                    # 工具函数
│   ├── types/                  # TypeScript 类型定义
│   ├── i18n/                   # 国际化文案
│   └── App.tsx                 # 根组件 + 路由配置
│
├── tauri-gui/                  # 后端源代码（Rust）
│   ├── src/
│   │   ├── main.rs             # 程序入口
│   │   ├── lib.rs              # Tauri 应用启动 + 命令注册
│   │   ├── commands/
│   │   │   └── mod.rs          # 所有 Tauri 命令（前端可调用的后端函数）
│   │   └── mole/
│   │       ├── mod.rs          # 模块声明
│   │       ├── process.rs      # Mole CLI 进程管理（启动、流式读取、超时、取消）
│   │       ├── settings.rs     # 用户配置持久化（Mole CLI 路径）
│   │       └── sudo.rs         # macOS sudo 权限管理
│   ├── Cargo.toml              # Rust 依赖配置
│   └── tauri.conf.json         # Tauri 应用配置（窗口、图标、打包等）
│
├── index.html                  # HTML 入口文件
├── vite.config.ts              # Vite 构建配置
├── tailwind.config.js          # TailwindCSS 配置
└── package.json                # Node.js 依赖 + 脚本
```

---

## 🚀 快速开始

### ⚠️ macOS 安装说明（重要）

**如果你是从 GitHub Releases 下载 DMG 文件：**

由于本项目未购买 Apple Developer 账号，GitHub CI 构建的 DMG **未经过代码签名和公证**，macOS Gatekeeper 会阻止直接安装。

**请按照以下步骤完成安装：**

1. 下载 DMG 后，**右键点击** → 选择 `"Open"`（打开）
2. 系统弹出警告时，点击 `"Open"`（而不是 "Move to Trash"）
3. 输入管理员密码确认
4. 将 MoleGui.app 拖拽到 Applications 文件夹
5. ✅ 之后就可以正常使用了！

详细安装指南请查看：[INSTALLATION.md](INSTALLATION.md)

---

### 前置条件

1. **安装 Mole CLI**（核心依赖，没有它 GUI 无法运行）
   ```bash
   # 通过 Homebrew 安装
   brew install mole
   
   # 或手动安装后在 Settings 页面配置路径
   ```

2. **安装 Rust 工具链**（需要 1.75+）
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

3. **安装 Node.js 和 pnpm**
   ```bash
   # 安装 Node.js（推荐 20+）
   # 安装 pnpm
   npm install -g pnpm
   ```

### 开发模式运行

```bash
# 1. 克隆仓库
git clone https://github.com/your-username/Mole-GUI.git
cd Mole-GUI

# 2. 安装前端依赖
pnpm install

# 3. 启动开发模式（同时启动 Vite 开发服务器 + Tauri 窗口）
pnpm tauri dev
```

> 首次运行会编译 Rust 代码，需要几分钟，请耐心等待。

### 生产构建

```bash
# 构建可分发的 .app 和 .dmg 文件
pnpm tauri build
```

构建产物位于 `tauri-gui/target/release/bundle/` 目录。

---

## 📦 版本管理

Mole GUI 使用统一的版本管理机制，确保 Git tag、构建产物文件名和应用内部版本号保持一致。

**核心原则：Git tag 是唯一真实来源（Single Source of Truth）**

### 发布流程

1. **更新版本号**
   ```bash
   # 方法一：指定版本号
   ./scripts/update-version.sh 1.1.0
   
   # 方法二：从最新 Git tag 自动提取
   ./scripts/update-version.sh
   ```

2. **提交并创建标签**
   ```bash
   git add package.json tauri-gui/Cargo.toml tauri-gui/tauri.conf.json
   git commit -m "chore: bump version to 1.1.0"
   git tag v1.1.0
   git push origin main
   git push origin v1.1.0
   ```

3. **GitHub Actions 自动构建**
   
   推送 `v*` 格式的 tag 后，GitHub Actions 会自动：
   - 提取版本号并同步所有配置文件
   - 构建 macOS DMG 和 APP 包
   - 创建 GitHub Release，附带构建产物

详细文档请查看 [VERSION_MANAGEMENT.md](VERSION_MANAGEMENT.md)。

**快速入门**：查看 [QUICK_VERSION_GUIDE.md](QUICK_VERSION_GUIDE.md) 了解 3 步发布流程。

---

## 🔧 配置

### Mole CLI 路径

应用启动时会按以下顺序查找 Mole CLI：

1. 用户在 **Settings** 页面手动配置的路径
2. 系统 `PATH` 中的 `mole`（通过 `which mole` 查找）
3. 常见安装路径：`/opt/homebrew/bin/mole`、`/usr/local/bin/mole`、`/usr/bin/mole`
4. `~/.local/bin/mole`
5. 开发模式：从可执行文件目录向上查找兄弟项目中的 `mole` 二进制

如果自动查找失败，可以在 Settings 页面手动指定路径。

### Tauri 应用配置

关键配置项（`tauri-gui/tauri.conf.json`）：

| 配置项 | 值 | 说明 |
|--------|----|------|
| `productName` | `Mole` | 应用名称 |
| `version` | `1.0.0` | 应用版本 |
| `minimumSystemVersion` | `11.0` | 最低支持 macOS 版本（Big Sur） |
| `devUrl` | `http://localhost:1420` | 开发服务器地址 |

---

## 🏛️ 架构说明

### 前后端通信

```
前端 (React/JS)                    后端 (Rust)
      │                                  │
      │  invoke('command_name', args)    │
      │ ──────────────────────────────> │
      │                                  │  执行 Mole CLI 子进程
      │  window.listen('event-name')    │
      │ <────────────────── emit() ───── │  流式推送每行输出
      │                                  │
      │  ← Result<T, String>            │  返回最终结果
```

- **命令（Command）**：前端调用 `invoke()`，后端 `#[tauri::command]` 函数处理，返回结果
- **事件（Event）**：后端通过 `window.emit()` 主动推送实时数据给前端（用于流式输出）

### Rust 后端关键设计

- **进程管理**：使用 `tokio::process::Command` 异步启动 Mole CLI 子进程
- **流式读取**：通过 `tokio::io::BufReader` + `lines()` 逐行读取标准输出
- **防卡死**：单独的后台任务持续排空 stderr，防止 OS 管道缓冲区（~64KB）满导致子进程阻塞
- **超时控制**：`tokio::time::timeout` 包装读取操作，超时后强制 kill 进程
- **节流推送**：Analyze 扫描使用 100ms 批量推送，防止每秒数百事件涌入前端造成 UI 卡顿
- **取消机制**：`AtomicBool` 全局标志位，前端点击"停止"时将其设为 `true`，扫描循环检测到后退出

---

## 🔒 权限说明

部分功能需要管理员权限（如卸载应用、深度清理系统目录）。

应用使用 **macOS 原生密码对话框**（通过 `osascript` AppleScript）获取权限，而不是终端提示符，符合 macOS GUI 应用规范。

权限相关文件：
- `tauri-gui/entitlements.plist` —— 应用沙盒权限声明
- `tauri-gui/src/mole/sudo.rs` —— sudo 会话管理逻辑

---

## 📝 开发说明

### 添加新的 Tauri 命令

1. 在 `tauri-gui/src/commands/mod.rs` 中定义函数并添加 `#[tauri::command]`
2. 在 `tauri-gui/src/lib.rs` 的 `invoke_handler![]` 宏中注册新命令
3. 在前端通过 `invoke('command_name', { args })` 调用

### 代码规范

- Rust 代码遵循 `rustfmt` 默认格式化规则
- 前端代码使用 TypeScript 严格模式
- 所有 Rust 代码包含中文注释（面向 Java 开发者友好的说明）

---

## 📄 许可证

MIT License

---

## 🙏 致谢

- [Tauri](https://tauri.app/) —— 让 Rust + Web 技术构建桌面应用成为可能
- [Mole CLI](https://github.com/tw93/Mole) —— 核心清理工具
