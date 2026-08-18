# Configuring AI Providers

Aegis AI supports 90+ AI providers. This section explains how to configure
and switch between them.

## Adding a Provider

1. Open **Settings** → **Providers**.
2. Click **Add Provider**.
3. Select a provider from the catalog (or choose **Custom** for a
   provider with an OpenAI-compatible API).
4. Enter the required configuration:
   - **API Key** — Stored in your OS keychain (never in plaintext).
   - **Base URL** — Auto-filled for known providers; customize for
     self-hosted or proxy endpoints.
   - **Model** — Select from the provider's model catalog, or enter a
     custom model name.
5. Click **Test Connection** to verify your credentials and connectivity.
6. Click **Save**.

## Provider Categories

### Cloud Providers

| Provider | API Shape | Streaming | Notes |
|---|---|---|---|
| OpenAI | Native | ✅ | GPT-4o, GPT-4, GPT-3.5 |
| Anthropic | Native | ✅ | Claude 3.5 Sonnet, Opus |
| Google Gemini | Native | ✅ | Gemini Pro, Ultra |
| Mistral | OpenAI-compat | ✅ | Mistral Large, Medium |
| Cohere | Native | ✅ | Command R+ |
| Groq | OpenAI-compat | ✅ | Fast inference |
| DeepSeek | OpenAI-compat | ✅ | DeepSeek V3, R1 |
| xAI (Grok) | OpenAI-compat | ✅ | Grok-2 |

### Local Providers

| Provider | API Shape | Streaming | Notes |
|---|---|---|---|
| Ollama | Native | ✅ | Runs models locally |
| LM Studio | OpenAI-compat | ✅ | GUI for local models |
| llama.cpp server | OpenAI-compat | ✅ | Lightweight server |
| GPT4All | OpenAI-compat | ✅ | Local inference |
| Jan | OpenAI-compat | ✅ | Local AI assistant |

### Router / Proxy Providers

| Provider | Purpose |
|---|---|
| OpenRouter | Routes to multiple providers |
| Aegis Cloud | Pre-configured, built-in provider |
| Requesty | Smart routing |
| Together AI | Serverless inference |

### Enterprise Providers

| Provider | Notes |
|---|---|
| AWS Bedrock | Requires AWS credentials + SigV4 |
| Azure OpenAI | Requires Azure endpoint + key |
| Google Vertex AI | Requires GCP service account |

## Switching Providers

- Use the **provider dropdown** at the top of the Chat view.
- Or open **Settings** → **Providers** and set a provider as **Active**.
- Switching providers starts a fresh conversation context.

## Aegis Cloud (Built-in)

Aegis Cloud is a pre-configured provider that works out of the box with
no API key configuration. It provides access to popular models through
the Aegis AI cloud service. To activate it, click **Use Aegis Cloud** in
the provider setup dialog.

## Custom Providers

If your provider exposes an OpenAI-compatible API (`/v1/chat/completions`),
you can add it as a **Custom** provider:

1. Choose **Custom** from the provider catalog.
2. Enter the base URL (e.g., `https://my-api.example.com/v1`).
3. Enter the API key.
4. Enter the model name.

## Provider Testing

The **Test Connection** button sends a minimal chat request to verify:

- The API key is valid.
- The endpoint is reachable.
- The specified model is available.

If the test fails, check the error message for details (network error,
authentication failure, model not found, etc.).
