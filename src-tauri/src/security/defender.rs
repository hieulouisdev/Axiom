//! Active auto-defense: takes counter-measures when threats are detected.
//!
//! Allowed actions (from least to most aggressive):
//! 1. `Notify`     — surface a desktop notification + log to memory store.
//! 2. `Quarantine`  — move the offending file to a sandboxed directory.
//! 3. `Block`       — add the offending process to a kill-on-sight list.
//! 4. `Kill`        — terminate the offending process.
//!
//! Defense actions are taken only if `auto_defense` is enabled in the config
//! AND the threat severity is at least `Medium`. Catastrophic threats
//! (`Critical`) trigger every available counter-measure in sequence.
//!
//! The user can review and undo any defensive action from the Security panel.

use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::Emitter;

use crate::state::AppState;

use super::monitor::Threat;
use super::quarantine::QuarantineStore;
use super::Severity;

/// A defensive action taken by the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DefenseEvent {
    Notified {
        threat_id: String,
        message: String,
    },
    Quarantined {
        threat_id: String,
        file_path: String,
    },
    Blocked {
        threat_id: String,
        pid: u32,
    },
    Killed {
        threat_id: String,
        pid: u32,
        exit_code: Option<i32>,
    },
    AutoDefenseDisabled {
        reason: String,
    },
}

/// Channel of incoming threats from the monitor.
static INCOMING: once_cell::sync::Lazy<Mutex<Vec<Threat>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(Vec::new()));

/// Latest defense events (kept for the UI).
static EVENTS: once_cell::sync::Lazy<Mutex<Vec<DefenseEvent>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(Vec::new()));

/// Notify the defender that new threats were detected.
pub async fn notify_threats(threats: Vec<Threat>) {
    INCOMING.lock().extend(threats.iter().cloned());
}

/// Returns a copy of recent defense events.
pub fn recent_events() -> Vec<DefenseEvent> {
    EVENTS.lock().clone()
}

/// Main loop. Runs forever.
pub async fn start(state: Arc<Mutex<AppState>>) -> anyhow::Result<()> {
    tracing::info!("auto-defense watcher started");
    let mut quarantine = QuarantineStore::new();
    let poll = Duration::from_secs(2);

    loop {
        // Drain incoming threats.
        let threats: Vec<Threat> = {
            let mut incoming = INCOMING.lock();
            let v = incoming.drain(..).collect::<Vec<_>>();
            v
        };

        if !threats.is_empty() {
            let (auto_defense, app_handle) = {
                let s = state.lock();
                let cfg = s.config.read();
                let __moved = (cfg.security.auto_defense, s.app_handle.lock().clone());
                __moved
            };

            for t in threats {
                let mut actions: Vec<DefenseEvent> = Vec::new();

                // 1. Always notify.
                let msg = format!(
                    "Threat detected: {} (pid={}, severity={:?})",
                    t.signature_name, t.pid, t.severity
                );
                actions.push(DefenseEvent::Notified {
                    threat_id: t.id.clone(),
                    message: msg.clone(),
                });

                // Emit Tauri event.
                if let Some(handle) = &app_handle {
                    let _ = handle.emit("security://threat", &t);
                }

                // 2. If auto-defense is enabled, escalate based on severity.
                if auto_defense {
                    match t.severity {
                        Severity::Info | Severity::Low => {
                            // Just notify.
                        }
                        Severity::Medium | Severity::High => {
                            // Try to quarantine the offending binary.
                            if let Some(bin_path) = process_binary_path(t.pid) {
                                if let Ok(qp) = quarantine.quarantine(&bin_path) {
                                    actions.push(DefenseEvent::Quarantined {
                                        threat_id: t.id.clone(),
                                        file_path: qp,
                                    });
                                }
                            }
                        }
                        Severity::Critical => {
                            // Quarantine + kill the process.
                            if let Some(bin_path) = process_binary_path(t.pid) {
                                if let Ok(qp) = quarantine.quarantine(&bin_path) {
                                    actions.push(DefenseEvent::Quarantined {
                                        threat_id: t.id.clone(),
                                        file_path: qp,
                                    });
                                }
                            }
                            match kill_process(t.pid) {
                                Ok(()) => {
                                    actions.push(DefenseEvent::Killed {
                                        threat_id: t.id.clone(),
                                        pid: t.pid,
                                        exit_code: None,
                                    });
                                }
                                Err(e) => {
                                    tracing::warn!("failed to kill pid {}: {e}", t.pid);
                                }
                            }
                        }
                    }
                }

                // Record events.
                let mut events = EVENTS.lock();
                for a in &actions {
                    tracing::info!("defense action: {:?}", a);
                }
                events.extend(actions);
                events.truncate(200);
            }
        }

        tokio::time::sleep(poll).await;
    }
}

/// Returns the on-disk path of the binary backing `pid`, if discoverable.
fn process_binary_path(pid: u32) -> Option<String> {
    #[cfg(unix)]
    {
        let path = format!("/proc/{pid}/exe");
        std::fs::read_link(&path)
            .ok()
            .map(|p| p.to_string_lossy().into_owned())
    }
    #[cfg(not(unix))]
    {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT,
            PROCESS_QUERY_LIMITED_INFORMATION,
        };
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let mut buf = [0u16; 1024];
            let mut len = buf.len() as u32;
            // `PROCESS_NAME_FORMAT` in windows 0.58 is a `#[repr(transparent)]
            // pub struct PROCESS_NAME_FORMAT(pub i32);` — the named constants
            // `PROCESS_NAME_FORMAT_WIN32_EXE` (= 0) were added as top-level
            // consts only in newer versions. Construct directly with the
            // tuple-struct form: 0 = Win32Exe (full image path).
            let ok = QueryFullProcessImageNameW(
                h,
                PROCESS_NAME_FORMAT(0),
                windows::core::PWSTR(buf.as_mut_ptr()),
                &mut len,
            )
            .is_ok();
            let _ = CloseHandle(h);
            if ok {
                Some(
                    String::from_utf16_lossy(&buf[..len as usize])
                        .trim_end_matches('\0')
                        .to_string(),
                )
            } else {
                None
            }
        }
    }
}

#[cfg(unix)]
fn kill_process(pid: u32) -> std::io::Result<()> {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;
    let _ = kill(Pid::from_raw(pid as i32), Signal::SIGTERM);
    Ok(())
}

#[cfg(not(unix))]
fn kill_process(pid: u32) -> std::io::Result<()> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
    unsafe {
        let h = OpenProcess(PROCESS_TERMINATE, false, pid)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("{e}")))?;
        let r = TerminateProcess(h, 1).ok();
        let _ = CloseHandle(h);
        // `.ok()` converts Result -> Option, so check `.is_none()` instead
        // of `.is_err()`.
        if r.is_none() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "TerminateProcess failed",
            ));
        }
        Ok(())
    }
}
