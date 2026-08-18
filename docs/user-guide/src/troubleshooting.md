# Troubleshooting

## Installation

| Issue | Solution |
|---|---|
| Linux AppImage won't launch | `chmod +x aegis-ai_*.AppImage && ./aegis-ai_*.AppImage` |
| macOS "damaged and can't be opened" | `xattr -cr /Applications/AegisAI.app` |
| Windows SmartScreen warning | Click **More info** → **Run anyway** |

## Providers

| Issue | Solution |
|---|---|
| "Keyring error" on Linux | `sudo apt install gnome-keyring` or `pass` |
| "Connection refused" | Check internet, proxy (`HTTPS_PROXY`), local provider running, firewall |
| "Model not found" | Check provider model catalog; for local, ensure model downloaded |
| "Rate limit exceeded" | Wait and retry; check provider usage dashboard |

## Security

| Issue | Solution |
|---|---|
| Threat false positive | Review in **Security** → **Events**; remove signature or disable auto-defense |
| Quarantined file I need | **Security** → **Quarantine** → **Restore** |

## Voice

| Issue | Solution |
|---|---|
| Microphone not working | Grant OS permission; check exclusive use by other app |
| Poor STT quality | Set correct language; speak clearly; use headset mic |
| TTS not working (Linux) | `sudo apt install espeak` |

## Performance

| Issue | Solution |
|---|---|
| High CPU | Switch to on-demand mode; increase monitoring interval |
| Large database | Delete old conversations; check **Memory** → **Stats** |

## Getting Help

- [GitHub Issues](https://github.com/hieulouisdev/Axiom/issues)
- [GitHub Discussions](https://github.com/hieulouisdev/Axiom/discussions)
- Security issues: see [SECURITY.md](../../../SECURITY.md)
