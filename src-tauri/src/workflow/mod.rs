//! v1.6.0 — Declarative Workflow Engine.
//!
//! The workflow engine lets users (and the agent itself) author reusable,
//! version-controlled automation pipelines as data instead of code. A
//! workflow is a directed graph of typed actions (`AiCall`, `ShellCommand`,
//! `WebSearch`, `FileWrite`, `Sleep`) with conditional branches, declarative
//! `depends_on` edges, and per-step retry policy.
//!
//! ## DSL shape (JSON, serializable to/from YAML via `serde_yaml` if added)
//!
//! ```json
//! {
//!   "name": "nightly-security-sweep",
//!   "trigger": "manual",
//!   "steps": [
//!     {
//!       "id": "scan",
//!       "name": "Run YARA scan",
//!       "action": { "kind": "shell_command", "command": "aegis yara scan /home" },
//!       "depends_on": []
//!     },
//!     {
//!       "id": "alert",
//!       "name": "Send alert if infected",
//!       "action": { "kind": "ai_call", "prompt": "Summarize the scan results" },
//!       "depends_on": ["scan"],
//!       "condition": "{{ scan.infected == true }}"
//!     }
//!   ]
//! }
//! ```
//!
//! Condition syntax is intentionally minimal: `{{ lhs op rhs }}` where `lhs`
//! is a step-id-dot-key path into the previous step's JSON result and `op`
//! is one of `==`, `!=`, `contains`, `>`, `<`, `>=`, `<=`. Expressions that
//! fail to parse or refer to missing keys evaluate to `false` (step is
//! skipped). This keeps the DSL safe to evaluate — no arbitrary code exec.

pub mod dsl;
pub mod executor;

pub use dsl::{Condition, Workflow, WorkflowAction, WorkflowStep, WorkflowTrigger};
pub use executor::{WorkflowEngine, WorkflowRunResult, WorkflowRunStatus};
