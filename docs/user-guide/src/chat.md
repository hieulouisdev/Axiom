# Chat

The Chat view is the primary interface for interacting with AI providers.

## Starting a Conversation

1. Click the **Chat** icon in the sidebar (or press `Ctrl+1`).
2. Type your message in the input field at the bottom of the window.
3. Press **Enter** to send. Press **Shift+Enter** for a new line.
4. The AI response appears as a new message bubble.

## Streaming Responses

By default, AI responses are streamed token-by-token. You will see the
response build up in real time. Streaming can be disabled in Settings if
you prefer to wait for the complete response.

## Multi-turn Conversations

Aegis AI maintains conversation context across messages. The AI sees the
full conversation history (up to the provider's context window limit) and
can reference earlier messages.

## Managing Conversations

- **New conversation** — Click the **+** button or press `Ctrl+N`.
- **Switch conversations** — Click a conversation in the sidebar list.
- **Search conversations** — Use the search bar in the sidebar to find
  conversations by content.
- **Delete conversation** — Right-click a conversation and select **Delete**.
  This permanently removes the conversation and all its messages from the
  database.

## Message Actions

Each message bubble has a context menu (right-click):

- **Copy** — Copy the message text to clipboard.
- **Regenerate** — Request a new response from the AI (for assistant messages).
- **Edit** — Edit the message and resubmit (for user messages).

## Canceling a Response

During streaming, click the **Stop** button (square icon) next to the
input field to cancel the current response. This also works for agent
loops — see [Agent](agent.md).

## System Prompt

You can set a custom system prompt in Settings. This prompt is prepended to
every conversation and instructs the AI on how to behave. The default system
prompt includes instructions for the computer-use agent tools.

## Markdown Rendering

AI responses are rendered as Markdown, including:

- **Code blocks** with syntax highlighting.
- **Tables**, lists, and blockquotes.
- **Links** (clickable, open in default browser).
- **Math** (LaTeX notation, rendered via KaTeX).

## Token Usage

When using cloud providers, the chat input field shows an estimated token
count for your message. After each response, the total tokens used
(prompt + completion) are displayed below the response bubble.
