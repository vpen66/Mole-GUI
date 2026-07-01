// ============================================================
// lib.rs —— Tauri 应用的"库入口"文件
// 相当于 Java 里的 Application.java 或 Spring Boot 的启动类
// ============================================================

// 声明子模块：告诉 Rust 编译器去找 src/commands/mod.rs 文件
// 相当于 Java 里的 import com.example.commands.*;
mod commands;

// 声明子模块：告诉 Rust 编译器去找 src/mole/mod.rs 文件
mod mole;

// 将 commands 模块下的所有公开符号（函数、结构体等）引入当前作用域
// 相当于 Java 里的 import com.example.commands.*;（通配符导入）
use commands::*;

// 条件编译属性：如果目标平台是移动端（iOS/Android），使用 tauri 的移动端入口点宏
// 普通桌面端不需要关心这个，它只在编译移动端时生效
#[cfg_attr(mobile, tauri::mobile_entry_point)]
/// run() 是整个 Tauri 应用的启动函数，由 main.rs 调用
/// pub 表示这个函数是公开的，可以被外部（main.rs）访问
/// 相当于 Java 中的 public void run()
pub fn run() {
    // 使用 Tauri 的建造者模式（Builder Pattern）配置并启动应用
    // 类似 Java Spring Boot 里的 SpringApplication.run() 或 Vert.x 的 Vertx.vertx()
    tauri::Builder::default()
        // 注册 Shell 插件：允许应用执行系统 shell 命令（如 which、kill 等）
        .plugin(tauri_plugin_shell::init())
        // 注册 Store 插件：提供持久化键值存储功能（用于保存用户配置，如 Mole CLI 路径）
        // 类似 Java 里的 Properties 文件或 SQLite
        .plugin(tauri_plugin_store::Builder::default().build())
        // 注册所有 Tauri 命令（即前端 JS 可以调用的后端函数）
        // 相当于注册 REST 接口，但走的是 IPC 通道而不是 HTTP
        .invoke_handler(tauri::generate_handler![
            get_mole_version,        // 获取 Mole CLI 版本
            get_gui_version,         // 获取 GUI 应用自身版本
            get_free_space_kb,       // 获取磁盘剩余空间（KB）
            get_system_status,       // 获取系统状态（CPU、内存、磁盘等）
            clean_dry_run,           // 清理预览（不实际删除，只列出会删什么）
            clean_execute,           // 执行实际清理
            uninstall_scan_apps,     // 扫描可卸载的应用
            uninstall_preview,       // 预览卸载（新增）
            uninstall_execute,       // 执行卸载
            purge_dry_run,           // 深度清理预览
            purge_execute,           // 执行深度清理
            optimize_dry_run,        // 系统优化预览
            optimize_execute,        // 执行系统优化
            analyze_scan,            // 扫描磁盘大文件/目录（分析模式）
            cancel_analyze_scan,     // 取消正在进行的分析扫描
            analyze_delete,          // 删除分析结果中选中的文件
            get_history,             // 获取操作历史记录
            check_sudo_session,      // 检查 sudo 权限会话是否有效
            request_sudo_session,    // 请求 sudo 权限（弹出 macOS 密码对话框）
            stop_sudo_session,       // 停止/失效 sudo 会话
            get_mole_path_config,    // 获取 Mole CLI 的路径配置
            set_mole_path_config,    // 设置 Mole CLI 的自定义路径
            get_mole_use_json,       // 获取是否使用 JSON 输出
            set_mole_use_json,       // 设置是否使用 JSON 输出
            check_full_disk_access,  // 检查完全磁盘访问权限 (FDA)
            get_whitelist_config,    // 获取白名单配置
            save_whitelist_config,   // 保存白名单配置
            get_purge_paths,         // 获取项目扫描路径
            save_purge_paths,        // 保存项目扫描路径
            get_touchid_status,      // 获取 Touch ID 状态
            set_touchid_enabled,     // 启用/禁用 Touch ID
            open_fda_settings,       // 打开完全磁盘访问权限设置
            open_path_in_finder,     // 在访达中打开指定路径
            get_directory_entries,   // 获取子目录直接子项
            get_overview_dirs,       // 获取概览扫描白名单
            set_overview_dirs,       // 设置概览扫描白名单
        ])
        // 应用初始化回调：在应用窗口创建之前执行的设置逻辑
        // |app| 是一个闭包（相当于 Java 的 Lambda 或匿名内部类），app 是 Tauri 应用实例
        .setup(|app| {
            // 创建主窗口
            // _window 前面的下划线表示"我知道这个变量存在，但暂时不会直接使用它"
            // 这样可以避免 Rust 编译器发出"未使用变量"的警告
            let _window = tauri::WebviewWindowBuilder::new(
                app,                                          // Tauri 应用实例
                "main",                                       // 窗口的唯一标识符（ID）
                tauri::WebviewUrl::App("index.html".into()),  // 加载的前端页面路径
            )
            .title("Mole")                   // 窗口标题栏显示的文字
            .inner_size(1100.0, 750.0)       // 默认窗口大小：宽 1100px，高 750px
            .min_inner_size(800.0, 600.0)    // 最小窗口大小：宽 800px，高 600px
            .resizable(true)                 // 允许用户调整窗口大小
            .center()                        // 窗口首次打开时居中显示
            .decorations(true)               // 显示系统原生窗口装饰（标题栏、关闭按钮等）
            .build()?;                       // 构建窗口；? 表示如果构建失败则向上抛出错误

            // Ok(()) 是 Rust 中表示"成功、没有返回值"的写法
            // 相当于 Java 里 void 方法正常返回（无异常）
            Ok(())
        })
        // 启动 Tauri 应用，传入自动生成的上下文（包含 tauri.conf.json 的配置）
        .run(tauri::generate_context!())
        // 如果应用运行时出现错误，打印错误信息并退出程序
        // expect 类似 Java 里的 assert 或 Objects.requireNonNull
        .expect("error while running tauri application");
}
