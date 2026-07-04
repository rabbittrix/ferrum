//! Stream ferrum CLI output to the Tauri frontend terminal.

use std::path::PathBuf;
use std::process::Stdio;

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[derive(Clone, Serialize)]
pub struct TerminalOutputEvent {
    pub session_id: String,
    pub data: String,
    pub stream: String,
}

#[derive(Clone, Serialize)]
pub struct TerminalExitEvent {
    pub session_id: String,
    pub code: i32,
}

pub fn resolve_ferrum_binary() -> PathBuf {
    if let Ok(p) = std::env::var("FERRUM_BIN") {
        return PathBuf::from(p);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let name = if cfg!(windows) { "ferrum.exe" } else { "ferrum" };
            let sibling = dir.join(name);
            if sibling.is_file() {
                return sibling;
            }
            if let Some(parent) = dir.parent() {
                let release = parent.join("release").join(name);
                if release.is_file() {
                    return release;
                }
            }
        }
    }
    if let Some(home) = dirs::home_dir() {
        let cargo_bin = home.join(".cargo").join("bin").join(if cfg!(windows) {
            "ferrum.exe"
        } else {
            "ferrum"
        });
        if cargo_bin.is_file() {
            return cargo_bin;
        }
    }
    PathBuf::from(if cfg!(windows) { "ferrum.exe" } else { "ferrum" })
}

#[tauri::command]
pub async fn ferrum_terminal_exec(
    app: AppHandle,
    session_id: String,
    command_line: String,
    cwd: Option<String>,
) -> Result<i32, String> {
    let line = command_line.trim();
    if line.is_empty() {
        return Ok(0);
    }

    let work_dir = cwd.unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().into())
            .unwrap_or_else(|_| ".".into())
    });

    let (program, args) = parse_command_line(line)?;

    let mut cmd = Command::new(&program);
    cmd.args(&args)
        .current_dir(&work_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.as_std_mut().creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = cmd.spawn().map_err(|e| format!("spawn {}: {e}", program.display()))?;

    let sid_out = session_id.clone();
    let sid_err = session_id.clone();
    let app_out = app.clone();
    let app_err = app.clone();

    if let Some(stdout) = child.stdout.take() {
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut buf = Vec::new();
            loop {
                buf.clear();
                match reader.read_until(b'\n', &mut buf).await {
                    Ok(0) => break,
                    Ok(_) => {
                        let data = String::from_utf8_lossy(&buf).into_owned();
                        let _ = app_out.emit(
                            "terminal-output",
                            TerminalOutputEvent {
                                session_id: sid_out.clone(),
                                data,
                                stream: "stdout".into(),
                            },
                        );
                    }
                    Err(_) => break,
                }
            }
        });
    }

    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            let mut buf = Vec::new();
            loop {
                buf.clear();
                match reader.read_until(b'\n', &mut buf).await {
                    Ok(0) => break,
                    Ok(_) => {
                        let data = String::from_utf8_lossy(&buf).into_owned();
                        let _ = app_err.emit(
                            "terminal-output",
                            TerminalOutputEvent {
                                session_id: sid_err.clone(),
                                data,
                                stream: "stderr".into(),
                            },
                        );
                    }
                    Err(_) => break,
                }
            }
        });
    }

    let status = child.wait().await.map_err(|e| e.to_string())?;
    let code = status.code().unwrap_or(-1);
    let _ = app.emit(
        "terminal-exit",
        TerminalExitEvent {
            session_id,
            code,
        },
    );
    Ok(code)
}

fn parse_command_line(line: &str) -> Result<(PathBuf, Vec<String>), String> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.is_empty() {
        return Err("empty command".into());
    }

    if parts[0] == "ferrum"
        || parts[0].ends_with("/ferrum")
        || parts[0].ends_with("\\ferrum")
        || parts[0].ends_with("\\ferrum.exe")
    {
        let bin = resolve_ferrum_binary();
        let args: Vec<String> = parts.iter().skip(1).map(|s| s.to_string()).collect();
        return Ok((bin, args));
    }

    Ok((
        PathBuf::from(parts[0]),
        parts.iter().skip(1).map(|s| s.to_string()).collect(),
    ))
}

#[tauri::command]
pub fn ferrum_terminal_binary_path() -> String {
    resolve_ferrum_binary().to_string_lossy().into()
}
