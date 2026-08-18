# Configuring AI Providers

Aegis AI supports 90+ AI providers across four categories.

## Adding a Provider

1. **Settings** → **Providers** → **Add Provider**
2. Select from catalog or choose **Custom**
3. Enter API key (stored in OS keychain), base URL, model
4. **Test Connection** → **Save**

## Provider Categories

| Category | Examples |
|---|---|
| **Cloud** | OpenAI, Anthropic, Gemini, DeepSeek, Groq, Mistral, Cohere, xAI, Perplexity, Cerebras, NVIDIA |
| **Local** | Ollama, LM Studio, llama.cpp, GPT4All, Jan |
| **Router/Proxy** | OpenRouter, Aegis Cloud, Together AI |
| **Enterprise** | AWS Bedrock, Azure OpenAI, Google Vertex AI |

## Switching Providers

Use the **provider dropdown** at the top of Chat, or **Settings** → **Providers** → **Set as Active**. Switching starts a fresh conversation context.

## Aegis Cloud (Built-in)

Pre-configured, zero-config provider. Click **Use Aegis Cloud** to activate.

## Custom Providers

For any OpenAI-compatible endpoint: choose **Custom**, enter base URL + API key + model name.

## Testing

**Test Connection** verifies: API key valid, endpoint reachable, model available.
