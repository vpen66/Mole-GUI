use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;

use super::settings;

/// Idle timeout: if no stdout output for this many seconds, assume the
/// process is stuck (e.g. blocked on stderr write) and kill it.
const IDLE_TIMEOUT_SECS: u64 = 60;

/// Spawn a background task that drains a child process's stderr.
/// This prevents the OS pipe buffer (~64 KB) from filling up and
/// blocking the child process when it writes to stderr.
fn drain_stderr(child: &mut tokio::process::Child) {
    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];
            let mut reader = stderr;
            loop {
                match reader.read(&mut buf).await {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        let snippet = String::from_utf8_lossy(&buf[..n]);
                        eprintln!("[mole stderr] {}", snippet.trim_end());
                    }
                }
            }
        });
    }
}

/// Global singleton to track the current analyze scan task.
/// This ensures only one analyze scan runs at a time and old ones are cancelled.
static ANALYZE_TASK: Mutex<Option<u64>> = Mutex::new(None);
static NEXT_REQUEST_ID: Mutex<u64> = Mutex::new(0);

/// Result of a streaming execution that may have timed out or been cancelled.
pub struct StreamingResult {
    pub exit_code: i32,
    pub timed_out: bool,
    pub cancelled: bool,
}

/// Locate the Mole CLI binary on the system.
/// If an AppHandle is provided, the user-configured path is checked first.
pub fn find_mole_path(app: Option<&tauri::AppHandle>) -> Option<PathBuf> {
    // 0. Check user-configured custom path first
    if let Some(app_handle) = app {
        if let Some(configured_path) = settings::get_configured_mole_path(app_handle) {
            eprintln!("[mole-gui] Using user-configured mole path: {}", configured_path.display());
            return Some(configured_path);
        }
    }

    // 1. Check PATH via `which` (most reliable for installed binaries)
    if let Ok(output) = std::process::Command::new("which")
        .arg("mole")
        .output()
    {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout);
            let path = PathBuf::from(path_str.trim());
            if path.exists() {
                eprintln!("[mole-gui] Found mole CLI via which: {}", path.display());
                return Some(path);
            }
        }
    }

    // 2. Check common install paths
    let candidates = [
        "/opt/homebrew/bin/mole",
        "/usr/local/bin/mole",
        "/usr/bin/mole",
    ];
    for candidate in candidates {
        let path = PathBuf::from(candidate);
        if path.exists() {
            return Some(path);
        }
    }

    // 3. Check ~/.local/bin
    if let Ok(home) = std::env::var("HOME") {
        let local_bin = PathBuf::from(format!("{}/.local/bin/mole", home));
        if local_bin.exists() {
            return Some(local_bin);
        }
    }

    // 4. Walk up from the executable; at each ancestor, check for a sibling "mole" project.
    //    This handles dev setups where Mole-GUI and mole are sibling directories.
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.as_path();
        while let Some(parent) = dir.parent() {
            dir = parent;
            // Check: <ancestor>/mole/mole  (exe is inside a "mole" project)
            let inside_mole = dir.join("mole/mole");
            if inside_mole.is_file() {
                eprintln!("[mole-gui] Found mole CLI inside ancestor: {}", inside_mole.display());
                return Some(inside_mole);
            }
            // Check: <ancestor>/../mole/mole  (sibling project)
            let sibling_mole = dir.join("../mole/mole");
            if sibling_mole.is_file() {
                eprintln!("[mole-gui] Found mole CLI as sibling: {}", sibling_mole.display());
                return Some(sibling_mole);
            }
        }
    }

    None
}

/// Execute a Mole CLI command and stream NDJSON output line-by-line.
/// Calls `on_line` for each stdout line. Returns the exit status.
#[allow(dead_code)]
pub async fn run_mole_streaming<F>(
    app: Option<&tauri::AppHandle>,
    args: &[&str],
    mut on_line: F,
) -> Result<i32, String>
where
    F: FnMut(String) + Send + 'static,
{
    let mole_path = find_mole_path(app).ok_or_else(|| {
        "Mole CLI not found. Please install it first or configure the path in Settings.".to_string()
    })?;

    let mut child = Command::new(&mole_path)
        .args(args)
        .env("LC_ALL", "C")
        .env("NO_COLOR", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start Mole: {}", e))?;

    // Drain stderr in background to prevent pipe buffer blocking
    drain_stderr(&mut child);

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture stdout".to_string())?;

    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        on_line(line);
    }

    let status = child
        .wait()
        .await
        .map_err(|e| format!("Failed to wait for Mole: {}", e))?;

    Ok(status.code().unwrap_or(-1))
}

/// Execute a Mole CLI command with a timeout. If the command does not finish
/// within `timeout_secs`, the process is killed and `StreamingResult.timed_out`
/// is set to `true`. Any lines received before the timeout are still delivered
/// through the callback.
pub async fn run_mole_streaming_with_timeout<F>(
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

    // Check if this is an analyze command - if so, cancel any existing analyze task first
    let is_analyze = args.first().map(|s| *s == "analyze").unwrap_or(false);
    let request_id = if is_analyze {
        let new_id = get_next_request_id();
        cancel_existing_analyze_task(new_id);
        new_id
    } else {
        0
    };

    let mut child = Command::new(&mole_path)
        .args(args)
        .env("LC_ALL", "C")
        .env("NO_COLOR", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start Mole: {}", e))?;

    // Drain stderr in background to prevent pipe buffer blocking
    drain_stderr(&mut child);

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture stdout".to_string())?;

    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();

    let timeout_duration = Duration::from_secs(timeout_secs);
    let mut timed_out = false;

    loop {
        // Check if we've been cancelled by a newer request
        if is_analyze && is_task_cancelled(request_id) {
            eprintln!("[mole-gui] Analyze task #{} was cancelled by newer request", request_id);
            let _ = child.kill().await;
            return Err(format!("Scan cancelled by new request #{}", request_id));
        }

        match tokio::time::timeout(timeout_duration, lines.next_line()).await {
            Ok(Ok(Some(line))) => {
                on_line(line);
            }
            Ok(Ok(None)) => {
                // EOF reached
                break;
            }
            Ok(Err(_e)) => {
                // Read error, treat as EOF
                break;
            }
            Err(_elapsed) => {
                // Timeout: kill the process and break
                timed_out = true;
                let _ = child.kill().await;
                break;
            }
        }
    }

    // Clear tracking when done (only if this is still the current task)
    if is_analyze {
        clear_analyze_task_if_current(request_id);
    }

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

/// Execute a Mole CLI command with throttled event delivery and cancellation support.
///
/// Lines are collected into a buffer and flushed to `on_batch` every ~100ms,
/// which prevents flooding the frontend with hundreds of events per second.
///
/// If `cancel_flag` is set to `true` the child process is killed and the
/// function returns with `cancelled = true`.
pub async fn run_mole_streaming_throttled<F>(
    app: Option<&tauri::AppHandle>,
    args: &[&str],
    timeout_secs: u64,
    cancel_flag: &AtomicBool,
    mut on_batch: F,
) -> Result<StreamingResult, String>
where
    F: FnMut(&[String]) + Send + 'static,
{
    let mole_path = find_mole_path(app).ok_or_else(|| {
        "Mole CLI not found. Please install it first or configure the path in Settings.".to_string()
    })?;

    let mut child = Command::new(&mole_path)
        .args(args)
        .env("LC_ALL", "C")
        .env("NO_COLOR", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start Mole: {}", e))?;

    // Drain stderr in background to prevent pipe buffer blocking
    drain_stderr(&mut child);

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture stdout".to_string())?;

    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();

    let timeout_duration = Duration::from_secs(timeout_secs);
    let idle_timeout = Duration::from_secs(IDLE_TIMEOUT_SECS);
    let flush_interval = Duration::from_millis(100);
    let mut timed_out = false;
    let mut buffer: Vec<String> = Vec::with_capacity(64);
    let start_time = tokio::time::Instant::now();
    let mut last_output_time = start_time;
    let mut loop_count: u64 = 0;

    loop {
        // Check cancel flag
        if cancel_flag.load(Ordering::SeqCst) {
            eprintln!("[mole-gui] Cancel flag detected – killing analyze process");
            let _ = child.kill().await;
            if !buffer.is_empty() {
                on_batch(&buffer);
            }
            return Ok(StreamingResult {
                exit_code: -1,
                timed_out: false,
                cancelled: true,
            });
        }

        match tokio::time::timeout(flush_interval, lines.next_line()).await {
            Ok(Ok(Some(line))) => {
                last_output_time = tokio::time::Instant::now();
                buffer.push(line);
                // Flush if buffer is large enough
                if buffer.len() >= 50 {
                    on_batch(&buffer);
                    buffer.clear();
                }
            }
            Ok(Ok(None)) => break, // EOF
            Ok(Err(_)) => break,   // read error
            Err(_elapsed) => {
                // flush_interval elapsed – flush accumulated buffer
                if !buffer.is_empty() {
                    on_batch(&buffer);
                    buffer.clear();
                }
            }
        }

        loop_count += 1;

        // Check idle timeout every 50 iterations (~5s): if no output at all
        // for IDLE_TIMEOUT_SECS, the process is likely stuck.
        if loop_count % 50 == 0 {
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

        // Check overall timeout every 100 iterations (~10s)
        if loop_count % 100 == 0 && start_time.elapsed() > timeout_duration {
            eprintln!(
                "[mole-gui] Overall timeout {}s reached – killing process",
                timeout_secs
            );
            timed_out = true;
            let _ = child.kill().await;
            break;
        }
    }

    // Final flush
    if !buffer.is_empty() {
        on_batch(&buffer);
    }

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

/// Get a unique request ID for tracking
fn get_next_request_id() -> u64 {
    if let Ok(mut guard) = NEXT_REQUEST_ID.lock() {
        *guard += 1;
        *guard
    } else {
        0
    }
}

/// Cancel any existing analyze task to prevent multiple concurrent scans
fn cancel_existing_analyze_task(new_request_id: u64) {
    if let Ok(mut guard) = ANALYZE_TASK.lock() {
        if let Some(old_id) = *guard {
            eprintln!("[mole-gui] Cancelling previous analyze task #{} for new task #{}", old_id, new_request_id);
        }
        *guard = Some(new_request_id);
    }
}

/// Check if a task has been cancelled (i.e., a newer task has taken over)
fn is_task_cancelled(request_id: u64) -> bool {
    if let Ok(guard) = ANALYZE_TASK.lock() {
        // If the current task ID is different from our request ID, we've been superseded
        if let Some(current_id) = *guard {
            return current_id != request_id;
        }
    }
    false
}

/// Clear the analyze task tracking only if it's still the current task
fn clear_analyze_task_if_current(request_id: u64) {
    if let Ok(mut guard) = ANALYZE_TASK.lock() {
        if let Some(current_id) = *guard {
            if current_id == request_id {
                *guard = None;
                eprintln!("[mole-gui] Cleared analyze task #{} tracking", request_id);
            } else {
                eprintln!("[mole-gui] Task #{} finished but task #{} is now active, keeping tracking", request_id, current_id);
            }
        }
    }
}

/// Execute a Mole CLI command and return stdout as a string.
pub async fn run_mole_capture(
    app: Option<&tauri::AppHandle>,
    args: &[&str],
) -> Result<String, String> {
    let mole_path = find_mole_path(app).ok_or_else(|| {
        "Mole CLI not found. Please install it first or configure the path in Settings.".to_string()
    })?;

    // Add timeout for version check (5 seconds)
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        Command::new(&mole_path)
            .args(args)
            .env("LC_ALL", "C")
            .env("NO_COLOR", "1")
            .output(),
    )
    .await
    .map_err(|_| "Mole version check timed out".to_string())?
    .map_err(|e| format!("Failed to run Mole: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(if stderr.is_empty() {
            format!("Mole exited with code {}", output.status.code().unwrap_or(-1))
        } else {
            stderr
        })
    }
}

/// Get the Mole CLI version string.
pub async fn get_mole_version(app: Option<&tauri::AppHandle>) -> Result<String, String> {
    let output = run_mole_capture(app, &["--version"]).await?;
    // Extract just the version number from the first non-empty line
    // e.g., "Mole version 1.44.1" -> "1.44.1"
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(version_part) = trimmed.strip_prefix("Mole version ") {
            return Ok(version_part.trim().to_string());
        }
    }
    Ok(output.trim().to_string())
}
