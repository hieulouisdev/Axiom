# Voice I/O

Aegis AI supports voice input (speech-to-text) and voice output
(text-to-speech) for hands-free interaction.

## Speech-to-Text (STT)

Voice input uses cloud-based Whisper for high-accuracy transcription.

### Using Voice Input

1. Click the **microphone icon** in the chat input area, or
2. Press and hold the **push-to-talk hotkey** (default: `Ctrl+Alt+V`).
3. Speak your message.
4. Release the hotkey (or click the mic icon again to stop).
5. The transcription appears in the chat input field.
6. Press **Enter** to send, or edit the transcription before sending.

### Configuration

- **STT provider** — Cloud Whisper (default) or OS-native.
- **Language** — Set the expected language for better accuracy.
- **Auto-send** — Automatically send the transcribed message (no Enter
  required). Disabled by default.

### Privacy Note

Cloud STT sends your audio to the Whisper API for transcription. The audio
is not stored locally after transcription. If you prefer fully local voice
input, use a local STT provider (e.g., Whisper.cpp via Ollama) when
available.

## Text-to-Speech (TTS)

Voice output reads AI responses aloud.

### Using Voice Output

1. Click the **speaker icon** on any message bubble to read it aloud.
2. Or enable **auto-speak** in Settings to automatically read every AI
   response.

### TTS Providers

| Provider | Quality | Latency | Privacy |
|---|---|---|---|
| OS-native (default) | Good | Low | On-device |
| ElevenLabs | Excellent | Medium | Cloud |

OS-native TTS uses your operating system's built-in speech synthesis
(macOS: `say`, Linux: `espeak`, Windows: SAPI). It requires no API key
and works entirely on-device.

ElevenLabs provides high-quality neural TTS but requires an API key and
sends text to the ElevenLabs API.

### Configuration

- **TTS provider** — OS-native (default) or ElevenLabs.
- **Voice** — Select from available voices for the chosen provider.
- **Speed** — Adjust speech rate (0.5× – 2.0×).
- **Auto-speak** — Automatically read AI responses aloud.

## Push-to-Talk Hotkey

The push-to-talk hotkey activates voice input when held down:

- **Default:** `Ctrl+Alt+V`
- **Customize:** Settings → Voice → Push-to-Talk Hotkey
- The hotkey works even when Aegis AI is not the focused window (global
  shortcut).

### Hotkey States

| State | Indicator | Meaning |
|---|---|---|
| Idle | Mic icon (gray) | Not listening |
| Listening | Mic icon (red, pulsing) | Recording audio |
| Processing | Mic icon (yellow) | Transcribing |

## Troubleshooting

- **"No microphone access"** — Grant microphone permission in your OS
  settings.
- **"STT service unavailable"** — Check your internet connection (cloud
  Whisper requires network access).
- **Poor transcription quality** — Set the correct language in Settings.
  Speak clearly and minimize background noise.
- **"Keyring error on Linux"** — Install a Secret Service daemon
  (`gnome-keyring` or `pass`) for ElevenLabs API key storage.
