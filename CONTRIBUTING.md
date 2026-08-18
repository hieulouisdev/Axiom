# Contributing to Aegis AI

Thanks for your interest! This guide covers setup, coding standards, and the PR process.

---

## Development Setup

### Prerequisites

- **Rust 1.97.1** — pinned via `rust-toolchain.toml` (auto-installed on first `cargo` run)
- **Node.js 20+** and **npm**
- **Tauri 2 system deps**:
  - Linux: `sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev libssl-dev patchelf`
  - Windows: WebView2 runtime (pre-installed on Windows 11)

### First Run

```bash
git clone https://github.com/hieulouisdev/Axiom.git
cd Axiom
npm install
npm run tauri:dev    # first build takes 5–10 min; hot reload after
```

---

## Coding Standards

### Rust

- `cargo fmt --all` enforced in CI
- `cargo clippy --all-targets -- -D warnings` enforced in CI
- Every public function/struct has a doc comment
- Error handling: `anyhow::Result` internally, `AegisError` (with serde) at Tauri boundaries
- No `unwrap()` in non-test code — use `?` or `.context("…")`
- Async: `tokio` runtime; `async_trait` for trait methods

### TypeScript / React

- `tsc --noEmit` must pass
- Functional components + hooks only
- Use Tailwind utility classes (`aegis-card`, `aegis-btn`, `aegis-input`, `aegis-toggle`)
- All user-visible strings through `t("…")`
- No inline styles

---

## Project Layout

- `src-tauri/src/ai/` — AI providers (one file per provider)
- `src-tauri/src/computer/` — computer-use actions + safety policy
- `src-tauri/src/security/` — monitor, scanner, defender, quarantine
- `src-tauri/src/memory/` — SQLite-backed stores
- `src-tauri/src/modes/` — continuous / on-demand
- `src-tauri/src/i18n/` — 7-locale translation tables
- `src-tauri/src/commands.rs` — Tauri IPC bridge
- `src/components/` — React UI components

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the full module map.

---

## Submitting Changes

1. **Open an issue** describing the change. Tag it with the relevant phase.
2. **Fork** and create a branch: `git checkout -b feat/my-feature`.
3. **Make changes** — one logical change per commit.
4. **Run checks**:
   ```bash
   cd src-tauri && cargo fmt --all && cargo test
   cd .. && npm run build
   ```
5. **Open a PR** against `main`. Reference the original issue.
6. **Address review feedback**. CI must pass before merge.

### Branch Naming

- `feat/<desc>` — new features
- `fix/<desc>` — bug fixes
- `security/<desc>` — security changes
- `docs/<desc>` — documentation

### Commit Format

Use conventional commits: `feat:`, `fix:`, `security:`, `docs:`.

---

## Adding a New AI Provider

See [docs/PROVIDERS.md](docs/PROVIDERS.md) for the full guide.

- **OpenAI-compatible**: one file, ~15 lines, using `openai_compat::make(descriptor(...))`.
- **Bespoke API** (Anthropic, Gemini, Ollama): implement the `Provider` trait directly.

---

## Reporting Bugs

Open a GitHub issue with:

- Aegis AI version (`app_version` output or `Cargo.toml`)
- OS + version
- Steps to reproduce
- Expected vs. actual behavior
- Logs from `<data_dir>/logs/aegis.log` (if applicable)

## Reporting Security Vulnerabilities

**Do not open a public issue.** See [SECURITY.md](SECURITY.md) for the responsible disclosure process.

---

## Code of Conduct

Be kind. Be patient with newcomers. Assume good intent. Disagree about ideas, not people.

## License

By contributing, you agree that your contributions are licensed under the [MIT license](LICENSE).
