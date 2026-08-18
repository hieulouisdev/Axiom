# Security Monitoring

Built-in security subsystem: process monitor, file scanner, quarantine, integrity monitor, network monitor.

## Process Monitor

Polls running processes every 15s against threat signatures. Detects: reverse shells, credential dumpers, crypto miners, port scanners.

When a threat is detected → notification toast → audit log → if auto-defense: quarantine (Medium+) or kill (Critical).

### Custom Signatures

```toml
[[threat_signatures]]
name = "my_custom_threat"
pattern = "suspicious_process_name"
severity = "high"
```

## File Scanner

On-demand SHA-256 hash scanning via **Security** → **Scanner**.

## Quarantine

Quarantined files stored in `~/.local/share/aegis-ai/quarantine/`. View, restore, or permanently delete under **Security** → **Quarantine**.

## Integrity Monitoring

SHA-256 baselines of critical files. **Security** → **Integrity** → Check or Save Baseline.

## Network Monitor

Detects anomalous outbound connections: unusual ports, high-volume transfers, unrecognized hosts. View under **Security** → **Network**.

## YARA Rules

Place `.yar` files in the YARA rules directory. Loaded automatically for file scans.

## Auto-Defense

- **Off** — detect only (notify, no action)
- **On** — quarantine ≥ Medium, kill Critical

Every action logged for review.
