// ============================================================
// commands/mod.rs —— Tauri 命令层（前端 ↔ 后端 的桥梁）
//
// 这个文件定义了所有可以从前端 JavaScript 调用的后端函数。
// 类似于 Java Spring 里的 @RestController，但走的是 IPC 通道而非 HTTP。
//
// 每个标注了 #[tauri::command] 的 pub async fn 都会被注册到 lib.rs 的
// invoke_handler 中，前端通过 invoke('函数名', { 参数 }) 来调用。
// ============================================================

// 引入 serde 的序列化/反序列化 derive 宏
// Serialize   = 将 Rust 结构体转换为 JSON（相当于 Java 的 @JsonSerialize）
// Deserialize = 将 JSON 转换为 Rust 结构体（相当于 Java 的 @JsonDeserialize）
use serde::{Deserialize, Serialize};
// 引入原子布尔类型和内存排序（用于实现线程安全的取消标志）
use std::sync::atomic::{AtomicBool, Ordering};
// 引入 Tauri 的核心类型：
//   AppHandle — 应用句柄（全局上下文，类似 Spring 的 ApplicationContext）
//   Emitter   — 事件发送 trait，让 window 具备向前端推送事件的能力
//   Window    — 窗口引用（用于向前端发送事件）
use tauri::{AppHandle, Emitter, Window};
// 引入我们自定义的进程管理模块和配置模块
use crate::mole::process;
use crate::mole::settings;

// ============================================================
// 超时常量（各操作的最大允许运行时间，单位：秒）
// ============================================================

/// 清理操作（clean）的超时时间：2 分钟
const CLEAN_TIMEOUT_SECS: u64 = 120;
/// 卸载扫描（uninstall scan）的超时时间：1 分钟
const UNINSTALL_TIMEOUT_SECS: u64 = 60;
/// 深度清理（purge）的超时时间：3 分钟
const PURGE_TIMEOUT_SECS: u64 = 180;
/// 优化（optimize）的超时时间：1 分钟
const OPTIMIZE_TIMEOUT_SECS: u64 = 60;
/// 磁盘分析（analyze）的超时时间：5 分钟
const ANALYZE_TIMEOUT_SECS: u64 = 300;

// ============================================================
// 数据结构定义（这些结构体会被自动序列化为 JSON 传给前端）
// ============================================================

/// Mole CLI 输出的通用事件结构体（用于向前端推送进度/结果）
/// #[derive(...)] 是 Rust 的派生宏，自动实现指定的 trait
///   Serialize   — 可以序列化为 JSON
///   Deserialize — 可以从 JSON 反序列化
///   Clone       — 可以被复制（类似 Java 的 .clone() 或 Cloneable 接口）
#[derive(Serialize, Deserialize, Clone)]
pub struct MoleEvent {
    /// 事件类型（如 "progress"、"item"、"error"）
    /// #[serde(rename = "type")] 将 Rust 字段名 event_type 序列化为 JSON 键 "type"
    /// 因为 "type" 是 Rust 的关键字，所以字段名用 event_type，但 JSON 里仍用 type
    #[serde(rename = "type")]
    pub event_type: String,
    /// 事件携带的任意 JSON 数据（可以是对象、数组等）
    /// #[serde(flatten)] 将 data 内的所有字段"展平"到父级 JSON 对象中
    /// 例如：data = {"percent": 50} → 最终 JSON 包含 {"type": "...", "percent": 50}
    #[serde(flatten)]
    pub data: serde_json::Value,
}

/// Mole CLI 路径配置（用户自定义路径 + 实际解析路径）
#[derive(Serialize)]
pub struct MolePathConfig {
    /// 用户手动设置的路径（可能为空字符串，表示使用自动检测）
    pub custom_path: String,
    /// 实际解析后使用的路径（自动检测或用户配置的路径）
    pub resolved_path: String,
}

/// Mole CLI 版本信息
#[derive(Serialize)]
pub struct MoleVersionInfo {
    /// 版本号字符串（如 "1.44.1"）
    pub version: String,
    /// 是否已安装（找到可执行文件则为 true）
    pub installed: bool,
    /// 可执行文件的完整路径
    pub path: String,
}

/// 清理/卸载/优化等操作的执行结果
#[derive(Serialize)]
pub struct CleanResult {
    /// 操作是否成功
    pub success: bool,
    /// 操作过程中的输出行（目前保留但一般为空，输出通过事件实时推送）
    pub lines: Vec<String>,
    /// 错误信息（如果没有错误则为 None）
    /// #[serde(skip_serializing_if = "Option::is_none")] 表示：
    ///   如果这个字段是 None，则在 JSON 中完全省略这个键（不序列化为 null）
    ///   相当于 Java 的 @JsonInclude(NON_NULL)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 单个系统优化项
#[derive(Serialize, Clone)]
pub struct OptimizeItem {
    /// 操作动作标识符（如 "disable_spotlight"、"clear_dns_cache"）
    pub action: String,
    /// 优化项的显示名称
    pub name: String,
    /// 优化项的详细描述
    pub description: String,
    /// 是否为安全操作（不会导致数据丢失）
    pub safe: bool,
    /// 是否需要 sudo 权限才能执行
    pub requires_sudo: bool,
    /// 是否已启用（用户选中）
    /// #[serde(default)] 表示：反序列化时如果 JSON 中没有这个键，使用类型的默认值（bool 默认为 false）
    #[serde(default)]
    pub enabled: bool,
    /// 执行状态（如 "applied"、"skipped"、"failed"）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

/// 系统健康状况摘要（内存和磁盘使用情况）
#[derive(Serialize, Clone)]
pub struct SystemHealth {
    /// 已用内存（GB）
    pub memory_used_gb: f64,
    /// 总内存（GB）
    pub memory_total_gb: f64,
    /// 已用磁盘（GB）
    pub disk_used_gb: f64,
    /// 磁盘总容量（GB）
    pub disk_total_gb: f64,
    /// 系统已运行天数
    pub uptime_days: u64,
}

/// 系统优化操作的整体结果
#[derive(Serialize)]
pub struct OptimizeResult {
    /// 系统健康状况（可能为 None，如果 Mole CLI 没有返回该信息）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_health: Option<SystemHealth>,
    /// 所有优化项的列表
    pub optimizations: Vec<OptimizeItem>,
    /// 优化项总数
    pub total_items: usize,
    /// 已应用的优化数量
    pub applied_count: usize,
}

/// 应用信息（用于卸载功能）
/// Debug — 实现调试打印（可以用 {:?} 格式化输出）
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppInfo {
    /// 应用名称（如 "Slack"）
    pub name: String,
    /// 应用的完整路径（如 "/Applications/Slack.app"）
    pub path: String,
    /// Bundle ID（macOS 应用的唯一标识符，如 "com.tinyspeck.slackmacgap"）
    pub bundle_id: String,
    /// 应用大小（KB）
    pub size_kb: u64,
    /// 应用当前是否正在运行
    #[serde(default)]
    pub is_running: bool,
    /// 是否可以通过 Homebrew Cask 卸载
    #[serde(default)]
    pub has_brew_cask: bool,
    /// 是否在系统保护白名单中（受保护的应用，不能卸载）
    #[serde(default)]
    pub is_blocked: bool,
    /// 最后使用时间（ISO 格式字符串，可能为 None）
    /// #[serde(rename = "last_used")] 指定 JSON 键名，这里键名和字段名相同（显式声明）
    #[serde(rename = "last_used", default)]
    pub last_used: Option<String>,
}

/// 系统状态信息（仪表盘使用），从 `mole status --json` 输出解析
#[derive(Serialize, Clone)]
pub struct SystemStatus {
    /// 主机名
    pub host: String,
    /// 操作系统平台（如 "macOS"）
    pub platform: String,
    /// 系统运行时长（人类可读格式，如 "3 days, 2 hours"）
    pub uptime: String,
    /// 系统运行时长（秒数，用于数值计算）
    pub uptime_seconds: u64,
    /// 系统健康评分（0-100）
    pub health_score: u64,
    /// 健康评分的描述信息（如 "Your system is healthy"）
    pub health_score_msg: String,
    /// CPU 使用率（百分比，0.0-100.0）
    pub cpu_usage: f64,
    /// CPU 核心数
    pub cpu_core_count: u64,
    /// 已用内存（字节）
    pub memory_used: u64,
    /// 总内存（字节）
    pub memory_total: u64,
    /// 可用内存（字节）
    pub memory_available: u64,
    /// 内存使用率（百分比，0.0-100.0）
    pub memory_used_percent: f64,
    /// 已用磁盘空间（字节）
    pub disk_used: u64,
    /// 磁盘总容量（字节）
    pub disk_total: u64,
    /// 磁盘剩余空间（字节）
    pub disk_free: u64,
    /// 磁盘使用率（百分比）
    pub disk_used_percent: f64,
    /// 磁盘总容量（人类可读格式，如 "1 TB"）
    pub disk_size: String,
    /// Mac 型号（如 "MacBook Pro 14-inch, 2023"）
    pub model: String,
    /// CPU 型号（如 "Apple M2 Pro"）
    pub cpu_model: String,
    /// 总内存（人类可读格式，如 "16 GB"）
    pub total_ram: String,
    /// macOS 版本（如 "macOS 14.3 Sonoma"）
    pub os_version: String,
    /// 废纸篓大小（字节）
    pub trash_size: u64,
}

// ============================================================
// 内部辅助函数（私有，不暴露给前端）
// ============================================================

/// 解析 Mole CLI 输出的一行文本，并将其作为 Tauri 事件发送到前端窗口。
///
/// 参数：
///   window     — 目标窗口（事件接收方）
///   event_name — 事件名称（前端用 listen(event_name, ...) 来订阅）
///   line       — 要解析和发送的一行文本
///
/// 这个函数是私有的（没有 pub），只在 commands 模块内部使用
/// 将 Mole CLI 输出的一行文本解析为相应的事件对象列表。
///
/// 参数：
///   line — 要解析的一行文本（&str 相当于 Java 中的 String，这里是只读引用）
/// 返回：
///   Vec<MoleEvent> — 解析出的事件列表（Vec<T> 相当于 Java 中的 ArrayList<T>）
fn parse_mole_line_to_event(line: &str) -> Vec<MoleEvent> {
    // Vec::new() 相当于 Java 中的 new ArrayList<>()
    let mut events = Vec::new();

    // 去掉首尾空白
    let trimmed = line.trim();
    // 忽略空行，如果是空行直接返回空列表（相当于 Java 的 if (trimmed.isEmpty()) return events;）
    if trimmed.is_empty() {
        return events;
    }

    // 首先尝试将这一行解析为 JSON 格式
    // serde_json::from_str::<serde_json::Value> 指定目标类型为通用 JSON 值
    // ::<serde_json::Value> 是 Rust 的"涡轮鱼"语法（turbofish），用于指定泛型参数
    // 类似 Java 的：serde_json.fromJson(trimmed, JsonValue.class)
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
        // 如果 JSON 对象有 "type" 字段，则视为结构化事件
        if json.get("type").is_some() {
            let event = MoleEvent {
                event_type: json
                    .get("type")                     // 获取 "type" 字段 of JSON 值
                    .and_then(|v| v.as_str())         // 尝试将 JSON 值转换为 &str
                    .unwrap_or("unknown")             // 如果失败，使用 "unknown" 作为默认值
                    .to_string(),                     // 转换为拥有所有权的 String
                data: json,
            };
            events.push(event); // push 相当于 Java 的 list.add()
            return events; // 已处理，提前返回
        }
    }

    // 处理章节标题行（以 ➤ 开头，如 "➤ User essentials"）
    if trimmed.starts_with("➤") {
        // trim_start_matches 去掉字符串开头所有匹配的字符/字符串
        // 类似 Java 的 s.replaceAll("^➤\\s*", "")
        let section_name = trimmed.trim_start_matches("➤").trim();
        // serde_json::json! 宏用于方便地创建 JSON 对象字面量
        // 相当于 Java 的 new JSONObject().put("type","progress").put("section",...)
        let json_obj = serde_json::json!({
            "type": "progress",
            "section": section_name,
            "message": format!("Scanning {}...", section_name),
            "percent": 0
        });

        let event = MoleEvent {
            event_type: "progress".to_string(),
            data: json_obj,
        };
        events.push(event);
        return events;
    }

    // 跳过其他格式的标题行，但将特定标题（⚙ 开头或包含 "Free space:"）作为进度更新发送
    if is_header_line(trimmed) {
        if trimmed.starts_with("⚙") || trimmed.contains("Free space:") {
            let json_obj = serde_json::json!({
                "type": "progress",
                "section": "System Info",
                "message": trimmed,
                "percent": 5
            });

            let event = MoleEvent {
                event_type: "progress".to_string(),
                data: json_obj,
            };
            events.push(event);
        }
        return events;
    }

    // 解析普通文本输出行（mole CLI 的人类可读格式）
    // 先判断这行属于哪个清理分区
    let section = determine_section(trimmed);

    // 尝试解析为一个具体的清理条目（有大小信息的行）
    if let Some(item_info) = parse_item_line(trimmed, &section) {
        // 发送 "item" 类型事件（包含条目详情）
        let json_obj = serde_json::json!({
            "type": "item",
            "section": item_info.section,
            "description": item_info.description,
            "size_kb": item_info.size_kb,
            "size_human": item_info.size_human,
            "status": item_info.status
        });

        let event = MoleEvent {
            event_type: "item".to_string(),
            data: json_obj,
        };
        events.push(event);

        // 同时发送一个 "progress" 事件（告知前端当前正在处理哪个条目）
        let progress_json = serde_json::json!({
            "type": "progress",
            "section": item_info.section,
            "message": format!("Found: {}", item_info.description),
            "percent": 50
        });

        let progress_event = MoleEvent {
            event_type: "progress".to_string(),
            data: progress_json,
        };
        events.push(progress_event);
    }

    events
}

/// 解析 Mole CLI 输出的一行文本，并将其作为 Tauri 事件发送到前端窗口。
///
/// 参数：
///   window     — 目标窗口（事件接收方）
///   event_name — 事件名称（前端用 listen(event_name, ...) 来订阅）
///   line       — 要解析和发送的一行文本
///
/// 这个函数是私有的（没有 pub），只在 commands 模块内部使用
fn emit_mole_event(window: &Window, event_name: &str, line: &str) {
    let events = parse_mole_line_to_event(line);
    for event in events {
        // window.emit 向前端推送事件（类似 Java 的 EventBus.post()）
        // let _ = 忽略返回值（emit 可能失败，比如窗口已关闭，但我们不需要处理这种情况）
        let _ = window.emit(event_name, &event);
    }
}

/// 解析后的输出条目（内部数据结构，不暴露给外部）
struct ParsedItem {
    /// 所属的清理分区名称
    section: String,
    /// 条目的描述文字
    description: String,
    /// 大小（KB，用于数值计算和排序）
    size_kb: f64,
    /// 大小（人类可读格式，如 "1.2GB"）
    size_human: String,
    /// 状态（"dry_run" / "cleaned" / "skipped"）
    status: String,
}

/// 根据输出行的内容推断它属于哪个清理分区。
/// 这是一个简单的关键词匹配函数（Mole CLI 文本输出没有明确的分区标记）。
fn determine_section(line: &str) -> String {
    // contains() 检查字符串是否包含子串（相当于 Java 的 String.contains()）
    if line.contains("User app cache") || line.contains("User app logs")
        || line.contains("Darwin user cache")
        || line.contains("Trash")
    {
        "User essentials".to_string()
    } else if line.contains("cache") || line.contains("temp files") {
        "App caches".to_string()
    } else if line.contains("logs") {
        "Logs".to_string()
    } else if line.contains("leftover") || line.contains("orphaned") {
        "Leftovers".to_string()
    } else {
        "Other".to_string()
    }
}

/// 判断一行文本是否是标题行（非数据行）。
/// 标题行应该被跳过或以特殊方式处理，而不是解析为条目数据。
fn is_header_line(line: &str) -> bool {
    line.starts_with("Clean Your Mac")
        || line.starts_with("Dry Run Mode")
        || line.starts_with("◎")    // 圆形符号，通常表示概要信息
        || line.starts_with("⚙")    // 齿轮符号，通常表示系统信息
        || line.starts_with("✓ Whitelist")
        || line.starts_with("➤")    // 箭头符号，表示分区标题
}

/// 尝试从一行文本中解析出清理条目的信息（大小、描述、状态等）。
///
/// Mole CLI 的输出格式示例：
///   "✓ User app cache · already empty"
///   "→ Safari caches, 2 items, 1.23GB dry"
///   "✓ Safari caches, 2 items, 1.23GB cleaned"
///
/// 参数 `default_section` 是通过 determine_section() 预先推断出的分区名。
/// 返回 Option<ParsedItem>：Some 表示成功解析，None 表示这行不是条目格式。
fn parse_item_line(line: &str, default_section: &str) -> Option<ParsedItem> {
    // ── 格式一："✓ 描述 · already empty" ──────────────────────
    // contains() 检查是否包含特定子串
    if line.contains("· already empty") {
        // split('·') 按中点字符分割字符串
        // collect::<Vec<&str>>() 将迭代器收集为向量（类似 Java 的 stream().collect(...)）
        let parts: Vec<&str> = line.split('·').collect();
        if parts.len() >= 2 {
            let description = parts[0].trim().trim_start_matches("✓").trim();
            return Some(ParsedItem {
                section: default_section.to_string(),
                description: format!("{} (empty)", description),
                size_kb: 0.0,
                size_human: "0KB".to_string(),
                status: "dry_run".to_string(),
            });
        }
    }

    // ── 格式二："→ 描述, N items, X.XXGB dry/cleaned" ──────────
    // 同时包含逗号（,）和 "items"/"item"
    if line.contains(",") && (line.contains("items") || line.contains("item")) {
        // split(',').next() 按逗号分割，取第一部分（相当于 Java 的 split(",")[0]）
        // ? 如果是 None 则提前从函数返回 None
        let before_comma = line.split(',').next()?.trim();
        let description = before_comma
            .trim_start_matches("→")  // 去掉箭头前缀
            .trim_start_matches("✓")  // 去掉复选标记前缀
            .trim();

        // 获取逗号后的第二段（如 " 2 items, 1.23GB dry" 的后半部分）
        let after_comma = line.split(',').nth(1)?.trim();
        // split_whitespace() 按空白字符分割（相当于 Java 的 split("\\s+")）
        // find() 找到第一个满足条件的元素
        // 条件：结尾是 "GB"、"MB" 或 "KB"（即找大小字符串）
        let size_str = after_comma
            .split_whitespace()
            .find(|s| s.ends_with("GB") || s.ends_with("MB") || s.ends_with("KB"))?;

        // 将大小字符串转换为 KB 数值
        let size_kb = parse_size_to_kb(size_str)?;

        // 根据行内容判断状态（面向 Java 开发者）：
        //   - contains 用于检查字符串是否包含特定字串，相当于 Java 的 String.contains()
        //   - Rust 中 '✓' 是字符型（char），而 "✓" 是字符串型（&str）
        let status = if line.contains("dry") {
            "dry_run"   // 预览模式，文件未被删除
        } else if line.contains("cleaned") || line.contains('✓') || line.contains("✓") {
            "cleaned"   // 已经清理（删除）或者包含成功勾选标记
        } else {
            "skipped"   // 跳过（如文件被占用）
        };

        return Some(ParsedItem {
            section: default_section.to_string(),
            description: description.to_string(),
            size_kb,
            size_human: size_str.to_string(),
            status: status.to_string(),
        });
    }

    // 无法解析为条目格式，返回 None
    None
}

/// 将人类可读的大小字符串（如 "1.23GB"、"512MB"）转换为 KB 数值。
///
/// 参数：size_str — 大小字符串
/// 返回：Option<f64> — Some(KB数值) 或 None（格式不识别）
fn parse_size_to_kb(size_str: &str) -> Option<f64> {
    let size_str = size_str.trim();

    if size_str.ends_with("GB") {
        // trim_end_matches 去掉末尾的指定字符串
        // parse::<f64>() 将字符串解析为 f64（64位浮点数）
        // .ok()? 将 Result 转为 Option，如果解析失败则返回 None
        let num: f64 = size_str.trim_end_matches("GB").parse().ok()?;
        Some(num * 1024.0 * 1024.0) // 1 GB = 1024 MB = 1024*1024 KB
    } else if size_str.ends_with("MB") {
        let num: f64 = size_str.trim_end_matches("MB").parse().ok()?;
        Some(num * 1024.0) // 1 MB = 1024 KB
    } else if size_str.ends_with("KB") {
        let num: f64 = size_str.trim_end_matches("KB").parse().ok()?;
        Some(num) // 已经是 KB
    } else {
        None // 不识别的格式
    }
}

/// 解析 `mole optimize` 命令的输出，返回结构化的优化结果。
///
/// 先尝试 JSON 格式解析；如果 Mole CLI 输出的是人类可读文本，
/// 则退回（fallback）到逐行解析模式。
fn parse_optimize_output(output: &str) -> OptimizeResult {
    // 用于收集解析出的优化项
    let mut optimizations: Vec<OptimizeItem> = Vec::new();

    // ── 第一尝试：JSON 格式解析 ─────────────────────────────
    // output.trim() 去掉首尾空白（JSON 解析对前缀空格敏感）
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(output.trim()) {
        // 获取 "optimizations" 数组
        // json.get("optimizations") 返回 Option<&Value>
        // .and_then(|v| v.as_array()) 如果存在则尝试转为数组引用
        // and_then 相当于 Optional.flatMap()
        if let Some(items) = json.get("optimizations").and_then(|v| v.as_array()) {
            // 遍历 JSON 数组中的每个元素
            for item in items {
                // 必须有 "action" 字段才算有效条目
                if let Some(action) = item.get("action").and_then(|v| v.as_str()) {
                    optimizations.push(OptimizeItem {
                        action: action.to_string(),
                        // unwrap_or("") 如果字段不存在则使用空字符串作为默认值
                        name: item.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        description: item.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        // unwrap_or(true) 安全相关的字段默认为 true（安全优先）
                        safe: item.get("safe").and_then(|v| v.as_bool()).unwrap_or(true),
                        requires_sudo: item.get("requires_sudo").and_then(|v| v.as_bool()).unwrap_or(false),
                        enabled: false, // 默认未启用，等用户选择
                        // .map(|s| s.to_string()) 将 &str 转换为 String（如果存在的话）
                        status: item.get("status").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    });
                }
            }
        }

        // 解析 "system_health" 对象
        // .map(|h| SystemHealth {...}) 如果 system_health 存在，构建结构体
        let system_health = json.get("system_health").map(|h| SystemHealth {
            memory_used_gb: h.get("memory_used_gb").and_then(|v| v.as_f64()).unwrap_or(0.0),
            memory_total_gb: h.get("memory_total_gb").and_then(|v| v.as_f64()).unwrap_or(0.0),
            disk_used_gb: h.get("disk_used_gb").and_then(|v| v.as_f64()).unwrap_or(0.0),
            disk_total_gb: h.get("disk_total_gb").and_then(|v| v.as_f64()).unwrap_or(0.0),
            uptime_days: h.get("uptime_days").and_then(|v| v.as_u64()).unwrap_or(0),
        });

        // JSON 解析成功，直接返回结果
        return OptimizeResult {
            system_health,
            // optimizations.clone() 深复制优化项列表（因为要同时用于计算 total_items）
            optimizations: optimizations.clone(),
            total_items: optimizations.len(), // .len() 相当于 Java 的 .size()
            applied_count: 0,
        };
    }

    // ── 第二尝试（Fallback）：解析人类可读文本格式 ──────────
    // output.lines() 将字符串按换行符分割，返回迭代器（类似 Java 的 BufferedReader.lines()）
    for line in output.lines() {
        let trimmed = line.trim();
        // 跳过空行和标题行
        if trimmed.is_empty() || is_header_line(trimmed) {
            continue;
        }

        // 解析以 → 或 ✓ 开头的条目行
        if trimmed.starts_with("→") || trimmed.starts_with("✓") {
            let content = trimmed.trim_start_matches("→").trim_start_matches("✓").trim();

            // 按空白字符分割为单词列表
            let parts: Vec<&str> = content.split_whitespace().collect();
            if !parts.is_empty() {
                // 第一个词作为 action（转小写，去掉逗号）
                // replace(",", "") 去掉逗号（有时 action 后跟逗号）
                let action = parts[0].to_lowercase().replace(",", "");
                // 第 2-4 个词作为名称（跳过第一个 action 词，取接下来 3 个词）
                // .skip(1) 跳过第 1 个元素
                // .take(3) 最多取 3 个元素
                // .copied() 将 &&str 复制为 &str
                // .collect::<Vec<_>>().join(" ") 合并为字符串
                let name = parts.iter().skip(1).take(3).copied().collect::<Vec<_>>().join(" ");
                // 第 5 个词之后作为描述
                let description = parts.iter().skip(4).copied().collect::<Vec<_>>().join(" ");
                let requires_sudo = content.contains("[sudo]") || content.contains("sudo");

                optimizations.push(OptimizeItem {
                    action,
                    name,
                    description,
                    safe: true,
                    requires_sudo,
                    enabled: false,
                    status: None,
                });
            }
        }
    }

    OptimizeResult {
        system_health: None,
        total_items: optimizations.len(),
        optimizations,
        applied_count: 0,
    }
}

// ============================================================
// Tauri 命令（前端可以直接调用的后端函数）
// 每个函数都标注了 #[tauri::command]
// pub async fn 表示公开的异步函数（async/await 相当于 Java 的 CompletableFuture）
// ============================================================

/// 获取 Mole CLI 的版本信息（版本号、是否安装、路径）。
///
/// 前端调用：await invoke('get_mole_version')
/// 返回：MoleVersionInfo 结构体（序列化为 JSON）
#[tauri::command]
pub async fn get_mole_version(app: AppHandle) -> Result<MoleVersionInfo, String> {
    // 调用 process 模块的 get_mole_version 函数
    // Some(&app) 将 AppHandle 的引用包装在 Option 中（函数签名需要 Option<&AppHandle>）
    match process::get_mole_version(Some(&app)).await {
        Ok(version) => {
            // 获取 Mole 可执行文件的路径
            // map(|p| p.to_string_lossy().to_string()) 将 PathBuf 转换为 String
            // unwrap_or_default() 如果是 None 则使用类型默认值（String 的默认值是 ""）
            let path = process::find_mole_path(Some(&app))
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            Ok(MoleVersionInfo {
                version,
                installed: true,
                path,
            })
        }
        // 如果获取版本失败（Mole 未安装），返回"未安装"状态而不是错误
        // _（下划线）表示忽略错误值，我们不需要知道具体错误原因
        Err(_) => Ok(MoleVersionInfo {
            version: String::new(), // String::new() 创建空字符串（相当于 Java 的 ""）
            installed: false,
            path: String::new(),
        }),
    }
}

/// 获取系统磁盘剩余空间（单位：KB）。
///
/// 优先从 `mole status --json` 获取；如果失败则回退到系统的 `df` 命令。
/// 前端调用：await invoke('get_free_space_kb')
#[tauri::command]
pub async fn get_free_space_kb(app: AppHandle) -> Result<u64, String> {
    // 先尝试通过 mole status --json 获取磁盘信息
    let output = process::run_mole_capture(Some(&app), &["status", "--json"]).await?;
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&output) {
        // 尝试从 JSON 的 "disks" 数组中获取第一个磁盘的信息
        if let Some(disks) = json.get("disks").and_then(|d| d.as_array()) {
            if let Some(first) = disks.first() {
                // JSON 里可能没有直接的 "free" 字段，需要用 total - used 计算
                let total = first.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
                let used = first.get("used").and_then(|v| v.as_u64()).unwrap_or(0);
                if total > used {
                    // 磁盘大小单位是字节，除以 1024 转换为 KB
                    return Ok((total - used) / 1024);
                }
                // 某些版本可能有直接的 "free" 字段，作为备选
                if let Some(free) = first.get("free").and_then(|f| f.as_u64()) {
                    return Ok(free / 1024);
                }
            }
        }
    }

    // 回退方案：使用系统 `df` 命令获取磁盘信息
    // df -k / 表示：以 KB 为单位显示根目录所在磁盘的信息
    let output = tokio::process::Command::new("df")
        .args(["-k", "/"])  // args 接受实现了 IntoIterator 的类型，数组字面量也行
        .output()
        .await
        .map_err(|e| format!("Failed to run df: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    // df 输出的第一行是标题，skip(1) 跳过它，只处理数据行
    for line in stdout.lines().skip(1) {
        // split_whitespace() 按任意数量的空白字符分割（处理 df 输出中不固定数量的空格）
        let parts: Vec<&str> = line.split_whitespace().collect();
        // df -k 输出格式：Filesystem 1K-blocks Used Available ...
        // 第 4 列（index 3）是 Available（可用空间，单位已是 KB）
        if parts.len() >= 4 {
            // parse::<u64>() 将字符串解析为 64位无符号整数
            if let Ok(kb) = parts[3].parse::<u64>() {
                return Ok(kb);
            }
        }
    }
    Err("Could not determine free space".to_string())
}

/// 检查当前应用（Mole GUI）是否已被授予 macOS 的「完全磁盘访问权限 (Full Disk Access)」。
///
/// 检查原理：尝试读取受系统保护的 `~/Library/Safari` 目录。
/// 如果没有授予 FDA，系统会返回 `PermissionDenied` 错误。
///
/// 返回：bool，true 表示已授予 FDA（或无需检测），false 表示未授予
#[tauri::command]
pub fn check_full_disk_access() -> bool {
    // std::env::var("HOME") 获取当前用户的 Home 目录路径（相当于 Java 的 System.getProperty("user.home")）
    // Ok/Err 是 Result 枚举变体，类似 Java 的 Optional
    let home = match std::env::var("HOME") {
        Ok(val) => val,
        Err(_) => return true, // 如果获取失败，默认为 true 避免报错
    };

    // std::path::Path::new 创建路径对象，.join 连接子目录（相当于 Java 的 Paths.get(home, "Library/Safari")）
    let safari_dir = std::path::Path::new(&home).join("Library/Safari");

    // exists() 检查该路径在磁盘上是否存在（相当于 Java 的 file.exists()）
    if safari_dir.exists() {
        // std::fs::read_dir 尝试读取目录下的文件列表（相当于 Java 的 file.listFiles()）
        // is_ok() 检查结果是否是 Ok（即成功读取，无权限拒绝等报错，如果没有 FDA 则会 PermissionDenied）
        std::fs::read_dir(safari_dir).is_ok()
    } else {
        // 如果 Safari 目录不存在，默认返回 true 避免误报
        true
    }
}


/// 获取完整的系统状态信息（CPU、内存、磁盘、硬件信息等）。
///
/// 从 `mole status --json` 输出中提取并返回结构化数据。
/// 前端调用：await invoke('get_system_status')
#[tauri::command]
pub async fn get_system_status(app: AppHandle) -> Result<SystemStatus, String> {
    let output = process::run_mole_capture(Some(&app), &["status", "--json"]).await?;

    // 将 JSON 字符串解析为通用 JSON 值
    // 如果解析失败，map_err 将错误转换为友好的错误信息字符串
    let json: serde_json::Value = serde_json::from_str(&output)
        .map_err(|e| format!("Failed to parse status JSON: {}", e))?;

    // 定义两个闭包（lambda），用于从 JSON 中方便地提取字段值
    // |key: &str| ... 是闭包语法，| | 包裹参数，后面是闭包体
    // 相当于 Java 的：Function<String, String> getStr = key -> ...;

    // 提取字符串字段的闭包
    let get_str = |key: &str| json.get(key).and_then(|v| v.as_str()).unwrap_or("").to_string();
    // 提取无符号整数字段的闭包
    let get_u64 = |key: &str| json.get(key).and_then(|v| v.as_u64()).unwrap_or(0);

    // 从 "hardware" 嵌套对象中提取硬件信息
    // json.get("hardware") 返回 Option<&Value>（对 json 变量的引用）
    let hw = json.get("hardware");
    let model = hw.and_then(|h| h.get("model")).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let cpu_model = hw.and_then(|h| h.get("cpu_model")).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let total_ram = hw.and_then(|h| h.get("total_ram")).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let os_version = hw.and_then(|h| h.get("os_version")).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let disk_size = hw.and_then(|h| h.get("disk_size")).and_then(|v| v.as_str()).unwrap_or("").to_string();

    // 从 "cpu" 嵌套对象中提取 CPU 信息
    let cpu_obj = json.get("cpu");
    let cpu_usage = cpu_obj
        .and_then(|c| c.get("usage"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let cpu_core_count = cpu_obj
        .and_then(|c| c.get("core_count"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    // 从 "memory" 嵌套对象中提取内存信息
    let mem = json.get("memory");
    let memory_used = mem.and_then(|m| m.get("used")).and_then(|v| v.as_u64()).unwrap_or(0);
    let memory_total = mem.and_then(|m| m.get("total")).and_then(|v| v.as_u64()).unwrap_or(0);
    let memory_available = mem.and_then(|m| m.get("available")).and_then(|v| v.as_u64()).unwrap_or(0);
    let memory_used_percent = mem.and_then(|m| m.get("used_percent")).and_then(|v| v.as_f64()).unwrap_or(0.0);

    // 从 "disks" 数组的第一个元素中提取磁盘信息
    // and_then(|a| a.first()) 取数组的第一个元素（如果数组不为空）
    let disk = json.get("disks").and_then(|d| d.as_array()).and_then(|a| a.first());
    let disk_used = disk.and_then(|d| d.get("used")).and_then(|v| v.as_u64()).unwrap_or(0);
    let disk_total = disk.and_then(|d| d.get("total")).and_then(|v| v.as_u64()).unwrap_or(0);
    let disk_used_percent = disk.and_then(|d| d.get("used_percent")).and_then(|v| v.as_f64()).unwrap_or(0.0);
    // 计算剩余磁盘空间（使用 saturating_sub 防止下溢，比 if-else 更安全且地道）
    let disk_free = disk_total.saturating_sub(disk_used);

    // 获取废纸篓大小（使用前面定义的 get_u64 闭包）
    let trash_size = get_u64("trash_size");

    // 构建并返回 SystemStatus 结构体
    Ok(SystemStatus {
        host: get_str("host"),
        platform: get_str("platform"),
        uptime: get_str("uptime"),
        uptime_seconds: get_u64("uptime_seconds"),
        health_score: get_u64("health_score"),
        health_score_msg: get_str("health_score_msg"),
        cpu_usage,
        cpu_core_count,
        memory_used,
        memory_total,
        memory_available,
        memory_used_percent,
        disk_used,
        disk_total,
        disk_free,
        disk_used_percent,
        disk_size,
        model,
        cpu_model,
        total_ram,
        os_version,
        trash_size,
    })
}

/// 清理预览（dry-run）：列出可以清理的文件，但不实际删除。
///
/// 运行 `mole clean --dry-run`，通过事件实时推送每行输出给前端。
/// 前端调用：await invoke('clean_dry_run')
#[tauri::command]
pub async fn clean_dry_run(app: AppHandle, window: Window) -> Result<CleanResult, String> {
    // .clone() 复制 AppHandle 和 Window（因为要 move 进闭包，需要所有权）
    // Tauri 的 AppHandle 和 Window 内部是 Arc（引用计数指针），clone 只是增加引用计数
    // 相当于 Java 中传对象引用，但 Rust 的 move 语义要求显式 clone
    let window_clone = window.clone();
    let app_clone = app.clone();

    // tokio::spawn 在 Tokio 异步运行时中启动一个新的异步任务
    // async move { ... } 中的 move 表示将 window_clone、app_clone 的所有权移入闭包
    // 这是 Rust 的所有权系统要求：变量必须在一个"地方"被使用
    let handle = tokio::spawn(async move {
        // lines 用于收集输出行（目前未填充，输出通过事件实时推送）
        let lines: Vec<String> = Vec::new();

        // 调用流式执行函数，运行 `mole clean --dry-run`
        let result = process::run_mole_streaming_with_timeout(
            Some(&app_clone),
            &["clean", "--dry-run"],   // 命令参数数组
            CLEAN_TIMEOUT_SECS,        // 超时时间（120秒）
            move |line| {
                // 每读到一行，就向前端发送一个事件
                emit_mole_event(&window_clone, "mole-clean_dry_run-event", &line);
            },
        )
        .await;

        // 根据执行结果构建 CleanResult
        match result {
            Ok(streaming) => {
                // 如果超时，生成错误信息
                let error = if streaming.timed_out {
                    Some(format!(
                        "Scan timed out after {}s. Showing partial results.",
                        CLEAN_TIMEOUT_SECS
                    ))
                } else {
                    None
                };
                CleanResult {
                    // 同时满足：退出码为 0（正常结束）且没有超时错误
                    success: streaming.exit_code == 0 && error.is_none(),
                    lines,
                    error,
                }
            }
            // 如果流式执行本身出错（如无法启动进程），返回失败结果
            Err(e) => CleanResult {
                success: false,
                lines,
                error: Some(e),
            },
        }
    });

    // 等待异步任务完成并获取结果
    // handle.await 等待 tokio::spawn 的任务完成
    // map_err 将 JoinError（任务 panic 等）转换为字符串错误
    handle.await.map_err(|e| format!("Task error: {}", e))
}

/// 执行实际清理（删除文件）。
///
/// 运行 `mole clean`（没有 --dry-run），通过事件实时推送每行输出给前端。
/// 前端调用：await invoke('clean_execute')
#[tauri::command]
pub async fn clean_execute(app: AppHandle, window: Window) -> Result<CleanResult, String> {
    let window_clone = window.clone();
    let app_clone = app.clone();

    let handle = tokio::spawn(async move {
        let lines: Vec<String> = Vec::new();

        let result = process::run_mole_streaming_with_timeout(
            Some(&app_clone),
            &["clean"],              // 不带 --dry-run，直接执行清理
            CLEAN_TIMEOUT_SECS,
            move |line| {
                emit_mole_event(&window_clone, "mole-clean_execute-event", &line);
            },
        )
        .await;

        match result {
            Ok(streaming) => {
                let error = if streaming.timed_out {
                    Some(format!("Clean timed out after {}s.", CLEAN_TIMEOUT_SECS))
                } else {
                    None
                };
                CleanResult {
                    success: streaming.exit_code == 0 && error.is_none(),
                    lines,
                    error,
                }
            }
            Err(e) => CleanResult {
                success: false,
                lines,
                error: Some(e),
            },
        }
    });

    handle.await.map_err(|e| format!("Task error: {}", e))
}

/// 扫描系统中已安装的应用（用于卸载功能）。
///
/// 运行 `mole uninstall --json`，解析每行 JSON 输出为 AppInfo 结构体。
/// 同时通过事件实时推送非 JSON 的进度信息给前端。
/// 前端调用：await invoke('uninstall_scan_apps')
/// 返回：Vec<AppInfo>（应用列表）
#[tauri::command]
pub async fn uninstall_scan_apps(app: AppHandle, window: Window) -> Result<Vec<AppInfo>, String> {
    // 引入线程安全的引用计数指针和互斥锁（在函数内部局部引入）
    // Arc<Mutex<T>> 是 Rust 中多线程共享可变数据的标准模式
    // 相当于 Java 的 CopyOnWriteArrayList 或 Collections.synchronizedList(new ArrayList<>())
    use std::sync::{Arc, Mutex};

    let window_clone = window.clone();
    let app_clone = app.clone();

    // 创建线程安全的 Vec，在主任务和回调闭包之间共享
    // Arc::new 创建引用计数指针（多个 Arc 可以指向同一个数据）
    // Mutex::new 包装数据，保证同一时间只有一个线程可以修改它
    let apps_arc = Arc::new(Mutex::new(Vec::<AppInfo>::new()));
    // clone() 复制 Arc（增加引用计数），让闭包也能访问同一个 Vec
    let apps_clone = apps_arc.clone();

    let handle = tokio::spawn(async move {
        let result = process::run_mole_streaming_with_timeout(
            Some(&app_clone),
            &["uninstall", "--json"], // --json 让 mole 以 JSON Lines 格式输出
            UNINSTALL_TIMEOUT_SECS,
            move |line| {
                // 检查这行是否是 JSON 对象（以 { 开头）
                if line.starts_with("{") {
                    // 尝试将 JSON 行解析为 AppInfo 结构体
                    if let Ok(app_info) = serde_json::from_str::<AppInfo>(&line) {
                        // 获取 Mutex 锁，将解析出的 AppInfo 加入列表
                        if let Ok(mut apps) = apps_clone.lock() {
                            apps.push(app_info);
                        }
                    }
                } else {
                    // 非 JSON 行（如进度信息），通过事件发送给前端显示
                    emit_mole_event(&window_clone, "mole-uninstall_scan_apps-event", &line);
                }
            },
        )
        .await;

        match result {
            Ok(_) => {
                // 获取最终的应用列表
                // unwrap_or_else(|e| e.into_inner()) 处理 Mutex "中毒"的情况
                // Mutex 中毒：如果持有锁的线程 panic，Mutex 变为"中毒"状态
                // into_inner() 从中毒的 MutexGuard 中恢复数据（Java 中没有对应概念）
                let apps = apps_arc.lock().unwrap_or_else(|e| e.into_inner()).clone();
                Ok(apps)
            }
            Err(e) => Err(e),
        }
    });

    // ? 将 JoinError 传播为函数错误
    handle.await.map_err(|e| format!("Task error: {}", e))?
}

/// 执行应用卸载。
///
/// 运行 `mole uninstall --targets "App1|App2|..."` 以管理员权限卸载选定的应用。
/// 使用 osascript 弹出 macOS 密码对话框获取权限。
/// 前端调用：await invoke('uninstall_execute', { targets: ["Slack", "Zoom"] })
#[tauri::command]
pub async fn uninstall_execute(
    app: AppHandle,
    window: Window,
    targets: Vec<String>, // 要卸载的应用名称列表（从前端传入）
) -> Result<CleanResult, String> {
    // 将应用名称列表用 | 连接成单个字符串（mole 的参数格式要求）
    // 如 ["Slack", "Zoom"] → "Slack|Zoom"
    let targets_str = targets.join("|");
    let window_clone = window.clone();
    let app_clone = app.clone();

    let handle = tokio::spawn(async move {
        let lines: Vec<String> = Vec::new();

        // 使用 sudo 版本的流式执行（会弹出 macOS 密码对话框）
        let result = process::run_mole_streaming_with_timeout_sudo(
            Some(&app_clone),
            &["uninstall", "--targets", &targets_str],
            UNINSTALL_TIMEOUT_SECS,
            move |line| {
                emit_mole_event(&window_clone, "mole-uninstall_execute-event", &line);
            },
        )
        .await;

        match result {
            Ok(streaming) => {
                let error = if streaming.timed_out {
                    Some(format!("Uninstall timed out after {}s.", UNINSTALL_TIMEOUT_SECS))
                } else {
                    None
                };
                CleanResult {
                    success: streaming.exit_code == 0 && error.is_none(),
                    lines,
                    error,
                }
            }
            Err(e) => CleanResult {
                success: false,
                lines,
                error: Some(e),
            },
        }
    });

    handle.await.map_err(|e| format!("Task error: {}", e))
}

/// 深度清理预览（dry-run）：列出将被深度清理的内容，但不实际删除。
///
/// 运行 `mole purge --dry-run`。
/// 前端调用：await invoke('purge_dry_run')
#[tauri::command]
pub async fn purge_dry_run(app: AppHandle, window: Window) -> Result<String, String> {
    let window_clone = window.clone();
    let app_clone = app.clone();

    let handle = tokio::spawn(async move {
        let result = process::run_mole_streaming_with_timeout(
            Some(&app_clone),
            &["purge", "--dry-run"],
            PURGE_TIMEOUT_SECS,
            move |line| {
                emit_mole_event(&window_clone, "mole-purge_dry_run-event", &line);
            },
        )
        .await;

        // match result { ... } 匹配执行结果
        // Ok(streaming) if streaming.timed_out => 带守卫的匹配：
        //   只有当 streaming.timed_out 为 true 时，这个 arm 才匹配
        //   if ... 部分叫做"匹配守卫"（match guard）
        match result {
            Ok(streaming) if streaming.timed_out => {
                Ok(format!(
                    "Purge scan timed out after {}s. Showing partial results.",
                    PURGE_TIMEOUT_SECS
                ))
            }
            Ok(_) => Ok(String::new()), // 正常结束，返回空字符串
            Err(e) => Err(e),
        }
    });

    handle
        .await
        .map_err(|e| format!("Task error: {}", e))?
}

/// 执行深度清理（删除选定目标）。
///
/// 运行 `mole purge --targets "Target1|Target2|..."`。
/// 前端调用：await invoke('purge_execute', { targets: ["..."] })
#[tauri::command]
pub async fn purge_execute(
    app: AppHandle,
    window: Window,
    targets: Vec<String>,
) -> Result<CleanResult, String> {
    let targets_str = targets.join("|");
    let window_clone = window.clone();
    let app_clone = app.clone();

    let handle = tokio::spawn(async move {
        let lines: Vec<String> = Vec::new();

        let result = process::run_mole_streaming_with_timeout(
            Some(&app_clone),
            &["purge", "--targets", &targets_str],
            PURGE_TIMEOUT_SECS,
            move |line| {
                emit_mole_event(&window_clone, "mole-purge_execute-event", &line);
            },
        )
        .await;

        match result {
            Ok(streaming) => {
                let error = if streaming.timed_out {
                    Some(format!("Purge timed out after {}s.", PURGE_TIMEOUT_SECS))
                } else {
                    None
                };
                CleanResult {
                    success: streaming.exit_code == 0 && error.is_none(),
                    lines,
                    error,
                }
            }
            Err(e) => CleanResult {
                success: false,
                lines,
                error: Some(e),
            },
        }
    });

    handle.await.map_err(|e| format!("Task error: {}", e))
}

/// 系统优化预览：获取可用的优化项列表，但不执行。
///
/// 优先尝试 `mole optimize --dry-run --json`（获取结构化数据）；
/// 如果失败，回退到 `mole optimize --dry-run`（文本输出）并手动解析。
/// 前端调用：await invoke('optimize_dry_run')
#[tauri::command]
pub async fn optimize_dry_run(app: AppHandle, window: Window) -> Result<OptimizeResult, String> {
    let window_clone = window.clone();
    let app_clone = app.clone();

    let handle = tokio::spawn(async move {
        // 首先尝试获取 JSON 格式的输出（更准确、更结构化）
        let json_output = process::run_mole_capture(
            Some(&app_clone),
            &["optimize", "--dry-run", "--json"],
        ).await;

        // 如果 JSON 模式成功，直接解析并返回
        if let Ok(output) = json_output {
            return parse_optimize_output(&output);
        }

        // 回退：使用流式模式，同时收集所有输出行用于文本解析
        // Arc<Mutex<Vec<String>>> 允许在回调闭包和主流程之间共享可变的行列表
        let collected_lines = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let collected_lines_clone = collected_lines.clone();

        let result = process::run_mole_streaming_with_timeout(
            Some(&app_clone),
            &["optimize", "--dry-run"],
            OPTIMIZE_TIMEOUT_SECS,
            move |line| {
                // 同时推送事件给前端（实时显示）和收集到本地（用于后续解析）
                emit_mole_event(&window_clone, "mole-optimize_dry_run-event", &line);
                if let Ok(mut lines) = collected_lines_clone.lock() {
                    lines.push(line.to_string());
                }
            },
        )
        .await;

        match result {
            Ok(_streaming) => {
                // 将收集的行合并为一个字符串，用于文本格式解析
                // join("\n") 相当于 Java 的 String.join("\n", lines)
                let output = if let Ok(lines) = collected_lines.lock() {
                    lines.join("\n")
                } else {
                    String::new()
                };
                // 解析并返回结果（超时或正常结束都返回已收集到的内容）
                parse_optimize_output(&output)
            }
            Err(_e) => {
                // 执行出错，返回空结果（而不是向前端抛出错误）
                OptimizeResult {
                    system_health: None,
                    optimizations: Vec::new(),
                    total_items: 0,
                    applied_count: 0,
                }
            }
        }
    });

    handle.await.map_err(|e| format!("Task error: {}", e))
}

/// 执行系统优化（应用选定的优化项）。
///
/// 运行 `mole optimize --actions "action1,action2,..."`。
/// 前端调用：await invoke('optimize_execute', { actions: ["disable_spotlight"] })
#[tauri::command]
pub async fn optimize_execute(
    app: AppHandle,
    window: Window,
    actions: Vec<String>, // 要执行的优化动作列表（从前端传入）
) -> Result<CleanResult, String> {
    // 将动作列表用逗号连接（mole 的参数格式）
    let actions_str = actions.join(",");
    let window_clone = window.clone();
    let app_clone = app.clone();

    let handle = tokio::spawn(async move {
        let lines: Vec<String> = Vec::new();

        let result = process::run_mole_streaming_with_timeout(
            Some(&app_clone),
            &["optimize", "--actions", &actions_str],
            OPTIMIZE_TIMEOUT_SECS,
            move |line| {
                emit_mole_event(&window_clone, "mole-optimize_execute-event", &line);
            },
        )
        .await;

        match result {
            Ok(streaming) => {
                let error = if streaming.timed_out {
                    Some(format!("Optimize timed out after {}s.", OPTIMIZE_TIMEOUT_SECS))
                } else {
                    None
                };
                CleanResult {
                    success: streaming.exit_code == 0 && error.is_none(),
                    lines,
                    error,
                }
            }
            Err(e) => CleanResult {
                success: false,
                lines,
                error: Some(e),
            },
        }
    });

    handle.await.map_err(|e| format!("Task error: {}", e))
}

/// 获取操作历史记录（JSON 格式）。
///
/// 运行 `mole history --json --limit N`。
/// 前端调用：await invoke('get_history', { limit: 50 })
/// 返回：JSON 字符串（由前端自行解析）
#[tauri::command]
pub async fn get_history(app: AppHandle, limit: Option<u32>) -> Result<String, String> {
    // unwrap_or(50) 如果前端没有传 limit 参数（None），默认取最近 50 条
    let limit_str = limit.unwrap_or(50).to_string();
    // 直接调用并返回 JSON 字符串，不做额外解析
    process::run_mole_capture(Some(&app), &["history", "--json", "--limit", &limit_str]).await
}

// ============================================================
// Analyze（磁盘分析）相关命令
// ============================================================

/// 全局原子标志位：用于向正在运行的 analyze 进程发送取消信号。
/// 当前端点击"停止"按钮时，cancel_analyze_scan 命令会将这个标志设为 true，
/// 正在运行的 run_mole_streaming_throttled 函数会检测到并停止进程。
///
/// AtomicBool::new(false) 初始化为 false（未取消）
static CANCEL_ANALYZE: AtomicBool = AtomicBool::new(false);

/// 开始磁盘分析扫描（异步扫描大文件/目录）。
///
/// 运行 `mole analyze --json [path]`，通过节流事件实时推送结果给前端。
/// 扫描是批量推送的（每 100ms 一批），防止事件洪水导致 UI 卡顿。
///
/// 前端调用：await invoke('analyze_scan', { path: '/Users/xxx' })
///   path 是可选的，不传则扫描整个系统
#[tauri::command]
pub async fn analyze_scan(
    app: AppHandle,
    window: Window,
    path: Option<String>, // 可选的扫描路径（None 表示扫描整个系统）
) -> Result<String, String> {
    let window_clone = window.clone();
    let app_clone = app.clone();

    // 启动新扫描前，重置取消标志（确保上次取消不影响本次扫描）
    // store(false, ...) 原子地将标志设为 false
    // Ordering::SeqCst 是最严格的内存顺序，确保所有线程立即看到更新
    CANCEL_ANALYZE.store(false, Ordering::SeqCst);

    eprintln!("[mole-gui] analyze_scan called with path: {:?}", path);

    let handle = tokio::spawn(async move {
        // 构建命令参数
        // mut 允许后续修改 args 向量（添加可选的路径参数）
        let mut args = vec!["analyze", "--json"];

        // 处理可选的路径参数
        // let path_ref 声明变量，稍后在 if 块中赋值（Rust 要求变量在使用前赋值）
        let path_ref;
        if let Some(ref p) = path {
            // ref p 是引用模式：p 是 path 内部字符串的引用，不移走所有权
            // as_str() 将 &String 转换为 &str
            path_ref = p.as_str();
            args.push(path_ref); // 如果指定了路径，加到参数末尾
        }

        // 使用节流版本的流式执行（批量推送，防止事件洪水）
        let result = process::run_mole_streaming_throttled(
            Some(&app_clone),
            &args,
            ANALYZE_TIMEOUT_SECS,
            &CANCEL_ANALYZE,       // 传入取消标志的引用
            move |lines: &[String]| {
                // 批量处理回调：将该批次的所有行解析并存入 batch_events 列表中，最后一次性发送
                // Vec::with_capacity(lines.len()) 预分配内存，相当于 Java 中的 new ArrayList<>(capacity)
                // 能够避免数组频繁扩容带来的性能损耗
                let mut batch_events = Vec::with_capacity(lines.len());
                for line in lines {
                    let parsed = parse_mole_line_to_event(line);
                    // extend 相当于 Java 中的 list.addAll(collection)
                    // 将一行解析出来的 0 到多个事件追加到 batch_events 列表末尾
                    batch_events.extend(parsed);
                }
                if !batch_events.is_empty() {
                    // window.emit 发送一个批量 Tauri 事件，载荷（Payload）是一个事件数组
                    // 这样原本每行发送一次 IPC，现在每 100ms/50条才发送一次，IPC 传输次数减少 95% 以上
                    // let _ = 忽略返回值（Result），防止编译器发出未使用 Result 的警告
                    let _ = window_clone.emit("mole-analyze_scan-event", &batch_events);
                }
            },
        )
        .await;

        match result {
            Ok(streaming) => {
                if streaming.timed_out {
                    // 超时：返回错误（部分结果已通过事件推送）
                    return Err(format!(
                        "Analyze scan timed out after {}s. Showing partial results.",
                        ANALYZE_TIMEOUT_SECS
                    ));
                }
                if streaming.cancelled {
                    // 用户取消：正常结束，返回空字符串
                    eprintln!("[mole-gui] Analyze scan was cancelled by user");
                    return Ok(String::new());
                }
                Ok(String::new()) // 正常完成
            }
            Err(e) => {
                // 如果错误信息包含 "cancelled"，也视为正常取消（而非错误）
                if e.contains("cancelled") {
                    eprintln!("[mole-gui] Analyze scan was gracefully cancelled: {}", e);
                    Ok(String::new())
                } else {
                    Err(e) // 真正的错误，向前端报告
                }
            }
        }
    });

    // ? 在这里的作用：将两层 Result 拍平
    // handle.await → Result<Result<String, String>, JoinError>
    // map_err 将 JoinError 转为 String
    // ? 将外层 Result 解包（如果是 Err 则向上传播）
    handle.await.map_err(|e| format!("Task error: {}", e))?
}

/// 取消正在进行的 analyze 扫描。
///
/// 通过设置全局原子标志位来通知扫描循环停止。
/// 前端调用：await invoke('cancel_analyze_scan')（由"停止"按钮触发）
#[tauri::command]
pub async fn cancel_analyze_scan() -> Result<(), String> {
    // 将取消标志设为 true，下次扫描循环检查到时会杀掉子进程
    CANCEL_ANALYZE.store(true, Ordering::SeqCst);
    Ok(())
}

/// 删除分析扫描结果中选定的文件/目录（移入废纸篓）。
///
/// 通过 macOS Finder 的 AppleScript 接口将文件移入废纸篓（而不是直接 rm -rf）。
/// 前端调用：await invoke('analyze_delete', { paths: ["/path/to/file"] })
#[tauri::command]
pub async fn analyze_delete(
    _app: AppHandle,          // _app 前缀下划线表示这个参数暂时未使用（避免编译警告）
    window: Window,
    paths: Vec<String>,       // 要删除的文件/目录路径列表
) -> Result<CleanResult, String> {
    // 参数验证：不允许空列表
    if paths.is_empty() {
        return Err("No paths to delete".to_string());
    }

    // 安全验证：对每个路径进行合法性检查
    // for path in &paths 迭代路径的引用（& 表示借用，不移走所有权）
    // 这样 paths 在后面还能继续使用
    for path in &paths {
        validate_path(path)?; // ? 如果验证失败，直接将 Err 传播给调用方
    }

    // 统计成功删除的文件数
    let mut success_count = 0;
    // 收集失败的错误信息
    let mut errors: Vec<String> = Vec::new();

    // 逐个将文件移入废纸篓
    for path in &paths {
        match move_to_trash(path).await {
            Ok(_) => {
                success_count += 1;
                // 通知前端这个文件已成功移入废纸篓
                emit_mole_event(&window, "mole-analyze_delete-event",
                    &format!("Successfully moved to trash: {}", path));
            }
            Err(e) => {
                // 某个文件失败，记录错误但继续处理其他文件
                errors.push(format!("Failed to delete {}: {}", path, e));
            }
        }
    }

    // 如果有任何失败，返回错误信息（部分成功也算失败）
    if !errors.is_empty() {
        return Ok(CleanResult {
            success: false,
            lines: vec![], // 空向量字面量（相当于 Java 的 new ArrayList<>()）
            // errors.join("\n") 将所有错误信息用换行连接
            error: Some(errors.join("\n")),
        });
    }

    // 全部成功
    Ok(CleanResult {
        success: true,
        lines: vec![format!("Successfully moved {} item(s) to trash", success_count)],
        error: None,
    })
}

/// 验证路径的合法性和安全性，防止危险操作。
///
/// 检查：
///   1. 路径不能为空
///   2. 必须是绝对路径（以 / 开头）
///   3. 不能包含 null 字节（防止注入攻击）
///   4. 不能包含路径穿越（..）（防止删除意料之外的目录）
fn validate_path(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("Path is empty".to_string());
    }
    // starts_with('/') 检查是否以正斜杠开头（即绝对路径）
    // ! 是逻辑非（相当于 Java 的 !）
    if !path.starts_with('/') {
        return Err(format!("Path must be absolute: {}", path));
    }
    // '\0' 是 null 字节（字符字面量用单引号）
    // 路径中的 null 字节是许多系统调用的字符串终止符，可能导致路径被截断
    if path.contains('\0') {
        return Err("Path contains null bytes".to_string());
    }
    // 防止路径穿越攻击（如 "/Users/xxx/../../../etc/passwd"）
    if path.contains("..") {
        return Err(format!("Path contains traversal components: {}", path));
    }
    Ok(())
}

/// 将文件或目录移入 macOS 废纸篓（通过 Finder AppleScript）。
///
/// 使用 Finder 而不是直接 rm，可以：
/// 1. 让用户有机会恢复误删的文件
/// 2. 避免权限问题（Finder 有时比进程有更多权限）
/// 3. 符合 macOS 用户体验规范
async fn move_to_trash(path: &str) -> Result<(), String> {
    // 转义路径中的特殊字符（防止 AppleScript 注入）
    // replace('\\', "\\\\") 单个反斜杠 → 双反斜杠（字符串转义）
    // replace('"', "\\\"")  双引号 → 转义的双引号（防止 AppleScript 字符串被截断）
    let escaped_path = path.replace('\\', "\\\\").replace('"', "\\\"");

    // 构建 AppleScript 脚本：告诉 Finder 删除指定路径（"删除"在 Finder 中就是移入废纸篓）
    // POSIX file "..." 将 POSIX 路径转换为 Finder 文件引用
    let script = format!("tell application \"Finder\" to delete POSIX file \"{}\"", escaped_path);

    // 通过 osascript 执行 AppleScript
    let output = tokio::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .await
        .map_err(|e| format!("Failed to execute osascript: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "Finder failed to move to trash: {}",
            // 优先使用 stderr，如果 stderr 为空则用 stdout
            // .then(|| ...) 是 bool 的方法：如果条件为 true 则返回 Some(...)，否则返回 None
            // unwrap_or(stderr.trim()) 如果 stdout 也为空，回退到 stderr
            if stderr.trim().is_empty() { stdout.trim() } else { stderr.trim() }
        ));
    }

    Ok(())
}

// ============================================================
// sudo 权限管理命令（委托给 mole::sudo 模块处理）
// ============================================================

/// 检查当前 sudo 权限会话是否有效。
///
/// 前端调用：await invoke('check_sudo_session')
/// 返回：true 表示有缓存的 sudo 会话（无需重新输入密码）
#[tauri::command]
pub async fn check_sudo_session() -> Result<bool, String> {
    // crate::mole::sudo 是使用绝对路径访问模块（crate 表示当前包的根）
    // 相当于 Java 的完全限定类名：com.example.mole.sudo.checkSudoSession()
    Ok(crate::mole::sudo::check_sudo_session().await)
}

/// 通过 macOS GUI 密码对话框请求管理员权限。
///
/// 前端调用：await invoke('request_sudo_session')
/// 返回：true 表示用户成功输入密码，false 表示取消
#[tauri::command]
pub async fn request_sudo_session() -> Result<bool, String> {
    crate::mole::sudo::request_sudo_session().await
}

/// 使当前 sudo 权限会话失效（清除密码缓存）。
///
/// 前端调用：await invoke('stop_sudo_session')
#[tauri::command]
pub async fn stop_sudo_session() -> Result<(), String> {
    crate::mole::sudo::stop_sudo_session().await;
    Ok(())
}

// ============================================================
// Mole CLI 路径配置命令
// ============================================================

/// 获取当前的 Mole CLI 路径配置。
///
/// 返回用户自定义路径和实际使用的解析路径（二者可能不同）。
/// 前端调用：await invoke('get_mole_path_config')
#[tauri::command]
pub async fn get_mole_path_config(app: AppHandle) -> Result<MolePathConfig, String> {
    // 读取用户自定义路径（可能为 None）
    let custom_path = settings::get_configured_mole_path(&app)
        .map(|p| p.to_string_lossy().to_string()) // 将 PathBuf 转为 String
        .unwrap_or_default();                       // None 时使用默认值（空字符串）

    // 解析实际使用的 Mole 路径（考虑自定义配置、PATH 查找等所有因素）
    let resolved_path = process::find_mole_path(Some(&app))
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    Ok(MolePathConfig {
        custom_path,
        resolved_path,
    })
}

/// 设置用户自定义的 Mole CLI 路径。
///
/// 保存配置后，返回更新后的路径配置（包含自定义路径和新的解析路径）。
/// 传入空字符串 "" 表示清除自定义路径，恢复自动检测。
///
/// 前端调用：await invoke('set_mole_path_config', { path: '/usr/local/bin/mole' })
#[tauri::command]
pub async fn set_mole_path_config(app: AppHandle, path: String) -> Result<MolePathConfig, String> {
    // 将新路径保存到持久化存储（? 如果保存失败则向上传播错误）
    settings::set_configured_mole_path(&app, &path)?;

    // 计算保存后的 custom_path（用于返回给前端）
    let custom_path = if path.is_empty() {
        // 用户清除了自定义路径
        String::new()
    } else {
        // 检查新路径是否存在
        let p = std::path::PathBuf::from(&path);
        if p.exists() {
            p.to_string_lossy().to_string() // 路径存在，返回它
        } else {
            String::new() // 路径不存在，返回空字符串提示前端
        }
    };

    // 重新解析实际使用的路径（会用到刚刚保存的自定义路径）
    let resolved_path = process::find_mole_path(Some(&app))
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    Ok(MolePathConfig {
        custom_path,
        resolved_path,
    })
}

/// 卸载关联文件的信息
/// #[derive(...)] 是 Rust 中的派生宏，自动实现特定 trait：
///   Serialize — 自动序列化为 JSON，以便前端 JavaScript 接收
///   Clone — 允许进行深复制（类似 Java 中的 clone()）
///   Debug — 允许使用 {:?} 进行格式化打印，用于开发调试
#[derive(Serialize, Clone, Debug)]
pub struct UninstallFileItem {
    /// 关联文件或目录的绝对路径，例如 "/Users/xxx/Library/Caches/..."
    pub path: String,
    /// 关联文件或目录的大小（单位：KB）
    pub size_kb: u64,
}

/// 单个应用的卸载预览计划（整合了应用主路径及各种残留/关联文件）
#[derive(Serialize, Clone, Debug)]
pub struct UninstallPreviewApp {
    /// 应用程序在系统中的显示名称，例如 "Slack"
    pub app_name: String,
    /// 应用程序本身的安装路径，例如 "/Applications/Slack.app"
    pub app_path: String,
    /// 用户私有的关联文件列表（如 Caches、Application Support、Preferences 等）
    pub user_files: Vec<UninstallFileItem>,
    /// 系统全局 of 关联文件列表（通常需要 sudo 权限才能写入/删除，如 /Library/LaunchDaemons 中的服务描述文件）
    pub system_files: Vec<UninstallFileItem>,
    /// 仅供审查查看的关联路径列表（受系统保护或仅用于展示提示，不会在当前阶段执行删除）
    pub review_only_files: Vec<UninstallFileItem>,
    /// 当前应用预计删除的文件总大小（单位：KB）
    pub total_size_kb: u64,
}

/// 整个卸载操作的预览结果摘要（会被序列化并直接返回给前端）
#[derive(Serialize, Clone, Debug)]
pub struct UninstallPreviewResult {
    /// 用户选定的待卸载应用的目标名称列表，例如 ["Slack", "Zoom"]
    pub targets: Vec<String>,
    /// 每个选定应用的详细卸载与残留文件清除计划
    pub removal_plan: Vec<UninstallPreviewApp>,
    /// 所有待卸载文件预计释放的总大小（单位：KB）
    pub total_size_kb: u64,
    /// 是否需要使用系统管理员（Sudo）权限（当存在 system_files 时为 true，提示前端在确认时触发密码对话框）
    pub requires_sudo: bool,
}

/// 预览应用卸载计划，扫描出要卸载的关联文件但不会实际执行删除。
///
/// 参数：
///   app — Tauri 应用句柄（类似于 Java 中的 ApplicationContext，用于寻找 Mole CLI 路径配置）
///   targets — 要卸载的应用程序名称列表（对应前端传入 of targets 数组）
/// 返回：
///   Result<UninstallPreviewResult, String> — 成功时返回卸载预览计划，失败时返回错误描述字符串。
///   Result<T, E> 类比于 Java 中带有 Checked Exception 的返回值，
///   Ok(T) 相当于正常返回结果，Err(E) 相当于抛出异常。
#[tauri::command]
pub async fn uninstall_preview(
    app: AppHandle,
    targets: Vec<String>,
) -> Result<UninstallPreviewResult, String> {
    // .clone() 复制 AppHandle（因为要在异步闭包中 move 所有权）
    // AppHandle 内部是由 Arc（引用计数指针）包装的，clone 只是增加计数，性能开销极小
    let app_clone = app.clone();
    
    // 参数验证，如果传入的目标应用列表为空，立即抛出错误（类似 Java 抛出 IllegalArgumentException）
    if targets.is_empty() {
        return Err("No targets specified for preview".to_string());
    }

    // 将应用名称列表使用 | 连接成单个字符串（Mole CLI 参数格式要求，如 "Slack|Zoom"）
    let targets_str = targets.join("|");

    // tokio::spawn 在 Tokio 异步运行时中启动一个新的异步任务（Java 中的多线程）
    // async move 声明这是一个异步闭包，并且 move 会把局部变量（app_clone 等）的所有权转移进闭包内
    let handle = tokio::spawn(async move {
        // 查找 Mole CLI 在系统中的可用路径
        // Some(&app_clone) 将 AppHandle 引用包装在 Option 中
        // ok_or_else 类似于 Java 的 optional.orElseThrow()，用于在为空时生成一个 Err 结果
        // ? 语法：如果是 Err 则直接从闭包返回，不再向下执行
        let mole_path = process::find_mole_path(Some(&app_clone)).ok_or_else(|| {
            "Mole CLI not found. Please install it first or configure the path in Settings.".to_string()
        })?;

        // 运行 `mole uninstall --json --dry-run --targets "..."` 子进程
        // tokio::process::Command 类似于 Java 中的 ProcessBuilder
        // .output() 异步执行子进程，并捕获它的 stdout、stderr 和 status
        // .await 表示暂停当前协程，等待子进程返回（非阻塞）
        let output = tokio::process::Command::new(&mole_path)
            .args(["uninstall", "--json", "--dry-run", "--targets", &targets_str])
            .env("LC_ALL", "C")
            .env("NO_COLOR", "1")
            .output()
            .await
            .map_err(|e| format!("Failed to run Mole: {}", e))?;

        // 将子进程的标准错误 stderr 转换为 UTF-8 字符串
        // 在 json 模式下，Mole CLI 将人可读的流式文件删除预览结果通过 stderr 吐出以保持 stdout 只有 JSON
        // String::from_utf8_lossy 会把非法字节安全地替换为 unicode 占位符，类似 Java 的 CharsetDecoder.replaceWith
        let stderr_str = String::from_utf8_lossy(&output.stderr);
        
        // 用于存储解析出的所有应用的删除计划，类似于 Java 的 ArrayList<UninstallPreviewApp>
        let mut removal_plan = Vec::new();
        // 追踪当前正在解析的应用程序，Option 类似于 Java 中的 Optional
        let mut current_app: Option<UninstallPreviewApp> = None;
        // 统计所有应用需要删除的文件总大小
        let mut total_size_kb = 0;
        // 标识是否需要管理员 sudo 权限
        let mut requires_sudo = false;

        // stderr_str.lines() 获取行的迭代器，类似于 Java 中的 BufferedReader.lines()
        for line in stderr_str.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // 检查这行是否是一个应用的起始头（例如：◎ PmsetMenu , 3.4MB）
            // 我们通过它是否以指定的前缀 ◎/✓/✔ 开头、是否包含逗号且不缩进（没有两个前导空格）来判断
            // !line.starts_with("  ") 类似于 Java 中的 !line.startsWith("  ")
            if (trimmed.starts_with("◎") || trimmed.starts_with("✓") || trimmed.starts_with("✔"))
                && trimmed.contains(',')
                && !line.starts_with("  ")
            {
                // 如果在上一个应用未存入 removal_plan 时，current_app 有值，则先推入数组
                // take() 会取出 Option 中的值，并将 Option 还原为 None（拥有所有权转移，避免 clone()）
                if let Some(app) = current_app.take() {
                    removal_plan.push(app);
                }

                // 剔除前缀 ◎/✓/✔ 等标识符号
                let cleaned = trimmed
                    .trim_start_matches('◎')
                    .trim_start_matches('✓')
                    .trim_start_matches('✔')
                    .trim();
                // 按照逗号分割为应用名和大小描述，例如 ["PmsetMenu", "3.4MB"]
                let parts: Vec<&str> = cleaned.split(',').collect();
                let app_name = parts[0].trim().to_string();
                let app_size_str = if parts.len() > 1 { parts[1].trim() } else { "0KB" };
                // 调用已有的 parse_size_to_kb 解析大小，转换为 u64 整数
                let app_size_kb = parse_size_to_kb(app_size_str).unwrap_or(0.0) as u64;

                current_app = Some(UninstallPreviewApp {
                    app_name,
                    app_path: String::new(),
                    user_files: Vec::new(),
                    system_files: Vec::new(),
                    review_only_files: Vec::new(),
                    total_size_kb: app_size_kb,
                });
                continue;
            }

            // 检查这行是否是具体的关联文件条目（有 2 个空格缩进）
            if line.starts_with("  ") {
                // 如果当前正在记录某个应用的信息
                // ref mut app 绑定了 Some 内部的可变借用（类似于 Java 的 app 引用）
                if let Some(ref mut app) = current_app {
                    // 剥离符号
                    let item_line = trimmed
                        .trim_start_matches('✓')
                        .trim_start_matches('◎')
                        .trim_start_matches('✔')
                        .trim();

                    // 分类文件类别
                    if item_line.starts_with("System:") {
                        requires_sudo = true; // 包含系统文件，需要管理员权限
                        let path_and_size = item_line.trim_start_matches("System:").trim();
                        let (path, size) = parse_path_and_size(path_and_size);
                        app.system_files.push(UninstallFileItem { path, size_kb: size });
                    } else if item_line.starts_with("Review only:") {
                        let path_and_size = item_line.trim_start_matches("Review only:").trim();
                        let (path, size) = parse_path_and_size(path_and_size);
                        app.review_only_files.push(UninstallFileItem { path, size_kb: size });
                    } else {
                        let (path, size) = parse_path_and_size(item_line);
                        // 若 app_path 为空，且此项以 ".app" 结尾，可断定这就是应用主程序路径
                        if app.app_path.is_empty() && path.ends_with(".app") {
                            app.app_path = path.clone();
                        }
                        app.user_files.push(UninstallFileItem { path, size_kb: size });
                    }
                }
            }
        }

        // 把循环结束后最后一个正在记录的应用也放进 removal_plan 列表中
        if let Some(app) = current_app {
            removal_plan.push(app);
        }

        // 累加总大小
        for app in &removal_plan {
            total_size_kb += app.total_size_kb;
        }

        // 返回整合好的最终结果
        Ok(UninstallPreviewResult {
            targets,
            removal_plan,
            total_size_kb,
            requires_sudo,
        })
    });

    // handle.await 可能会有 JoinError 发生（比如子线程崩溃/被强杀，相当于 Java 的 ExecutionException）
    // ? 将错误向外层传播解包
    handle.await.map_err(|e| format!("Task error: {}", e))?
}

/// 辅助函数：从预览文件行中分离路径与可能携带的文件大小后缀。
///
/// 参数：
///   line — 包含路径与可能的大小后缀的字符串（例如："/Applications/PmsetMenu.app , 3.3MB"）
/// 返回：
///   (String, u64) — 元组类型（Tuple），第一部分为解析后的路径，第二部分为大小（KB）
fn parse_path_and_size(line: &str) -> (String, u64) {
    // 如果行中包含逗号，说明末尾有大小规格，需要进行切割
    if line.contains(',') {
        let parts: Vec<&str> = line.split(',').collect();
        // 最后一个元素代表大小，例如 "3.3MB"
        let size_str = parts[parts.len() - 1].trim();
        // 前面的所有元素以逗号拼接回原样，以防路径中本身含有逗号
        let path_parts = &parts[0..parts.len() - 1];
        let path = path_parts.join(",").trim().to_string();
        // 转换为 KB 大小数值，默认值为 0.0
        let size_kb = parse_size_to_kb(size_str).unwrap_or(0.0) as u64;
        (path, size_kb)
    } else {
        // 如果没有逗号，说明没有大小信息，默认为 0
        (line.trim().to_string(), 0)
    }
}

/// 获取指定模式（"clean" 或 "optimize"）下的用户自定义白名单配置。
///
/// 参数：mode — 模式名称，可选值为 "clean"（清理白名单）或 "optimize"（优化忽略项）
/// 返回：Result<Vec<String>, String> — 成功时返回包含规则列表的 Vec，失败时返回错误描述字符串。
///   Result 是 Rust 中处理错误的核心类型，相当于 Java 的抛出受检异常（checked exception）。
///   Vec<String> 相当于 Java 里的 ArrayList<String>。
#[tauri::command]
pub async fn get_whitelist_config(mode: String) -> Result<Vec<String>, String> {
    // 获取用户 HOME 环境变量，相当于 Java 里的 System.getenv("HOME")
    // std::env::var 返回的是一个 Result，因此用 map_err 将可能发生的 VarError 转换为 String，
    // 然后用 ? 语法糖快速解包。如果是 Err，函数会提前返回错误（相当于 Java 的 throw new Exception()）
    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .map_err(|e| format!("无法获取 HOME 目录环境变量: {}", e))?;
    
    // 根据模式确定对应文件名，相当于 Java 的 if-else 条件赋值
    let file_name = if mode == "optimize" {
        "whitelist_optimize"
    } else {
        "whitelist"
    };
    
    // 拼接完整的文件路径
    let path = home.join(".config").join("mole").join(file_name);
    
    // 检查文件是否存在，如果不存在则返回一个空向量（相当于 Java 中的 new ArrayList<>()）
    if !path.exists() {
        return Ok(Vec::new());
    }
    
    // 读取文件内容为完整的 UTF-8 字符串
    // std::fs::read_to_string 类似于 Java 的 Files.readString(path)
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("读取白名单文件失败: {}", e))?;
    
    let mut patterns = Vec::new();
    // 遍历文件的每一行并过滤掉空行和注释行
    // content.lines() 返回行的迭代器，相当于 Java 的 BufferedReader.lines()
    for line in content.lines() {
        let trimmed = line.trim();
        // 忽略空行和以 '#' 开头的注释行
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        // to_string() 将只读引用 &str 转换为具有所有权的 String，类似于 Java 的 new String(line)
        patterns.push(trimmed.to_string());
    }
    
    // Ok() 包裹成功返回的数据，代表 Rust 中的成功流
    Ok(patterns)
}

/// 保存白名单配置到配置文件中。
///
/// 参数：
///   mode     — 模式名称（"clean" 或 "optimize"）
///   patterns — 要保存的白名单规则列表
/// 返回：
///   Result<(), String> — 成功返回 Ok(())，相当于 Java 中的 void 方法；失败时返回 Err。
#[tauri::command]
pub async fn save_whitelist_config(mode: String, patterns: Vec<String>) -> Result<(), String> {
    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .map_err(|e| format!("无法获取 HOME 目录环境变量: {}", e))?;
    
    // 获取对应的文件名和文件头注释说明
    let (file_name, header) = if mode == "optimize" {
        ("whitelist_optimize", "# Mole 优化白名单 - 在执行系统优化时，以下列出的任务将被跳过\n# 每行填写一个任务名称（如 system_maintenance）")
    } else {
        ("whitelist", "# Mole 清理白名单 - 被匹配到的路径在清理时不会被删除\n# 支持使用 ~ 代表用户主目录，支持 * 通配符，每行填写一个模式")
    };
    
    // 确保父级配置目录存在（`~/.config/mole`），相当于 Java 中的 file.getParentFile().mkdirs()
    let config_dir = home.join(".config").join("mole");
    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("创建配置文件夹失败: {}", e))?;
        
    let path = config_dir.join(file_name);
    
    // 构建文件内容，以自定义的头部注释开始
    let mut content = header.to_string();
    content.push_str("\n\n");
    // 将各项模式添加进内容中
    for pattern in patterns {
        let trimmed = pattern.trim();
        if !trimmed.is_empty() {
            content.push_str(trimmed);
            content.push('\n');
        }
    }
    
    // 将字符串内容写入文件，覆盖原有内容，类似于 Java 的 Files.writeString()
    std::fs::write(&path, content)
        .map_err(|e| format!("保存白名单配置失败: {}", e))?;
        
    Ok(())
}

/// 获取清除命令（mo purge）的自定义扫描目录配置。
///
/// 返回：Result<Vec<String>, String> — 扫描路径列表。
#[tauri::command]
pub async fn get_purge_paths() -> Result<Vec<String>, String> {
    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .map_err(|e| format!("无法获取 HOME 目录环境变量: {}", e))?;
    
    let path = home.join(".config").join("mole").join("purge_paths");
    if !path.exists() {
        return Ok(Vec::new());
    }
    
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("读取项目扫描路径配置失败: {}", e))?;
    
    let mut paths = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        paths.push(trimmed.to_string());
    }
    
    Ok(paths)
}

/// 保存清除命令（mo purge）的自定义扫描目录配置。
///
/// 参数：paths — 扫描目录路径列表
/// 返回：Result<(), String>
#[tauri::command]
pub async fn save_purge_paths(paths: Vec<String>) -> Result<(), String> {
    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .map_err(|e| format!("无法获取 HOME 目录环境变量: {}", e))?;
    
    let header = "# Mole 项目清理路径 - 自动发现并清理代码构建产物的扫描根目录\n# 可运行: mo purge --paths 进行交互式配置，或者在此处直接修改\n# 每行填写一个文件夹路径（支持以 ~ 开头代表用户家目录）";
    
    let config_dir = home.join(".config").join("mole");
    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("创建配置文件夹失败: {}", e))?;
        
    let path = config_dir.join("purge_paths");
    
    let mut content = header.to_string();
    content.push_str("\n\n");
    for p in paths {
        let trimmed = p.trim();
        if !trimmed.is_empty() {
            content.push_str(trimmed);
            content.push('\n');
        }
    }
    
    std::fs::write(&path, content)
        .map_err(|e| format!("保存项目扫描路径失败: {}", e))?;
        
    Ok(())
}

/// 获取当前系统的 Touch ID Sudo 授权状态。
///
/// 参数：app — Tauri 应用句柄（在后端获取已配置的 Mole 路径）
/// 返回：Result<bool, String> — true 表示已配置启用，false 表示未配置（使用普通密码）
#[tauri::command]
pub async fn get_touchid_status(app: AppHandle) -> Result<bool, String> {
    // 运行 `mole touchid status` 命令来查询状态
    // process::run_mole_capture 是我们项目中封装好的 CLI 调用助手，内置超时保护以防止阻塞
    let output = process::run_mole_capture(Some(&app), &["touchid", "status"]).await?;
    
    // 如果命令行输出包含 "enabled"，说明当前已启用 Touch ID 授权
    // .contains() 相当于 Java 中的 String.contains()
    Ok(output.contains("enabled"))
}

/// 启用或禁用系统 Touch ID Sudo 授权（指纹密码）。
///
/// **重要设计决策**：这里不再通过 `mole touchid enable/disable` CLI 命令来操作，
/// 而是直接在 osascript 的 shell 脚本中内联执行 PAM 文件操作。
///
/// **原因**：`mole touchid` 脚本内部使用 `sudo tee` 和 `sudo install` 写入 `/etc/pam.d/sudo_local`。
/// 当通过 `osascript "do shell script ... with administrator privileges"` 调用时，
/// 虽然 shell 以 root 身份运行，但 macOS 的 TCC (Transparency, Consent, and Control) 机制
/// 不会将父级应用的「完全磁盘访问权限 (FDA)」传递给 osascript 子进程。
/// 这导致 `tee`/`install` 写入 `/etc/pam.d/` 时报 `Operation not permitted`。
///
/// **解决方案**：跳过 mole CLI，直接在 osascript 提权 shell 中用 `/bin/sh` 原生命令
/// （如 `echo >>`、`grep -v`、`cat`、`chmod`、`chown`）操作文件。
/// `do shell script ... with administrator privileges` 本身就是 root，
/// 在该上下文中直接操作文件不受 TCC 的 FDA 检查限制。
///
/// 参数：
///   enabled — true 表示启用，false 表示禁用
/// 返回：
///   Result<(), String> — 成功返回 Ok(())，若在密码确认中取消或执行失败则返回错误。
#[tauri::command]
pub async fn set_touchid_enabled(enabled: bool) -> Result<(), String> {
    // PAM 配置文件路径（macOS Sonoma+ 使用 sudo_local）
    let pam_sudo = "/etc/pam.d/sudo";
    let pam_sudo_local = "/etc/pam.d/sudo_local";
    // PAM 行：Touch ID 认证模块（sufficient 表示"如果此模块认证成功就足够了"）
    let pam_tid_line = "auth       sufficient     pam_tid.so";

    // 根据 enable/disable 构建不同的 shell 脚本
    let shell_script = if enabled {
        // 启用 Touch ID：
        // 1. 检查 /etc/pam.d/sudo 中是否引用了 sudo_local（macOS Sonoma+ 的新方式）
        // 2. 如果引用了，则在 sudo_local 中添加 pam_tid.so 行
        // 3. 如果没引用（旧版 macOS），则直接修改 /etc/pam.d/sudo
        format!(
            r#"
if grep -q 'sudo_local' '{pam_sudo}' 2>/dev/null; then
    if [ -f '{pam_sudo_local}' ] && grep -q 'pam_tid.so' '{pam_sudo_local}' 2>/dev/null; then
        echo 'already_enabled'
    else
        if [ ! -f '{pam_sudo_local}' ]; then
            echo '# sudo_local: local customizations for sudo' > '{pam_sudo_local}'
        fi
        echo '{pam_tid_line}' >> '{pam_sudo_local}'
        chmod 444 '{pam_sudo_local}'
        chown root:wheel '{pam_sudo_local}'
        echo 'ok'
    fi
else
    if grep -q 'pam_tid.so' '{pam_sudo}' 2>/dev/null; then
        echo 'already_enabled'
    else
        TMP=$(mktemp)
        head -1 '{pam_sudo}' > "$TMP"
        echo '{pam_tid_line}' >> "$TMP"
        tail -n +2 '{pam_sudo}' >> "$TMP"
        cat "$TMP" > '{pam_sudo}'
        rm -f "$TMP"
        echo 'ok'
    fi
fi
"#,
            pam_sudo = pam_sudo,
            pam_sudo_local = pam_sudo_local,
            pam_tid_line = pam_tid_line,
        )
    } else {
        // 禁用 Touch ID：
        // 1. 从 sudo_local 和 sudo 中移除所有包含 pam_tid.so 的行
        format!(
            r#"
FOUND=0
if [ -f '{pam_sudo_local}' ] && grep -q 'pam_tid.so' '{pam_sudo_local}' 2>/dev/null; then
    TMP=$(mktemp)
    grep -v 'pam_tid.so' '{pam_sudo_local}' > "$TMP"
    cat "$TMP" > '{pam_sudo_local}'
    chmod 444 '{pam_sudo_local}'
    chown root:wheel '{pam_sudo_local}'
    rm -f "$TMP"
    FOUND=1
fi
if grep -q 'pam_tid.so' '{pam_sudo}' 2>/dev/null; then
    TMP=$(mktemp)
    grep -v 'pam_tid.so' '{pam_sudo}' > "$TMP"
    cat "$TMP" > '{pam_sudo}'
    rm -f "$TMP"
    FOUND=1
fi
if [ "$FOUND" -eq 0 ]; then
    echo 'already_disabled'
else
    echo 'ok'
fi
"#,
            pam_sudo = pam_sudo,
            pam_sudo_local = pam_sudo_local,
        )
    };

    // 将 shell 脚本包装进 AppleScript 的 `do shell script ... with administrator privileges`
    // 这会弹出 macOS 原生密码对话框，用户输入密码后以 root 身份执行脚本
    // 注意：shell 脚本中的反斜杠和双引号需要在 AppleScript 字符串中进行转义
    let escaped = shell_script
        .replace('\\', "\\\\")   // 反斜杠 → 双反斜杠
        .replace('"', "\\\"");   // 双引号 → 转义双引号

    let apple_script = format!(
        "do shell script \"{}\" with administrator privileges",
        escaped
    );

    // 使用 tokio 异步 Command 执行 osascript
    // osascript 是 macOS 内置的 AppleScript 解释器
    let output = tokio::process::Command::new("osascript")
        .arg("-e")
        .arg(&apple_script)
        .output()
        .await
        .map_err(|e| format!("启动 osascript 失败: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        // 提取 stderr 中的错误信息，帮助用户理解失败原因
        let stderr = String::from_utf8_lossy(&output.stderr);
        // 如果 stderr 中包含 "User canceled"，说明用户在密码对话框中点击了取消
        if stderr.contains("User canceled") {
            Err("操作已取消".to_string())
        } else {
            Err(format!("配置 Touch ID 授权失败: {}", stderr.trim()))
        }
    }
}

/// 调用 macOS 的 open 工具打开系统偏好设置的完全磁盘访问权限 (FDA) 页面。
///
/// 前端调用：await invoke('open_fda_settings')
/// 返回：Result<(), String>
#[tauri::command]
pub fn open_fda_settings() -> Result<(), String> {
    // std::process::Command 类似于 Java 的 ProcessBuilder
    // 使用 open 指令调用系统自带协议直接定位到完全磁盘访问权限设置界面
    std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles")
        .status()
        .map_err(|e| format!("无法打开系统偏好设置: {}", e))?;
    Ok(())
}

