//! go-plugin gRPC handshake for Terraform provider subprocesses.

use std::path::Path;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

use crate::error::{BridgeError, Result};

/// Terraform Plugin SDK magic cookie (protocol v5).
pub const MAGIC_COOKIE_KEY: &str = "TF_PLUGIN_MAGIC_COOKIE";
pub const MAGIC_COOKIE_VALUE: &str = "d602bf8f-9708-464a-9529-e715bae47820";
pub const CORE_PROTOCOL: u32 = 1;
pub const PLUGIN_PROTOCOL_V5: u32 = 5;

#[derive(Clone, Debug)]
pub struct HandshakeResult {
    pub endpoint: String,
    pub network: String,
    pub plugin_protocol: u32,
}

/// Launch a provider binary and parse the go-plugin handshake line from stdout.
pub async fn launch_and_handshake(binary: &Path) -> Result<(tokio::process::Child, HandshakeResult)> {
    let mut child = Command::new(binary)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .env(MAGIC_COOKIE_KEY, MAGIC_COOKIE_VALUE)
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| BridgeError::Handshake(format!("spawn {}: {e}", binary.display())))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| BridgeError::Handshake("no stdout from provider".into()))?;

    let mut reader = BufReader::new(stdout);
    let mut line = String::new();

    timeout(Duration::from_secs(30), reader.read_line(&mut line))
        .await
        .map_err(|_| BridgeError::Handshake("handshake timed out".into()))?
        .map_err(|e| BridgeError::Handshake(format!("read stdout: {e}")))?;

    let hs = parse_handshake_line(line.trim())?;
    Ok((child, hs))
}

pub fn parse_handshake_line(line: &str) -> Result<HandshakeResult> {
    let parts: Vec<&str> = line.split('|').collect();
    if parts.len() < 4 {
        return Err(BridgeError::Handshake(format!(
            "invalid handshake line: '{line}'"
        )));
    }

    let core: u32 = parts[0]
        .parse()
        .map_err(|_| BridgeError::Handshake(format!("invalid core protocol: {}", parts[0])))?;
    let plugin: u32 = parts[1]
        .parse()
        .map_err(|_| BridgeError::Handshake(format!("invalid plugin protocol: {}", parts[1])))?;

    if core != CORE_PROTOCOL {
        return Err(BridgeError::Handshake(format!(
            "unsupported core protocol {core}"
        )));
    }
    if plugin < PLUGIN_PROTOCOL_V5 {
        return Err(BridgeError::Handshake(format!(
            "unsupported plugin protocol {plugin} (minimum v{PLUGIN_PROTOCOL_V5})"
        )));
    }

    let network = parts[2].to_string();
    let address = parts[3].to_string();

    let endpoint = match network.as_str() {
        "tcp" => format!("http://{address}"),
        "unix" => {
            #[cfg(unix)]
            {
                format!("http://[::]/{address}") // tonic uses unix via custom connector; use tcp fallback message
            }
            #[cfg(not(unix))]
            {
                return Err(BridgeError::Handshake(
                    "unix socket providers require Unix platform".into(),
                ));
            }
        }
        other => {
            return Err(BridgeError::Handshake(format!(
                "unsupported network type '{other}'"
            )));
        }
    };

    Ok(HandshakeResult {
        endpoint,
        network,
        plugin_protocol: plugin,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tcp_handshake() {
        let hs = parse_handshake_line("1|5|tcp|127.0.0.1:12345|").unwrap();
        assert_eq!(hs.endpoint, "http://127.0.0.1:12345");
        assert_eq!(hs.plugin_protocol, PLUGIN_PROTOCOL_V5);
    }
}
