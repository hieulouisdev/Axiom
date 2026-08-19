//! Aegis AI Skills System.
//!
//! A "skill" is a named, declarative specialization that the AI agent can
//! load to bias its behavior toward a particular task domain. Each skill
//! provides:
//!
//! - a `system_prompt_fragment` — appended to the agent's system prompt when
//!   the skill is active.
//! - a list of `tool_allowlist` — the tools the skill expects to use (the
//!   agent is *encouraged* to prefer these tools, but not restricted to them
//!   exclusively; the safety policy still applies).
//! - a list of `trigger_examples` — sample user messages that would
//!   naturally invoke this skill (used by the UI's "skill picker" and by
//!   the auto-router in future versions).
//!
//! Skills are NOT the same as providers or tools. They are an orthogonal
//! axis: pick a skill to specialize the agent's persona; pick a provider to
//! choose which LLM answers; pick a tool to choose what the agent can do.
//!
//! ## v0.4 builtin skills
//!
//! | Skill id          | Domain                                  |
//! |-------------------|-----------------------------------------|
//! | `code_writer`     | Writing new code from a spec.           |
//! | `code_reviewer`   | Reviewing existing code.                |
//! | `refactor`        | Refactoring / reorganizing code.        |
//! | `test_writer`     | Generating unit / integration tests.    |
//! | `doc_writer`      | Writing docs (README, ADRs, API docs).  |
//! | `git_helper`      | Git operations and PR workflow.         |
//! | `sysadmin`        | Shell + system administration.          |
//! | `researcher`      | Web research + summarization.           |
//! | `data_analyst`    | CSV / JSON data analysis.               |
//! | `translator`      | Translation between languages.          |
//! | `summarizer`      | Document summarization.                 |
//! | `email_drafter`   | Drafting emails and messages.           |
//! | `debugger`        | Debugging + log analysis.               |
//! | `architect`       | System design + architecture reviews.   |
//! | `security_auditor`| Code security review.                   |
//!
//! The active skill is persisted in `AppConfig::active_skill` and surfaced
//! to the agent loop, which injects the skill's prompt fragment into the
//! system message.

use serde::Serialize;

/// A declarative skill pack.
///
/// NOTE: We deliberately do NOT implement `Deserialize` — skills are a
/// compile-time catalog, never loaded from a config file. Adding Deserialize
/// would require `&'static [&'static str]` to implement it, which it doesn't.
#[derive(Debug, Clone, Serialize)]
pub struct Skill {
    /// Stable unique id, e.g. `code_writer`.
    pub id: &'static str,
    /// Human-readable name, e.g. "Code Writer".
    pub name: &'static str,
    /// One-line description for the skill picker UI.
    pub description: &'static str,
    /// Appended to the agent system prompt when this skill is active.
    pub system_prompt_fragment: &'static str,
    /// Tools this skill prefers (informational; not enforced).
    pub tool_allowlist: &'static [&'static str],
    /// Example user messages that would naturally trigger this skill.
    pub trigger_examples: &'static [&'static str],
}

/// Returns every builtin skill.
pub fn all_skills() -> &'static [Skill] {
    SKILLS
}

/// Look up a skill by id.
pub fn find(id: &str) -> Option<&'static Skill> {
    SKILLS.iter().find(|s| s.id == id)
}

/// Returns the system prompt fragment for the active skill (or empty if none).
pub fn prompt_for(id: Option<&str>) -> &'static str {
    match id {
        Some(id) => find(id).map(|s| s.system_prompt_fragment).unwrap_or(""),
        None => "",
    }
}

static SKILLS: &[Skill] = &[
    Skill {
        id: "code_writer",
        name: "Code Writer",
        description: "Write new code from a natural-language spec. Prefer idiomatic style, complete files, and runnable snippets.",
        system_prompt_fragment: "You are operating in CODE WRITER mode. When the user asks you to write code, produce a complete, runnable solution. Prefer writing the code directly to disk via the `file_write` tool over printing it in chat. Include error handling, comments where non-obvious, and a brief usage example. If the spec is ambiguous, ask one focused clarifying question, then proceed with the most reasonable interpretation.",
        tool_allowlist: &[
            "file_read",
            "file_write",
            "file_list",
            "shell",
            "regex_search",
            "path_glob",
        ],
        trigger_examples: &[
            "Write a Python script that downloads all images from a webpage.",
            "Create a Rust CLI that parses a CSV file.",
            "Scaffold a Next.js project with Tailwind.",
        ],
    },
    Skill {
        id: "code_reviewer",
        name: "Code Reviewer",
        description: "Review an existing codebase. Report bugs, security issues, style violations, and improvement opportunities.",
        system_prompt_fragment: "You are operating in CODE REVIEWER mode. Read the user's code via `file_read` (and `regex_search` for cross-cutting concerns), then produce a structured review: (1) Critical issues (bugs, security, data loss), (2) Major issues (design, correctness in edge cases), (3) Minor issues (style, naming, performance), (4) Praise (things done well). For each issue, cite the file + line and suggest a concrete fix. Do not modify code unless the user explicitly asks.",
        tool_allowlist: &[
            "file_read",
            "file_list",
            "regex_search",
            "path_glob",
            "shell",
        ],
        trigger_examples: &[
            "Review my src/auth module.",
            "Find any SQL injection vulnerabilities in this repo.",
            "What do you think of my refactoring of utils.rs?",
        ],
    },
    Skill {
        id: "refactor",
        name: "Refactor",
        description: "Reorganize existing code without changing behavior. Apply well-known patterns and produce a diff.",
        system_prompt_fragment: "You are operating in REFACTOR mode. Before modifying anything, use `file_read` to understand the existing code and `shell` to run the test suite (if present). Propose the refactor as a numbered list of changes, then apply them via `file_write`. After applying, run tests again to verify behavior is unchanged. If a change is risky, ask for confirmation first.",
        tool_allowlist: &[
            "file_read",
            "file_write",
            "file_list",
            "shell",
            "regex_search",
            "diff_apply",
        ],
        trigger_examples: &[
            "Extract the auth logic into its own module.",
            "Replace the nested if-else with a match.",
            "Convert this class to use composition over inheritance.",
        ],
    },
    Skill {
        id: "test_writer",
        name: "Test Writer",
        description: "Generate unit and integration tests for existing code. Prefer edge-case coverage.",
        system_prompt_fragment: "You are operating in TEST WRITER mode. Read the target code via `file_read`, identify its public API and edge cases, then write tests via `file_write`. Cover: happy path, boundary conditions, error paths, and at least one stress case. Use the project's existing test framework (detect: `cargo test`, `pytest`, `jest`, `go test`, `mvn test`). Run the tests via `shell` after writing them and report pass/fail counts.",
        tool_allowlist: &["file_read", "file_write", "shell", "regex_search"],
        trigger_examples: &[
            "Write unit tests for src/parser.rs.",
            "Add integration tests for the /api/login endpoint.",
            "Generate property-based tests for the sort function.",
        ],
    },
    Skill {
        id: "doc_writer",
        name: "Doc Writer",
        description: "Write README files, ADRs, API docs, and inline doc comments.",
        system_prompt_fragment: "You are operating in DOC WRITER mode. Read the target code via `file_read` and produce documentation that matches the project's existing style. For READMEs, include: project summary, install, usage, configuration, troubleshooting, and contribution sections. For API docs, include: signature, parameters, return value, errors, and a runnable example. Write docs to disk via `file_write`.",
        tool_allowlist: &["file_read", "file_write", "file_list", "regex_search"],
        trigger_examples: &[
            "Write a README.md for this project.",
            "Document the public API of src/auth.rs.",
            "Generate an ADR for the choice of SQLite.",
        ],
    },
    Skill {
        id: "git_helper",
        name: "Git Helper",
        description: "Git operations and PR workflow: branch, commit, push, open PRs, resolve conflicts.",
        system_prompt_fragment: "You are operating in GIT HELPER mode. Use `git_op` and `shell` to perform git operations. Default behavior: stage only files explicitly mentioned by the user, write conventional-commit messages (feat/fix/docs/refactor/test/chore/perf), and never force-push to main/master. If a merge conflict occurs, read the conflicting file, propose a resolution, and ask for confirmation before committing.",
        tool_allowlist: &["git_op", "shell", "file_read", "file_write", "diff_apply"],
        trigger_examples: &[
            "Commit my staged changes with a good message.",
            "Create a PR for the feature/auth branch.",
            "Resolve the merge conflict in src/api.rs.",
        ],
    },
    Skill {
        id: "sysadmin",
        name: "Sysadmin",
        description: "Shell + system administration: manage services, packages, users, files.",
        system_prompt_fragment: "You are operating in SYSADMIN mode. Use `shell`, `process_list`, `process_kill`, `file_read`, and `file_write` to administer the system. Prefer non-destructive commands first (e.g. `systemctl status` before `systemctl restart`). Always show the user what you're about to run before running it. If a command requires root, surface it for confirmation rather than silently using sudo.",
        tool_allowlist: &[
            "shell",
            "process_list",
            "process_kill",
            "file_read",
            "file_write",
            "http_fetch",
        ],
        trigger_examples: &[
            "Why is nginx not starting?",
            "Free up disk space on the root partition.",
            "Show me the top 10 memory-consuming processes.",
        ],
    },
    Skill {
        id: "researcher",
        name: "Researcher",
        description: "Web research + summarization. Use `http_fetch` to read URLs and synthesize findings.",
        system_prompt_fragment: "You are operating in RESEARCHER mode. Use `http_fetch` to retrieve web pages, then synthesize findings into a structured report: (1) Key findings, (2) Sources (with URLs), (3) Open questions. Never fabricate sources — if you can't find an authoritative answer, say so. Prefer official documentation, peer-reviewed papers, and primary sources over blog posts.",
        tool_allowlist: &[
            "http_fetch",
            "web_search",
            "memory_remember",
            "memory_lookup",
        ],
        trigger_examples: &[
            "What are the latest Rust async runtime benchmarks?",
            "Compare Postgres vs MySQL for OLTP workloads.",
            "Find three peer-reviewed papers on RAG retrieval quality.",
        ],
    },
    Skill {
        id: "data_analyst",
        name: "Data Analyst",
        description: "Analyze CSV / JSON / SQLite data. Compute stats, generate summaries, suggest visualizations.",
        system_prompt_fragment: "You are operating in DATA ANALYST mode. Read data files via `file_read`, then use `shell` (with python3 / jq / sqlite3) to compute statistics. Report: (1) Schema overview, (2) Summary statistics per column, (3) Missing values, (4) Top 3 interesting patterns, (5) Suggested visualizations. Never modify the source data; write derived artifacts to ~/Documents/AegisAI/.",
        tool_allowlist: &["file_read", "file_write", "shell", "regex_search"],
        trigger_examples: &[
            "Summarize the sales.csv file.",
            "Find outliers in the user_events.json.",
            "What's the average order value by region?",
        ],
    },
    Skill {
        id: "translator",
        name: "Translator",
        description: "Translate text between any pair of natural languages. Preserve tone, idioms, and formatting.",
        system_prompt_fragment: "You are operating in TRANSLATOR mode. When the user asks for a translation, detect the source language (unless stated), produce a fluent target-language version, and add a brief translator's note for any culturally-specific terms or idioms. Preserve markdown / HTML formatting exactly. If the input is ambiguous, ask one focused question before translating.",
        tool_allowlist: &[
            "clipboard_read",
            "clipboard_write",
            "file_read",
            "file_write",
        ],
        trigger_examples: &[
            "Translate this README to Vietnamese.",
            "Translate 'good morning' to Japanese.",
            "Localize my app's UI strings to Spanish.",
        ],
    },
    Skill {
        id: "summarizer",
        name: "Summarizer",
        description: "Summarize long documents, conversations, or logs into concise briefings.",
        system_prompt_fragment: "You are operating in SUMMARIZER mode. Read the source via `file_read` (or accept it pasted in chat). Produce: (1) A 3-sentence executive summary, (2) Key bullet points (max 7), (3) Action items (if any). For logs, also include: error counts, top error patterns, and a sample stack trace. Never exceed 250 words unless the user explicitly asks for a longer summary.",
        tool_allowlist: &["file_read", "memory_remember"],
        trigger_examples: &[
            "Summarize this 50-page PDF.",
            "Give me the TLDR of today's meeting notes.",
            "Summarize the last 1000 lines of /var/log/syslog.",
        ],
    },
    Skill {
        id: "email_drafter",
        name: "Email Drafter",
        description: "Draft emails, Slack messages, and announcements. Match a chosen tone.",
        system_prompt_fragment: "You are operating in EMAIL DRAFTER mode. Ask the user for: recipient, goal, and tone (formal / casual / persuasive / apologetic). If they didn't specify, infer from context. Draft the message, then offer 1-2 alternative phrasings for any sensitive parts. Never invent facts — if the email needs a number or date the user hasn't given you, leave a [FILL IN: ...] placeholder.",
        tool_allowlist: &["clipboard_write", "memory_lookup"],
        trigger_examples: &[
            "Draft a polite follow-up to the recruiter.",
            "Write a Slack announcement for the v0.4 release.",
            "Apologize to a customer for the outage.",
        ],
    },
    Skill {
        id: "debugger",
        name: "Debugger",
        description: "Diagnose and fix bugs. Read logs, run the failing test, propose a fix.",
        system_prompt_fragment: "You are operating in DEBUGGER mode. Start by reproducing the issue: read the failing test or run the command via `shell`. Capture the error output, then form a hypothesis. Use `regex_search` to find related code, `file_read` to inspect it, and propose a fix. Apply the fix via `file_write`, then re-run the test to confirm. If the fix doesn't work, revert and try a different hypothesis — never pile on changes blindly.",
        tool_allowlist: &[
            "shell",
            "file_read",
            "file_write",
            "regex_search",
            "process_list",
        ],
        trigger_examples: &[
            "Why does my test_auth.py fail on CI but pass locally?",
            "Diagnose the panic in src/router.rs line 142.",
            "The app crashes on startup — help me debug.",
        ],
    },
    Skill {
        id: "architect",
        name: "Architect",
        description: "System design + architecture reviews. Propose patterns, trade-offs, and migration paths.",
        system_prompt_fragment: "You are operating in ARCHITECT mode. When asked to design or review a system, produce: (1) Context (the problem being solved), (2) High-level diagram (described in text), (3) Component breakdown with responsibilities, (4) Data flow, (5) Failure modes + mitigations, (6) Trade-offs considered + why we picked this option. Cite prior art (papers, well-known systems) where relevant. Never recommend a technology without explaining what it's better than and why.",
        tool_allowlist: &["file_read", "file_list", "regex_search"],
        trigger_examples: &[
            "Design a rate limiter for our API gateway.",
            "Should we move from REST to gRPC?",
            "Review our microservices architecture.",
        ],
    },
    Skill {
        id: "security_auditor",
        name: "Security Auditor",
        description: "Code security review: find injection, auth, crypto, and secret-leak issues.",
        system_prompt_fragment: "You are operating in SECURITY AUDITOR mode. Use `regex_search` to scan for known-bad patterns (hardcoded secrets, eval/exec calls, SQL string concat, disabled TLS verification, weak crypto). Use `file_read` to inspect findings in context. Report by severity (Critical / High / Medium / Low / Informational), each with: location, evidence, impact, and a remediation. Never exploit a finding — this is a defensive review.",
        tool_allowlist: &["file_read", "file_list", "regex_search", "shell"],
        trigger_examples: &[
            "Audit my auth module for OWASP Top 10.",
            "Find any hardcoded API keys in the repo.",
            "Is my TLS configuration secure?",
        ],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skills_are_nonempty() {
        assert!(SKILLS.len() >= 12);
    }

    #[test]
    fn skill_ids_are_unique() {
        let mut ids: Vec<_> = SKILLS.iter().map(|s| s.id).collect();
        ids.sort();
        let initial = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), initial, "duplicate skill ids found");
    }

    #[test]
    fn find_known_skill() {
        assert!(find("code_writer").is_some());
        assert!(find("nonexistent_skill").is_none());
    }

    #[test]
    fn prompt_for_returns_fragment() {
        assert!(prompt_for(Some("code_writer")).contains("CODE WRITER"));
        assert_eq!(prompt_for(None), "");
        assert_eq!(prompt_for(Some("nonexistent")), "");
    }
}
