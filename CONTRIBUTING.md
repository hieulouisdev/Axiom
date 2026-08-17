# Contributing to Aegis AI

Thanks for your interest in contributing! This document explains how to set
up a development environment and submit changes.

## Development setup

### Prerequisites

- **Rust 1.97.1** — pinned via `rust-toolchain.toml`. The correct version is
  installed automatically when you run `cargo` for the first time.
- **Node.js 20+** and **npm**.
- **Tauri 2 system deps**:
  - Linux: `sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev libssl-dev patchelf`
  - Windows: WebView2 runtime (pre-installed on Windows 11)

### First run

```bash
git clone https://github.com/hieulouisdev/Axiom.git
cd Axiom
npm install
npm run tauri:dev
```

The first build takes 5–10 minutes (Tauri compiles all Rust deps). After
that, hot reload is fast.

## Coding standards

### Rust

- `cargo fmt --all` is enforced in CI.
- `cargo clippy --all-targets -- -D warnings` is enforced in CI
  (warnings are tolerated in v0.1 due to intentional stubs).
- Every public function / struct has a doc comment.
- Error handling: use `anyhow::Result` for internal code,
  `crate::error::AegisError` (with serde) for Tauri command boundaries.
- No `unwrap()` in non-test code. Use `?` or `.context("…")`.
- Async: prefer `tokio` runtime; use `async_trait` for trait methods.

### TypeScript / React

- `tsc --noEmit` must pass.
- Functional components + hooks only (no class components).
- Use the existing Tailwind utility classes (`aegis-card`, `aegis-btn`,
  `aegis-input`, `aegis-toggle`) for consistent styling.
- All user-visible strings go through the `t("…")` translation function.
- No inline styles; all styling in `index.css` or component className.

## Project layout

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the module map.

In short:

- `src-tauri/src/ai/` — AI providers (one file per provider).
- `src-tauri/src/computer/` — computer-use actions + safety policy.
- `src-tauri/src/security/` — monitor, scanner, defender, quarantine.
- `src-tauri/src/memory/` — SQLite-backed stores.
- `src-tauri/src/modes/` — continuous / on-demand.
- `src-tauri/src/i18n/` — translation tables.
- `src-tauri/src/commands.rs` — Tauri IPC bridge.
- `src/components/` — React UI components.

## Submitting changes

1. **Open an issue** describing what you intend to change (especially for
   anything beyond a typo fix). Tag it with the relevant phase
   (`phase-1`, `phase-2`, …) from the [Roadmap](ROADMAP.md).
2. **Fork the repo** and create a branch: `git checkout -b feat/my-feature`.
3. **Make your changes**. Keep commits focused — one logical change per commit.
4. **Run checks**:
   ```bash
   cd src-tauri && cargo fmt --all && cargo test
   cd .. && npm run build
   ```
5. **Open a PR** against `main`. Reference the original issue.
6. **Address review feedback**. CI must pass before merge.

## Picking up a Phase 2+ task

The [Roadmap](ROADMAP.md) lists every Phase 2+ task as a checkbox. To claim
one:

1. Open an issue titled `<task title>` and tag it `phase-2` (or whichever
   phase applies) plus `help-wanted` if you'd welcome collaboration.
2. Link to the relevant roadmap section.
3. Tag a maintainer for assignment.

## Adding a new AI provider

This is the most common contribution. See
[docs/PROVIDERS.md](docs/PROVIDERS.md) for the full guide. In short:

- If your provider is OpenAI-compatible: one file, ~15 lines, using the
  `openai_compat::make(descriptor(...))` helper.
- If it has a bespoke API shape (Anthropic, Gemini, Ollama): implement the
  `Provider` trait directly.

## Reporting bugs

Open a GitHub issue with:

- Aegis AI version (`app_version` command output, or `Cargo.toml`).
- OS + version.
- Steps to reproduce.
- Expected vs. actual behavior.
- Logs from `<data_dir>/logs/aegis.log` (if applicable).

## Reporting security vulnerabilities

**Do not open a public issue.** See [SECURITY.md](SECURITY.md) for the
responsible disclosure process.

## Code of conduct

Be kind. Be patient with newcomers. Assume good intent. Disagree about
ideas, not people.

## License

By contributing, you agree that your contributions are licensed under the
[MIT license](LICENSE) that covers the project.
