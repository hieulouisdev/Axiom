# Memory, Knowledge Base & RAG

Aegis AI maintains persistent memory across sessions, including conversation
history, a knowledge base, and entity extraction.

## Conversations

All conversations are stored in a local SQLite database and persist across
application restarts. You can:

- **Browse** conversations in the sidebar.
- **Search** across all conversations by content.
- **Resume** a conversation by clicking on it.
- **Delete** a conversation (permanently removes it from the database).
- **Summarize** a long conversation using the **Summarize** button.

## Knowledge Base

The knowledge base stores key-value entries that the AI can reference during
conversations. This is useful for:

- Storing project documentation that the AI should know about.
- Saving frequently-used code snippets or templates.
- Maintaining a personal reference library.

### Adding Knowledge

1. Open **Memory** → **Knowledge**.
2. Click **Add Entry**.
3. Enter a **key** (short identifier, e.g., "project:api-design") and
   a **value** (the content).
4. Optionally set a **source** (where the knowledge came from) and a
   **confidence** score (0.0–1.0).

### RAG (Retrieval-Augmented Generation)

When you ask a question, Aegis AI can automatically search the knowledge
base for relevant entries and include them as context for the AI. This is
called retrieval-augmented generation.

How it works:

1. Your query is converted to a vector embedding.
2. The embedding is compared against all knowledge entries using cosine
   similarity.
3. The top-K most similar entries are included as context in the AI prompt.
4. The AI response is informed by both your query and the retrieved context.

The current embedding method uses character-trigram hashing — fast and
deterministic, but not semantically aware. A future update will switch to
neural embeddings for better semantic search.

## Entity Extraction

Aegis AI can automatically extract named entities (people, organizations,
dates, locations, etc.) from conversations. Entities are stored in the
database and can be browsed under **Memory** → **Entities**.

## Activities

The activity log records all significant actions (chat messages, agent
actions, security events) with timestamps. Browse it under
**Memory** → **Activities**.

## Memory Statistics

View database statistics under **Memory** → **Stats**:

- Total conversations and messages.
- Knowledge base entries and embeddings.
- Activity log entries.
- Database file size.

## Data Privacy

- All data is stored locally in `aegis.db` (SQLite).
- No data is sent to any server except your chosen AI provider.
- You can export all data (JSON) via **Settings** → **Export All**.
- You can delete all data (GDPR right to erasure) via **Settings** →
  **Forget All**. This is irreversible.

## Encryption

You can opt in to SQLCipher encryption for the database at rest. When
enabled, the database is encrypted with AES-256. Check encryption status
under **Memory** → **Encryption Status**.
