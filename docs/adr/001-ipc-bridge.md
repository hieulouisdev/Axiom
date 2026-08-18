# ADR 001: IPC Bridge

**Status:** Accepted

## Context

Rust backend and React frontend need a communication channel. Options: custom WebSocket, shared memory/FFI, Tauri `invoke_handler`, custom event bus.

Requirements: type safety, capability gating, streaming support, low latency (<5ms), security.

## Decision

Use **Tauri's `invoke_handler`** as primary IPC, with Tauri events for streaming.

- Zero additional infrastructure (no WebSocket server, no port binding)
- Full type safety via `serde`
- Tauri 2.0 capability system provides explicit command gating
- IPC channel inaccessible from outside the application
- Streaming via `app.emit("ai://chunk", payload)` — no separate WebSocket

## Consequences

**Positive:** Minimal infrastructure, full type safety, capability-based security, internal-only channel.

**Negative:** JSON serialization overhead for large payloads (binary data uses base64); new commands require explicit `lib.rs` registration (intentional for security); Tauri events unidirectional (backend → frontend only).

**Risk:** Misconfigured capability file could block or over-expose commands. CI should validate capability file against registered command set.
