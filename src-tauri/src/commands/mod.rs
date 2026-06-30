use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Window};
use crate::mole::process;
use crate::mole::settings;

/// Default timeout for clean operations (seconds).
const CLEAN_TIMEOUT_SECS: u64 = 120;
/// Default timeout for uninstall scan operations (seconds).
const UNINSTALL_TIMEOUT_SECS: u64 = 60;
/// Default timeout for purge operations (seconds).
const PURGE_TIMEOUT_SECS: u64 = 180;
/// Default timeout for optimize operations (seconds).
const OPTIMIZE_TIMEOUT_SECS: u64 = 60;
/// Default timeout for analyze operations (seconds).
const ANALYZE_TIMEOUT_SECS: u64 = 300;

#[derive(Serialize, Deserialize, Clone)]
pub struct MoleEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(flatten)]
    pub data: serde_json::Value,
}

#[derive(Serialize)]
pub struct MolePathConfig {
    pub custom_path: String,
    pub resolved_path: String,
}

#[derive(Serialize)]
pub struct MoleVersionInfo {
    pub version: String,
    pub installed: bool,
    pub path: String,
}

#[derive(Serialize)]
pub struct CleanResult {
    pub success: bool,
    pub lines: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppInfo {
    pub name: String,
    pub path: String,
    pub bundle_id: String,
    pub size_kb: u64,
    #[serde(default)]
    pub is_running: bool,
    #[serde(default)]
    pub has_brew_cask: bool,
    #[serde(default)]
    pub is_blocked: bool,
    #[serde(rename = "last_used", default)]
    pub last_used: Option<String>,
}

/// System status info for the dashboard, parsed from `mo status --json`.
#[derive(Serialize, Clone)]
pub struct SystemStatus {
    pub host: String,
    pub platform: String,
    pub uptime: String,
    pub uptime_seconds: u64,
    pub health_score: u64,
    pub health_score_msg: String,
    pub cpu_usage: f64,
    pub cpu_core_count: u64,
    pub memory_used: u64,
    pub memory_total: u64,
    pub memory_available: u64,
    pub memory_used_percent: f64,
    pub disk_used: u64,
    pub disk_total: u64,
    pub disk_free: u64,
    pub disk_used_percent: f64,
    pub disk_size: String,
    pub model: String,
    pub cpu_model: String,
    pub total_ram: String,
    pub os_version: String,
    pub trash_size: u64,
}

/// Parse a line from mole CLI output and emit a Tauri event to the frontend.
fn emit_mole_event(window: &Window, event_name: &str, line: &str) {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return;
    }

    // Try to parse as JSON first (in case mole ever outputs JSON)
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if json.get("type").is_some() {
            let event = MoleEvent {
                event_type: json
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                data: json,
            };
            let _ = window.emit(event_name, &event);
            return;
        }
    }

    // Handle section headers (➤ User essentials, ➤ App caches, etc.)
    if trimmed.starts_with("➤") {
        let section_name = trimmed.trim_start_matches("➤").trim();
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
        let _ = window.emit(event_name, &event);
        return;
    }

    // Skip other header lines but send them as progress updates
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
            let _ = window.emit(event_name, &event);
        }
        return;
    }

    // Parse human-readable text output
    let section = determine_section(trimmed);

    if let Some(item_info) = parse_item_line(trimmed, &section) {
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
        let _ = window.emit(event_name, &event);

        // Also send a progress update for this item
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
        let _ = window.emit(event_name, &progress_event);
    }
}

struct ParsedItem {
    section: String,
    description: String,
    size_kb: f64,
    size_human: String,
    status: String,
}

fn determine_section(line: &str) -> String {
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

fn is_header_line(line: &str) -> bool {
    line.starts_with("Clean Your Mac")
        || line.starts_with("Dry Run Mode")
        || line.starts_with("◎")
        || line.starts_with("⚙")
        || line.starts_with("✓ Whitelist")
        || line.starts_with("➤")
}

fn parse_item_line(line: &str, default_section: &str) -> Option<ParsedItem> {
    // Check for "already empty" pattern
    if line.contains("· already empty") {
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

    // Check for item with size pattern: "→ Description N items, X.XXGB dry"
    if line.contains(",") && (line.contains("items") || line.contains("item")) {
        let before_comma = line.split(',').next()?.trim();
        let description = before_comma
            .trim_start_matches("→")
            .trim_start_matches("✓")
            .trim();

        let after_comma = line.split(',').nth(1)?.trim();
        let size_str = after_comma
            .split_whitespace()
            .find(|s| s.ends_with("GB") || s.ends_with("MB") || s.ends_with("KB"))?;

        let size_kb = parse_size_to_kb(size_str)?;

        let status = if line.contains("dry") {
            "dry_run"
        } else if line.contains("cleaned") {
            "cleaned"
        } else {
            "skipped"
        };

        return Some(ParsedItem {
            section: default_section.to_string(),
            description: description.to_string(),
            size_kb,
            size_human: size_str.to_string(),
            status: status.to_string(),
        });
    }

    None
}

fn parse_size_to_kb(size_str: &str) -> Option<f64> {
    let size_str = size_str.trim();

    if size_str.ends_with("GB") {
        let num: f64 = size_str.trim_end_matches("GB").parse().ok()?;
        Some(num * 1024.0 * 1024.0)
    } else if size_str.ends_with("MB") {
        let num: f64 = size_str.trim_end_matches("MB").parse().ok()?;
        Some(num * 1024.0)
    } else if size_str.ends_with("KB") {
        let num: f64 = size_str.trim_end_matches("KB").parse().ok()?;
        Some(num)
    } else {
        None
    }
}

#[tauri::command]
pub async fn get_mole_version(app: AppHandle) -> Result<MoleVersionInfo, String> {
    match process::get_mole_version(Some(&app)).await {
        Ok(version) => {
            let path = process::find_mole_path(Some(&app))
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            Ok(MoleVersionInfo {
                version,
                installed: true,
                path,
            })
        }
        Err(_) => Ok(MoleVersionInfo {
            version: String::new(),
            installed: false,
            path: String::new(),
        }),
    }
}

#[tauri::command]
pub async fn get_free_space_kb(app: AppHandle) -> Result<u64, String> {
    let output = process::run_mole_capture(Some(&app), &["status", "--json"]).await?;
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&output) {
        if let Some(disks) = json.get("disks").and_then(|d| d.as_array()) {
            if let Some(first) = disks.first() {
                // Calculate free = total - used (the JSON doesn't have a "free" field)
                let total = first.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
                let used = first.get("used").and_then(|v| v.as_u64()).unwrap_or(0);
                if total > used {
                    return Ok((total - used) / 1024);
                }
                // Also check for explicit "free" field as fallback
                if let Some(free) = first.get("free").and_then(|f| f.as_u64()) {
                    return Ok(free / 1024);
                }
            }
        }
    }
    // Fallback: use df
    let output = tokio::process::Command::new("df")
        .args(["-k", "/"])
        .output()
        .await
        .map_err(|e| format!("Failed to run df: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            if let Ok(kb) = parts[3].parse::<u64>() {
                return Ok(kb);
            }
        }
    }
    Err("Could not determine free space".to_string())
}

#[tauri::command]
pub async fn get_system_status(app: AppHandle) -> Result<SystemStatus, String> {
    let output = process::run_mole_capture(Some(&app), &["status", "--json"]).await?;
    let json: serde_json::Value = serde_json::from_str(&output)
        .map_err(|e| format!("Failed to parse status JSON: {}", e))?;

    let get_str = |key: &str| json.get(key).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let get_u64 = |key: &str| json.get(key).and_then(|v| v.as_u64()).unwrap_or(0);

    // Hardware info
    let hw = json.get("hardware");
    let model = hw.and_then(|h| h.get("model")).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let cpu_model = hw.and_then(|h| h.get("cpu_model")).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let total_ram = hw.and_then(|h| h.get("total_ram")).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let os_version = hw.and_then(|h| h.get("os_version")).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let disk_size = hw.and_then(|h| h.get("disk_size")).and_then(|v| v.as_str()).unwrap_or("").to_string();

    // CPU
    let cpu_obj = json.get("cpu");
    let cpu_usage = cpu_obj
        .and_then(|c| c.get("usage"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let cpu_core_count = cpu_obj
        .and_then(|c| c.get("core_count"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    // Memory
    let mem = json.get("memory");
    let memory_used = mem.and_then(|m| m.get("used")).and_then(|v| v.as_u64()).unwrap_or(0);
    let memory_total = mem.and_then(|m| m.get("total")).and_then(|v| v.as_u64()).unwrap_or(0);
    let memory_available = mem.and_then(|m| m.get("available")).and_then(|v| v.as_u64()).unwrap_or(0);
    let memory_used_percent = mem.and_then(|m| m.get("used_percent")).and_then(|v| v.as_f64()).unwrap_or(0.0);

    // Disk (first entry)
    let disk = json.get("disks").and_then(|d| d.as_array()).and_then(|a| a.first());
    let disk_used = disk.and_then(|d| d.get("used")).and_then(|v| v.as_u64()).unwrap_or(0);
    let disk_total = disk.and_then(|d| d.get("total")).and_then(|v| v.as_u64()).unwrap_or(0);
    let disk_used_percent = disk.and_then(|d| d.get("used_percent")).and_then(|v| v.as_f64()).unwrap_or(0.0);
    let disk_free = if disk_total > disk_used { disk_total - disk_used } else { 0 };

    let trash_size = get_u64("trash_size");

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

#[tauri::command]
pub async fn clean_dry_run(app: AppHandle, window: Window) -> Result<CleanResult, String> {
    let window_clone = window.clone();
    let app_clone = app.clone();

    let handle = tokio::spawn(async move {
        let lines: Vec<String> = Vec::new();

        let result = process::run_mole_streaming_with_timeout(
            Some(&app_clone),
            &["clean", "--dry-run"],
            CLEAN_TIMEOUT_SECS,
            move |line| {
                emit_mole_event(&window_clone, "mole-clean_dry_run-event", &line);
            },
        )
        .await;

        match result {
            Ok(streaming) => {
                let error = if streaming.timed_out {
                    Some(format!(
                        "Scan timed out after {}s. Showing partial results.",
                        CLEAN_TIMEOUT_SECS
                    ))
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

#[tauri::command]
pub async fn clean_execute(app: AppHandle, window: Window) -> Result<CleanResult, String> {
    let window_clone = window.clone();
    let app_clone = app.clone();

    let handle = tokio::spawn(async move {
        let lines: Vec<String> = Vec::new();

        let result = process::run_mole_streaming_with_timeout(
            Some(&app_clone),
            &["clean"],
            CLEAN_TIMEOUT_SECS,
            move |line| {
                emit_mole_event(&window_clone, "mole-clean_execute-event", &line);
            },
        )
        .await;

        match result {
            Ok(streaming) => {
                let error = if streaming.timed_out {
                    Some(format!(
                        "Clean timed out after {}s.",
                        CLEAN_TIMEOUT_SECS
                    ))
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

#[tauri::command]
pub async fn uninstall_scan_apps(app: AppHandle, window: Window) -> Result<Vec<AppInfo>, String> {
    use std::sync::{Arc, Mutex};

    let window_clone = window.clone();
    let app_clone = app.clone();
    let apps_arc = Arc::new(Mutex::new(Vec::<AppInfo>::new()));
    let apps_clone = apps_arc.clone();

    let handle = tokio::spawn(async move {
        let result = process::run_mole_streaming_with_timeout(
            Some(&app_clone),
            &["uninstall", "--json"],
            UNINSTALL_TIMEOUT_SECS,
            move |line| {
                // Parse JSON lines from mole uninstall --json output
                if line.starts_with("{") {
                    if let Ok(app_info) = serde_json::from_str::<AppInfo>(&line) {
                        if let Ok(mut apps) = apps_clone.lock() {
                            apps.push(app_info);
                        }
                    }
                } else {
                    // Non-JSON lines are sent as events for progress display
                    emit_mole_event(&window_clone, "mole-uninstall_scan_apps-event", &line);
                }
            },
        )
        .await;

        match result {
            Ok(_) => {
                let apps = apps_arc.lock().unwrap_or_else(|e| e.into_inner()).clone();
                Ok(apps)
            }
            Err(e) => Err(e),
        }
    });

    handle.await.map_err(|e| format!("Task error: {}", e))?
}

#[tauri::command]
pub async fn uninstall_execute(
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
                    Some(format!(
                        "Uninstall timed out after {}s.",
                        UNINSTALL_TIMEOUT_SECS
                    ))
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

        match result {
            Ok(streaming) if streaming.timed_out => {
                Ok(format!(
                    "Purge scan timed out after {}s. Showing partial results.",
                    PURGE_TIMEOUT_SECS
                ))
            }
            Ok(_) => Ok(String::new()),
            Err(e) => Err(e),
        }
    });

    handle
        .await
        .map_err(|e| format!("Task error: {}", e))?
}

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
                    Some(format!(
                        "Purge timed out after {}s.",
                        PURGE_TIMEOUT_SECS
                    ))
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

#[tauri::command]
pub async fn optimize_dry_run(app: AppHandle, window: Window) -> Result<String, String> {
    let window_clone = window.clone();
    let app_clone = app.clone();

    let handle = tokio::spawn(async move {
        let result = process::run_mole_streaming_with_timeout(
            Some(&app_clone),
            &["optimize", "--dry-run"],
            OPTIMIZE_TIMEOUT_SECS,
            move |line| {
                emit_mole_event(&window_clone, "mole-optimize_dry_run-event", &line);
            },
        )
        .await;

        match result {
            Ok(streaming) if streaming.timed_out => {
                Ok(format!(
                    "Optimize scan timed out after {}s. Showing partial results.",
                    OPTIMIZE_TIMEOUT_SECS
                ))
            }
            Ok(_) => Ok(String::new()),
            Err(e) => Err(e),
        }
    });

    handle
        .await
        .map_err(|e| format!("Task error: {}", e))?
}

#[tauri::command]
pub async fn optimize_execute(
    app: AppHandle,
    window: Window,
    actions: Vec<String>,
) -> Result<CleanResult, String> {
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
                    Some(format!(
                        "Optimize timed out after {}s.",
                        OPTIMIZE_TIMEOUT_SECS
                    ))
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

#[tauri::command]
pub async fn get_history(app: AppHandle, limit: Option<u32>) -> Result<String, String> {
    let limit_str = limit.unwrap_or(50).to_string();
    process::run_mole_capture(Some(&app), &["history", "--json", "--limit", &limit_str]).await
}

#[tauri::command]
pub async fn analyze_scan(
    app: AppHandle,
    window: Window,
    path: Option<String>,
) -> Result<String, String> {
    let window_clone = window.clone();
    let app_clone = app.clone();

    let handle = tokio::spawn(async move {
        let mut args = vec!["analyze", "--json"];
        let path_ref;
        if let Some(ref p) = path {
            path_ref = p.as_str();
            args.push(path_ref);
        }

        // Use streaming to show real-time progress
        let result = process::run_mole_streaming_with_timeout(
            Some(&app_clone),
            &args,
            ANALYZE_TIMEOUT_SECS,
            move |line| {
                emit_mole_event(&window_clone, "mole-analyze_scan-event", &line);
            },
        )
        .await;

        match result {
            Ok(streaming) => {
                if streaming.timed_out {
                    return Err(format!(
                        "Analyze scan timed out after {}s. Showing partial results.",
                        ANALYZE_TIMEOUT_SECS
                    ));
                }
                // For analyze, we still need to return the final JSON result
                // The streaming events will be used for progress display
                Ok(String::new())
            }
            Err(e) => {
                // If the scan was cancelled by a new request, don't treat it as an error
                // This is expected behavior when user navigates away and back
                if e.contains("cancelled by new request") {
                    // Return empty string to indicate graceful cancellation
                    eprintln!("[mole-gui] Analyze scan was gracefully cancelled: {}", e);
                    Ok(String::new())
                } else {
                    Err(e)
                }
            }
        }
    });

    handle.await.map_err(|e| format!("Task error: {}", e))?
}

#[tauri::command]
pub async fn analyze_delete(
    app: AppHandle,
    window: Window,
    paths: Vec<String>,
) -> Result<CleanResult, String> {
    let window_clone = window.clone();
    let app_clone = app.clone();

    let handle = tokio::spawn(async move {
        let lines: Vec<String> = Vec::new();

        // Build arguments for mo delete command
        let mut args = vec!["delete"];
        for path in &paths {
            args.push(path);
        }

        let result = process::run_mole_streaming_with_timeout(
            Some(&app_clone),
            &args,
            CLEAN_TIMEOUT_SECS,
            move |line| {
                emit_mole_event(&window_clone, "mole-analyze_delete-event", &line);
            },
        )
        .await;

        match result {
            Ok(streaming) => {
                let error = if streaming.timed_out {
                    Some(format!(
                        "Delete timed out after {}s.",
                        CLEAN_TIMEOUT_SECS
                    ))
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

#[tauri::command]
pub async fn check_sudo_session() -> Result<bool, String> {
    Ok(crate::mole::sudo::check_sudo_session().await)
}

#[tauri::command]
pub async fn request_sudo_session() -> Result<bool, String> {
    crate::mole::sudo::request_sudo_session().await
}

#[tauri::command]
pub async fn stop_sudo_session() -> Result<(), String> {
    crate::mole::sudo::stop_sudo_session().await;
    Ok(())
}

#[tauri::command]
pub async fn get_mole_path_config(app: AppHandle) -> Result<MolePathConfig, String> {
    let custom_path = settings::get_configured_mole_path(&app)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let resolved_path = process::find_mole_path(Some(&app))
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    Ok(MolePathConfig {
        custom_path,
        resolved_path,
    })
}

#[tauri::command]
pub async fn set_mole_path_config(app: AppHandle, path: String) -> Result<MolePathConfig, String> {
    settings::set_configured_mole_path(&app, &path)?;
    // Return the updated config
    let custom_path = if path.is_empty() {
        String::new()
    } else {
        let p = std::path::PathBuf::from(&path);
        if p.exists() {
            p.to_string_lossy().to_string()
        } else {
            String::new()
        }
    };
    let resolved_path = process::find_mole_path(Some(&app))
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    Ok(MolePathConfig {
        custom_path,
        resolved_path,
    })
}
