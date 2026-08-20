//! Workflow executor: topologically sorts steps, evaluates conditions,
//! runs independent branches concurrently, and emits per-step Tauri events
//! so the UI can render live progress.
//!
//! ## Event channels
//!
//! - `workflow://started`    — `{ run_id, workflow_id, name, step_count }`
//! - `workflow://step`       — `{ run_id, step_id, status, output? }`
//! - `workflow://completed`  — `{ run_id, duration_ms, success_count }`
//! - `workflow://failed`     — `{ run_id, step_id, error }`

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::Emitter;

use super::dsl::{CondOp, Workflow, WorkflowAction, WorkflowStep};
use crate::ai::provider::{ChatMessage, ChatRequest};
use crate::computer::commands::exec_command;
use crate::computer::files::{file_read as fs_file_read, file_write as fs_file_write};
use crate::computer::safety::SafetyPolicy;

/// Result of executing a single step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepOutput {
    /// The step id this output belongs to.
    pub step_id: String,
    /// The serialized JSON value of the step's output. Shape depends on the
    /// action: `AiCall` → `{ "response": "..." }`, `ShellCommand` →
    /// `{ "stdout": "...", "stderr": "...", "exit_code": 0 }`, `WebSearch`
    /// → `{ "results": [...] }`, etc.
    pub value: serde_json::Value,
    /// Whether the step ran (false if skipped by a falsey condition).
    pub ran: bool,
}

/// Final aggregate result of running an entire workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRunResult {
    pub run_id: String,
    pub workflow_id: String,
    pub status: WorkflowRunStatus,
    pub outputs: HashMap<String, StepOutput>,
    pub duration_ms: u64,
}

/// High-level run status.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowRunStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
    Cancelled,
}

/// Process-wide workflow registry + runner.
pub struct WorkflowEngine {
    /// All known workflow definitions, keyed by `workflow_id`.
    workflows: Arc<Mutex<HashMap<String, Workflow>>>,
    /// All runs (active and historical), keyed by `run_id`.
    runs: Arc<Mutex<HashMap<String, WorkflowRunResult>>>,
}

impl WorkflowEngine {
    pub fn new() -> Self {
        Self {
            workflows: Arc::new(Mutex::new(HashMap::new())),
            runs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register or replace a workflow definition.
    pub fn upsert(&self, wf: Workflow) -> String {
        let id = wf.id.clone();
        self.workflows.lock().insert(id.clone(), wf);
        id
    }

    /// Remove a workflow.
    pub fn delete(&self, workflow_id: &str) -> bool {
        self.workflows.lock().remove(workflow_id).is_some()
    }

    /// Get a workflow by id.
    pub fn get(&self, workflow_id: &str) -> Option<Workflow> {
        self.workflows.lock().get(workflow_id).cloned()
    }

    /// List all registered workflows.
    pub fn list(&self) -> Vec<Workflow> {
        self.workflows.lock().values().cloned().collect()
    }

    /// List all known runs.
    pub fn runs(&self) -> Vec<WorkflowRunResult> {
        self.runs.lock().values().cloned().collect()
    }

    /// Get a run by id.
    pub fn run(&self, run_id: &str) -> Option<WorkflowRunResult> {
        self.runs.lock().get(run_id).cloned()
    }

    /// Execute a workflow. The workflow must already be registered via
    /// [`Self::upsert`]. Returns the final `WorkflowRunResult`.
    ///
    /// This function is `async` and may take a long time if the workflow
    /// has many AI calls. The caller (a Tauri command) is expected to spawn
    /// it on a background task and emit `workflow://step` events for the
    /// UI to render progress incrementally.
    pub async fn execute(
        self: Arc<Self>,
        workflow_id: String,
        app: &tauri::AppHandle,
        state: crate::SharedState,
    ) -> anyhow::Result<WorkflowRunResult> {
        let wf = self
            .get(&workflow_id)
            .ok_or_else(|| anyhow::anyhow!("workflow not found: {workflow_id}"))?;

        let run_id = uuid::Uuid::new_v4().to_string();
        let start = SystemTime::now();
        let total = wf.steps.len();

        let initial = WorkflowRunResult {
            run_id: run_id.clone(),
            workflow_id: workflow_id.clone(),
            status: WorkflowRunStatus::Running,
            outputs: HashMap::new(),
            duration_ms: 0,
        };
        self.runs.lock().insert(run_id.clone(), initial);

        let _ = app.emit(
            "workflow://started",
            serde_json::json!({
                "run_id": run_id,
                "workflow_id": workflow_id,
                "name": wf.name,
                "step_count": total,
            }),
        );

        // Topologically execute, batch by batch.
        let mut completed: HashSet<String> = HashSet::new();
        let mut failed: HashSet<String> = HashSet::new();
        let mut outputs: HashMap<String, StepOutput> = HashMap::new();
        let mut success_count = 0usize;

        loop {
            let ready: Vec<WorkflowStep> = wf
                .steps
                .iter()
                .filter(|s| {
                    !completed.contains(&s.id)
                        && !failed.contains(&s.id)
                        && s.depends_on.iter().all(|d| completed.contains(d))
                })
                .cloned()
                .collect();

            if ready.is_empty() {
                break;
            }

            // Run the ready batch concurrently via `join_all` (single-task
            // concurrency — no `Send`/`'static` requirement, which keeps
            // the borrow of `outputs` valid). The agent's network calls
            // still progress concurrently because they `.await` internally.
            use futures::future::join_all;

            // Take references to `run_id` and `outputs` OUTSIDE the closure
            // so each async block captures a `Copy` reference (`&str` and
            // `&HashMap`) rather than trying to move the underlying owned
            // values out of the parent FnMut closure.
            let run_id_ref: &str = run_id.as_str();
            let outputs_ref: &HashMap<String, StepOutput> = &outputs;
            let futs = ready.into_iter().map(|step| {
                let app_clone = app.clone();
                let state_clone = state.clone();
                async move {
                    Self::execute_step(&step, &app_clone, run_id_ref, &state_clone, outputs_ref)
                        .await
                }
            });
            let results = join_all(futs).await;

            for out in results {
                let step_id = out.step_id.clone();
                if !out.ran {
                    // Skipped by condition — counts as "completed" for DAG
                    // progression but not as a success.
                    completed.insert(step_id.clone());
                    outputs.insert(step_id.clone(), out.clone());
                    let _ = app.emit(
                        "workflow://step",
                        serde_json::json!({
                            "run_id": run_id,
                            "step_id": step_id,
                            "status": "skipped",
                        }),
                    );
                    continue;
                }

                let is_err = out.value.get("__error__").is_some();
                if is_err {
                    failed.insert(step_id.clone());
                    let _ = app.emit(
                        "workflow://step",
                        serde_json::json!({
                            "run_id": run_id,
                            "step_id": step_id,
                            "status": "failed",
                            "output": out.value,
                        }),
                    );
                    let _ = app.emit(
                        "workflow://failed",
                        serde_json::json!({
                            "run_id": run_id,
                            "step_id": step_id,
                            "error": out.value["__error__"],
                        }),
                    );
                    let duration_ms = SystemTime::now()
                        .duration_since(start)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0);
                    let final_result = WorkflowRunResult {
                        run_id: run_id.clone(),
                        workflow_id: workflow_id.clone(),
                        status: WorkflowRunStatus::Failed,
                        outputs,
                        duration_ms,
                    };
                    self.runs
                        .lock()
                        .insert(run_id.clone(), final_result.clone());
                    return Ok(final_result);
                } else {
                    completed.insert(step_id.clone());
                    success_count += 1;
                    outputs.insert(step_id.clone(), out.clone());
                    let _ = app.emit(
                        "workflow://step",
                        serde_json::json!({
                            "run_id": run_id,
                            "step_id": step_id,
                            "status": "completed",
                            "output": out.value,
                        }),
                    );
                }
            }

            if completed.len() + failed.len() >= total {
                break;
            }
        }

        let duration_ms = SystemTime::now()
            .duration_since(start)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let final_result = WorkflowRunResult {
            run_id: run_id.clone(),
            workflow_id: workflow_id.clone(),
            status: WorkflowRunStatus::Completed,
            outputs,
            duration_ms,
        };

        let _ = app.emit(
            "workflow://completed",
            serde_json::json!({
                "run_id": run_id,
                "duration_ms": duration_ms,
                "success_count": success_count,
            }),
        );

        self.runs
            .lock()
            .insert(run_id.clone(), final_result.clone());
        Ok(final_result)
    }

    /// Execute one step. Static so it can be called from a `join_all`
    /// future without borrowing `&self`.
    async fn execute_step(
        step: &WorkflowStep,
        app: &tauri::AppHandle,
        run_id: &str,
        state: &crate::SharedState,
        outputs: &HashMap<String, StepOutput>,
    ) -> StepOutput {
        // Evaluate condition first.
        if let Some(cond) = &step.condition
            && !Self::eval_condition(cond, outputs)
        {
            return StepOutput {
                step_id: step.id.clone(),
                value: serde_json::json!({ "skipped": true }),
                ran: false,
            };
        }

        let _ = app.emit(
            "workflow://step",
            serde_json::json!({
                "run_id": run_id,
                "step_id": step.id,
                "status": "running",
            }),
        );

        // Dispatch on action kind. Each arm returns a JSON value that will
        // be exposed to downstream steps via `outputs[step_id].value`.
        let value = match &step.action {
            WorkflowAction::AiCall { prompt, model } => {
                Self::run_ai_call(prompt, model.clone(), state).await
            }
            WorkflowAction::ShellCommand { command } => Self::run_shell(command, state),
            WorkflowAction::WebSearch { query } => Self::run_web_search(query).await,
            WorkflowAction::FileRead { path } => Self::run_file_read(path),
            WorkflowAction::FileWrite { path, content } => {
                Self::run_file_write(path, content, state)
            }
            WorkflowAction::Sleep { ms } => {
                tokio::time::sleep(Duration::from_millis(*ms)).await;
                serde_json::json!({ "slept_ms": ms })
            }
            WorkflowAction::Noop { value } => serde_json::json!({ "noop": value }),
        };

        StepOutput {
            step_id: step.id.clone(),
            value,
            ran: true,
        }
    }

    /// Evaluate a `Condition` against the current outputs map. Unknown
    /// paths / type mismatches evaluate to `false` (step is skipped).
    fn eval_condition(cond: &super::dsl::Condition, outputs: &HashMap<String, StepOutput>) -> bool {
        // Walk the dotted path `step_id.field1.field2...` into the outputs map.
        let parts: Vec<&str> = cond.lhs.split('.').collect();
        if parts.is_empty() {
            return false;
        }
        let step_id = parts[0];
        let Some(out) = outputs.get(step_id) else {
            return false;
        };
        let mut cur = &out.value;
        for p in &parts[1..] {
            // Index into array if numeric.
            if let Ok(idx) = p.parse::<usize>()
                && let Some(arr) = cur.as_array()
                && let Some(v) = arr.get(idx)
            {
                cur = v;
                continue;
            }
            if let Some(v) = cur.get(p) {
                cur = v;
            } else {
                return false;
            }
        }

        let lhs = cur;
        let rhs = &cond.rhs;
        match cond.op {
            CondOp::Eq => lhs == rhs,
            CondOp::Ne => lhs != rhs,
            CondOp::Contains => {
                // String contains substring.
                if let (Some(l), Some(r)) = (lhs.as_str(), rhs.as_str()) {
                    l.contains(r)
                } else {
                    false
                }
            }
            CondOp::Gt => cmp_numbers(lhs, rhs).map(|o| o > 0).unwrap_or(false),
            CondOp::Lt => cmp_numbers(lhs, rhs).map(|o| o < 0).unwrap_or(false),
            CondOp::Ge => cmp_numbers(lhs, rhs).map(|o| o >= 0).unwrap_or(false),
            CondOp::Le => cmp_numbers(lhs, rhs).map(|o| o <= 0).unwrap_or(false),
        }
    }

    // ---- per-action runners ----

    async fn run_ai_call(
        prompt: &str,
        model: Option<String>,
        state: &crate::SharedState,
    ) -> serde_json::Value {
        // Snapshot router + providers.
        let (router, providers) = {
            let s = state.lock();
            (s.router.clone(), s.providers.lock().clone())
        };
        let req = ChatRequest {
            messages: vec![
                ChatMessage::system(
                    "You are part of an Aegis AI workflow. Answer concisely. Your answer will \
                     be consumed by downstream steps as JSON.",
                ),
                ChatMessage::user(prompt.to_string()),
            ],
            model,
            temperature: Some(0.2),
            max_tokens: Some(1024),
            top_p: None,
            stop: vec![],
            extra: Default::default(),
        };
        match router.chat(&providers, req).await {
            Ok(resp) => serde_json::json!({ "response": resp.message.content }),
            Err(e) => serde_json::json!({ "__error__": format!("ai call failed: {e}") }),
        }
    }

    fn run_shell(command: &str, state: &crate::SharedState) -> serde_json::Value {
        // Workflow shell commands go through the same safety policy as the
        // agent loop — the user explicitly trusts workflows they've authored,
        // but we still hard-deny irrevocably-destructive commands and surface
        // confirmation prompts for medium/high-risk actions.
        let policy = {
            let s = state.lock();
            SafetyPolicy::from_config(&s.config.read())
        };
        match exec_command(&policy, command) {
            Ok(r) => serde_json::json!({
                "stdout": r.stdout,
                "stderr": r.stderr,
                "exit_code": r.exit_code,
                "duration_ms": r.duration_ms,
            }),
            Err(e) => serde_json::json!({ "__error__": format!("shell failed: {e}") }),
        }
    }

    async fn run_web_search(query: &str) -> serde_json::Value {
        match crate::ai::web::web_search(query).await {
            Ok(results) => serde_json::json!({ "results": results }),
            Err(e) => serde_json::json!({ "__error__": format!("web_search failed: {e}") }),
        }
    }

    fn run_file_read(path: &str) -> serde_json::Value {
        match fs_file_read(path) {
            Ok(r) => serde_json::json!({
                "path": r.path,
                "content": r.content,
                "truncated": r.truncated,
            }),
            Err(e) => serde_json::json!({ "__error__": format!("file_read failed: {e}") }),
        }
    }

    fn run_file_write(path: &str, content: &str, state: &crate::SharedState) -> serde_json::Value {
        // File writes inside workflows go through the sandbox policy so the
        // user's `bypass_mode` and `allowed_dirs` settings apply uniformly.
        let policy = {
            let s = state.lock();
            SafetyPolicy::from_config(&s.config.read())
        };
        match fs_file_write(&policy, path, content) {
            Ok(_) => serde_json::json!({ "path": path, "bytes_written": content.len() }),
            Err(e) => serde_json::json!({ "__error__": format!("file_write failed: {e}") }),
        }
    }
}

impl Default for WorkflowEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn cmp_numbers(l: &serde_json::Value, r: &serde_json::Value) -> Option<i64> {
    // Try as f64 for both. Returns the sign of (lhs - rhs) so callers can
    // implement >, <, >=, <= by mapping the sign to a boolean.
    let lf = l.as_f64();
    let rf = r.as_f64();
    if let (Some(a), Some(b)) = (lf, rf) {
        return Some(if (a - b).abs() < f64::EPSILON {
            0
        } else if a > b {
            1
        } else {
            -1
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::dsl::{Condition, WorkflowTrigger};

    fn make_engine() -> WorkflowEngine {
        WorkflowEngine::new()
    }

    #[test]
    fn upsert_and_get() {
        let e = make_engine();
        let wf = Workflow {
            id: "w1".into(),
            name: "W1".into(),
            description: None,
            trigger: WorkflowTrigger::Manual,
            steps: vec![],
            tags: vec![],
        };
        let id = e.upsert(wf);
        assert_eq!(id, "w1");
        assert!(e.get("w1").is_some());
        assert_eq!(e.list().len(), 1);
        assert!(e.delete("w1"));
        assert!(e.get("w1").is_none());
    }

    #[test]
    fn eval_condition_eq_string() {
        let mut outputs: HashMap<String, StepOutput> = HashMap::new();
        outputs.insert(
            "s1".into(),
            StepOutput {
                step_id: "s1".into(),
                value: serde_json::json!({ "results": [{ "title": "hello" }] }),
                ran: true,
            },
        );
        let cond = Condition {
            lhs: "s1.results.0.title".into(),
            op: CondOp::Eq,
            rhs: serde_json::Value::String("hello".into()),
        };
        assert!(WorkflowEngine::eval_condition(&cond, &outputs));
    }

    #[test]
    fn eval_condition_missing_step_is_false() {
        let outputs: HashMap<String, StepOutput> = HashMap::new();
        let cond = Condition {
            lhs: "missing.x".into(),
            op: CondOp::Eq,
            rhs: serde_json::Value::Bool(true),
        };
        assert!(!WorkflowEngine::eval_condition(&cond, &outputs));
    }

    #[test]
    fn eval_condition_gt_numbers() {
        let mut outputs: HashMap<String, StepOutput> = HashMap::new();
        outputs.insert(
            "s1".into(),
            StepOutput {
                step_id: "s1".into(),
                value: serde_json::json!({ "exit_code": 0 }),
                ran: true,
            },
        );
        let cond = Condition {
            lhs: "s1.exit_code".into(),
            op: CondOp::Ge,
            rhs: serde_json::Value::Number(0.into()),
        };
        assert!(WorkflowEngine::eval_condition(&cond, &outputs));

        let cond2 = Condition {
            lhs: "s1.exit_code".into(),
            op: CondOp::Gt,
            rhs: serde_json::Value::Number(0.into()),
        };
        assert!(!WorkflowEngine::eval_condition(&cond2, &outputs));
    }

    #[test]
    fn cmp_numbers_signs() {
        let one = serde_json::Value::Number(1.into());
        let zero = serde_json::Value::Number(0.into());
        assert_eq!(cmp_numbers(&one, &zero), Some(1));
        assert_eq!(cmp_numbers(&zero, &one), Some(-1));
        assert_eq!(cmp_numbers(&one, &one), Some(0));
    }
}
