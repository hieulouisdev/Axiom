# Web Search & Page Fetching

Aegis AI can search the web and fetch page content, either directly from
the chat or via agent tools.

## Web Search

Search the web from within Aegis AI:

1. Type a search query in the chat (e.g., "Search for: Rust Tauri tutorial").
2. Or use the **Web** view in the sidebar to search directly.
3. Results are shown with titles, URLs, and snippets.
4. Click a result to open it in your default browser, or ask the AI to
   fetch and summarize the page content.

### How It Works

Web search uses a search API to retrieve results. The AI can then:

- **Summarize** — Fetch the page content and provide a summary.
- **Extract** — Pull specific information from the page.
- **Cite** — Include the source URL in its response.

### Privacy

Search queries are sent to the search API provider. The AI does not track
your search history, but the search API provider may. Use a privacy-focused
search provider if this is a concern.

## Page Fetching

Fetch and parse the content of any web page:

### Via Chat

Ask the AI to fetch a page:

> "Fetch and summarize https://example.com/article"

### Via Agent Tool

The `web_fetch` and `web_fetch_raw` agent tools fetch page content:

- **`web_fetch`** — Fetches the page and extracts the main content
  (article text, removing navigation, ads, etc.) using a readability
  algorithm.
- **`web_fetch_raw`** — Fetches the raw HTML of the page.

### Content Safety

Fetched web content is treated as **untrusted input**. The AI is instructed
to treat web content as information, not as instructions. This mitigates
indirect prompt injection attacks where a malicious web page contains
hidden instructions for the AI.

## Web View

The **Web** sidebar view provides a dedicated interface for:

- **Search** — Enter a query and browse results.
- **Recent** — View recently fetched pages.
- **Bookmarks** — Save frequently accessed pages (planned for v1.0).

## Configuration

Configure web access in **Settings** → **Web**:

- **Search provider** — Choose your preferred search API.
- **User agent** — Custom user agent string for fetching (default: Aegis AI).
- **Timeout** — Request timeout in seconds (default: 30).
- **Max content size** — Maximum page size to fetch (default: 1 MB).

## Using Web Content with RAG

You can add fetched page content to the knowledge base for long-term
reference:

1. Fetch a page using the `web_fetch` tool.
2. Ask the AI to "Save this to the knowledge base as [key]".
3. The content is stored and will be available for RAG in future
  conversations.
