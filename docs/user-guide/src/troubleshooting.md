# Troubleshooting

This section covers common issues and their solutions.

## Installation Issues

### Linux: AppImage won't launch

**Symptom:** Double-clicking the AppImage does nothing, or you get a
"permission denied" error.

**Solution:**
```bash
chmod +x aegis-ai_0.7.0_amd64.AppImage
./aegis-ai_0.7.0_amd64.AppImage
```

### macOS: "App is damaged and can't be opened"

**Symptom:** Gatekeeper blocks the app on first launch.

**Solution:**
```bash
xattr -cr /Applications/AegisAI.app
```

Then open it from Finder.

### Windows: SmartScreen warning

**Symptom:** Windows SmartScreen warns that the app is unrecognized.

**Solution:** Click **More info** → **Run anyway**. This appears because
the app is not code-signed with a Microsoft-recognized certificate.

## Provider Issues

### "API key not found" or "Keyring error"

**Symptom:** You've entered an API key but get a keyring error when
trying to use the provider.

**Linux solution:** Install a Secret Service daemon:
```bash
# Ubuntu/Debian
sudo apt install gnome-keyring

# Or use pass (command-line password manager)
sudo apt install pass
```

**macOS solution:** Ensure Keychain Access is working. Try locking and
unlocking your keychain.

**Windows solution:** Ensure the Credential Manager service is running.

### "Connection refused" or "Network error"

**Symptom:** Provider test fails with a network error.

**Solutions:**
- Check your internet connection.
- If behind a proxy, configure the `HTTPS_PROXY` environment variable.
- If using a local provider (Ollama, LM Studio), ensure it's running and
  the base URL is correct (default: `http://localhost:11434` for Ollama).
- Check firewall settings — Aegis AI needs outbound access on port 443.

### "Model not found"

**Symptom:** The provider doesn't recognize the model name.

**Solutions:**
- Check the provider's model catalog in **Settings** → **Providers** →
  **Models**.
- For local providers, ensure the model is downloaded and loaded.
- For cloud providers, verify the model name matches the provider's
  documentation exactly.

### "Rate limit exceeded"

**Symptom:** Provider returns a 429 rate limit error.

**Solutions:**
- Wait a few seconds and retry.
- Check your provider's usage dashboard for rate limit details.
- Consider switching to a provider with higher rate limits.

## Security Issues

### "Threat detected" false positive

**Symptom:** The security monitor flags a legitimate process.

**Solutions:**
- Review the detected threat in **Security** → **Events**.
- If it's a false positive, remove the matching signature from
  `config.toml` or add an exception.
- You can also temporarily disable auto-defense to prevent automatic
  quarantine.

### Quarantined file I need

**Symptom:** A legitimate file was quarantined by auto-defense.

**Solution:**
1. Open **Security** → **Quarantine**.
2. Find the file and click **Restore**.
3. The file is copied back to its original location.

## Voice Issues

### Microphone not working

**Symptom:** Push-to-talk doesn't record audio.

**Solutions:**
- Grant microphone permission in your OS settings.
- Check that no other application is exclusively using the microphone.
- Try a different microphone input device.

### STT produces poor transcriptions

**Symptom:** Voice input text is inaccurate.

**Solutions:**
- Set the correct language in **Settings** → **Voice** → **Language**.
- Speak clearly and minimize background noise.
- Use a close-talk microphone (headset) for best results.

### TTS not working

**Symptom:** No audio output when using text-to-speech.

**Solutions:**
- Check your speaker/volume settings.
- For OS-native TTS, ensure speech synthesis is available:
  - Linux: Install `espeak` (`sudo apt install espeak`).
  - macOS: Built-in, should work out of the box.
  - Windows: Built-in, should work out of the box.
- For ElevenLabs TTS, verify your API key is correct.

## Performance Issues

### High CPU usage

**Symptom:** Aegis AI uses excessive CPU.

**Solutions:**
- If in continuous mode, switch to on-demand mode to reduce background
  activity.
- Check if the security monitor is scanning frequently (increase the
  monitoring interval in Settings).
- Close unused conversations (large conversation histories increase
  memory usage).

### Large database file

**Symptom:** `aegis.db` is growing very large.

**Solutions:**
- Delete old conversations you no longer need.
- Use **Memory** → **Stats** to see the breakdown.
- Export and delete old activity logs.

## Getting Help

If your issue is not covered here:

- Check the [GitHub Issues](https://github.com/aegis-ai/axiom/issues)
  for known problems.
- Start a discussion on the
  [GitHub Discussions](https://github.com/aegis-ai/axiom/discussions) page.
- For security issues, email security@aegis-ai.dev (see the
  Vulnerability Disclosure Policy in the Security Whitepaper).
