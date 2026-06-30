// ============================================================
// mole/process.rs —— Mole CLI 进程管理核心模块
//
// 这个文件负责：
//   1. 定位系统中 Mole CLI 可执行文件的路径
//   2. 启动 Mole CLI 子进程并流式读取其输出
//   3. 管理进程超时、取消和 PID 追踪
//
// 关键 Rust 概念提示（对 Java 开发者）：
//   - async/await  ≈ Java 的 CompletableFuture + async
//   - Option<T>    ≈ Java 的 Optional<T>
//   - Result<T,E>  ≈ Java 的 checked exception（但更安全）
//   - &str         ≈ Java 的 String（只读引用，不拥有内存）
//   - String       ≈ Java 的 StringBuilder（拥有内存，可修改）
//   - Vec<T>       ≈ Java 的 ArrayList<T>
//   - Mutex<T>     ≈ Java 的 synchronized 块 / ReentrantLock
// ============================================================

// 引入标准库的路径类型：PathBuf 是可变路径（相当于 Java 的 java.io.File）
use std::path::PathBuf;
// 引入标准库的进程 I/O 流控制（Stdio::piped() = 创建管道, Stdio::null() = 丢弃）
use std::process::Stdio;
// 引入原子布尔类型（线程安全的 bool，无需锁）和内存排序选项
// AtomicBool ≈ Java 的 AtomicBoolean
// Ordering::SeqCst = 最严格的内存顺序，保证所有线程看到一致的值
use std::sync::atomic::{AtomicBool, Ordering};
// 引入互斥锁（Mutex）：保护共享数据，保证同一时间只有一个线程能访问
// Mutex<T> ≈ Java 的 synchronized(obj) 或 ReentrantLock
use std::sync::Mutex;
// 引入时间间隔类型（Duration），用于表示超时时长
use std::time::Duration;
// 引入 tokio 异步 I/O：异步行读取器 + 异步读取 trait + 带缓冲区的读取器
// BufReader 相当于 Java 的 BufferedReader，用于按行读取流式输出
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
// 引入 tokio 的异步进程命令（tokio 版的 std::process::Command，支持 async/await）
use tokio::process::Command;

// 引入同模块下的 settings 子模块（用于读取用户配置的 Mole 路径）
use super::settings;

// ============================================================
// 常量定义
// ============================================================

/// 空闲超时：如果 Mole 进程在这么多秒内没有输出任何内容，
/// 则认为它卡住了（可能被 stderr 的管道缓冲区撑满而阻塞），直接杀掉它。
/// OS 管道缓冲区大约 64KB，如果 stderr 填满而没人读，进程会卡死。
const IDLE_TIMEOUT_SECS: u64 = 60;

// ============================================================
// 辅助函数
// ============================================================

/// 启动一个后台任务，专门负责排空（drain）子进程的 stderr 输出流。
///
/// 为什么需要这个？
/// 操作系统的管道缓冲区大约只有 64KB。如果子进程向 stderr 写了很多内容
/// 但没人读，缓冲区满了之后，子进程的 write() 系统调用会阻塞，
/// 导致整个子进程卡死。这个函数通过异步读取 stderr 来防止这种情况。
///
/// 参数：child —— 对子进程的可变引用（&mut 表示独占的可变借用，类似 Java 的传对象引用）
fn drain_stderr(child: &mut tokio::process::Child) {
    // take() 从 child.stderr 中取出 stderr 流的所有权，并将 child.stderr 设为 None
    // 这类似 Java 的 stream = process.getErrorStream(); process.errorStream = null;
    // if let Some(stderr) = ... 是模式匹配：只有当 stderr 不为 None 时才执行 if 块
    if let Some(stderr) = child.stderr.take() {
        // tokio::spawn 启动一个新的异步任务（类似 Java 的 new Thread(...).start()）
        // async move {...} 是异步闭包（move 表示将外部变量的所有权移入闭包）
        tokio::spawn(async move {
            // 分配一个 4096 字节（4KB）的缓冲区，用于批量读取 stderr 数据
            // vec![0u8; 4096] 创建一个长度为 4096、值全为 0 的字节数组
            // u8 = 无符号 8 位整数，即一个字节（相当于 Java 的 byte，但无符号）
            let mut buf = vec![0u8; 4096];
            // 将 stderr 赋给局部变量（用于循环读取）
            let mut reader = stderr;
            // loop 是 Rust 的无限循环（相当于 Java 的 while(true)）
            loop {
                // reader.read(&mut buf) 异步读取数据到 buf
                // &mut buf 表示传入 buf 的可变引用（允许函数修改 buf 的内容）
                match reader.read(&mut buf).await {
                    // 读取到 0 字节（EOF，流结束）或出错（Err）时，退出循环
                    // | 在 match 中表示"或"，两种情况统一处理
                    Ok(0) | Err(_) => break,
                    // 成功读取了 n 个字节
                    Ok(n) => {
                        // 将 buf 中前 n 个字节转换为字符串（允许非 UTF-8 字节，用 ? 替代）
                        // &buf[..n] 是切片语法，取 buf 的前 n 个元素（相当于 Arrays.copyOf(buf, n)）
                        let snippet = String::from_utf8_lossy(&buf[..n]);
                        // 将 stderr 内容打印到我们自己的终端（调试用），去掉末尾空白
                        eprintln!("[mole stderr] {}", snippet.trim_end());
                    }
                }
            }
        });
    }
}

// ============================================================
// 全局状态（使用 Mutex 保护的线程安全单例）
// ============================================================

/// 全局单例：记录当前正在运行的 analyze 扫描任务的 ID。
/// 确保同一时间只有一个 analyze 扫描在运行，旧的会被新的取消。
/// static 变量是全局的，整个程序生命周期内只有一份（相当于 Java 的 static 字段）
/// Mutex<Option<u64>>：
///   - Mutex 保证线程安全访问
///   - Option<u64>：None = 没有任务在运行，Some(id) = 当前任务的 ID
static ANALYZE_TASK: Mutex<Option<u64>> = Mutex::new(None);

/// 全局单例：用于生成递增的唯一请求 ID。
/// 每次新的 analyze 请求到来时，这个计数器自增，产生唯一 ID。
static NEXT_REQUEST_ID: Mutex<u64> = Mutex::new(0);

/// 全局单例：记录当前正在运行的 analyze 子进程的 PID（进程 ID）。
/// 当新的 analyze_scan 被调用时，用这个 PID 来杀掉旧的进程。
/// u32 是 32 位无符号整数，进程 ID 在 Unix 上通常是 u32
static ANALYZE_CHILD_PID: Mutex<Option<u32>> = Mutex::new(None);

// ============================================================
// 数据结构定义
// ============================================================

/// 流式执行的结果，包含退出码、是否超时、是否被取消三个状态。
/// pub struct 定义一个公开的结构体，相当于 Java 的 public class（只有字段，没有方法）
pub struct StreamingResult {
    /// 进程退出码（0 = 成功，非 0 = 失败，-1 = 超时/强制终止）
    pub exit_code: i32,
    /// true 表示进程因超时被杀掉
    pub timed_out: bool,
    /// true 表示进程因用户取消被杀掉
    pub cancelled: bool,
}

// ============================================================
// 核心功能函数
// ============================================================

/// 在系统上查找 Mole CLI 可执行文件的路径。
/// 按优先级依次尝试以下方式：
///   0. 用户在设置中手动配置的路径
///   1. 通过 `which mole` 命令在 PATH 中查找
///   2. 检查常见的安装路径（/opt/homebrew/bin、/usr/local/bin 等）
///   3. 检查 ~/.local/bin 目录
///   4. 沿可执行文件目录向上查找，支持开发环境（Mole-GUI 和 mole 是兄弟目录）
///
/// 参数：app —— 可选的 Tauri 应用句柄，有时调用方没有 AppHandle 所以是 Option
/// 返回：Option<PathBuf> —— Some(路径) 表示找到了，None 表示找不到
pub fn find_mole_path(app: Option<&tauri::AppHandle>) -> Option<PathBuf> {
    // ── 步骤 0：检查用户手动配置的路径 ──────────────────────────
    // if let Some(app_handle) = app 是模式解构：
    //   如果 app 是 Some(x)，则将 x 绑定到 app_handle 并进入 if 块
    //   如果 app 是 None，跳过 if 块
    //   相当于 Java 的：if (app != null) { AppHandle appHandle = app; ... }
    if let Some(app_handle) = app {
        if let Some(configured_path) = settings::get_configured_mole_path(app_handle) {
            eprintln!("[mole-gui] Using user-configured mole path: {}", configured_path.display());
            // return Some(configured_path) 表示函数在这里提前返回，包裹在 Some 里
            return Some(configured_path);
        }
    }

    // ── 步骤 1：通过 `which mole` 在 PATH 中查找 ────────────────
    // std::process::Command（注意：这里用的是同步版本，不是 tokio 的异步版本）
    if let Ok(output) = std::process::Command::new("which")
        .arg("mole")
        .output()   // output() 同步等待命令完成并收集全部输出
    {
        if output.status.success() {
            // output.stdout 是字节切片（Vec<u8>），转换为字符串（允许非 UTF-8 字节）
            let path_str = String::from_utf8_lossy(&output.stdout);
            // trim() 去掉首尾的空白字符（包括 `which` 输出末尾的换行符）
            let path = PathBuf::from(path_str.trim());
            if path.exists() {
                eprintln!("[mole-gui] Found mole CLI via which: {}", path.display());
                return Some(path);
            }
        }
    }

    // ── 步骤 2：检查常见的安装路径 ─────────────────────────────
    // 数组字面量：&[T] 是切片类型，这里定义了 3 个字符串引用的数组
    let candidates = [
        "/opt/homebrew/bin/mole", // Homebrew 在 Apple Silicon Mac 上的默认路径
        "/usr/local/bin/mole",    // Homebrew 在 Intel Mac 上的默认路径
        "/usr/bin/mole",          // 系统默认 bin 目录
    ];
    // for...in 循环迭代数组中的每个元素
    // candidate 的类型是 &&str（数组元素本身是 &str，for 循环再取引用）
    for candidate in candidates {
        let path = PathBuf::from(candidate);
        if path.exists() {
            return Some(path);
        }
    }

    // ── 步骤 3：检查 ~/.local/bin ───────────────────────────────
    // std::env::var("HOME") 读取环境变量，返回 Result<String, VarError>
    // if let Ok(home) = ... 只处理成功的情况
    if let Ok(home) = std::env::var("HOME") {
        // format! 宏用于字符串格式化，相当于 Java 的 String.format() 或 "" + ""
        let local_bin = PathBuf::from(format!("{}/.local/bin/mole", home));
        if local_bin.exists() {
            return Some(local_bin);
        }
    }

    // ── 步骤 4：沿当前可执行文件的目录向上查找（开发模式）─────
    // std::env::current_exe() 获取当前运行的可执行文件路径
    if let Ok(exe) = std::env::current_exe() {
        // as_path() 将 PathBuf 转换为 &Path（不可变路径引用）
        let mut dir = exe.as_path();
        // while let Some(parent) = dir.parent() 循环向上遍历目录：
        //   dir.parent() 返回当前路径的父目录
        //   当到达根目录时 parent() 返回 None，循环结束
        while let Some(parent) = dir.parent() {
            dir = parent;

            // 检查：<祖先目录>/mole/mole（适合 mole 项目内的情况）
            let inside_mole = dir.join("mole/mole");
            if inside_mole.is_file() {
                eprintln!("[mole-gui] Found mole CLI inside ancestor: {}", inside_mole.display());
                return Some(inside_mole);
            }

            // 检查：<祖先目录>/../mole/mole（适合 Mole-GUI 和 mole 是兄弟目录的情况）
            let sibling_mole = dir.join("../mole/mole");
            if sibling_mole.is_file() {
                eprintln!("[mole-gui] Found mole CLI as sibling: {}", sibling_mole.display());
                return Some(sibling_mole);
            }
        }
    }

    // 所有方式都找不到，返回 None
    None
}

/// 执行 Mole CLI 命令，逐行流式读取标准输出。
///
/// 这个函数是泛型函数（`<F>` 是类型参数，类似 Java 的 `<T>`）：
///   F 必须满足 `FnMut(String) + Send + 'static` 约束：
///     FnMut(String) — 可以被多次调用（mut），接收一个 String 参数的闭包
///     Send          — 可以安全地跨线程传递（类似 Java 的线程安全）
///     'static       — 闭包中不能持有局部变量的引用（生命周期必须足够长）
///
/// 参数：
///   app     — 可选的应用句柄
///   args    — 传给 Mole CLI 的命令行参数（如 &["clean", "--dry-run"]）
///   on_line — 每读到一行输出就调用一次的回调函数（相当于 Java 的 Consumer<String>）
///
/// 返回：Result<i32, String> —— Ok(退出码) 或 Err(错误信息)
///
/// #[allow(dead_code)] 告诉编译器：即使这个函数现在没有被调用，也不要发出警告
/// （因为它可能在未来被用到，或者在其他条件下被调用）
#[allow(dead_code)]
pub async fn run_mole_streaming<F>(
    app: Option<&tauri::AppHandle>,
    args: &[&str],          // &[&str] 是字符串切片的切片，相当于 Java 的 String[]
    mut on_line: F,         // mut 表示 on_line 这个变量可被修改（FnMut 要求）
) -> Result<i32, String>
where
    // where 子句用于声明泛型约束（类似 Java 的 <F extends Function & Serializable>）
    F: FnMut(String) + Send + 'static,
{
    // 查找 Mole CLI 路径，找不到则返回错误
    // ok_or_else 将 None 转换为 Err（只在需要时才执行闭包，比 ok_or 更高效）
    let mole_path = find_mole_path(app).ok_or_else(|| {
        "Mole CLI not found. Please install it first or configure the path in Settings.".to_string()
    })?;

    // 启动 Mole CLI 子进程
    let mut child = Command::new(&mole_path)
        .args(args)               // 传入所有参数
        .env("LC_ALL", "C")       // 设置区域设置为 C（确保输出为 ASCII，避免多语言问题）
        .env("NO_COLOR", "1")     // 禁用颜色输出（ANSI 转义码会干扰文本解析）
        .stdout(Stdio::piped())   // 将标准输出重定向到管道（我们要读取它）
        .stderr(Stdio::piped())   // 将标准错误也重定向到管道（防止缓冲区满）
        .spawn()                  // 启动进程（异步，立即返回，不等待进程结束）
        // map_err 将 io::Error 转换为 String 格式的错误信息
        // |e| 是闭包的参数（e 是错误对象），相当于 Java 的 e -> "..." + e
        .map_err(|e| format!("Failed to start Mole: {}", e))?;

    // 在后台启动一个任务来持续读取 stderr，防止管道缓冲区撑满导致子进程阻塞
    drain_stderr(&mut child);

    // 取出子进程的 stdout 流（take() 移走所有权，child.stdout 变为 None）
    let stdout = child
        .stdout
        .take()
        // ok_or_else 将 None 转为 Err
        .ok_or_else(|| "Failed to capture stdout".to_string())?;

    // 用 BufReader 包装 stdout，以支持按行读取（类似 Java 的 new BufferedReader(new InputStreamReader(stdout))）
    let reader = BufReader::new(stdout);
    // lines() 返回一个异步行迭代器
    let mut lines = reader.lines();

    // 循环读取每一行输出
    // while let Ok(Some(line)) 是模式匹配的循环：
    //   lines.next_line().await 返回 Result<Option<String>, io::Error>
    //   Ok(Some(line)) = 成功读到一行，绑定到 line
    //   Ok(None) 或 Err = 退出循环
    while let Ok(Some(line)) = lines.next_line().await {
        on_line(line); // 调用回调函数处理这一行
    }

    // 等待子进程完全结束，获取退出状态
    let status = child
        .wait()
        .await
        .map_err(|e| format!("Failed to wait for Mole: {}", e))?;

    // status.code() 返回 Option<i32>（如果进程被信号杀死则为 None）
    // unwrap_or(-1) 表示：如果是 None，则使用 -1 作为默认值
    Ok(status.code().unwrap_or(-1))
}

/// 执行 Mole CLI 命令，带有总体超时控制。
///
/// 如果在 `timeout_secs` 秒内命令未完成，进程会被强制杀死，
/// 并在返回的 `StreamingResult` 中设置 `timed_out = true`。
/// 超时前接收到的输出仍然通过回调函数正常传递。
///
/// 此函数还支持 analyze 命令的取消机制：
/// 如果一个新的 analyze 请求到来，旧的请求会被取消。
pub async fn run_mole_streaming_with_timeout<F>(
    app: Option<&tauri::AppHandle>,
    args: &[&str],
    timeout_secs: u64,  // u64 = 64位无符号整数，用于表示秒数
    mut on_line: F,
) -> Result<StreamingResult, String>
where
    F: FnMut(String) + Send + 'static,
{
    let mole_path = find_mole_path(app).ok_or_else(|| {
        "Mole CLI not found. Please install it first or configure the path in Settings.".to_string()
    })?;

    // 检查是否是 analyze 命令（analyze 命令需要额外的取消/追踪逻辑）
    // args.first() 返回切片的第一个元素 Option<&&str>
    // map(|s| *s == "analyze") 解引用并比较字符串
    // unwrap_or(false) 如果切片为空则默认为 false
    let is_analyze = args.first().map(|s| *s == "analyze").unwrap_or(false);

    // 如果是 analyze 命令，分配一个新的唯一请求 ID 并取消任何已有的 analyze 任务
    let request_id = if is_analyze {
        let new_id = get_next_request_id();
        cancel_existing_analyze_task(new_id); // 通知旧任务：你已被取代
        new_id
    } else {
        0 // 非 analyze 命令使用 ID 0（不参与 analyze 任务追踪）
    };

    // 启动子进程
    let mut child = Command::new(&mole_path)
        .args(args)
        .env("LC_ALL", "C")
        .env("NO_COLOR", "1")
        // 设置环境变量 MOLE_TIMEOUT_HINT_SCAN_SEC 为 2 秒，限制项目构建产物扫描的耗时
        // 这样可以避免每次扫描都卡顿 15+ 秒，大幅提升系统清理页面的响应速度
        .env("MOLE_TIMEOUT_HINT_SCAN_SEC", "2")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start Mole: {}", e))?;

    drain_stderr(&mut child);

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture stdout".to_string())?;

    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();

    // 将超时秒数转换为 Duration 类型
    let timeout_duration = Duration::from_secs(timeout_secs);
    // 标记是否发生了超时
    let mut timed_out = false;

    // 主读取循环
    loop {
        // 如果是 analyze 任务，检查是否已被更新的请求取消
        if is_analyze && is_task_cancelled(request_id) {
            eprintln!("[mole-gui] Analyze task #{} was cancelled by newer request", request_id);
            let _ = child.kill().await; // 强制杀死子进程
            return Err(format!("Scan cancelled by new request #{}", request_id));
        }

        // tokio::time::timeout 包装异步操作，添加超时控制
        // 如果 lines.next_line() 在 timeout_duration 内没有返回，则返回 Err(Elapsed)
        // 这类似 Java 的 future.get(timeout, TimeUnit.SECONDS)
        match tokio::time::timeout(timeout_duration, lines.next_line()).await {
            // 成功读到一行（注意 Ok(Ok(Some(line))) 是三层包裹：timeout/IO/Option）
            Ok(Ok(Some(line))) => {
                on_line(line);
            }
            // 读到 EOF（流结束，进程正常输出完毕）
            Ok(Ok(None)) => {
                break; // 退出循环
            }
            // 读取发生 I/O 错误，也当作 EOF 处理
            Ok(Err(_e)) => {
                break;
            }
            // 超时：timeout_duration 内没有读到任何数据
            // _elapsed 是超时错误对象，前缀 _ 表示我们不使用它
            Err(_elapsed) => {
                timed_out = true;
                let _ = child.kill().await; // 超时则强制杀死进程
                break;
            }
        }
    }

    // 如果是 analyze 任务，完成后清除追踪记录（仅当这仍是最新任务时）
    if is_analyze {
        clear_analyze_task_if_current(request_id);
    }

    // 获取退出码
    let exit_code = if timed_out {
        -1 // 超时时没有正常退出码，使用 -1 表示异常
    } else {
        // 等待进程自然结束并获取退出码
        let status = child
            .wait()
            .await
            .map_err(|e| format!("Failed to wait for Mole: {}", e))?;
        status.code().unwrap_or(-1)
    };

    // 构建并返回结果结构体
    // 注意：这里 cancelled 永远是 false，因为取消的情况在循环里已经提前 return 了
    Ok(StreamingResult {
        exit_code,
        timed_out,
        cancelled: false,
    })
}

/// 执行 Mole CLI 命令，带有节流（throttle）的批量事件推送和取消支持。
///
/// 与 `run_mole_streaming_with_timeout` 的区别：
/// - 输出行先积累到内存缓冲区，每 100ms 统一推送一批（而不是每行单独推送）
/// - 这样可以防止每秒数百个事件涌向前端，避免 UI 卡顿（"洪水"问题）
/// - 支持通过原子标志位（`cancel_flag`）随时取消
///
/// `on_batch` 回调接收的是 `&[String]`（字符串切片），即一批行的只读引用
pub async fn run_mole_streaming_throttled<F>(
    app: Option<&tauri::AppHandle>,
    args: &[&str],
    timeout_secs: u64,
    cancel_flag: &AtomicBool,  // 原子布尔标志位，共享引用（&），用于跨任务发送取消信号
    mut on_batch: F,           // 批量回调：每次接收一组行
) -> Result<StreamingResult, String>
where
    // &[String] 是字符串数组的只读切片，相当于 Java 的 List<String>（只读视图）
    F: FnMut(&[String]) + Send + 'static,
{
    let mole_path = find_mole_path(app).ok_or_else(|| {
        "Mole CLI not found. Please install it first or configure the path in Settings.".to_string()
    })?;

    // 在启动新进程之前，先杀掉之前可能还在运行的 analyze 进程
    // 防止多个 `mole analyze` 进程同时运行（会导致 CPU 飙升到 200%+）
    kill_previous_analyze();

    // 启动子进程
    let mut child = Command::new(&mole_path)
        .args(args)
        .env("LC_ALL", "C")
        .env("NO_COLOR", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start Mole: {}", e))?;

    // 记录这个新进程的 PID（进程 ID），以便下次调用时可以杀掉它
    // child.id() 返回 Option<u32>（有时子进程已经退出，则没有 PID）
    if let Some(pid) = child.id() {
        // 获取 Mutex 的锁（相当于 Java 的 synchronized 块）
        // guard 是锁的守卫对象（MutexGuard），持有它期间其他线程不能访问 ANALYZE_CHILD_PID
        // 当 guard 离开作用域时，锁自动释放（Rust 的 RAII，不需要手动 unlock）
        if let Ok(mut guard) = ANALYZE_CHILD_PID.lock() {
            *guard = Some(pid); // *guard 解引用守卫，获取内部值；= Some(pid) 更新它
        }
    }

    drain_stderr(&mut child);

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture stdout".to_string())?;

    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();

    let timeout_duration = Duration::from_secs(timeout_secs);
    // 空闲超时（60秒没有任何输出则认为进程卡死）
    let idle_timeout = Duration::from_secs(IDLE_TIMEOUT_SECS);
    // 节流间隔：每 100ms 批量推送一次积累的行
    let flush_interval = Duration::from_millis(100);
    let mut timed_out = false;
    // 行缓冲区：积累输出行，等待批量推送
    // Vec::with_capacity(64) 预分配 64 个元素的容量（优化性能，减少内存重分配）
    let mut buffer: Vec<String> = Vec::with_capacity(64);
    // 记录函数开始执行的时刻（用于总体超时检查）
    let start_time = tokio::time::Instant::now();
    // 记录最后一次收到输出的时刻（用于空闲超时检查）
    let mut last_output_time = start_time;
    // 循环计数器（用于周期性触发超时检查，避免每次循环都检查带来的性能开销）
    let mut loop_count: u64 = 0;

    // 主读取循环
    loop {
        // 检查取消标志（由前端"停止"按钮触发）
        // load(Ordering::SeqCst) 原子读取布尔值（最严格的内存顺序，确保所有线程看到最新值）
        if cancel_flag.load(Ordering::SeqCst) {
            eprintln!("[mole-gui] Cancel flag detected – killing analyze process");
            let _ = child.kill().await; // 强制杀死进程
            // 在取消前，把缓冲区里还没推送的行先推出去
            if !buffer.is_empty() {
                on_batch(&buffer);
            }
            // 返回"已取消"状态
            return Ok(StreamingResult {
                exit_code: -1,
                timed_out: false,
                cancelled: true,
            });
        }

        // 使用 flush_interval (100ms) 作为读取超时
        // 如果 100ms 内没有新行，超时后刷新缓冲区
        match tokio::time::timeout(flush_interval, lines.next_line()).await {
            Ok(Ok(Some(line))) => {
                // 收到一行数据，更新最后输出时间
                last_output_time = tokio::time::Instant::now();
                buffer.push(line); // 加入缓冲区
                // 如果缓冲区积累了 1000 行以上，立即批量推送（不等 100ms 间隔）
                // 提高阈值至 1000 能够在大目录快速扫描时大幅减少 IPC 交互次数（降低至原本的 20 分之一）
                // 在扫描慢速或零星输出时，仍然靠下面的 100ms 超时（flush_interval）来保证界面的实时更新
                if buffer.len() >= 1000 {
                    on_batch(&buffer);
                    buffer.clear(); // clear() 清空向量但保留分配的内存容量（避免重复进行内存分配）
                }
            }
            Ok(Ok(None)) => break, // EOF，进程已结束
            Ok(Err(_)) => break,   // 读取错误，也视为结束
            Err(_elapsed) => {
                // 100ms 间隔到了，将积累的行批量推送给回调
                if !buffer.is_empty() {
                    on_batch(&buffer);
                    buffer.clear();
                }
            }
        }

        loop_count += 1;

        // 每 50 次循环（约 5 秒）检查一次空闲超时
        // is_multiple_of(50) 相当于 Java 的 loop_count % 50 == 0，但更安全/地道
        if loop_count.is_multiple_of(50) {
            // elapsed() 返回自 last_output_time 以来经过的 Duration
            let idle_elapsed = last_output_time.elapsed();
            if idle_elapsed > idle_timeout {
                eprintln!(
                    "[mole-gui] No output for {}s (idle timeout {}s) – killing process",
                    idle_elapsed.as_secs(),
                    IDLE_TIMEOUT_SECS
                );
                timed_out = true;
                let _ = child.kill().await;
                break;
            }
        }

        // 每 100 次循环（约 10 秒）检查一次总体超时
        if loop_count.is_multiple_of(100) && start_time.elapsed() > timeout_duration {
            eprintln!(
                "[mole-gui] Overall timeout {}s reached – killing process",
                timeout_secs
            );
            timed_out = true;
            let _ = child.kill().await;
            break;
        }
    }

    // 最终刷新：把循环结束后缓冲区里还剩的行推送出去
    if !buffer.is_empty() {
        on_batch(&buffer);
    }

    // 清除 PID 追踪（进程已结束，不需要再记录 PID）
    if let Ok(mut guard) = ANALYZE_CHILD_PID.lock() {
        *guard = None;
    }

    // 获取退出码
    let exit_code = if timed_out {
        -1
    } else {
        let status = child
            .wait()
            .await
            .map_err(|e| format!("Failed to wait for Mole: {}", e))?;
        status.code().unwrap_or(-1)
    };

    Ok(StreamingResult {
        exit_code,
        timed_out,
        cancelled: false,
    })
}

/// 杀掉之前记录的 analyze 进程（如果有的话）。
/// 防止多个 `mole analyze` 进程同时运行消耗大量 CPU。
fn kill_previous_analyze() {
    // 先从全局状态中取出 PID（take() 取出 Some 里的值并将状态置为 None）
    // if let Ok(mut guard) = ... 获取 Mutex 锁，失败则跳过（发生中毒时）
    let pid = if let Ok(mut guard) = ANALYZE_CHILD_PID.lock() {
        guard.take() // 取走 PID，将 *guard 设为 None
    } else {
        None
    };

    // 如果确实有之前的进程 PID
    if let Some(pid) = pid {
        eprintln!("[mole-gui] Killing previous analyze process (PID={})", pid);
        // 使用 kill -9 强制终止进程（SIGKILL，不可捕获的信号）
        // 这里使用同步的 std::process::Command（因为我们不需要异步等待）
        let _ = std::process::Command::new("kill")
            .arg("-9")               // 发送 SIGKILL 信号
            .arg(pid.to_string())    // 目标进程的 PID（转换为字符串参数）
            .status();               // 等待 kill 命令完成（忽略返回值）
    }
}

/// 生成下一个唯一的请求 ID（自增计数器）。
/// 通过 Mutex 保证线程安全（同一时间只有一个线程能修改计数器）。
fn get_next_request_id() -> u64 {
    if let Ok(mut guard) = NEXT_REQUEST_ID.lock() {
        *guard += 1;  // 计数器自增（*guard 解引用 MutexGuard 访问内部 u64 值）
        *guard        // 返回自增后的值（Rust 中语句块的最后一个表达式是其返回值，无需 return）
    } else {
        0 // 获取锁失败时（极少发生），返回 0
    }
}

/// 取消任何已存在的 analyze 任务，注册新任务的 ID。
///
/// 通过将 ANALYZE_TASK 更新为新 ID，旧任务在下次检查 is_task_cancelled() 时会发现
/// 当前 ID 已经变了，从而知道自己已被取代，然后停止运行。
fn cancel_existing_analyze_task(new_request_id: u64) {
    if let Ok(mut guard) = ANALYZE_TASK.lock() {
        // 如果有旧任务在运行，打印日志（旧任务自己会在下次循环时检测到被取消）
        if let Some(old_id) = *guard {
            eprintln!("[mole-gui] Cancelling previous analyze task #{} for new task #{}", old_id, new_request_id);
        }
        // 将全局当前任务 ID 更新为新任务的 ID
        *guard = Some(new_request_id);
    }
}

/// 检查指定 ID 的任务是否已被取消（即是否有更新的任务取代了它）。
///
/// 原理：如果 ANALYZE_TASK 里存储的 ID 与我的 request_id 不同，
/// 说明有新任务已经注册，我（旧任务）应该停止。
fn is_task_cancelled(request_id: u64) -> bool {
    if let Ok(guard) = ANALYZE_TASK.lock() {
        // 解引用 guard 获取内部的 Option<u64>，再检查其值
        if let Some(current_id) = *guard {
            // 如果当前记录的 ID 不是我的 ID，说明我已被取消
            return current_id != request_id;
        }
    }
    false // 获取锁失败或没有任务记录，默认为未取消
}

/// 如果指定 ID 的任务仍然是当前活跃的任务，则清除追踪记录。
///
/// 这个检查是必要的：如果在这个任务运行过程中，
/// 另一个任务已经开始并注册了新的 ID，我们不应该清除那个新任务的记录。
fn clear_analyze_task_if_current(request_id: u64) {
    if let Ok(mut guard) = ANALYZE_TASK.lock() {
        if let Some(current_id) = *guard {
            if current_id == request_id {
                // 我仍然是当前任务，清除记录
                *guard = None;
                eprintln!("[mole-gui] Cleared analyze task #{} tracking", request_id);
            } else {
                // 已经有新任务取代我了，不要清除新任务的记录
                eprintln!("[mole-gui] Task #{} finished but task #{} is now active, keeping tracking", request_id, current_id);
            }
        }
    }
}

/// 使用 osascript（AppleScript）以管理员权限执行 Mole CLI 命令，带超时控制。
///
/// 在 GUI 应用中，我们不能弹出终端 sudo 提示符，所以使用 osascript 的
/// `do shell script ... with administrator privileges` 来触发 macOS 系统级密码弹窗。
///
/// 执行时会自动附加 `--permanent` 参数（跳过 macOS 废纸篓，直接用 rm -rf 删除）
/// 并用 `echo y |` 自动回答任何确认提示。
pub async fn run_mole_streaming_with_timeout_sudo<F>(
    app: Option<&tauri::AppHandle>,
    args: &[&str],
    timeout_secs: u64,
    mut on_line: F,
) -> Result<StreamingResult, String>
where
    F: FnMut(String) + Send + 'static,
{
    let mole_path = find_mole_path(app).ok_or_else(|| {
        "Mole CLI not found. Please install it first or configure the path in Settings.".to_string()
    })?;

    // 将参数数组拼接成空格分隔的字符串（相当于 String.join(" ", args)）
    let cmd_args = args.join(" ");

    // 构建完整的 shell 命令字符串：
    //   echo y | "/path/to/mole" clean --targets "..." --permanent
    // echo y | 用于自动回答交互式确认提示（如"确定要删除吗？[y/n]"）
    let shell_cmd = format!("echo y | \"{}\" {} --permanent", mole_path.display(), cmd_args);

    // 构建 AppleScript 脚本
    // replace('\\', "\\\\") 将单个反斜杠转义为双反斜杠（AppleScript 字符串转义）
    // replace('"', "\\\"")  将双引号转义为 \" （在 AppleScript 字符串内部使用）
    let script = format!(
        "do shell script \"{}\" with administrator privileges",
        shell_cmd.replace('\\', "\\\\").replace('"', "\\\"")
    );

    // 启动 osascript 进程（macOS 的 AppleScript/JXA 解释器）
    let mut child = Command::new("osascript")
        .arg("-e")       // -e 参数：后面跟脚本内容（而非脚本文件）
        .arg(&script)    // AppleScript 脚本内容
        .env("LC_ALL", "C")
        .env("NO_COLOR", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start osascript: {}", e))?;

    drain_stderr(&mut child);

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture stdout".to_string())?;

    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();

    let timeout_duration = Duration::from_secs(timeout_secs);
    let mut timed_out = false;

    // 读取循环（与 run_mole_streaming_with_timeout 类似，但没有 analyze 取消逻辑）
    loop {
        match tokio::time::timeout(timeout_duration, lines.next_line()).await {
            Ok(Ok(Some(line))) => {
                on_line(line);
            }
            Ok(Ok(None)) => {
                break; // EOF
            }
            Ok(Err(_e)) => {
                break; // 读取错误
            }
            Err(_elapsed) => {
                timed_out = true;
                let _ = child.kill().await;
                break;
            }
        }
    }

    let exit_code = if timed_out {
        -1
    } else {
        let status = child
            .wait()
            .await
            .map_err(|e| format!("Failed to wait for osascript: {}", e))?;
        status.code().unwrap_or(-1)
    };

    Ok(StreamingResult {
        exit_code,
        timed_out,
        cancelled: false,
    })
}

/// 执行 Mole CLI 命令，将全部标准输出作为字符串返回（非流式）。
///
/// 适用于输出量小、需要一次性获取结果的场景（如获取版本号）。
/// 内置 5 秒超时，防止命令卡死。
pub async fn run_mole_capture(
    app: Option<&tauri::AppHandle>,
    args: &[&str],
) -> Result<String, String> {
    let mole_path = find_mole_path(app).ok_or_else(|| {
        "Mole CLI not found. Please install it first or configure the path in Settings.".to_string()
    })?;

    // 用 tokio::time::timeout 包装整个命令执行过程
    // Duration::from_secs(5) 表示 5 秒超时
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        // Command::new().args().env().output() 构建并执行命令，等待完成并收集输出
        Command::new(&mole_path)
            .args(args)
            .env("LC_ALL", "C")
            .env("NO_COLOR", "1")
            .env("MOLE_TIMEOUT_HINT_SCAN_SEC", "2")
            .output(), // output() 收集 stdout + stderr + 退出状态（不是流式的）
    )
    .await
    // 第一个 map_err：处理超时错误（Elapsed 类型）
    .map_err(|_| "Mole version check timed out".to_string())?
    // 第二个 map_err：处理 I/O 错误（命令启动失败等）
    .map_err(|e| format!("Failed to run Mole: {}", e))?;

    if output.status.success() {
        // 将字节输出转换为 UTF-8 字符串
        // to_string() 将 Cow<str>（String::from_utf8_lossy 返回的类型）转换为 String
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        // 命令以非零状态码退出，返回 stderr 内容作为错误信息
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(if stderr.is_empty() {
            // 如果 stderr 为空，用退出码构造错误信息
            format!("Mole exited with code {}", output.status.code().unwrap_or(-1))
        } else {
            stderr
        })
    }
}

/// 获取 Mole CLI 的版本字符串。
///
/// 运行 `mole --version`，解析输出中的版本号。
/// 例如：输入 "Mole version 1.44.1"，输出 "1.44.1"
pub async fn get_mole_version(app: Option<&tauri::AppHandle>) -> Result<String, String> {
    // 运行 mole --version 并捕获输出
    let output = run_mole_capture(app, &["--version"]).await?;

    // 逐行解析输出（? 从 run_mole_capture 传播错误）
    for line in output.lines() {
        let trimmed = line.trim();
        // 跳过空行
        if trimmed.is_empty() {
            continue;
        }
        // strip_prefix 尝试去掉字符串的指定前缀
        //   如果成功，返回 Some(剩余部分)
        //   如果字符串不以该前缀开头，返回 None
        // 相当于 Java 的：if (s.startsWith("Mole version ")) { return s.substring(...); }
        if let Some(version_part) = trimmed.strip_prefix("Mole version ") {
            return Ok(version_part.trim().to_string());
        }
    }

    // 如果没有找到标准格式的版本行，返回整个输出（去掉首尾空白）
    Ok(output.trim().to_string())
}
