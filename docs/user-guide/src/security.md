# Security Monitoring

Aegis AI includes a built-in security subsystem that monitors your computer
for threats and can take automated defensive action.

## Overview

The security subsystem has five components:

1. **Process Monitor** — Continuously watches running processes for known
   threat signatures.
2. **File Scanner** — On-demand scanning of files against known-bad hashes.
3. **Quarantine** — Isolates suspicious files in a safe directory.
4. **Integrity Monitor** — Detects tampering with critical application files.
5. **Network Monitor** — Detects anomalous outbound connections.

## Process Monitor

The process monitor runs in the background, checking running processes
every 15 seconds against a list of threat signatures. Default signatures
detect:

- **Reverse shells** — `/dev/tcp/`, `nc -e`, `bash -i`
- **Credential dumpers** — `mimikatz`, `procdump`, `lsass`
- **Crypto miners** — `xmrig`, `stratum+tcp`, `minerd`
- **Port scanners** — `nmap`, `masscan`, `zmap`

When a threat is detected, Aegis AI:

1. Shows a notification toast in the UI.
2. Logs the event to the audit log.
3. If **auto-defense** is enabled, escalates the response:
   - **Medium severity** → Quarantine the binary.
   - **Critical severity** → Kill the process.

### Custom Signatures

Add custom threat signatures in `config.toml`:

```toml
[[threat_signatures]]
name = "my_custom_threat"
pattern = "suspicious_process_name"
severity = "high"
```

## File Scanner

Scan files or directories for known-bad content:

1. Open **Security** → **Scanner**.
2. Select a file or directory to scan.
3. Click **Scan**. Results show matches against known-bad SHA-256 hashes.

## Quarantine

Quarantined files are stored in `~/.local/share/aegis-ai/quarantine/`
with their original filename and metadata. You can:

- **View** quarantined files in **Security** → **Quarantine**.
- **Restore** a file to its original location.
- **Delete** a file permanently.

## Integrity Monitoring

The integrity monitor maintains SHA-256 baselines of critical files
(configuration, database, quarantine directory). Run a check:

1. Open **Security** → **Integrity**.
2. Click **Check Integrity** to compare current state against the baseline.
3. Click **Save Baseline** to update the baseline after legitimate changes.

## Network Monitor

The network monitor detects anomalous outbound connections:

- Connections to unusual ports (not 80 or 443).
- High-volume data transfers.
- Connections to unrecognized hosts.

View network events in **Security** → **Network**.

## YARA Rules

Aegis AI supports custom YARA rules for advanced threat detection:

1. Open **Security** → **YARA Rules**.
2. Place `.yar` rule files in the YARA rules directory (shown in the UI).
3. Rules are loaded automatically and used during file scans.

## Auto-Defense

Enable auto-defense in **Security** → **Settings**:

- **Off** — Detect only (notify, no action).
- **On** — Automatically quarantine threats ≥ Medium severity and kill
  Critical severity processes.

When auto-defense takes action, every step is logged in the audit log
for review.

## Security Dashboard

The **Security** view provides a dashboard with:

- Current threat level (overall assessment).
- Recent security events.
- Process monitor status.
- Quarantine contents.
- Network anomalies.
- Integrity check results.
