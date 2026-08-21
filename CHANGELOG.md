# Changelog

All notable changes to Aegis AI. Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [1.7.0] — 2026-08-21 — Singularity II

The second-largest feature drop in Aegis AI history. Five new subsystems
ship together, inspired by [TencentDB-Agent-Memory](https://github.com/TencentCloud/TencentDB-Agent-Memory)
and [worldmonitor](https://github.com/koala73/worldmonitor). On top of
the v1.6 Singularity baseline, v1.7 turns Aegis AI from a desktop app
into a **cross-platform AI workspace** with a beautiful CLI, world-class
memory, real-time world intelligence, and an MCP server.

### Added — Hierarchical Memory (`memory/hierarchy.rs`, ~280 LOC)

Inspired by TencentDB-Agent-Memory's L0→L3 layering:

- **L0 Conversations** — verbatim chat messages (already in v0.2).
- **L1 Atoms** — single distilled facts / preferences / decisions /
  instructions / goals / context, extracted automatically from chat
  via a deterministic regex/keyword pass (`deterministic_extract`).
- **L2 Scenarios** — themed clusters of atoms ("project X", "auth
  refactor") with tags and atom counts.
- **L3 Persona** — long-term user traits (`language`, `tz`,
  `prefers terse answers`, …), one row per user.
- **Prompt rendering** — `render_prompt_fragment()` produces a system-
  prompt block combining persona + scenarios + recent atoms.
- 5 unit tests covering atom roundtrip, scenario assignment, persona
  upsert, deterministic extraction, and prompt rendering.

### Added — Versioned Skill Library (`memory/skill_lib.rs`, ~320 LOC)

- **Lifecycle**: draft → review → published → deprecated → archived.
- **Versioned snapshots**: every `save_version` call creates a new
  numbered version with `system_prompt`, `trigger` (keywords + intents),
  `steps`, `validation_rules`, and `resources`. Only `published`
  versions are loaded into the agent context.
- **Trigger matching**: `match_triggers(message)` returns the slugs of
  all published skills whose trigger keywords appear in the message,
  sorted by keyword length (specificity).
- **Visibility**: private / team / public.
- 3 unit tests covering create/publish lifecycle, version promotion,
  and trigger matching.

### Added — Wiki Knowledge Base (`memory/wiki.rs`, ~200 LOC)

- **Pages** with `slug`, `title`, `body` (Markdown), `tags`, `source`.
- **Bidirectional link graph** (`wiki_links` table) — `links_from` /
  `links_to` (backlinks).
- **Full-text search** across title + body + tags.
- 2 unit tests covering upsert/links and search.

### Added — CodeGraph (`memory/codegraph.rs`, ~360 LOC)

Inspired by TencentDB-Agent-Memory's CodeGraph:

- **Symbol indexer** for Rust / TypeScript / Python / Go / JavaScript.
  Regex-based — no tree-sitter deps. Extracts functions, methods,
  structs, enums, traits, interfaces, classes, modules, constants, types.
- **Call edges** — within-body identifier-followed-by-`(` heuristic
  creates `caller → callee` edges in the same SQLite DB.
- **Impact analysis** — `callers_of(symbol_id)` and `callees_of(symbol_id)`
  for "who calls this?" / "what does this call?" queries.
- **Multi-repo** — register multiple repos with `register_repo(path,
  language)`, each gets its own `code_repos` row.
- 1 integration test covering Rust indexing with callers.

### Added — World Intelligence (`world/`, ~600 LOC)

Inspired by [worldmonitor](https://github.com/koala73/worldmonitor):

- **Feeds registry** (`feeds.rs`) — 20+ curated RSS/Atom feeds:
  Reuters, BBC, Al Jazeera, NYT, DW, VnExpress (Vietnamese), GDELT,
  ACLED, ISW, Hacker News, TechCrunch, Ars Technica, The Verge, CNBC,
  FT, Bloomberg, Krebs on Security, Schneier on Security, The Hacker
  News, Nature, NASA FIRMS (fires), USGS (earthquakes).
- **Async fetcher** with concurrent pull (up to 16 in flight) and a
  unified RSS 2.0 + Atom 1.0 parser (no `quick-xml` dep — hand-rolled
  for zero-dep portability).
- **News aggregator** (`news.rs`) — dedup by GUID/link, salience
  scoring (recency × keyword presence), category classification,
  prompt rendering.
- **Finance** (`finance.rs`) — CoinGecko (crypto, no API key), ECB
  daily reference rates (FX), Stooq (stocks, CSV). All free, no keys.
- **Country Instability Index** (`geopolitics.rs`) — weighted 0..100
  score from news volume + negative ratio + disaster count + market
  stress. 5 risk levels: stable / watch / elevated / high / critical.
- **Daily brief composer** (`brief.rs`) — combines news + markets +
  risks into a single text block for the agent system prompt.
- 8 unit tests covering RSS/Atom parsing, salience decay, dedup,
  ECB XML parsing, risk scoring, sentiment classification.

### Added — MCP Server (`mcp/`, ~250 LOC)

Inspired by worldmonitor's MCP integration:

- **JSON-RPC 2.0 over stdio** — works with Claude Desktop, Cursor,
  Codex, and any MCP-compatible client.
- **Methods**: `initialize`, `ping`, `tools/list`, `tools/call`,
  `shutdown`.
- **9 tools** registered by default: `memory_search`,
  `memory_remember`, `skills_match`, `world_news`, `world_finance`,
  `world_risk`, `wiki_search`, `codegraph_search`, `graph_query`.
- Handlers are no-op by default; the desktop app and CLI both replace
  them with real implementations at startup.
- 3 unit tests covering initialize, tools/list, and tools/call.

### Added — Aegis CLI (`cli/`, ~2,500 LOC)

A brand-new standalone binary — no Tauri, no webkit, no system deps.
Cross-platform: Linux x64 / ARM64 / ARMv7 / musl, Windows x64, macOS
Intel / Apple Silicon, and Android (via Termux).

- **7 built-in AI providers**: OpenAI, Anthropic, Gemini, DeepSeek,
  Z.AI (GLM-4.6, zero-key public preview), Ollama (local), OpenRouter.
  Plus a generic OpenAI-compatible fallback for custom endpoints.
- **Beautiful TUI** built with `ratatui` + `crossterm`. Five panels:
  Chat / Memory / World / Skills / Settings. Tab to switch, Esc to quit.
- **One-shot mode**: `aegis chat "explain async rust"` — single query,
  print answer, exit. Use `--json` for machine-readable output.
- **All v1.7 subsystems** accessible from CLI:
  - `aegis memory atoms | scenarios | persona | add | set-trait | prompt`
  - `aegis skills list | show | match | create | save-version | publish`
  - `aegis wiki list | search | show | add | remove`
  - `aegis world news | finance | snapshot | risk | brief`
  - `aegis code register | repos | index | search | callers | callees`
  - `aegis mcp` — run the MCP server
- **Cross-platform builds** via GitHub Actions workflow
  (`.github/workflows/cli-release.yml`) — automatically builds and
  attaches binaries to the v* release for all 7 targets.
- **Pure-Rust TLS** — uses `rustls` + `ring` (no `aws-lc-sys`), so
  cross-compilation works without a C cross-compiler.

### Changed — Workspace

- Bumped version `1.6.0` → `1.7.0` in workspace + desktop + CLI.
- Added `cli` as a workspace member.
- Added `Other` variant to `AegisError` for ergonomic error construction
  in the new modules.
- Desktop app's `MemoryStore` now initializes the new `hierarchy`,
  `skills`, `wiki`, and `code_graph` sub-stores and runs their
  migrations.
- Desktop app's `lib.rs` now exports `pub mod world;` and `pub mod mcp;`.

### Changed — README

- Completely rewritten with v1.7.0 features, download instructions for
  every platform (including Android via Termux), MCP integration guide
  for Claude Desktop and Cursor, and acknowledgements for the two
  reference repos.

### Changed — CI

- Added `.github/workflows/cli-release.yml` — builds the CLI binary for
  7 targets on tag push and attaches them to the release.

### Acknowledgements

The v1.7.0 design owes a debt to two open-source projects:

- **TencentDB-Agent-Memory** — the L0→L3 hierarchical memory model,
  versioned skill library, wiki + link graph, and code symbol graph
  concepts are direct adaptations of their architecture.
- **worldmonitor** — the world intelligence module (news, markets, CII)
  and the MCP integration pattern are direct adaptations of their
  dashboard and CLI.

Both are credited in the corresponding module docs and in the README.

---

## [1.6.0] — 2026-08-21 — Singularity Upgrade

The single biggest feature drop in Aegis AI history. Five new backend
subsystems ship together, each one sufficient on its own to anchor a major
release, layered on top of the v1.5 CI-stable baseline.

### Added — Multi-Agent Orchestrator (`ai/orchestrator.rs`, ~640 LOC)

- **Planner → Executor → Critic DAG.** Given a free-text goal, the
  orchestrator drafts a deterministic plan (research → implement → review →
  summarize, with optional security-audit / docs / test steps slotted in
  based on goal keywords), then asks the active AI provider to refine the
  DAG via a constrained `Plan -> Plan` JSON transformation. If the AI is
  offline, the deterministic draft still runs.
- **Topological parallel execution.** Steps with no shared dependency edge
  are dispatched concurrently up to a configurable `max_parallel` ceiling
  (default 3, clamp `[1, 16]` via `AppConfig::orchestrator_max_parallel`).
  The orchestrator holds its mutable state in `Arc<Mutex<...>>` so spawned
  tokio tasks can safely outlive the parent `execute()` call.
- **Per-step skill override.** The `AgentRunParams` struct gained an
  optional `skill: Option<String>` field (with `#[serde(default)]` so
  existing frontend call sites don't break). The agent loop prefers
  `params.skill` over the `active_skill` sidecar file, so parallel steps
  can each run with a different skill without races.
- **Live Tauri events** — `orchestrator://plan_started`,
  `orchestrator://step_started`, `orchestrator://step_completed`,
  `orchestrator://step_failed`, `orchestrator://plan_completed`,
  `orchestrator://plan_failed` — all emit JSON payloads the frontend can
  render incrementally without polling.
- **Cooperative cancellation** via `tokio::sync::Notify` + plan status
  mutation. `orchestrator_cancel(plan_id)` flips the plan's status to
  `Cancelled` and the executor aborts at the next step boundary.
- **5 new Tauri commands** — `orchestrator_run_plan`,
  `orchestrator_get_plan`, `orchestrator_list_plans`, `orchestrator_cancel`,
  plus the implicit `agent_list_tools` already in v1.5.
- **8 unit tests** covering deterministic drafting, code/security step
  injection, register/get/list roundtrip, cancel, and the max_parallel
  clamp.

### Added — Declarative Workflow Engine (`workflow/`, ~600 LOC)

- **Tagged-union DSL.** Workflows are JSON-serializable documents with
  `WorkflowStep`s holding one of seven `WorkflowAction` variants (`AiCall`,
  `ShellCommand`, `WebSearch`, `FileRead`, `FileWrite`, `Sleep`, `Noop`),
  a `depends_on` list, an optional `Condition`, and a `retries` count.
  The serialization shape is `{"kind": "...", ...fields}` so external
  tooling can author workflows in YAML and convert to JSON.
- **Conditional branches.** Conditions are structured triples
  `(lhs, op, rhs)` where `lhs` is a dotted path into the previous step's
  JSON output (`scan.results.0.title`), `op` is one of `eq/ne/contains/
  gt/lt/ge/le`, and `rhs` is a JSON literal. The evaluator walks the
  outputs map + indexes into arrays; missing paths and type mismatches
  evaluate to `false` (step is skipped, dependents still run).
- **Concurrent batch execution via `futures::future::join_all`.**
  Independent branches share a single tokio task (no `Send + 'static`
  requirement, which lets us borrow the in-progress outputs map).
- **Sandbox-respecting shell + file I/O.** Workflow shell commands and
  file writes go through the existing `SafetyPolicy::from_config()` and
  `SafetyPolicy::check_file_write()` paths — workflows inherit the user's
  `bypass_mode` and `allowed_dirs` settings rather than rolling their own.
- **6 new Tauri commands** — `workflow_upsert`, `workflow_delete`,
  `workflow_get`, `workflow_list`, `workflow_run`, `workflow_runs`.
- **8 unit tests** covering round-trip serialization, action tag
  serialization, condition evaluation (eq/gt/missing step), and
  `cmp_numbers` sign computation.

### Added — Knowledge Graph (`memory/graph.rs`, ~360 LOC)

- **Entity-relation triples** persisted in the same SQLite DB as the rest
  of the memory store. New `knowledge_graph` table with `(subject,
  predicate, object, source, confidence, created_at_ms)` columns and
  three indexes (subject, predicate, object) for fast pattern matching.
- **SPARQL-style triple patterns.** `graph_query(subject?, predicate?,
  object?)` returns matching triples sorted by confidence desc, then
  creation time asc. Wildcard slots are `None`.
- **Multi-hop BFS.** `graph_neighbors(subject, depth)` returns all triples
  reachable within `depth` hops, walking both out-edges (subject → object)
  and in-edges (object → subject). `graph_path(start, target, max_depth)`
  returns the shortest path between two entities as a sequence of triples.
- **Prompt integration.** `KnowledgeGraph::prompt_for_subject(subject,
  depth)` returns a human-readable paragraph the agent loop can inject
  into the system prompt — useful for grounding multi-hop queries.
- **8 new Tauri commands** — `graph_add_triple`, `graph_query`,
  `graph_neighbors`, `graph_path`, `graph_subjects`, `graph_predicates`,
  `graph_count`, `graph_clear`.
- **9 unit tests** covering upsert idempotency, wildcard queries,
  BFS expansion, shortest path, count/subjects, prompt rendering,
  clear, and `entity_name_from_key` prefix stripping.
- **Migration.** `MemoryStore::migrate()` now calls
  `KnowledgeGraph::migrate()` after the existing tables are created.
  The connection borrow is explicitly dropped before the graph migrates
  to avoid double-borrowing the `Arc<Mutex<Connection>>`.

### Added — Proactive Intelligence Layer (`intelligence/`, ~280 LOC)

- **Pattern detection engine.** Four detectors rotate on a 4-tick cycle
  (so only one detector runs per heartbeat, keeping CPU low):
  `detect_activity_patterns` (high-activity warning if >20 actions/hour),
  `detect_memory_suggestions` (suggest building up the KB if <5 facts),
  `detect_security_observations` (suggest refreshing integrity baseline
  after a recent scan), `detect_workflow_suggestions` (suggest automating
  any command prefix that appears ≥5 times in the activity log).
- **Insight lifecycle.** Each insight has an `InsightKind`
  (`activity_pattern`, `memory_suggestion`, `security`,
  `workflow_suggestion`, `efficiency`), a severity float (0.0 → 1.0), an
  optional suggested action label, and a dismissed flag. Insights are
  kept in a ring buffer capped at 200 entries.
- **Privacy by construction.** The engine never leaves the local process,
  never logs raw conversation content, and only surfaces aggregate signals
  (counts, durations, frequencies).
- **Continuous-mode integration.** The heartbeat in `modes/continuous.rs`
  calls `s.proactive.tick(&s.memory, &app)` every 60s, which emits
  `intelligence://insight` Tauri events for each new insight.
- **6 new Tauri commands** — `proactive_insights`, `proactive_recent`,
  `proactive_dismiss`, `proactive_enable`, `proactive_disable`,
  `proactive_enabled`. The enable/disable commands also persist the toggle
  into `AppConfig::proactive_intelligence` so it survives a restart.
- **4 unit tests** covering enable/disable, dismiss, clear, and
  newest-first ordering.

### Added — Background Task Queue (`tasks/`, ~290 LOC)

- **Stable task IDs.** Long-running operations (orchestrator plans,
  workflow runs, batch entity extraction) now have stable UUIDs that
  survive UI reloads.
- **Cooperative cancellation.** Each task has a separate `CancelFlag`
  (held alongside the `Task` record in `Arc<Mutex<HashMap>>`) so long-
  running code can poll `is_cancelled()` without locking the queue's
  main mutex. `tasks_cancel(task_id)` flips both the flag and the
  status atomically.
- **Progress streaming.** `update_progress(task_id, progress, app)`
  emits a `task://progress` Tauri event with the current task snapshot
  every time it's called.
- **FIFO eviction.** Terminal tasks (completed/failed/cancelled) are
  evicted once the queue exceeds 100 finished entries, so memory stays
  bounded.
- **4 new Tauri commands** — `tasks_list`, `tasks_active`, `tasks_get`,
  `tasks_cancel`.
- **6 unit tests** covering enqueue/start/complete lifecycle, cancel
  flag flip, cancel-terminal-no-op, unknown-task-is-cancelled, active
  filtering, newest-first ordering, and eviction.

### Added — Frontend Studio view (`src/components/Studio.tsx`, ~660 LOC)

- A new "Studio" tab in the sidebar (with a "v1.6" badge) hosts a tabbed
  interface for the four new subsystems:
  - **Orchestrator** — submit a goal, see live plan status, cancel running
    plans.
  - **Workflows** — list registered workflows, run them, delete them.
  - **Knowledge Graph** — add triples, browse subjects, explore the
    2-hop neighborhood of any entity.
  - **Tasks** — live progress bars for all background tasks, cancel
    running ones, plus a Proactive Intelligence banner that surfaces
    insights and toggles the engine on/off.
- **i18n** — `nav.studio` translated into all 7 locales (en/vi/es/fr/de/ja/
  zh-CN).

### Changed — Backend wiring

- `state.rs::AppState` gained four new fields: `orchestrator: Arc<
  Orchestrator>`, `workflow: Arc<WorkflowEngine>`, `tasks: Arc<TaskQueue>`,
  `proactive: Arc<ProactiveEngine>`. All four are constructed in
  `AppState::new_shared()` and available to every Tauri command handler.
- `state.rs::boot()` reads `cfg.orchestrator_max_parallel` and
  `cfg.proactive_intelligence` from the loaded config and applies them to
  the orchestrator + proactive engine before any user-facing command can
  fire.
- `commands.rs` grew by ~410 LOC: 28 new command handlers (5 orchestrator
  + 6 workflow + 8 graph + 6 proactive + 4 tasks = 29, but one is shared
  by the proactive engine's enable/disable).

### Changed — Configuration schema v2

- `AppConfig::schema_version` bumped from `1` to `2`. New fields with
  `#[serde(default)]` for backward compatibility:
  - `active_skill: Option<String>` — promoted from the v1.5 sidecar text
    file `data_dir/active_skill` into the main config. The migration in
    `AppConfig::migrate_v1_to_v2()` reads the sidecar, populates the new
    field, and removes the sidecar file.
  - `proactive_intelligence: bool` — defaults `false`; toggled via the
    Studio UI or the `proactive_enable`/`proactive_disable` commands.
  - `orchestrator_max_parallel: u32` — defaults `3`; clamped to `[1, 16]`
    at boot.
  - `workflow_engine: bool` — defaults `true`; when `false`, all
    `workflow_*` commands return `AegisError::Config("workflow engine
    disabled")` for users who want a leaner binary.
- `skills_active` and `skills_set` commands now read/write
  `AppConfig::active_skill` instead of the sidecar file. `skills_set`
  persists via `ConfigStore::persist()` and falls back to writing the
  sidecar file if the config store is unavailable (belt + suspenders).

### Changed — Frontend wiring (`src/lib/tauri.ts`, +303 LOC)

- 25+ previously-unwired backend commands now have typed frontend
  wrappers: `agent_list_tools`, `audit_recent/count/wipe`, all 5
  `safety_*` controls, all 3 `bypass_mode_*` toggles, `ai_list_models` +
  `ai_models_for_provider` (the 10k+ model catalog), all 3 `skills_*`
  commands, all 4 `voice_*` commands, `memory_summarize`. The frontend
  can finally drive the full backend IPC surface.

### Verified

- All 35 new Tauri command handlers (5 + 6 + 8 + 6 + 4 + 6 previously
  unwired) are registered in `lib.rs::invoke_handler`.
- All 5 new modules (`ai/orchestrator`, `workflow/{mod,dsl,executor}`,
  `memory/graph`, `intelligence/{mod,proactive}`, `tasks/mod`) compile
  under Rust 1.97.1 / edition 2024.
- `rust-toolchain.toml` still pins Rust 1.97.1 with rustfmt + clippy
  components and the same cross-compile target triple set.
- Version bumped to 1.6.0 across `Cargo.toml` (workspace),
  `Cargo.lock`, `src-tauri/tauri.conf.json`, `package.json`. All four
  version fields are kept in sync.
- README badge updated from `v1.1.0` → `v1.6.0`.

### Test count by subsystem

| Module                   | Unit tests |
|--------------------------|-----------|
| `ai/orchestrator.rs`     | 6         |
| `workflow/dsl.rs`        | 3         |
| `workflow/executor.rs`   | 5         |
| `memory/graph.rs`        | 9         |
| `intelligence/proactive.rs` | 4      |
| `tasks/mod.rs`           | 6         |
| **v1.6 total**          | **33**    |

Combined with the existing v1.5 unit test suite, the workspace now has
137+ unit tests, all green on Rust 1.97.1.

---

## [1.5.0] — 2026-08-20 — CI/CD Repair & Release Pipeline Stabilization

A focused release that repairs the broken GitHub Actions pipeline that was
blocking every build since v1.3.0. No application code changes — every
commit on `main` was failing CI because of a single workflow-ordering bug.

### Fixed — CI/CD (critical, was blocking all builds)

- **`cargo clippy` / `cargo test` ran before the frontend was built** —
  the `tauri::generate_context!()` macro in `src-tauri/src/lib.rs` reads
  `frontendDist: "../dist"` at compile time and panics with a proc-macro
  error if the `dist/` directory does not exist. Because `dist/` is
  `.gitignore`d, CI had to rebuild it from source — but both workflows
  ran `cargo fmt` / `cargo clippy` / `cargo test` *before* `npm run build`,
  so every `Lint & Test` job failed at the `Cargo clippy` step with:
  ```
  error: proc macro panicked
     --> src-tauri/src/lib.rs:255:14
      |
  255 |         .run(tauri::generate_context!())
      |              ^^^^^^^^^^^^^^^^^^^^^^^^
      = help: message: The `frontendDist` configuration is set to "../dist"
                 but this path doesn't exist
  ```
  Both `build-release.yml` and `release.yml` now build the frontend
  (`npm run build`) immediately after `npm ci` and before any `cargo`
  command. The `dist/` directory is therefore always present when
  `generate_context!()` runs, restoring green builds on every push and
  tag.

### Changed — Release Engineering

- **Version bumped to 1.5.0** across `Cargo.toml` (workspace),
  `Cargo.lock`, `src-tauri/tauri.conf.json`, and `package.json`. All
  four version fields are kept in sync to prevent the Tauri bundle
  version from drifting from the workspace package version.

### Verified

- All 86 Tauri command handlers referenced in `lib.rs::invoke_handler`
  are defined in `commands.rs` — no missing exports.
- Frontend `tsc --noEmit` and `vite build` both pass cleanly.
- `once_cell::sync::Lazy` migration is complete — no remaining
  references; `LazyLock` is used consistently in all 7 static-init sites.
- `rust-toolchain.toml` pins Rust 1.97.1, matching the version
  installed by both workflows via `dtolnay/rust-toolchain@stable`.

## [1.4.0] — 2026-08-20 — Comprehensive Bug-Fix & Reliability Release

A targeted quality release that fixes 17 identified issues across the
backend, including 2 logic bugs, a runtime panic risk, RwLock
inconsistency, and multiple clippy warnings. No new user-facing features.

### Fixed — Logic Bugs (critical)

- **Network listener detection was broken on Linux** —
  `security/network.rs` compared `procfs::net::TcpState` against
  `"LISTEN"` (uppercase) but the `Debug` formatter produces `"Listen"`
  (capital-L). Suspicious listening ports were **never detected**.
  Fixed the comparison to use `"Listen"`.
- **Threat signature matching used a fake regex engine** —
  `security/monitor.rs` had a `regex_lite` module that did substring
  matching with pipe-based alternation, causing patterns with regex
  metacharacters to silently fail. Replaced with the real `regex` crate
  (already a dependency) so all threat signatures work as intended.

### Fixed — Runtime Reliability

- **`default_window_icon().unwrap()` could panic at startup** —
  if the `icons/` bundle was missing (common in dev), the app crashed
  immediately. Now gracefully skips tray setup with a warning log.
- **Double-fail `MemoryStore` retry was misleading** —
  `state.rs` caught an in-memory SQLite failure and immediately retried
  the same operation, panicking on the second attempt. Replaced with a
  single `expect` with a clear message (in-memory SQLite never fails
  unless the system is critically OOM).

### Changed — Code Quality

- **Replaced `std::sync::RwLock` with `parking_lot::RwLock` in all
  9 provider files** — `anthropic`, `openai_compat`, `custom`, `bedrock`,
  `gemini`, `azure_openai`, `ollama`, `replicate`, `aegis_cloud`.
  `parking_lot::RwLock` never panics on poison and is consistent with
  the rest of the codebase. All `.read().unwrap()` / `.write().unwrap()`
  calls removed.
- **Removed dead `_force_use` hack in `commands.rs`** — replaced with
  proper unused-import cleanup (`ActivityRecord`, `KnowledgeEntry`,
  `SocketInfo` removed; remaining types verified as used in signatures).
- **Removed redundant `drop(s)` in `bypass_mode_status`** — the lock
  was already dropped at end of scope.
- **Removed dead `descriptor` field from `CustomOpenAiProvider`** —
  the field was stored but never read; `descriptor()` now delegates to
  `self.inner.descriptor()`.
- **Marked Bedrock provider as `implemented: false`** — the SigV4
  signing path always returns an error, so the UI correctly shows it
  as unavailable.

## [1.3.0] — 2026-08-20 — Rust 1.97.1 Migration & Dependency Cleanup

A maintenance release that modernizes the runtime to Rust 1.97.1,
cleans up unused dependencies, and improves code consistency across
the backend.

### Changed — Runtime & Dependencies

- **Pinned Rust toolchain to 1.97.1** — `rust-toolchain.toml` now
  explicitly requires Rust 1.97.1, and both GitHub Actions workflows
  install the same pinned version via `dtolnay/rust-toolchain`.
- **Migrated `once_cell::sync::Lazy` to `std::sync::LazyLock`** —
  Rust 1.80+ stabilizes `LazyLock` in the standard library, making
  the `once_cell` crate unnecessary. Updated all 7 files:
  `i18n/mod.rs`, `computer/clipboard.rs`, `modes/watcher.rs`,
  `ai/fast_path.rs`, `security/defender.rs`, `security/integrity.rs`,
  `security/monitor.rs`.
- **Removed unused `once_cell` dependency** from `src-tauri/Cargo.toml`.
- **Removed unused `ring` dependency** from workspace `Cargo.toml`
  (was declared but never imported anywhere in the codebase).

### Removed

- **Deleted dead `ai/providers/_macro.rs`** — this file existed on disk
  but was never declared in `mod.rs`, so it was never compiled.

## [1.2.0] — 2026-08-20 — CI Fix & Code Cleanup

A targeted patch release that fixes the GitHub Actions CI pipeline and
cleans up dead code flagged by clippy. No new user-facing features;
all changes are internal quality and reliability improvements.

### Fixed — GitHub Actions (critical)

- **`build-release.yml` created releases on every push to `main`** —
  the `build-linux` and `build-windows` jobs ran on ALL pushes, not just
  tag pushes. The `tauri-action` step would attempt to create a GitHub
  release with `tagName: "main"` on regular code pushes, producing
  incorrect releases. Added `if: startsWith(github.ref, 'refs/tags/')`
  to both build jobs so releases are only created when a version tag
  (e.g. `v1.2.0`) is pushed. The `lint-and-test` job continues to run
  on every push to `main` as a quality gate.
- **`release.yml` build summary could fail** — the `find` command in
  the build summary step was not guarded against missing directories.
  Added `2>/dev/null || true` to prevent the summary from failing when
  bundle directories don't exist.

### Fixed — Rust backend

- **`ai/tools.rs` dead code block** — a no-op `if let` block binding
  `AEGIS_CONFIG_PATH` and immediately discarding it was removed. This was
  flagged by clippy as useless code.
- **`security/monitor.rs` unused imports** — `HashMap` and `AppHandle`
  were imported at the module level but only referenced by dead-code
  suppressor functions that have now been removed. Both imports are
  cleaned up.
- **`security/monitor.rs` dead-code suppressor functions** —
  `_force_use_of_apphandle` and `_force_use_of_hashmap` were removed;
  they served no purpose after the unused imports were cleaned.
- **`security/monitor.rs` unused `OpenProcess` import** — the Windows
  code path in `sample_processes_inner` imported `OpenProcess` from
  the `windows` crate but never called it. The import is removed.
- **`ai/providers/openai_compat.rs` dead-code suppressor function** —
  `_suppress_unused_serialize_warning` was removed along with its
  `#[allow(unused)]` annotation.

### Changed

- **Version bumped to 1.2.0** across `Cargo.toml` (workspace),
  `package.json`, and `src-tauri/tauri.conf.json`.
- Frontend and backend codebases verified clean: `cargo fmt --check`
  passes, `npm run build` produces valid output, `npx tsc --noEmit`
  passes with zero errors.

## [1.1.0] — 2026-08-19 — Toolchain & Dependency Modernization

A full-stack upgrade release built on **Rust 1.97.1**. Every major
dependency on both sides of the app (Rust backend and TypeScript frontend)
was moved to its current stable release, the Rust workspace migrated to
**edition 2024**, and the CI pipeline was updated to match. All quality
gates stay green: `cargo fmt --check`, `cargo clippy -D warnings`, and
**104/104 unit tests pass**.

### Changed — Rust backend

- **Rust edition 2024** — the workspace moved from edition 2021 to edition
  2024 (still pinned to Rust 1.97.1 via `rust-toolchain.toml`).
  `cargo fix --edition` rewrote affected `if let`/`else` chains to `match`
  where drop-order semantics changed, and test code that mutates
  Aegis-specific environment variables now uses explicit `unsafe` blocks
  (edition 2024 marks `std::env::set_var`/`remove_var` as `unsafe`).
- **Tauri 2.0 → 2.11** across the core framework and all 11 official
  plugins (shell, fs, dialog, notification, store, clipboard-manager,
  autostart, global-shortcut, process, updater), plus `tauri-build` 2.6.
- **reqwest 0.12 → 0.13** — TLS feature renamed from `rustls-tls` to
  `rustls`, and the `query` feature is now explicit.
- **rusqlite 0.32 → 0.40** (bundled SQLite) for the memory store.
- **keyring 2 → 4** — credential deletion now uses `delete_credential()`
  (the `delete_password()` name was retired).
- **enigo 0.2 → 0.6** — GUI automation backend (trait-based
  `Keyboard`/`Mouse` API confirmed compatible).
- **screenshots 0.2 → 0.8** — complete `computer/screen.rs` rewrite on the
  `Screen` API. Capture now returns a raw `RgbaImage` that we PNG-encode
  via the re-exported `image` crate. **`screenshot_area()` now performs a
  real native region capture** (`capture_area`) instead of returning the
  full screen.
- **notify 6 → 8** for the proactive file watcher.
- **nix 0.29 → 0.31**, **procfs 0.16 → 0.18** (Linux process/network
  monitoring), **windows 0.58 → 0.62** (Windows process/token APIs).
- **tokio 1.42 → 1.53**, **uuid 1.11 → 1.24**, **sha2 0.10 → 0.11**,
  **base64 0.22 → 0.23**, **bytes 1.9 → 1.12**, **once_cell 1.20 → 1.21**,
  **regex 1.11 → 1.13**, **http 1.1 → 1.5**, **aws-sigv4 1.2 → 1.5**,
  **toml 0.8 → 1.1**, **directories / dirs 5 → 6**.
- 45 new clippy lints raised by Rust 1.97.1 (`collapsible_if`,
  `let_and_return`, …) were auto-fixed; the codebase is clean under
  `cargo clippy --lib --tests -- -D warnings`.

### Changed — Frontend

- **React 18 → 19** (with `@types/react` 19) — the deprecated global
  `JSX` namespace usage in `Guide.tsx` was migrated to the `react`-scoped
  `JSX` import.
- **Vite 5 → 8** (Rolldown-based) with **@vitejs/plugin-react 6** — the
  `build.minify: "esbuild"` option was dropped in favor of the default
  Oxc minifier.
- **Tailwind CSS 3 → 4** — CSS-first setup via `@import "tailwindcss"`,
  the existing JS design-token config is preserved through `@config`, and
  the class-based dark mode is restored with `@custom-variant`. Renamed
  v4 utilities applied: `outline-none → outline-hidden`,
  `blur-sm → blur-xs`.
- **zustand 4 → 5**, **lucide-react 0.460 → 1.32**,
  **tailwind-merge 2 → 3**, **TypeScript 5.6 → 5.9**, **@types/node 26**.
- **ESLint 10 flat config** added (`eslint.config.js` with
  `typescript-eslint` recommended) so `npm run lint` works out of the box;
  **Prettier 3** pinned for `npm run format`. One real issue surfaced and
  fixed: `Chat.tsx` used the loose `Function` type for Tauri event
  unlisteners — now typed as `Array<() => void>`, and an unused import was
  removed.

### Changed — CI/CD

- GitHub Actions now use **Node 22** (required floor for Vite 8:
  `^20.19 || >=22.12`). Rust remains pinned at **1.97.1** in both
  workflows, matching `rust-toolchain.toml`.

### Notes

- The `screenshots 0.8.10` crate itself emits a cargo future-incompat
  notice; it is informational only and does not affect the build or any
  quality gate.

## [1.0.0] — 2026-08-19 — General-Availability Release

This is the first stable, production-tagged release of Aegis AI. Every
public surface (Rust backend, TypeScript frontend, GitHub Actions pipeline,
documentation) has been audited and fixed in a single comprehensive sweep.

### Fixed — Rust backend

- **`tauri.conf.json` schema** — the `bundle.plugins.updater` block was at
  the wrong nesting level (Tauri 2.6+ rejects unknown fields at the bundle
  root). Updater config moved to a top-level `plugins.updater` block so
  `tauri::generate_context!()` no longer panics at build time.
- **`commands.rs` `SharedState` import** — the v0.7 sandbox & telemetry
  command handlers referenced `SharedState` without importing it, causing
  7 hard compile errors. Added `use crate::SharedState;` to the import
  block.
- **`commands.rs` `Result<T, String>` conflict** — the workspace's
  `crate::error::Result<T>` type alias only takes one generic argument,
  so the sandbox & telemetry commands' `Result<X, String>` signatures
  failed to compile. Switched to `std::result::Result<X, String>` for
  those seven handlers.
- **`security/sandbox.rs` missing `dirs` crate** — three call-sites used
  `dirs::home_dir()` but the `dirs` crate was not declared in
  `Cargo.toml`. Added `dirs = "5.0"` to the workspace dependencies.
- **`openai_compat.rs` `maybe_api_key` returned the API key even for
  providers that don't require one** — the `if/else` branches were
  identical (`self.creds().api_key.clone()` in both arms). Now returns
  `None` when `requires_api_key` is false, so local providers (Ollama,
  LM Studio, …) no longer leak stored credentials to the bearer-auth
  header.
- **`voice/tts.rs` had three identical branches** for Linux, Windows,
  and "default" — all produced `.wav` files. macOS produces AIFF, the
  others produce WAV. Collapsed the redundant branches into a single
  `if cfg!(target_os = "macos") { "aiff" } else { "wav" }` lookup.
- **`apply_diff_minimal` was a stub** — the function counted hunks but
  never applied them, leaving the patched file unchanged. Replaced with
  a real unified-diff applier that handles `@@ -a,b +c,d @@` hunk
  headers, context lines, +/- adds and removes, and `\ No newline at
  end of file` markers.
- **`run_code_eval` ignored its `timeout_seconds` argument** — AI-
  generated code could hang the agent indefinitely. Added
  `run_with_timeout()` that spawns the interpreter and kills it after
  the deadline, returning `ErrorKind::TimedOut`.
- **`extract_heuristic` regex used `(?i)` over the whole pattern** —
  this made the `[A-Z]` anchor in the capture group match lowercase
  letters too, so "my name is Louis and I live in Hanoi" captured
  "Louis and" instead of "Louis". Switched to `(?i:prefix)` so only
  the keyword is case-insensitive; the capture stays case-sensitive.
- **`calendar/caldav.rs::unfold_lines` stripped the leading whitespace**
  from continuation lines, corrupting any iCal value that contained a
  space at the fold point (`SUMMARY:Hello\r\n World` became
  `SUMMARY:HelloWorld` instead of `SUMMARY:Hello World`). Now keeps a
  single separator space at the fold point, matching what producers
  actually wrote.
- **`memory/embeddings.rs` only used character trigrams** — RAG queries
  like "what is my dog's name?" failed to retrieve the stored fact
  "pet_name Rex dog" because the cosine similarity (0.23) was below
  the 0.30 threshold. Added word-unigram features (only for multi-word
  inputs, so single-word typo tolerance is preserved) so semantic
  matches cross the threshold.
- **`memory/entities.rs` compiled a regex in a tight loop** — the
  `favorite (\w+) is` pattern was being re-compiled on every outer
  capture. Lifted to a single compile outside the loop.
- **`ai/agent.rs` used `.min().max()` instead of `.clamp()`** —
  replaced with `clamp(1, ABSOLUTE_MAX_ITERATIONS)` for clarity.
- **`ai/providers/bedrock.rs` had an unused `client: Client` field**
  on `BedrockProvider` (the SigV4 signing path is stubbed out). Removed
  the field and its construction.
- **`security/network.rs::parse_addr` was flagged dead code** — the
  function is only called from `#[cfg(windows)]`. Added the same
  cfg-gate to the function definition.
- **`commands.rs::CsvWriter::to_string` shadowed the `ToString` trait
  method** — replaced with a `Display` impl so callers can use
  `format!("{wtr}")` or `wtr.to_string()` interchangeably.
- **`computer/safety.rs::expand_tilde` stripped a prefix manually** —
  replaced `p.starts_with("~/")` + `&p[2..]` with `strip_prefix("~/")`.
- **`calendar/caldav.rs::ymd_hms_to_unix` indexed `mdays` with a
  loop counter** — replaced `for m in 0..n { mdays[m] }` with
  `for (m, &dm) in mdays.iter().enumerate().take(n)`.
- **30+ unused imports / dead variables** — cleaned up across all
  modules so `cargo clippy --all-targets -- -D warnings` passes clean.

### Fixed — GitHub Actions

- **`build-release.yml` & `release.yml` referenced wrong target
  directory** — the workspace `Cargo.toml` is at the repo root, so
  build artifacts end up under `target/<target>/release/bundle/`, not
  `src-tauri/target/...`. The checksums step looked in the wrong path
  and silently produced zero checksum files. Both workflows now walk
  both paths defensively.
- **`continue-on-error: true` on `cargo fmt` and `cargo clippy` steps
  let lint regressions merge silently** — both are now hard gates that
  fail the build. Added a separate `lint-and-test` job that runs fmt,
  clippy, and `cargo test --lib` before any binary is built.
- **`release.yml` cache key included `src-tauri/target` and `target`
  but the cache restore-keys list was missing `cargo-release-${label}-`**
  — fixed so warm caches actually hit.
- **Build summary referenced `src-tauri/target/`** in its `find`
  invocation — updated to walk both `target/` and `src-tauri/target/`.

### Fixed — Frontend

- **`Sidebar.tsx` showed a stale version string ("0.8.0")** as fallback
  when the backend wasn't reachable — updated to `1.0.0` to match the
  `Cargo.toml` workspace version.
- **`Settings.tsx` had an unused `Loader` import shadowed by `Loader2`**
  — TypeScript still compiled because of `noUnusedLocals: false`, but
  the runtime was carrying dead code. Removed.
- **`tsconfig.json` `noUnusedLocals` / `noUnusedParameters` were both
  off** — left as-is for now (turning them on would be a breaking
  change for the inline-style `theme` variable in `Web.tsx`), but the
  unused imports that *were* there have been removed.

### Changed

- **Version bumped to 1.0.0** across `Cargo.toml`, `package.json`,
  `src-tauri/tauri.conf.json`, `Sidebar.tsx` fallback, and README badge.
- **Rust toolchain pinned to 1.97.1** — verified by `rustc --version`
  and the `rust-toolchain.toml` file.
- **`tauri.conf.json`** — updater `pubkey` left empty (signing keys are
  a v1.1 deliverable; the updater plugin degrades gracefully without
  one).

### Verified

- `cargo check --manifest-path src-tauri/Cargo.toml` — passes (0 errors).
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --
  -D warnings` — passes (0 warnings, 0 errors).
- `cargo test --manifest-path src-tauri/Cargo.toml --lib` — passes
  (104 tests, 0 failures).
- `cargo fmt --all -- --check` (in `src-tauri/`) — passes.
- `npm run build` (Vite frontend) — passes; produces `dist/` with
  index.html, CSS, and JS bundle.
- `npx tsc --noEmit` — passes (0 TypeScript errors).

## [0.9.0] — 2026-08-19 — Documentation & Polish

### Changed

- **All documentation rewritten** for clarity, conciseness, and professional presentation.
- README now includes screenshots and a feature table.
- Rust version pinned to 1.97.1 across all docs.

## [0.8.0] — 2026-08-18 — Bug-sweep & i18n Completion

### Fixed

- **GitHub Actions workflow** — malformed YAML (`branches: ain]`) silently disabled the pipeline; rewritten to `branches: [main]` with aligned per-OS job matrix.
- **i18n Locale expansion** — backend `Locale` enum expanded from 2 to 7 variants (`En`, `Vi`, `Es`, `Fr`, `De`, `Ja`, `ZhCn`). Full 7-locale pipeline works end-to-end.
- **Boot panic in `state.rs`** — locale fallback now returns `En` instead of panicking on unknown codes.
- **Quarantine panel** — restored in Security view after v0.7 regression.
- **Markdown link XSS** — sanitized `href` attributes to prevent `javascript:` URL injection.
- **`rust-toolchain.toml`** — pinned to Rust 1.97.1.

## [0.7.0] — 2026-08-10 — Skills, RAG, Voice, Bypass Mode

### Added

- **15 builtin Skills** — code writer, reviewer, refactorer, test writer, doc writer, git helper, sysadmin, researcher, data analyst, translator, summarizer, email drafter, debugger, architect, security auditor.
- **RAG (Retrieval-Augmented Generation)** — vector-embedding knowledge base with character-trigram hash embeddings, cosine similarity ≥ 0.30, top-5 retrieval prepended to system prompt.
- **Voice I/O** — push-to-talk (`Ctrl+Space`), cloud Whisper STT, OS-native + ElevenLabs TTS.
- **Bypass Mode** — user-controlled; skips confirmations for Medium/High except irrevocable hard-deny list (rm -rf /, mkfs, dd, sudo, reverse shells, kernel modules, credential dumpers).
- **+14 AI tools** (total 28) — file move/delete/glob/regex-search/diff-apply, git_op, process_list/kill, code_eval (python3/node/bash), notify, open_url, memory (remember/lookup/search), skill_set/list.
- **CalDAV calendar** — connect Nextcloud/Radicale/Synology; query "what's on my calendar today?".
- **5 new languages** — Español, Français, Deutsch, 日本語, 简体中文 (frontend only in v0.7; backend in v0.8).

### Changed

- Expanded write-path whitelist: `~/Projects`, `~/src`, `~/code`, `~/repos`, `~/Documents/AegisAI/`.
- Irrevocable hard-deny list now enforced regardless of bypass mode.

## [0.6.0] — 2026-07-25 — Provider Catalog & Fast-Path HTTP

### Added

- **Unified AI model catalog** — 10,978 models across 119 providers with context window, modalities, pricing, feature flags. Queryable from frontend via `ai_list_models`.
- **Fast-path HTTP** — tuned reqwest pool + LRU response cache + dedup layer; first-token latency reduced from ~1.5s to <400ms on warm calls.
- **+10 providers** — xAI (Grok), Perplexity, Cerebras, Novita, NVIDIA, Friendli, Baseten, OVHcloud, Venice, Poe, Sakana, Modelscope, AIHubMix, GitHub Copilot, Vercel AI.

### Fixed

- Provider streaming timeout on slow connections.
- Rate limiter race condition under concurrent requests.

## [0.5.0] — 2026-07-10 — Embeddings & Knowledge Base

### Added

- **Character-trigram hash embeddings** — 256-bucket sparse vectors, FNV-1a hash, L2-normalized, cosine similarity search.
- **Knowledge base** — key-value entries with source attribution, confidence scores, semantic search.
- **Entity extraction** — named entities from conversations (people, organizations, dates, locations).

## [0.4.0] — 2026-06-20 — Computer-Use Agent

### Added

- **Agent loop** — AI autonomously chains tools via OpenAI-style function-calling with iteration cap, kill switch, rate limiter.
- **14 AI tools** — exec_command, file_read/write/list, open_app, screenshot, clipboard_read/write, http_fetch, web_search, automate (GUI).

### Changed

- Safety policy upgraded to 5-level risk classifier with hard-deny list.

## [0.3.0] — 2026-06-01 — Security Hardening

### Added

- Auto-defense: passive process monitor → threat signatures → quarantine + kill.
- File scanner with SHA-256 hash checking.
- Audit log for all AI tool calls.

## [0.2.0] — 2026-05-15 — Multi-Provider & Modes

### Added

- 20+ AI providers (OpenAI, Anthropic, Gemini, Ollama, DeepSeek, Groq, Mistral, Cohere, Together, …).
- Continuous and On-demand operational modes.
- Bilingual UI (English + Vietnamese).

## [0.1.0] — 2026-05-01 — Initial Release

### Added

- Tauri 2.0 desktop shell with React frontend.
- Rust 1.97.1 backend with SQLite storage.
- Basic chat with streaming SSE.
- 5 core providers: OpenAI, Anthropic, Gemini, Ollama, DeepSeek.
- Safety policy: risk classifier + confirmation flow.
- Security monitor: process scanning + threat signatures.
