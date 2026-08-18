# Memory, Knowledge Base & RAG

Aegis AI maintains persistent memory: conversation history, knowledge base, and entity extraction.

## Conversations

Stored in local SQLite, persist across restarts. Browse, search, resume, or delete conversations in the sidebar.

## Knowledge Base

Store key-value entries the AI can reference during conversations.

### Adding Knowledge

1. **Memory** → **Knowledge** → **Add Entry**
2. Enter key (e.g., "project:api-design"), value, optional source and confidence

### RAG

When you ask a question, Aegis AI searches the knowledge base for relevant entries and includes them as context:

1. Query → vector embedding
2. Cosine similarity search against all entries
3. Top-K results prepended to AI prompt
4. Response informed by both query and retrieved context

Current embedding: character-trigram hashing (fast, deterministic). Neural embeddings planned for v1.0.

## Entity Extraction

Named entities (people, organizations, dates, locations) automatically extracted from conversations. Browse under **Memory** → **Entities**.

## Activities

Activity log records all significant actions with timestamps. Browse under **Memory** → **Activities**.

## Data Privacy

- All data local in `aegis.db`
- No data sent except to your chosen AI provider
- **Export All** → JSON archive | **Forget All** → irreversible deletion (GDPR right to erasure)

## Encryption

Opt in to SQLCipher (AES-256) encryption at rest via **Memory** → **Encryption Status**.
