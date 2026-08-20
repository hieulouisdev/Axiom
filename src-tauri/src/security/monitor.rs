//! Process monitor: periodically samples running processes on the host and
//! flags those whose command line matches a threat signature.

use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::state::AppState;

use super::Severity;

/// Snapshot of a single running process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessSnapshot {
    pub pid: u32,
    pub name: String,
    pub command_line: String,
    pub started_at_ms: Option<u64>,
    pub parent_pid: Option<u32>,
}

/// A detected threat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Threat {
    pub id: String,
    pub timestamp_ms: u64,
    pub pid: u32,
    pub process_name: String,
    pub command_line: String,
    pub signature_id: String,
    pub signature_name: String,
    pub severity: Severity,
}

/// Latest snapshot of all running processes (refreshed every poll interval).
static LATEST: once_cell::sync::Lazy<Mutex<Vec<ProcessSnapshot>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(Vec::new()));

/// Recent threats detected by the monitor.
static RECENT_THREATS: once_cell::sync::Lazy<Mutex<Vec<Threat>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(Vec::new()));

/// Starts the monitor loop. Runs forever until the process exits.
pub async fn start(state: Arc<Mutex<AppState>>) -> anyhow::Result<()> {
    tracing::info!("security monitor started");
    let interval = Duration::from_secs(15);
    loop {
        // Sample processes.
        let snapshot = sample_processes();
        *LATEST.lock() = snapshot.clone();

        // Apply threat signatures.
        let signatures = {
            let s = state.lock();

            s.config.read().security.threat_signatures.clone()
        };

        let mut new_threats: Vec<Threat> = Vec::new();
        for proc in &snapshot {
            for sig in &signatures {
                if let Ok(re) = regex_lite::Regex::new(&sig.pattern) {
                    let haystack = format!("{} {}", proc.name, proc.command_line);
                    if re.is_match(&haystack) {
                        new_threats.push(Threat {
                            id: format!("threat-{}", uuid::Uuid::new_v4().simple()),
                            timestamp_ms: now_ms(),
                            pid: proc.pid,
                            process_name: proc.name.clone(),
                            command_line: proc.command_line.clone(),
                            signature_id: sig.id.clone(),
                            signature_name: sig.name.clone(),
                            severity: Severity::High,
                        });
                    }
                }
            }
        }

        if !new_threats.is_empty() {
            // v0.3 fix: scope the RECENT_THREATS lock so it's released before
            // we await notify_threats — the MutexGuard is not Send.
            {
                let mut recent = RECENT_THREATS.lock();
                for t in &new_threats {
                    tracing::warn!(
                        "threat detected: {} (pid={}, sig={})",
                        t.process_name,
                        t.pid,
                        t.signature_name
                    );
                }
                recent.extend(new_threats.iter().cloned());
                recent.truncate(200); // keep most recent 200
            }
            // Wake the defender (lock is now released, safe to await).
            crate::security::defender::notify_threats(new_threats).await;
        }

        tokio::time::sleep(interval).await;
    }
}

/// Returns a copy of the latest process snapshot.
pub fn latest_snapshot() -> Vec<ProcessSnapshot> {
    LATEST.lock().clone()
}

/// v0.4: alias for `latest_snapshot()` used by the AI's `process_list` tool.
pub fn snapshot_processes() -> Vec<ProcessSnapshot> {
    latest_snapshot()
}

/// v0.4: type alias for tools that need the process info type.
pub type ProcInfo = ProcessSnapshot;

/// Returns a copy of recently detected threats.
pub fn recent_threats() -> Vec<Threat> {
    RECENT_THREATS.lock().clone()
}

fn now_ms() -> u64 {
    time::OffsetDateTime::now_utc().unix_timestamp() as u64 * 1000
}

/// Samples running processes via platform-specific APIs.
/// v0.1: on Linux uses `/proc`, on Windows uses ToolHelp snapshot.
fn sample_processes() -> Vec<ProcessSnapshot> {
    let start = Instant::now();
    let mut out = Vec::new();
    sample_processes_inner(&mut out);
    tracing::debug!(
        "process sample took {}ms ({} procs)",
        start.elapsed().as_millis(),
        out.len()
    );
    out
}

#[cfg(unix)]
fn sample_processes_inner(out: &mut Vec<ProcessSnapshot>) {
    let entries = match std::fs::read_dir("/proc") {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy().into_owned();
        let pid: u32 = match name_str.parse() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let cmd = std::fs::read_to_string(format!("/proc/{pid}/cmdline"))
            .unwrap_or_default()
            .replace('\0', " ");
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap_or_default();
        let proc_name = stat
            .split_whitespace()
            .nth(1)
            .map(|s| s.trim_matches('(').trim_matches(')').to_string())
            .unwrap_or_else(|| name_str.clone());
        let started_at_ms = std::fs::read_to_string(format!("/proc/{pid}/stat"))
            .ok()
            .and_then(|s| {
                s.split_whitespace()
                    .nth(21)
                    .and_then(|s| s.parse::<u64>().ok())
            })
            .map(|ticks| ticks * 10); // USER_HZ=100 -> ms
        let parent_pid = stat
            .split_whitespace()
            .nth(3)
            .and_then(|s| s.parse::<u32>().ok());
        out.push(ProcessSnapshot {
            pid,
            name: proc_name,
            command_line: cmd,
            started_at_ms,
            parent_pid,
        });
    }
}

#[cfg(not(unix))]
fn sample_processes_inner(out: &mut Vec<ProcessSnapshot>) {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };

    unsafe {
        let snap = match CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            Ok(h) => h,
            Err(_) => return,
        };
        // Use the wide (Unicode) variant so szExeFile is [u16; 260], which
        // String::from_utf16_lossy accepts directly. The ANSI PROCESSENTRY32
        // uses [i8; 260] which doesn't.
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        if Process32FirstW(snap, &mut entry).is_err() {
            let _ = CloseHandle(snap);
            return;
        }
        loop {
            let name = String::from_utf16_lossy(&entry.szExeFile)
                .trim_end_matches('\0')
                .to_string();
            out.push(ProcessSnapshot {
                pid: entry.th32ProcessID,
                name: name.clone(),
                command_line: name, // Phase 2: full cmdline via NtQueryInformationProcess
                started_at_ms: None,
                parent_pid: Some(entry.th32ParentProcessID),
            });
            if Process32NextW(snap, &mut entry).is_err() {
                break;
            }
        }
        let _ = CloseHandle(snap);
    }
}

/// Lightweight regex implementation. We embed `regex-lite` to avoid pulling in
/// the full `regex` crate (smaller binary).
mod regex_lite {
    pub struct Regex {
        // Phase 2: use the real `regex` crate. For v0.1 we do substring matching.
        pattern: String,
    }

    impl Regex {
        pub fn new(pattern: &str) -> std::result::Result<Self, ()> {
            Ok(Self {
                pattern: pattern.to_string(),
            })
        }

        pub fn is_match(&self, haystack: &str) -> bool {
            // v0.1: simple substring match on each | -separated alternative.
            let lower_hay = haystack.to_lowercase();
            for alt in self.pattern.split('|') {
                let alt = alt.trim();
                if alt.is_empty() {
                    continue;
                }
                if lower_hay.contains(&alt.to_lowercase()) {
                    return true;
                }
            }
            false
        }
    }
}
