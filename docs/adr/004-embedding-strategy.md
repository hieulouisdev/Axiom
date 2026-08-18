# ADR 004: Embedding Strategy

## Status

Accepted (superseded by v1.0 neural embeddings planned)

## Context

Aegis AI's knowledge base requires a similarity search mechanism for
retrieval-augmented generation (RAG). When the AI receives a query, it
must find the most relevant knowledge entries to include as context.

Several embedding approaches were considered:

1. **Neural embeddings (all-MiniLM-L6-v2)** — Use a pre-trained sentence
   transformer model to generate 384-dimensional dense vectors. This is
   the industry standard for semantic search and provides excellent
   retrieval quality. However, it requires:
   - Downloading a 22 MB model file at build or install time.
   - ONNX Runtime (`ort` crate) as a dependency.
   - GPU/CPU inference on every embed call (~5ms per sentence on CPU).
   - Breaking the reproducible build pipeline (model file is a binary
     artifact).

2. **Cloud embedding API** — Use OpenAI's `text-embedding-3-small` or a
   similar API. Zero local compute, high quality. But:
   - Requires network access for every embedding.
   - Sends all knowledge base content to the embedding provider.
   - Adds latency (~100ms per embed call).
   - Violates the privacy-first design principle.

3. **Character-trigram hash embeddings** — Hash each piece of text into a
   fixed-dimensional sparse vector using a character-trigram hashing trick.
   Each character trigram (e.g., "the", "her", "eri") is hashed into one
   of N buckets, and the bucket value is the count of trigrams that landed
   there. Cosine similarity over these vectors provides a rough
   approximation of text similarity. This approach:
   - Requires zero external dependencies.
   - Is deterministic and reproducible.
   - Runs in microseconds, not milliseconds.
   - Handles typos and morphological variants reasonably well.
   - Is a poor man's semantic search — it cannot understand meaning, only
     character overlap.

4. **BM25 / TF-IDF (no embeddings)** — Traditional information retrieval
   without vector embeddings. Good for keyword search, poor for semantic
   similarity. Already partially implemented as the v0.4 Jaccard
   token-overlap baseline.

## Decision

We use **character-trigram hash embeddings** for v0.5–v0.7, with a planned
migration to neural embeddings (`all-MiniLM-L6-v2` via `ort`) in v1.0.

The trigram hash approach is implemented in `memory/embeddings.rs`:

- **Dimensionality:** 256 buckets (256 × 4 bytes = 1 KB per embedding).
- **Hash function:** FNV-1a 32-bit, deterministic and fast.
- **Normalization:** L2-normalized vectors so cosine similarity is a
  simple dot product.
- **Storage:** SQLite BLOB column (`knowledge_embeddings.vector`).

Rationale:

- v0.5–v0.7 is a rapid-iteration phase. We need a working RAG system
  that is fast, reproducible, and doesn't add heavyweight dependencies.
- The trigram hash approach is a **drop-in replacement** for neural
  embeddings — the storage interface (`EmbeddingStore::upsert`,
  `EmbeddingStore::search`) is the same. Swapping in neural embeddings
  requires only changing the `embed_text()` function.
- The quality gap is acceptable for v0.5–v0.7: the knowledge base is
  typically small (< 10k entries), and the user can see the retrieved
  context and judge relevance.

## Consequences

### Positive

- Zero external dependencies — no model downloads, no ONNX Runtime.
- Deterministic and reproducible — same text always produces the same
  embedding.
- Extremely fast — embedding generation is ~1μs per sentence.
- Small storage footprint — 1 KB per entry, 10 MB for 10k entries.
- Handles typos well (6 of 7 trigrams overlap between "calendar" and
  "calender").
- Drop-in interface — swapping to neural embeddings later requires only
  changing `embed_text()`.

### Negative

- **No semantic understanding** — "buy a car" and "purchase an
  automobile" have zero trigram overlap and will not match. This is the
  fundamental limitation of character-based approaches.
- **Collision risk** — With 256 buckets, different trigrams may hash to
  the same bucket, reducing discrimination. This is mitigated by
  L2-normalization and cosine similarity, but retrieval precision is
  lower than with 384-dimensional neural embeddings.
- **Fixed dimensionality** — 256 buckets is a compromise. Fewer buckets
  = more collisions; more buckets = sparser vectors = less meaningful
  cosine similarity. 256 works well for < 10k entries but may need
  adjustment for larger knowledge bases.

### Risks

- Users may perceive RAG as "not working well" if the trigram approach
  fails to retrieve obviously relevant entries. This is mitigated by
  showing the similarity score and source attribution in the UI.
- The migration to neural embeddings in v1.0 will require re-embedding
  the entire knowledge base. The `backfill()` method in
  `EmbeddingStore` supports this.
