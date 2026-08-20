//! v1.6.0 — Multi-Agent Orchestrator (Planner → Executor → Critic).
//!
//! The orchestrator is the crown jewel of Aegis AI v1.6.0. It decomposes a
//! high-level user goal into a directed-acyclic graph (DAG) of plan steps,
//! each tagged with the skill that should handle it and the dependencies it
//! must wait on. Steps that share no dependency edge can be dispatched in
//! parallel up to a configurable `max_parallel` ceiling.
//!
//! ## Architecture
//!
//! The orchestrator is intentionally model-light at the planning layer: it
//! uses a deterministic keyword/expansion pass to draft the DAG, then asks
//! the active AI provider to refine the plan via a constrained
//! `Plan -> Plan` transformation. This avoids a hard dependency on the AI
//! being online at planning time — if the AI is offline, the deterministic
//! draft still runs.
//!
//! ## Event channels (emitted via the Tauri `Emitter` trait)
//!
//! - `orchestrator://plan_started`    — `{ plan_id, goal, step_count }`
//! - `orchestrator://step_started`    — `{ plan_id, step_id, description }`
//! - `orchestrator://step_completed`  — `{ plan_id, step_id, result }`
//! - `orchestrator://step_failed`     — `{ plan_id, step_id, error }`
//! - `orchestrator://plan_completed`  — `{ plan_id, duration_ms, success_count }`
//! - `orchestrator://plan_failed`     — `{ plan_id, error }`
//!
//! The event payload is a JSON `serde_json::Value` so the frontend can render
//! progress incrementally without polling.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::Emitter;
use tokio::sync::Notify;

use crate::ai::provider::{ChatMessage, ChatRequest};
use crate::ai::router::AiRouter;
use crate::ai::skills;

/// Maximum number of parallel in-flight steps even if the DAG would allow more.
pub const DEFAULT_MAX_PARALLEL: usize = 3;

/// A single step in an orchestration plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    /// Stable identifier (e.g. `s1`, `s2`, ...).
    pub id: String,
    /// Human-readable description of what this step should accomplish.
    pub description: String,
    /// Skill id from [`crate::ai::skills`] that should be active during this step.
    /// `None` means "use whatever the user has configured".
    pub skill: Option<String>,
    /// IDs of steps that must complete before this one can start.
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Hint to the executor that this step is parallelizable with siblings.
    #[serde(default = "default_true")]
    pub parallelizable: bool,
}

fn default_true() -> bool {
    true
}

/// Top-level orchestration plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub goal: String,
    pub steps: Vec<PlanStep>,
    pub status: PlanStatus,
    pub created_ms: u64,
    pub updated_ms: u64,
    /// Per-step results keyed by `step_id`.
    #[serde(default)]
    pub results: HashMap<String, StepResult>,
}

/// Execution status of a plan or step.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PlanStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// The result of executing a single step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_id: String,
    pub status: PlanStatus,
    pub output: Option<String>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

/// Process-wide orchestrator state.
///
/// All mutable state is held in `Arc<Mutex<…>>` so that the executor can
/// clone the Arcs into spawned tokio tasks (the agent loop is I/O-bound, so
/// running multiple steps concurrently yields real wall-clock speedups even
/// on a single CPU core).
pub struct Orchestrator {
    /// All known plans, keyed by `plan_id`.
    plans: Arc<Mutex<HashMap<String, Plan>>>,
    /// Per-plan cancellation notifiers. Sending `notify_one()` cancels the
    /// plan at the next step boundary.
    cancels: Arc<Mutex<HashMap<String, Arc<Notify>>>>,
    /// Maximum number of steps a single plan may declare (defensive cap).
    max_steps: usize,
    /// Maximum number of steps that may be executing in parallel.
    max_parallel: AtomicUsize,
}

impl Orchestrator {
    pub fn new() -> Self {
        Self {
            plans: Arc::new(Mutex::new(HashMap::new())),
            cancels: Arc::new(Mutex::new(HashMap::new())),
            max_steps: 32,
            max_parallel: AtomicUsize::new(DEFAULT_MAX_PARALLEL),
        }
    }

    /// Override the parallelism ceiling (called from `state.rs::boot` based
    /// on `AppConfig::orchestrator_max_parallel`).
    pub fn set_max_parallel(&self, n: usize) {
        let n = n.clamp(1, 16);
        self.max_parallel.store(n, Ordering::Relaxed);
    }

    /// Draft a deterministic plan for a goal. This is the fallback planner
    /// used when no AI provider is available; the AI is free to refine it.
    pub fn draft_plan(&self, goal: &str) -> Plan {
        let steps = self.deterministic_steps(goal);
        let now = now_ms();
        Plan {
            id: uuid::Uuid::new_v4().to_string(),
            goal: goal.to_string(),
            steps,
            status: PlanStatus::Pending,
            created_ms: now,
            updated_ms: now,
            results: HashMap::new(),
        }
    }

    /// Heuristic step decomposition. This will be refined by the AI in
    /// [`Self::refine_plan_with_ai`].
    #[allow(unused_assignments)]
    fn deterministic_steps(&self, goal: &str) -> Vec<PlanStep> {
        let g = goal.to_lowercase();
        let mut steps: Vec<PlanStep> = Vec::new();
        let mut idx = 1usize;
        let mut prev_id = String::new();

        // Every plan starts with research/understanding.
        steps.push(PlanStep {
            id: format!("s{idx}"),
            description: format!("Research and understand: {}", goal),
            skill: Some("researcher".into()),
            depends_on: vec![],
            parallelizable: false,
        });
        prev_id = format!("s{idx}");
        idx += 1;

        // If the goal mentions code, add a code-writing step.
        if g.contains("code") || g.contains("implement") || g.contains("build") {
            steps.push(PlanStep {
                id: format!("s{idx}"),
                description: "Implement the code solution".into(),
                skill: Some("code_writer".into()),
                depends_on: vec![prev_id.clone()],
                parallelizable: false,
            });
            prev_id = format!("s{idx}");
            idx += 1;

            steps.push(PlanStep {
                id: format!("s{idx}"),
                description: "Review the implementation for correctness".into(),
                skill: Some("code_reviewer".into()),
                depends_on: vec![prev_id.clone()],
                parallelizable: false,
            });
            prev_id = format!("s{idx}");
            idx += 1;
        }

        // If the goal mentions security, add an audit step.
        if g.contains("security") || g.contains("audit") || g.contains("vulnerab") {
            steps.push(PlanStep {
                id: format!("s{idx}"),
                description: "Audit for security vulnerabilities".into(),
                skill: Some("security_auditor".into()),
                depends_on: vec![prev_id.clone()],
                parallelizable: true,
            });
            prev_id = format!("s{idx}");
            idx += 1;
        }

        // If the goal mentions docs / explain, add a doc step.
        if g.contains("document") || g.contains("explain") || g.contains("write up") {
            steps.push(PlanStep {
                id: format!("s{idx}"),
                description: "Produce the documentation".into(),
                skill: Some("doc_writer".into()),
                depends_on: vec![prev_id.clone()],
                parallelizable: true,
            });
            prev_id = format!("s{idx}");
            idx += 1;
        }

        // If the goal mentions test, add a test step.
        if g.contains("test") || g.contains("verify") || g.contains("tdd") {
            steps.push(PlanStep {
                id: format!("s{idx}"),
                description: "Write tests for the implementation".into(),
                skill: Some("test_writer".into()),
                depends_on: vec![prev_id.clone()],
                parallelizable: false,
            });
            prev_id = format!("s{idx}");
            idx += 1;
        }

        // Every plan ends with a summarization / synthesis step.
        steps.push(PlanStep {
            id: format!("s{idx}"),
            description: "Synthesize and present the final answer".into(),
            skill: Some("summarizer".into()),
            depends_on: vec![prev_id],
            parallelizable: false,
        });

        // Defensive cap.
        if steps.len() > self.max_steps {
            steps.truncate(self.max_steps);
        }
        steps
    }

    /// Ask the active AI provider to refine a deterministic draft plan.
    /// Falls back to the draft unchanged if the AI is unreachable.
    pub async fn refine_plan_with_ai(
        &self,
        draft: Plan,
        router: &AiRouter,
        providers: &crate::ai::provider::ProviderRegistry,
    ) -> Plan {
        let plan_json = serde_json::to_string(&draft.steps).unwrap_or_else(|_| "[]".into());
        let system = "You are Aegis AI's planning module. Refine the given plan steps to be \
                      more accurate, complete, and minimal. Return ONLY a JSON array of \
                      objects with fields: id, description, skill, depends_on, parallelizable. \
                      Do not include any other text.";
        let user = format!("Goal: {}\n\nDraft steps (JSON):\n{}", draft.goal, plan_json);

        let req = ChatRequest {
            messages: vec![ChatMessage::system(system), ChatMessage::user(user)],
            model: None,
            temperature: Some(0.2),
            max_tokens: Some(2048),
            top_p: None,
            stop: vec![],
            extra: Default::default(),
        };

        match router.chat(providers, req).await {
            Ok(resp) => {
                let content = resp.message.content;
                if let Ok(refined) = serde_json::from_str::<Vec<PlanStep>>(&content)
                    && !refined.is_empty()
                    && refined.len() <= self.max_steps
                {
                    let mut p = draft;
                    p.steps = refined;
                    return p;
                }
                draft
            }
            Err(_) => draft, // AI offline — use deterministic draft.
        }
    }

    /// Persist a plan and register a cancellation handle.
    pub fn register(&self, plan: Plan) -> String {
        let id = plan.id.clone();
        let notify = Arc::new(Notify::new());
        self.cancels.lock().insert(id.clone(), notify);
        self.plans.lock().insert(id.clone(), plan);
        id
    }

    /// Get a snapshot of a plan.
    pub fn get(&self, plan_id: &str) -> Option<Plan> {
        self.plans.lock().get(plan_id).cloned()
    }

    /// List all known plans.
    pub fn list(&self) -> Vec<Plan> {
        self.plans.lock().values().cloned().collect()
    }

    /// Cancel a running plan. Returns `true` if the plan was running.
    pub fn cancel(&self, plan_id: &str) -> bool {
        if let Some(notify) = self.cancels.lock().get(plan_id).cloned() {
            notify.notify_one();
            if let Some(p) = self.plans.lock().get_mut(plan_id) {
                p.status = PlanStatus::Cancelled;
                p.updated_ms = now_ms();
            }
            true
        } else {
            false
        }
    }

    /// Topologically execute the plan, dispatching independent steps in
    /// parallel up to `max_parallel`. Each step runs the agent loop with the
    /// step's skill active and the step description as the user message.
    ///
    /// The plan is mutated in-place inside `self.plans`. Tauri events are
    /// emitted at each transition.
    pub async fn execute(
        self: Arc<Self>,
        plan_id: String,
        app: &tauri::AppHandle,
        state: crate::SharedState,
    ) -> anyhow::Result<()> {
        // Snapshot the plan we're going to execute.
        let plan = self
            .get(&plan_id)
            .ok_or_else(|| anyhow::anyhow!("plan not found: {plan_id}"))?;
        let goal = plan.goal.clone();
        let steps = plan.steps.clone();
        let total = steps.len();

        // Mark plan as running.
        {
            let mut plans = self.plans.lock();
            if let Some(p) = plans.get_mut(&plan_id) {
                p.status = PlanStatus::Running;
                p.updated_ms = now_ms();
            }
        }
        let _ = app.emit(
            "orchestrator://plan_started",
            serde_json::json!({
                "plan_id": plan_id,
                "goal": goal,
                "step_count": total,
            }),
        );

        // Build dependency graph and execute in topological batches.
        let mut completed: HashSet<String> = HashSet::new();
        let mut failed: HashSet<String> = HashSet::new();
        let mut success_count = 0usize;
        let start = SystemTime::now();
        let max_parallel = self.max_parallel.load(Ordering::Relaxed);

        loop {
            // Check for cancellation.
            let cancelled = self
                .plans
                .lock()
                .get(&plan_id)
                .map(|p| p.status == PlanStatus::Cancelled)
                .unwrap_or(true);
            if cancelled {
                let _ = app.emit(
                    "orchestrator://plan_failed",
                    serde_json::json!({
                        "plan_id": plan_id,
                        "error": "cancelled by user",
                    }),
                );
                return Ok(());
            }

            // Find the next batch of ready steps.
            let ready: Vec<PlanStep> = steps
                .iter()
                .filter(|s| {
                    !completed.contains(&s.id)
                        && !failed.contains(&s.id)
                        && s.depends_on.iter().all(|d| completed.contains(d))
                })
                .take(max_parallel)
                .cloned()
                .collect();

            if ready.is_empty() {
                break;
            }

            // Execute the ready batch concurrently. We use `tokio::spawn`
            // for true parallelism; the agent loop is I/O-bound (network
            // calls to AI providers) so multiple steps make real progress
            // at the same time. Each task holds an `Arc<Self>` so it can
            // safely outlive the parent `execute` call.
            let mut tasks = Vec::with_capacity(ready.len());
            for step in ready {
                let orch = self.clone();
                let app_clone = app.clone();
                let state_clone = state.clone();
                let plan_id_clone = plan_id.clone();
                tasks.push(tokio::spawn(async move {
                    orch.execute_step(plan_id_clone, step, &app_clone, state_clone)
                        .await
                }));
            }

            // Await all parallel tasks.
            for task in tasks {
                match task.await {
                    Ok(Ok(step_result)) => {
                        if step_result.status == PlanStatus::Completed {
                            completed.insert(step_result.step_id.clone());
                            success_count += 1;
                        } else {
                            failed.insert(step_result.step_id.clone());
                            let _ = app.emit(
                                "orchestrator://step_failed",
                                serde_json::json!({
                                    "plan_id": plan_id,
                                    "step_id": step_result.step_id,
                                    "error": step_result.error.clone().unwrap_or_default(),
                                }),
                            );
                        }
                    }
                    Ok(Err(e)) => {
                        tracing::error!("orchestrator step failed: {e:#}");
                        let _ = app.emit(
                            "orchestrator://plan_failed",
                            serde_json::json!({
                                "plan_id": plan_id,
                                "error": format!("{e:#}"),
                            }),
                        );
                        let mut plans = self.plans.lock();
                        if let Some(p) = plans.get_mut(&plan_id) {
                            p.status = PlanStatus::Failed;
                            p.updated_ms = now_ms();
                        }
                        return Err(e);
                    }
                    Err(join_err) => {
                        tracing::error!("orchestrator task join failed: {join_err}");
                        let mut plans = self.plans.lock();
                        if let Some(p) = plans.get_mut(&plan_id) {
                            p.status = PlanStatus::Failed;
                            p.updated_ms = now_ms();
                        }
                        return Err(anyhow::anyhow!("task join failed: {join_err}"));
                    }
                }
            }

            // Termination check: if no steps were ready this iteration
            // (because of unsatisfiable deps), break to avoid infinite loop.
            if completed.len() + failed.len() >= total {
                break;
            }
        }

        let duration_ms = SystemTime::now()
            .duration_since(start)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let final_status = if failed.is_empty() {
            PlanStatus::Completed
        } else {
            PlanStatus::Failed
        };

        {
            let mut plans = self.plans.lock();
            if let Some(p) = plans.get_mut(&plan_id) {
                p.status = final_status;
                p.updated_ms = now_ms();
            }
        }

        let _ = app.emit(
            "orchestrator://plan_completed",
            serde_json::json!({
                "plan_id": plan_id,
                "duration_ms": duration_ms,
                "success_count": success_count,
                "failed_count": failed.len(),
                "status": match final_status {
                    PlanStatus::Completed => "completed",
                    PlanStatus::Failed => "failed",
                    _ => "partial",
                },
            }),
        );

        Ok(())
    }

    /// Execute a single plan step. Delegates the actual work to the agent
    /// loop in [`crate::ai::agent`].
    async fn execute_step(
        &self,
        plan_id: String,
        step: PlanStep,
        app: &tauri::AppHandle,
        state: crate::SharedState,
    ) -> anyhow::Result<StepResult> {
        let start = SystemTime::now();
        let _ = app.emit(
            "orchestrator://step_started",
            serde_json::json!({
                "plan_id": plan_id,
                "step_id": step.id,
                "description": step.description,
                "skill": step.skill,
            }),
        );

        // Build the agent run params. We pass the step's intended skill as
        // an inline override so the orchestrator doesn't race with the
        // global `active_skill` sidecar file when running parallel steps.
        let run_params = crate::ai::agent::AgentRunParams {
            conversation_id: None,
            user_message: step.description.clone(),
            model: None,
            temperature: Some(0.3),
            max_iterations: Some(8),
            skill: step.skill.clone(),
        };

        // `run_agent_loop` returns the conversation_id (the final assistant
        // message is emitted via `agent://done`). We read the latest
        // assistant message out of the conversation history to extract a
        // human-readable step output.
        let result = crate::ai::agent::run_agent_loop(state.clone(), app.clone(), run_params).await;

        let duration_ms = SystemTime::now()
            .duration_since(start)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let step_result = match result {
            Ok(conv_id) => {
                // Read the most recent assistant message out of the
                // conversation we just executed. Best-effort: if the read
                // fails, treat the step as completed without an output.
                let output = {
                    let s = state.lock();
                    s.memory
                        .conversations
                        .messages(&conv_id)
                        .ok()
                        .and_then(|mut msgs| {
                            msgs.reverse();
                            msgs.into_iter()
                                .find(|m| m.role == "assistant")
                                .map(|m| m.content)
                        })
                        .unwrap_or_default()
                };

                StepResult {
                    step_id: step.id.clone(),
                    status: PlanStatus::Completed,
                    output: if output.is_empty() {
                        None
                    } else {
                        Some(output)
                    },
                    error: None,
                    duration_ms,
                }
            }
            Err(e) => StepResult {
                step_id: step.id.clone(),
                status: PlanStatus::Failed,
                output: None,
                error: Some(format!("{e:#}")),
                duration_ms,
            },
        };

        // Persist the step result.
        {
            let mut plans = self.plans.lock();
            if let Some(p) = plans.get_mut(&plan_id) {
                p.results.insert(step.id.clone(), step_result.clone());
                p.updated_ms = now_ms();
            }
        }

        let _ = app.emit(
            "orchestrator://step_completed",
            serde_json::json!({
                "plan_id": plan_id,
                "step_id": step.id,
                "status": match step_result.status {
                    PlanStatus::Completed => "completed",
                    _ => "failed",
                },
                "duration_ms": duration_ms,
            }),
        );

        Ok(step_result)
    }
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self::new()
    }
}

/// Quick check that a skill id is known to the system. Used by the
/// orchestrator commands before submitting a plan so we can return a clean
/// error instead of letting the agent loop silently fall back to no skill.
pub fn is_known_skill(id: &str) -> bool {
    skills::find(id).is_some()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draft_plan_research_step_present() {
        let orch = Orchestrator::new();
        let p = orch.draft_plan("research rust async runtimes");
        assert!(!p.steps.is_empty());
        assert_eq!(p.steps[0].skill.as_deref(), Some("researcher"));
        // Final step is always summarizer.
        let last = p.steps.last().unwrap();
        assert_eq!(last.skill.as_deref(), Some("summarizer"));
    }

    #[test]
    fn draft_plan_adds_code_step_for_code_goal() {
        let orch = Orchestrator::new();
        let p = orch.draft_plan("implement a fibonacci function in rust");
        assert!(
            p.steps
                .iter()
                .any(|s| s.skill.as_deref() == Some("code_writer"))
        );
        assert!(
            p.steps
                .iter()
                .any(|s| s.skill.as_deref() == Some("code_reviewer"))
        );
    }

    #[test]
    fn draft_plan_adds_security_step_for_security_goal() {
        let orch = Orchestrator::new();
        let p = orch.draft_plan("audit my code for security vulnerabilities");
        assert!(
            p.steps
                .iter()
                .any(|s| s.skill.as_deref() == Some("security_auditor"))
        );
    }

    #[test]
    fn register_and_get_roundtrip() {
        let orch = Orchestrator::new();
        let p = orch.draft_plan("test");
        let id = orch.register(p);
        assert!(orch.get(&id).is_some());
        assert!(!orch.list().is_empty());
    }

    #[test]
    fn cancel_marks_plan_cancelled() {
        let orch = Orchestrator::new();
        let p = orch.draft_plan("test");
        let id = orch.register(p);
        assert!(orch.cancel(&id));
        let after = orch.get(&id).unwrap();
        assert_eq!(after.status, PlanStatus::Cancelled);
    }

    #[test]
    fn set_max_parallel_clamps_to_safe_range() {
        let orch = Orchestrator::new();
        orch.set_max_parallel(0);
        assert_eq!(orch.max_parallel.load(Ordering::Relaxed), 1);
        orch.set_max_parallel(100);
        assert_eq!(orch.max_parallel.load(Ordering::Relaxed), 16);
        orch.set_max_parallel(4);
        assert_eq!(orch.max_parallel.load(Ordering::Relaxed), 4);
    }
}
