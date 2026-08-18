# ADR 001: IPC Bridge

## Status

Accepted

## Context

Aegis AI is a Tauri 2.0 desktop application with a Rust backend and a React
frontend. These two components need a communication channel to exchange data
and trigger operations. Several options exist:

1. **Custom WebSocket server** — The Rust backend opens a local WebSocket
   server, and the frontend connects to it. This is how Electron apps
   typically communicate between main and renderer processes.
2. **Shared memory / FFI** — The frontend calls Rust functions directly
   through foreign function interface. This requires the frontend to have
   direct access to the Rust binary's symbol table.
3. **Tauri `invoke_handler`** — The built-in Tauri IPC mechanism where the
   frontend calls `invoke("command_name", args)` and Tauri dispatches to
   the corresponding Rust function annotated with `#[tauri::command]`.
4. **Custom event bus** — A bidirectional event channel where either side
   can publish events and the other subscribes. Tauri supports this via
   `app.emit()` and `window.listen()`.

The IPC mechanism must satisfy these requirements:

- **Type safety** — Arguments and return values should be strongly typed.
- **Capability gating** — Not all commands should be accessible from the
  frontend; the set must be explicitly declared.
- **Streaming support** — AI chat responses are streamed token-by-token.
- **Low latency** — The frontend should feel responsive (< 5ms IPC overhead).
- **Security** — The IPC channel must not be accessible from outside the
  application.

## Decision

We use **Tauri's `invoke_handler`** as the primary IPC mechanism, with
Tauri events for streaming and push notifications.

Rationale:

- `invoke_handler` is purpose-built for Tauri applications and requires
  zero additional infrastructure (no WebSocket server, no port binding).
- Arguments and return values are automatically serialized/deserialized
  via `serde`, providing full type safety on both sides.
- Tauri 2.0's capability system (`capabilities/default.json`) provides
  explicit gating of which commands the frontend may invoke — this is a
  critical security property.
- The IPC channel is internal to the Tauri webview and cannot be accessed
  by external processes or web pages.
- Streaming is handled by emitting Tauri events (`app.emit("ai://chunk",
  payload)`) from the Rust backend, which the frontend subscribes to via
  `window.listen()`. This avoids the need for a separate WebSocket.

## Consequences

### Positive

- Minimal IPC infrastructure — no server to bind, no port to secure.
- Full type safety via `serde` on both sides of the bridge.
- Capability-based command gating provides defense-in-depth.
- Streaming via Tauri events is simple and well-supported.
- The IPC channel is inaccessible from outside the application.

### Negative

- All arguments are serialized to JSON, which adds overhead for large
  payloads (e.g., file contents, screenshots). For binary data, we use
  base64 encoding within the JSON envelope.
- The command registration macro (`tauri::generate_handler![]`) requires
  every command to be listed explicitly — adding a new command requires
  editing `lib.rs`. This is intentional (security) but adds friction.
- Tauri events for streaming are unidirectional (backend → frontend). If
  we need frontend → backend streaming in the future, we will need to
  use a different mechanism (e.g., a separate channel or chunked
  invoke calls).

### Risks

- If the capability file is misconfigured, commands may be inaccessible
  or overly permissive. CI should validate the capability file against
  the registered command set.
