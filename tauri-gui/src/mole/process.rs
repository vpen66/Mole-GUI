use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Mutex;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use super::settings;

/// Global singleton to track the current analyze scan task.
/// This ensures only one analyze scan runs at a time and old ones are cancelled.
static ANALYZE_TASK: Mutex<Option<u64>> = Mutex::new(None);
static NEXT_REQUEST_ID: Mutex<u64> = Mutex::new(0);

/// Result of a streaming execution that may have timed out.
pub struct StreamingResult {
    pub exit_code: i32,
    pub timed_out: bool,
}

/// Locate the Mole CLI binary on the system.
/// If an AppHandle is provided, the user-configured path is checked first.
pub fn find_mole_path(app: Option<&tauri::AppHandle>) -> Option<PathBuf> {
    // 0. Check user-configured custom path first
    if let Some(app_handle) = app {
        if let Some(configured_path) = settings::get_configured_mole_path(app_handle) {
            return Some(configured_path);
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        // Try 5 levels up first (for Tauri builds)
        let repo_root_5 = exe
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .map(|p| p.join("mole"));
        if let Some(path) = repo_root_5 {
            if path.exists() {
                return Some(path);
            }
        }
        
        // Fallback to 4 levels up (in case of different build structures)
        let repo_root_4 = exe
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .map(|p| p.join("mole"));
        if let Some(path) = repo_root_4 {
            if path.exists() {
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

    // 4. Check PATH via `which`
    if let Ok(output) = std::process::Command::new("which")
        .arg("mole")
        .output()
    {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout);
            let path = PathBuf::from(path_str.trim());
            if path.exists() {
                return Some(path);
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
