# Web Search & Page Fetching

Search the web and fetch page content from within Aegis AI.

## Web Search

Type a search query in chat (e.g., "Search for: Rust Tauri tutorial") or use the **Web** sidebar view. Results show titles, URLs, snippets. Click to open in browser, or ask the AI to fetch and summarize.

**Privacy:** Search queries sent to the search API provider. Use a privacy-focused provider if concerned.

## Page Fetching

Ask the AI: "Fetch and summarize https://example.com/article"

Agent tools:
- **`web_fetch`** — extracts main content (readability algorithm)
- **`web_fetch_raw`** — raw HTML

Fetched content treated as **untrusted input** — AI instructed to treat as information, not instructions (mitigates indirect prompt injection).

## Using with RAG

Fetch a page → ask AI to "Save to knowledge base as [key]" → available for RAG in future conversations.

## Configuration

**Settings** → **Web**: search provider, user agent, timeout (30s), max content size (1 MB).
