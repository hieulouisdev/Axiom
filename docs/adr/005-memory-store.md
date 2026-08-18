# ADR 005: Memory Store

**Status:** Accepted

## Context

Need persistent storage for conversations, activities/audit, knowledge base, embeddings, entities, events. Requirements: reliability, performance (<1ms reads), concurrency, portability, schema evolution, minimal dependencies (no external DB server).

Options: JSON files, Sled, SQLite via `rusqlite` (bundled), SQLite via `sqlx` (async), Redb.

## Decision

**SQLite with `rusqlite` bundled** (statically linked). Single file (`aegis.db`) with `Arc<Mutex<Connection>>` (`SharedConn`).

- ACID transactions — no data loss on crash
- WAL mode — concurrent readers + single writer
- Schema migrations on startup (idempotent `CREATE TABLE IF NOT EXISTS`)
- SQLCipher opt-in for encryption at rest

Rationale: SQLite is the most deployed database (30+ years of testing). `rusqlite` with `bundled` statically links it. `SharedConn` pattern sufficient for Aegis AI's write volume. SQL enables complex relational queries.

## Consequences

**Positive:** Battle-tested, full SQL, ACID, WAL mode, simple migrations, statically linked, SQLCipher opt-in, single-file backup.

**Negative:** Single writer (Mutex serializes writes — fine for current volume); C dependency (SQLite statically linked, Rust safety guarantees don't apply to C code); synchronous I/O (wrapped with `spawn_blocking` for async contexts); filesystem locking (WAL mitigates but doesn't eliminate).

**Risk:** Database corruption on filesystem failure (extremely unlikely with WAL + atomic commit — recommend regular backups). Unbounded growth (recommend retention policy for v1.0). SQLCipher key management (if enabled, key stored in OS keychain — if lost, data unrecoverable).
