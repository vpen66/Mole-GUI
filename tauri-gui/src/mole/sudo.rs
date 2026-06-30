use std::process::Stdio;
use tokio::process::Command;

/// Check whether a cached sudo session is still valid.
/// Mirrors `sudo -n true` from lib/core/sudo.sh — returns `true` when the
/// user can run privileged commands without a password prompt.
pub async fn check_sudo_session() -> bool {
    match Command::new("sudo")
        .arg("-n")
        .arg("true")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
    {
        Ok(status) => status.success(),
        Err(_) => false,
    }
}

/// Request a new sudo session via osascript (macOS GUI password dialog).
/// Returns `true` if the user authenticated successfully.
///
/// In a GUI context we cannot use terminal-based sudo prompts, so we shell
/// out to `osascript` which presents a native password dialog.
pub async fn request_sudo_session() -> Result<bool, String> {
    // Use osascript to show a native macOS password dialog.
    // The script runs `sudo -v` via osascript's `do shell script ... with administrator privileges`
    // which triggers the standard macOS authentication dialog.
    let script = r#"do shell script "echo ok" with administrator privileges"#;

    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to run osascript: {}", e))?;

    Ok(output.status.success())
}

/// Invalidate the cached sudo session.
pub async fn stop_sudo_session() {
    let _ = Command::new("sudo")
        .arg("-k")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;
}
