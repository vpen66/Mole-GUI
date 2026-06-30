// ============================================================
// mole/sudo.rs —— macOS sudo 权限管理模块
// 负责检查、请求、清除 sudo（管理员）权限会话。
// 在 GUI 应用中不能弹出终端提示符，所以使用 osascript
// 来调用 macOS 原生密码对话框获取权限。
// ============================================================

// 引入标准库的 Stdio（标准输入/输出/错误流控制）
// 用于将子进程的输出流重定向（丢弃或捕获）
use std::process::Stdio;

// 引入 tokio 的异步进程命令（类似标准库的 std::process::Command，但支持 async/await）
// tokio 是 Rust 最流行的异步运行时，类似 Java 的 CompletableFuture + 线程池
use tokio::process::Command;

/// 检查当前是否存在有效的 sudo 权限会话（即用户最近是否已经输入过密码）。
///
/// 原理：运行 `sudo -n true`
///   -n 参数表示"非交互模式"：如果需要密码则直接失败，不弹出提示
///   如果命令成功（退出码 0），说明有缓存的 sudo 会话
///   如果失败，说明需要重新输入密码
///
/// 这是一个异步函数（async fn），需要用 .await 来等待结果
/// 返回值：bool —— true 表示有效，false 表示无效/过期
pub async fn check_sudo_session() -> bool {
    // match 相当于 Java 的 switch 表达式，但功能更强大（支持模式匹配）
    match Command::new("sudo")
        .arg("-n")            // 非交互模式参数
        .arg("true")          // 要执行的命令（shell 内置的 true，永远返回 0）
        .stdout(Stdio::null()) // 丢弃标准输出（我们不需要看输出内容）
        .stderr(Stdio::null()) // 丢弃标准错误（避免打印"password required"到终端）
        .status()             // 只获取退出状态码，不捕获输出
        .await                // 等待异步操作完成
    {
        // 命令成功启动，检查退出状态码
        // status.success() 等价于 exitCode == 0
        Ok(status) => status.success(),
        // 命令启动失败（比如系统找不到 sudo），则视为无权限
        Err(_) => false,
    }
}

/// 通过 macOS 原生 GUI 密码对话框请求管理员权限。
///
/// 在 Tauri GUI 应用中无法使用终端交互，所以使用 osascript（AppleScript 解释器）
/// 来弹出 macOS 系统级的密码验证对话框。
///
/// 返回值：Result<bool, String>
///   Ok(true)  —— 用户成功验证密码
///   Ok(false) —— 用户取消了对话框
///   Err(...)  —— 系统命令执行失败（osascript 本身出错）
pub async fn request_sudo_session() -> Result<bool, String> {
    // AppleScript 脚本内容：
    // "do shell script ... with administrator privileges" 会触发 macOS 系统密码弹窗
    // 这是 macOS 上 GUI 应用获取 root 权限的标准方式
    // r#"..."# 是 Rust 的原始字符串字面量（raw string literal）
    // 内部的引号不需要转义，相当于 Java 的 """ ... """（文本块）
    let script = r#"do shell script "echo ok" with administrator privileges"#;

    // 运行 osascript 命令（macOS 的 AppleScript/JavaScript for Automation 解释器）
    let output = Command::new("osascript")
        .arg("-e")       // -e 参数表示后面跟的是要执行的脚本内容（而不是脚本文件路径）
        .arg(script)     // 要执行的 AppleScript 脚本
        .stdout(Stdio::piped()) // 捕获标准输出（虽然这里不用，但保持管道打开）
        .stderr(Stdio::piped()) // 捕获标准错误
        .output()        // 等待命令完成并收集全部输出
        .await           // 异步等待
        // 如果命令启动失败，将错误转换为 String 并用 ? 向上抛出
        // map_err 相当于 Java 的 catch + rethrow，但以函数式风格书写
        .map_err(|e| format!("Failed to run osascript: {}", e))?;

    // 检查 osascript 的退出状态码
    // 用户成功输入密码 → exit 0 → success() = true
    // 用户点击取消   → exit 非0 → success() = false
    Ok(output.status.success())
}

/// 使当前的 sudo 权限会话失效（清除密码缓存）。
///
/// 运行 `sudo -k`：-k 参数会立即让所有缓存的 sudo 凭证过期，
/// 下次需要 sudo 时必须重新输入密码。
///
/// 这个函数没有返回值（不关心是否成功，因为失效操作的失败不影响程序正确性）
pub async fn stop_sudo_session() {
    // let _ = ... 表示"我知道这里有返回值，但我选择忽略它"
    // 这避免了 Rust 编译器发出"Result 未被处理"的警告
    // 相当于 Java 的：try { ... } catch (Exception ignored) {}
    let _ = Command::new("sudo")
        .arg("-k")             // 使所有 sudo 凭证立即过期
        .stdout(Stdio::null()) // 丢弃输出（不需要看）
        .stderr(Stdio::null()) // 丢弃错误（不需要看）
        .status()              // 只获取退出状态
        .await;                // 等待完成
}
