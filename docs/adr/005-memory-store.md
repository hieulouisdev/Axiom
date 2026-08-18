# ADR 005: Memory Store

## Status

Accepted

## Context

Aegis AI needs persistent storage for:

- **Conversations** — Chat messages with metadata (timestamps, provider,
  model).
- **Activities/Audit log** — Record of all significant actions for
  forensics and user review.
- **Knowledge base** — Key-value entries with source attribution and
  confidence scores.
- **Embeddings** — Vector representations of knowledge entries for RAG.
- **Entities** — Extracted named entities from conversations.
- **Events** — Security events, system events, and custom events.

The storage system must satisfy:

- **Reliability** — No data loss on crash or power failure.
- **Performance** — Reads should be < 1ms for typical queries.
- **Concurrency** — Multiple subsystems read/write simultaneously.
- **Portability** — Works on Linux, macOS, and Windows with zero
  configuration.
- **Schema evolution** — Must support migrations as the schema evolves.
- **Minimal dependencies** — No external database server (no PostgreSQL,
  no Redis).

Options considered:

1. **JSON files** — Each record stored as a JSON file or appended to a
   JSONL file. Simple but no querying, no transactions, poor concurrency,
   no schema enforcement.
2. **Sled** — Embedded key-value store written in Rust. Fast, pure Rust,
   but immature API, no SQL, no relational queries, and data corruption
   reports in production.
3. **SQLite via `rusqlite` (bundled)** — Use SQLite as an embedded
   database with the `rusqlite` crate, statically linking the SQLite
   library. Full SQL, ACID transactions, mature, well-tested, but adds
   a C dependency.
4. **SQLite via `sqlx` (async)** — Async SQLite driver. Better for
   async-heavy applications, but SQLite's concurrency model (single
   writer) makes async less beneficial. Adds tokio dependency to the
   database layer.
5. **Redb** — Simple ACID key-value store in pure Rust. Too limited for
   relational queries (conversation → messages, knowledge → embeddings).

## Decision

We use **SQLite with `rusqlite` bundled** (statically linked).

The database is opened as a single file (`aegis.db`) in the application
data directory. All subsystems share a single `Arc<Mutex<Connection>>`
(aliased as `SharedConn`), providing:

- **ACID transactions** — All writes are transactional; no partial state
  on crash.
- **WAL mode** — Concurrent readers and a single writer, without blocking.
- **Schema migrations** — Run on every startup; idempotent `CREATE TABLE
  IF NOT EXISTS` statements.
- **SQLCipher opt-in** — Can be enabled for encryption at rest (AES-256).

Rationale:

- SQLite is the most deployed database in the world, with 30+ years of
  testing. It is the correct choice for an embedded desktop application.
- `rusqlite` with `bundled` feature statically links SQLite, so users
  don't need it installed separately.
- The `SharedConn = Arc<Mutex<Connection>>` pattern is simple, safe,
  and sufficient for Aegis AI's write volume (tens of writes per second,
  not thousands).
- SQL provides relational queries that would be cumbersome with a
  key-value store (e.g., "find all conversations from the last 7 days
  that mentioned provider X").

## Consequences

### Positive

- Mature, battle-tested database engine with 30+ years of history.
- Full SQL for complex queries (joins, aggregations, full-text search).
- ACID transactions prevent data loss on crash.
- WAL mode allows concurrent reads without blocking writes.
- Schema migrations are simple and idempotent.
- Statically linked — no external SQLite dependency on the user's machine.
- SQLCipher can be enabled for encryption at rest with zero code changes
  (just a feature flag and a passphrase).
- Single-file database — easy to backup, copy, or delete.

### Negative

- **Single writer** — SQLite allows only one writer at a time. The
  `Mutex<Connection>` serializes all writes. For Aegis AI's volume this
  is fine, but it would be a bottleneck for high-throughput scenarios.
- **C dependency** — SQLite is a C library, statically linked via
  `rusqlite`'s `bundled` feature. This means:
  - Rust's memory safety guarantees don't apply to the SQLite code.
  - SQLite vulnerabilities must be patched by updating the `rusqlite`
    version (which bundles a specific SQLite release).
  - Cross-compilation requires the C toolchain for the target platform.
- **No async I/O** — SQLite operations are synchronous. We wrap them in
  `Mutex` and call from async contexts with `tokio::task::spawn_blocking`
  when needed. This works but isn't elegant.
- **File locking** — SQLite uses filesystem locks. If another process
  opens the same database file, it will contend for the lock. WAL mode
  mitigates this but doesn't eliminate it.

### Risks

- **Database corruption** — Possible on filesystem corruption, disk
  failure, or OS crash during a write. SQLite's WAL mode and atomic
  commit make this extremely unlikely, but the user should back up
  `aegis.db` regularly. The integrity monitoring module
  (`security/integrity.rs`) can detect database tampering.
- **Unbounded growth** — The database grows with usage. There is no
  automatic cleanup of old conversations or activities. Recommendation
  for v1.0: add a configurable retention policy that prunes old data.
- **SQLCipher key management** — If SQLCipher is enabled, the encryption
  key must be stored somewhere (OS keychain is the obvious choice). If
  the key is lost, the database is unrecoverable.
