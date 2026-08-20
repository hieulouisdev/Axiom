//! Workflow DSL data structures.

use serde::{Deserialize, Serialize};

/// A reusable, declarative workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// Unique identifier (e.g. `nightly-security-sweep`).
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Optional description for the editor UI.
    #[serde(default)]
    pub description: Option<String>,
    /// What kicks off this workflow. The runtime currently supports
    /// `Manual` (called from the UI or agent); `Cron` and `Event` are
    /// declared but deferred to v1.7 (the persistence + UI ship first).
    pub trigger: WorkflowTrigger,
    /// Ordered list of steps. Execution order is determined by `depends_on`,
    /// not by list position, so users can author steps in any order.
    pub steps: Vec<WorkflowStep>,
    /// Optional tags for the gallery / picker UI.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// When a workflow runs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowTrigger {
    /// User clicks "Run" or the agent calls `workflow_run`.
    Manual,
    /// Cron-style schedule. The cron expression is stored in
    /// `Workflow::description` for now (a dedicated `cron` field will land
    /// in v1.7 when we ship the scheduler).
    Cron,
    /// Triggered by an event bus event (e.g. `security.threat.detected`).
    Event,
}

/// One step in a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    /// Stable id within the workflow (e.g. `scan`, `alert`).
    pub id: String,
    /// Human-readable label.
    pub name: String,
    /// What this step does.
    pub action: WorkflowAction,
    /// IDs of steps that must complete before this one. The executor runs
    /// independent branches concurrently up to the configured parallelism
    /// ceiling.
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Optional condition expression (`{{ lhs op rhs }}`). If it evaluates
    /// to `false`, the step is skipped but its dependents still run.
    #[serde(default)]
    pub condition: Option<Condition>,
    /// Number of retry attempts on failure (0 = no retries, 3 = try up to
    /// 4 times total). Default 0.
    #[serde(default)]
    pub retries: u32,
}

/// Typed action a step performs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkflowAction {
    /// Call the active AI provider with a prompt. The provider's response
    /// text is exposed to subsequent steps as `{step_id}.response`.
    AiCall {
        prompt: String,
        #[serde(default)]
        model: Option<String>,
    },
    /// Run a shell command (gated by the safety policy).
    ShellCommand { command: String },
    /// Search the web and expose up to 8 results as `{step_id}.results`.
    WebSearch { query: String },
    /// Read a file (best-effort; bounded to 1 MB).
    FileRead { path: String },
    /// Write a file (gated by the sandbox policy). Parent directories are
    /// created automatically.
    FileWrite { path: String, content: String },
    /// Sleep for a number of milliseconds.
    Sleep { ms: u64 },
    /// Record a value into the workflow's outputs without doing anything.
    /// Useful for debugging the DAG.
    Noop { value: String },
}

/// A condition expression. The serialized form is `{{ lhs op rhs }}` but we
/// store it as a structured triple so we don't need a real expression parser.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    /// Step-relative path into the previous step's JSON result, e.g.
    /// `scan.infected` or `scan.results.0.title`.
    pub lhs: String,
    /// Comparison operator.
    pub op: CondOp,
    /// Right-hand-side literal (string, number, bool, null).
    pub rhs: serde_json::Value,
}

/// Operators supported by [`Condition`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CondOp {
    Eq,
    Ne,
    Contains,
    Gt,
    Lt,
    Ge,
    Le,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_roundtrip_json() {
        let wf = Workflow {
            id: "test-wf".into(),
            name: "Test workflow".into(),
            description: Some("For unit tests".into()),
            trigger: WorkflowTrigger::Manual,
            steps: vec![WorkflowStep {
                id: "s1".into(),
                name: "Say hi".into(),
                action: WorkflowAction::AiCall {
                    prompt: "say hello".into(),
                    model: None,
                },
                depends_on: vec![],
                condition: None,
                retries: 0,
            }],
            tags: vec!["test".into()],
        };

        let json = serde_json::to_string(&wf).expect("serialize");
        let back: Workflow = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.id, wf.id);
        assert_eq!(back.steps.len(), 1);
        assert!(matches!(
            back.steps[0].action,
            WorkflowAction::AiCall { .. }
        ));
    }

    #[test]
    fn action_tag_serialization() {
        let a = WorkflowAction::ShellCommand {
            command: "echo hi".into(),
        };
        let v = serde_json::to_value(&a).unwrap();
        assert_eq!(v["kind"], "shell_command");
        assert_eq!(v["command"], "echo hi");
    }

    #[test]
    fn condition_deserialize_typed() {
        let cond = Condition {
            lhs: "scan.infected".into(),
            op: CondOp::Eq,
            rhs: serde_json::Value::Bool(true),
        };
        let v = serde_json::to_value(&cond).unwrap();
        assert_eq!(v["lhs"], "scan.infected");
        assert_eq!(v["op"], "eq");
        assert_eq!(v["rhs"], true);
    }
}
