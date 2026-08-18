# ADR 004: Embedding Strategy

**Status:** Accepted (neural embeddings planned for v1.0)

## Context

Knowledge base needs similarity search for RAG. Options: neural embeddings (all-MiniLM-L6-v2, 22MB model + ONNX Runtime), cloud embedding API, character-trigram hash embeddings, BM25/TF-IDF.

## Decision

**Character-trigram hash embeddings** for v0.5–v0.9, migration to neural embeddings in v1.0.

Implementation (`memory/embeddings.rs`):
- **256 buckets** (1 KB per embedding)
- **FNV-1a 32-bit hash** — deterministic, fast
- **L2-normalized** — cosine similarity = dot product
- **Storage** — SQLite BLOB column

Rationale: rapid-iteration phase needs fast, reproducible RAG without heavyweight dependencies. Trigram hash is a drop-in replacement interface — swapping to neural requires only changing `embed_text()`.

## Consequences

**Positive:** Zero dependencies, deterministic, ~1μs per sentence, 1 KB per entry, handles typos well, drop-in interface.

**Negative:** No semantic understanding ("buy a car" ≠ "purchase an automobile"); collision risk at 256 buckets; fixed dimensionality tradeoff.

**Risk:** Users may perceive RAG as "not working" for semantically related but lexically different queries. Mitigated by showing similarity score + source attribution in UI. Migration to neural embeddings in v1.0 requires re-embedding entire knowledge base (supported by `backfill()`).
