# Voice I/O

Hands-free interaction via speech-to-text and text-to-speech.

## Speech-to-Text (STT)

1. Click **mic icon** or press **push-to-talk hotkey** (default: `Ctrl+Alt+V`)
2. Speak → release → transcription appears in chat input
3. Edit if needed → **Enter** to send

**Privacy:** Cloud STT sends audio to Whisper API (not stored locally). For fully local STT, use Whisper.cpp via Ollama when available.

## Text-to-Speech (TTS)

| Provider | Quality | Privacy |
|---|---|---|
| OS-native (default) | Good | On-device |
| ElevenLabs | Excellent | Cloud |

Click **speaker icon** on any message to read aloud, or enable **auto-speak** in Settings.

## Configuration

| Setting | Default |
|---|---|
| STT provider | Cloud Whisper |
| TTS provider | OS-native |
| Push-to-talk key | `Ctrl+Alt+V` |
| Auto-speak | Off |
| TTS speed | 1.0× |
