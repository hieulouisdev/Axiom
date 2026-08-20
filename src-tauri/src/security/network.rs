//! Network anomaly detection.
//!
//! Phase 2: Real socket enumeration using procfs (Linux) or
//! GetExtendedTcpTable (Windows). Detects suspicious outbound connections
//! and unexpected listeners.

use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAnomaly {
    pub kind: String,
    pub detail: String,
    pub severity: super::Severity,
    pub timestamp_ms: u64,
}

/// A listening socket or established connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocketInfo {
    pub local_addr: String,
    pub local_port: u16,
    pub remote_addr: String,
    pub remote_port: u16,
    pub state: String,
    pub pid: Option<u32>,
}

/// Well-known suspicious ports used by malware.
const SUSPICIOUS_PORTS: &[u16] = &[
    4444,  // Metasploit default
    5555,  // Android debug bridge
    6666,  // Various malware
    6667,  // IRC (botnet C2)
    6668,  // IRC
    6669,  // IRC
    8888,  // Various
    9999,  // Various
    31337, // Back Orifice
    1234,  // Various
    12345, // NetBus
    27374, // SubSeven
];

/// Detect network anomalies by enumerating sockets.
pub fn detect_anomalies() -> Vec<NetworkAnomaly> {
    let mut anomalies = Vec::new();
    let now_ms = time::OffsetDateTime::now_utc().unix_timestamp() as u64 * 1000;

    let sockets = match enumerate_sockets() {
        Ok(s) => s,
        Err(e) => {
            tracing::debug!("socket enumeration failed: {e}");
            return anomalies;
        }
    };

    for socket in &sockets {
        // Check for suspicious listening ports
        if (socket.state == "Listen" || socket.state.contains("Listen"))
            && SUSPICIOUS_PORTS.contains(&socket.local_port)
        {
            anomalies.push(NetworkAnomaly {
                kind: "suspicious_listener".into(),
                detail: format!(
                    "Suspicious port {} listening on {} (pid: {:?})",
                    socket.local_port, socket.local_addr, socket.pid
                ),
                severity: super::Severity::High,
                timestamp_ms: now_ms,
            });
        }

        // Check for suspicious outbound connections
        if !socket.remote_addr.is_empty()
            && socket.remote_addr != "0.0.0.0"
            && socket.remote_addr != "::"
            && SUSPICIOUS_PORTS.contains(&socket.remote_port)
        {
            anomalies.push(NetworkAnomaly {
                kind: "suspicious_outbound".into(),
                detail: format!(
                    "Connection to suspicious port {} at {} (pid: {:?})",
                    socket.remote_port, socket.remote_addr, socket.pid
                ),
                severity: super::Severity::Critical,
                timestamp_ms: now_ms,
            });
        }
    }

    anomalies
}

/// Enumerate all TCP sockets.
fn enumerate_sockets() -> Result<Vec<SocketInfo>> {
    let mut sockets = Vec::new();

    #[cfg(target_os = "linux")]
    {
        enumerate_sockets_procfs(&mut sockets)?;
    }

    #[cfg(windows)]
    {
        enumerate_sockets_windows(&mut sockets)?;
    }

    #[cfg(not(any(target_os = "linux", windows)))]
    {
        // No implementation for this platform (macOS, BSD, …).
        // procfs is Linux-only; on other unixes we'd need a different source.
    }

    Ok(sockets)
}

#[cfg(target_os = "linux")]
fn enumerate_sockets_procfs(sockets: &mut Vec<SocketInfo>) -> Result<()> {
    // Try using the procfs crate
    if let Ok(tcp_entries) = procfs::net::tcp() {
        for entry in tcp_entries {
            sockets.push(SocketInfo {
                local_addr: format!("{}", entry.local_address.ip()),
                local_port: entry.local_address.port(),
                remote_addr: format!("{}", entry.remote_address.ip()),
                remote_port: entry.remote_address.port(),
                state: format!("{:?}", entry.state),
                pid: None, // procfs net::tcp doesn't include PID
            });
        }
    }

    if let Ok(tcp6_entries) = procfs::net::tcp6() {
        for entry in tcp6_entries {
            sockets.push(SocketInfo {
                local_addr: format!("{}", entry.local_address.ip()),
                local_port: entry.local_address.port(),
                remote_addr: format!("{}", entry.remote_address.ip()),
                remote_port: entry.remote_address.port(),
                state: format!("{:?}", entry.state),
                pid: None,
            });
        }
    }

    Ok(())
}

#[cfg(windows)]
fn enumerate_sockets_windows(sockets: &mut Vec<SocketInfo>) -> Result<()> {
    // On Windows, we would use GetExtendedTcpTable from the windows crate.
    // For now, fall back to parsing netstat output.
    if let Ok(output) = std::process::Command::new("netstat")
        .args(["-ano", "-p", "TCP"])
        .output()
        && output.status.success()
    {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines().skip(4) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 5 {
                let local = parts[1];
                let remote = parts[2];
                let state = parts[3];
                let pid: Option<u32> = parts[4].parse().ok();

                let (local_addr, local_port) = parse_addr(local);
                let (remote_addr, remote_port) = parse_addr(remote);

                sockets.push(SocketInfo {
                    local_addr,
                    local_port,
                    remote_addr,
                    remote_port,
                    state: state.to_string(),
                    pid,
                });
            }
        }
    }
    Ok(())
}

#[cfg(windows)]
fn parse_addr(addr: &str) -> (String, u16) {
    if let Some(idx) = addr.rfind(':') {
        let ip = &addr[..idx];
        let port: u16 = addr[idx + 1..].parse().unwrap_or(0);
        (ip.to_string(), port)
    } else {
        (addr.to_string(), 0)
    }
}
