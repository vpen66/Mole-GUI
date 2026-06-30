// ============================================================
// mole/mod.rs —— mole 子模块的"目录文件"
// 在 Rust 中，mod.rs 的作用是声明这个目录下有哪些子模块
// 相当于 Java 里的 package-info.java，或者说是模块的"入口清单"
// ============================================================

// 声明 process 子模块（对应 src/mole/process.rs）
// pub 表示这个模块是公开的，外部代码（如 commands/mod.rs）可以访问它
// 这个模块负责：启动 Mole CLI 进程、流式读取输出、管理进程生命周期
pub mod process;

// 声明 settings 子模块（对应 src/mole/settings.rs）
// 负责：读写用户自定义的 Mole CLI 路径配置（持久化存储）
pub mod settings;

// 声明 sudo 子模块（对应 src/mole/sudo.rs）
// 负责：检查/请求/清除 macOS sudo 管理员权限会话
pub mod sudo;
