import { useEffect, useState } from "react";
import {
  Activity,
  Workflow as WorkflowIcon,
  Network,
  Zap,
  Play,
  Square,
  Plus,
  RefreshCw,
  Trash2,
} from "lucide-react";
import { t } from "../i18n";
import {
  orchestratorListPlans,
  orchestratorRunPlan,
  orchestratorCancel,
  type Plan,
  workflowList,
  workflowRun,
  workflowDelete,
  type Workflow,
  graphCount,
  graphSubjects,
  graphAddTriple,
  graphNeighbors,
  type Triple,
  tasksList,
  tasksCancel,
  type Task,
  proactiveInsights,
  proactiveDismiss,
  proactiveEnabled,
  proactiveEnable,
  proactiveDisable,
  type Insight,
} from "../lib/tauri";

type Tab = "orchestrator" | "workflows" | "graph" | "tasks";

const TABS: { id: Tab; icon: typeof Activity; label: string }[] = [
  { id: "orchestrator", icon: Zap, label: "Orchestrator" },
  { id: "workflows", icon: WorkflowIcon, label: "Workflows" },
  { id: "graph", icon: Network, label: "Knowledge Graph" },
  { id: "tasks", icon: Activity, label: "Tasks" },
];

export function Studio() {
  const [tab, setTab] = useState<Tab>("orchestrator");

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <header className="px-6 py-4 border-b border-aegis-200 dark:border-aegis-night-50 bg-white dark:bg-aegis-night-200">
        <div className="flex items-center gap-3">
          <WorkflowIcon className="h-5 w-5 text-aegis-accent" />
          <h2 className="text-lg font-semibold text-aegis-900 dark:text-aegis-100">
            {t("nav.studio")}
          </h2>
          <span className="ml-2 text-[10px] font-bold uppercase px-1.5 py-0.5 rounded-full bg-aegis-accent/10 text-aegis-accent">
            v1.6
          </span>
        </div>
        <p className="mt-1 text-xs text-aegis-500 dark:text-aegis-400">
          Multi-agent orchestration, declarative workflows, knowledge graph,
          and the background task queue — the v1.6 superpowers.
        </p>
      </header>

      {/* Tabs */}
      <div className="flex items-center gap-1 px-6 pt-3 bg-white dark:bg-aegis-night-200 border-b border-aegis-200 dark:border-aegis-night-50">
        {TABS.map(({ id, icon: Icon, label }) => {
          const active = tab === id;
          return (
            <button
              key={id}
              onClick={() => setTab(id)}
              className={`flex items-center gap-2 px-3 py-2 text-xs font-medium rounded-t-lg transition-colors
                ${
                  active
                    ? "bg-aegis-50 dark:bg-aegis-night-500 text-aegis-accent border-x border-t border-aegis-200 dark:border-aegis-night-50 -mb-px"
                    : "text-aegis-500 hover:text-aegis-700 dark:hover:text-aegis-300"
                }
              `}
            >
              <Icon className="h-3.5 w-3.5" />
              {label}
            </button>
          );
        })}
      </div>

      {/* Body */}
      <div className="flex-1 overflow-y-auto p-6 bg-aegis-50 dark:bg-aegis-night-500">
        {tab === "orchestrator" && <OrchestratorPanel />}
        {tab === "workflows" && <WorkflowsPanel />}
        {tab === "graph" && <GraphPanel />}
        {tab === "tasks" && <TasksPanel />}
      </div>
    </div>
  );
}

// ============================================================================
// Orchestrator Panel
// ============================================================================

function OrchestratorPanel() {
  const [goal, setGoal] = useState("");
  const [plans, setPlans] = useState<Plan[]>([]);
  const [running, setRunning] = useState(false);

  const refresh = () => {
    orchestratorListPlans()
      .then(setPlans)
      .catch(() => setPlans([]));
  };

  useEffect(() => {
    refresh();
  }, []);

  const run = async () => {
    if (!goal.trim()) return;
    setRunning(true);
    try {
      await orchestratorRunPlan({ goal: goal.trim(), refine_with_ai: true });
      setGoal("");
      setTimeout(refresh, 500);
    } finally {
      setRunning(false);
    }
  };

  const cancel = async (planId: string) => {
    await orchestratorCancel(planId);
    refresh();
  };

  return (
    <div className="space-y-4">
      {/* Goal input */}
      <div className="rounded-xl bg-white dark:bg-aegis-night-200 border border-aegis-200 dark:border-aegis-night-50 p-4 shadow-soft">
        <label className="block text-xs font-semibold uppercase text-aegis-500 dark:text-aegis-400 mb-2">
          New plan goal
        </label>
        <textarea
          value={goal}
          onChange={(e) => setGoal(e.target.value)}
          placeholder="e.g. research async Rust runtimes and implement a minimal executor with tests"
          className="w-full px-3 py-2 text-sm rounded-lg border border-aegis-200 dark:border-aegis-night-50 bg-aegis-50 dark:bg-aegis-night-500 text-aegis-900 dark:text-aegis-100 focus:outline-none focus:ring-2 focus:ring-aegis-accent/40 resize-y min-h-[80px]"
          rows={3}
        />
        <div className="flex justify-between items-center mt-3">
          <p className="text-[11px] text-aegis-500 dark:text-aegis-400">
            The orchestrator drafts a deterministic DAG, asks the active AI
            provider to refine it, then executes steps in parallel up to the
            configured ceiling.
          </p>
          <button
            onClick={run}
            disabled={running || !goal.trim()}
            className="inline-flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-medium bg-gradient-accent text-white shadow-soft disabled:opacity-50"
          >
            <Play className="h-3 w-3" />
            {running ? "Planning..." : "Run plan"}
          </button>
        </div>
      </div>

      {/* Plans list */}
      <div className="rounded-xl bg-white dark:bg-aegis-night-200 border border-aegis-200 dark:border-aegis-night-50 shadow-soft">
        <div className="flex items-center justify-between px-4 py-2 border-b border-aegis-200 dark:border-aegis-night-50">
          <h3 className="text-sm font-semibold text-aegis-900 dark:text-aegis-100">
            Plans ({plans.length})
          </h3>
          <button
            onClick={refresh}
            className="p-1 rounded hover:bg-aegis-100 dark:hover:bg-aegis-night-50 text-aegis-500"
          >
            <RefreshCw className="h-3 w-3" />
          </button>
        </div>
        {plans.length === 0 ? (
          <p className="px-4 py-6 text-xs text-aegis-500 dark:text-aegis-400 text-center">
            No plans yet. Submit a goal above to get started.
          </p>
        ) : (
          <ul className="divide-y divide-aegis-200 dark:divide-aegis-night-50">
            {plans.map((p) => (
              <li key={p.id} className="px-4 py-3">
                <div className="flex items-start gap-3">
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium text-aegis-900 dark:text-aegis-100 truncate">
                      {p.goal}
                    </p>
                    <p className="text-[11px] text-aegis-500 dark:text-aegis-400 mt-0.5">
                        {p.steps.length} steps · status:{" "}
                        <StatusBadge status={p.status} /> ·{" "}
                        {Object.values(p.results ?? {}).filter(
                          (r) => r.status === "completed"
                        ).length}
                        /{p.steps.length} done
                      </p>
                  </div>
                  {(p.status === "running" || p.status === "pending") && (
                    <button
                      onClick={() => cancel(p.id)}
                      className="inline-flex items-center gap-1 px-2 py-1 rounded text-[11px] font-medium text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20"
                    >
                      <Square className="h-3 w-3" /> Cancel
                    </button>
                  )}
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}

function StatusBadge({ status }: { status: string }) {
  const colors: Record<string, string> = {
    pending: "bg-aegis-100 text-aegis-700 dark:bg-aegis-night-50 dark:text-aegis-300",
    running: "bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300 animate-pulse-soft",
    completed: "bg-emerald-100 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-300",
    failed: "bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-300",
    cancelled: "bg-aegis-100 text-aegis-500 dark:bg-aegis-night-50 dark:text-aegis-400",
  };
  return (
    <span
      className={`inline-block px-1.5 py-0.5 rounded text-[10px] font-bold uppercase ${
        colors[status] ?? colors.pending
      }`}
    >
      {status}
    </span>
  );
}

// ============================================================================
// Workflows Panel
// ============================================================================

function WorkflowsPanel() {
  const [workflows, setWorkflows] = useState<Workflow[]>([]);
  const [running, setRunning] = useState<string | null>(null);

  const refresh = () => {
    workflowList()
      .then(setWorkflows)
      .catch(() => setWorkflows([]));
  };

  useEffect(() => {
    refresh();
  }, []);

  const run = async (id: string) => {
    setRunning(id);
    try {
      await workflowRun(id);
      setTimeout(refresh, 500);
    } finally {
      setRunning(null);
    }
  };

  const del = async (id: string) => {
    await workflowDelete(id);
    refresh();
  };

  return (
    <div className="space-y-4">
      <div className="rounded-xl bg-white dark:bg-aegis-night-200 border border-aegis-200 dark:border-aegis-night-50 p-4 shadow-soft">
        <p className="text-xs text-aegis-500 dark:text-aegis-400 leading-relaxed">
          Workflows are reusable automation pipelines. Register one via the{" "}
          <code className="px-1 py-0.5 rounded bg-aegis-100 dark:bg-aegis-night-50 text-aegis-700 dark:text-aegis-300">
            workflow_upsert
          </code>{" "}
          command (e.g. from the agent loop), then run it here. The DSL
          supports AI calls, shell commands, web search, file I/O, sleep,
          conditional branches, and parallel steps with declarative{" "}
          <code className="px-1 py-0.5 rounded bg-aegis-100 dark:bg-aegis-night-50 text-aegis-700 dark:text-aegis-300">
            depends_on
          </code>{" "}
          edges.
        </p>
      </div>

      <div className="rounded-xl bg-white dark:bg-aegis-night-200 border border-aegis-200 dark:border-aegis-night-50 shadow-soft">
        <div className="flex items-center justify-between px-4 py-2 border-b border-aegis-200 dark:border-aegis-night-50">
          <h3 className="text-sm font-semibold text-aegis-900 dark:text-aegis-100">
            Registered workflows ({workflows.length})
          </h3>
          <button
            onClick={refresh}
            className="p-1 rounded hover:bg-aegis-100 dark:hover:bg-aegis-night-50 text-aegis-500"
          >
            <RefreshCw className="h-3 w-3" />
          </button>
        </div>
        {workflows.length === 0 ? (
          <p className="px-4 py-6 text-xs text-aegis-500 dark:text-aegis-400 text-center">
            No workflows registered yet. Ask Aegis in the chat view to
            "create a workflow that..." and it'll be added here.
          </p>
        ) : (
          <ul className="divide-y divide-aegis-200 dark:divide-aegis-night-50">
            {workflows.map((w) => (
              <li key={w.id} className="px-4 py-3">
                <div className="flex items-start gap-3">
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium text-aegis-900 dark:text-aegis-100 truncate">
                      {w.name}
                      <span className="ml-2 text-[10px] text-aegis-500 dark:text-aegis-400">
                        ({w.id})
                      </span>
                    </p>
                    <p className="text-[11px] text-aegis-500 dark:text-aegis-400 mt-0.5">
                      {w.steps.length} steps · trigger: {w.trigger}
                      {w.tags.length > 0 && ` · tags: ${w.tags.join(", ")}`}
                    </p>
                  </div>
                  <button
                    onClick={() => run(w.id)}
                    disabled={running === w.id}
                    className="inline-flex items-center gap-1 px-2 py-1 rounded text-[11px] font-medium bg-gradient-accent text-white disabled:opacity-50"
                  >
                    <Play className="h-3 w-3" />
                    {running === w.id ? "Running..." : "Run"}
                  </button>
                  <button
                    onClick={() => del(w.id)}
                    className="inline-flex items-center gap-1 px-2 py-1 rounded text-[11px] font-medium text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20"
                  >
                    <Trash2 className="h-3 w-3" />
                  </button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}

// ============================================================================
// Knowledge Graph Panel
// ============================================================================

function GraphPanel() {
  const [count, setCount] = useState(0);
  const [subjects, setSubjects] = useState<string[]>([]);
  const [neighborhood, setNeighborhood] = useState<Triple[]>([]);
  const [selectedSubject, setSelectedSubject] = useState("");
  const [newTriple, setNewTriple] = useState({ s: "", p: "", o: "" });

  const refresh = async () => {
    try {
      const [c, subs] = await Promise.all([graphCount(), graphSubjects(100)]);
      setCount(c);
      setSubjects(subs);
    } catch {
      setCount(0);
      setSubjects([]);
    }
  };

  useEffect(() => {
    refresh();
  }, []);

  const addTriple = async () => {
    if (!newTriple.s || !newTriple.p || !newTriple.o) return;
    await graphAddTriple({
      subject: newTriple.s,
      predicate: newTriple.p,
      object: newTriple.o,
      confidence: 0.8,
    });
    setNewTriple({ s: "", p: "", o: "" });
    refresh();
  };

  const explore = async () => {
    if (!selectedSubject) return;
    try {
      const triples = await graphNeighbors(selectedSubject, 2);
      setNeighborhood(triples);
    } catch {
      setNeighborhood([]);
    }
  };

  return (
    <div className="space-y-4">
      {/* Stats */}
      <div className="grid grid-cols-3 gap-3">
        <StatCard label="Triples" value={count} />
        <StatCard label="Subjects" value={subjects.length} />
        <StatCard label="Predicates" value={"—"} />
      </div>

      {/* Add triple */}
      <div className="rounded-xl bg-white dark:bg-aegis-night-200 border border-aegis-200 dark:border-aegis-night-50 p-4 shadow-soft">
        <label className="block text-xs font-semibold uppercase text-aegis-500 dark:text-aegis-400 mb-2">
          Add a triple (subject — predicate — object)
        </label>
        <div className="grid grid-cols-[1fr_1fr_1fr_auto] gap-2">
          <input
            value={newTriple.s}
            onChange={(e) => setNewTriple({ ...newTriple, s: e.target.value })}
            placeholder="alice"
            className="px-2.5 py-1.5 text-xs rounded border border-aegis-200 dark:border-aegis-night-50 bg-aegis-50 dark:bg-aegis-night-500 text-aegis-900 dark:text-aegis-100"
          />
          <input
            value={newTriple.p}
            onChange={(e) => setNewTriple({ ...newTriple, p: e.target.value })}
            placeholder="knows"
            className="px-2.5 py-1.5 text-xs rounded border border-aegis-200 dark:border-aegis-night-50 bg-aegis-50 dark:bg-aegis-night-500 text-aegis-900 dark:text-aegis-100"
          />
          <input
            value={newTriple.o}
            onChange={(e) => setNewTriple({ ...newTriple, o: e.target.value })}
            placeholder="bob"
            className="px-2.5 py-1.5 text-xs rounded border border-aegis-200 dark:border-aegis-night-50 bg-aegis-50 dark:bg-aegis-night-500 text-aegis-900 dark:text-aegis-100"
          />
          <button
            onClick={addTriple}
            className="inline-flex items-center justify-center px-3 py-1.5 rounded text-xs font-medium bg-gradient-accent text-white"
          >
            <Plus className="h-3 w-3" />
          </button>
        </div>
      </div>

      {/* Explorer */}
      <div className="rounded-xl bg-white dark:bg-aegis-night-200 border border-aegis-200 dark:border-aegis-night-50 shadow-soft">
        <div className="px-4 py-2 border-b border-aegis-200 dark:border-aegis-night-50">
          <h3 className="text-sm font-semibold text-aegis-900 dark:text-aegis-100">
            Explore
          </h3>
        </div>
        <div className="px-4 py-3 space-y-3">
          <div className="flex items-center gap-2">
            <select
              value={selectedSubject}
              onChange={(e) => setSelectedSubject(e.target.value)}
              className="flex-1 px-2.5 py-1.5 text-xs rounded border border-aegis-200 dark:border-aegis-night-50 bg-aegis-50 dark:bg-aegis-night-500 text-aegis-900 dark:text-aegis-100"
            >
              <option value="">Select a subject...</option>
              {subjects.map((s) => (
                <option key={s} value={s}>
                  {s}
                </option>
              ))}
            </select>
            <button
              onClick={explore}
              disabled={!selectedSubject}
              className="inline-flex items-center gap-1 px-3 py-1.5 rounded text-xs font-medium bg-gradient-accent text-white disabled:opacity-50"
            >
              <Network className="h-3 w-3" /> Explore (depth 2)
            </button>
          </div>
          {neighborhood.length > 0 && (
            <ul className="text-xs space-y-1.5">
              {neighborhood.map((t) => (
                <li
                  key={t.id}
                  className="flex items-center gap-2 text-aegis-700 dark:text-aegis-300"
                >
                  <span className="font-medium">{t.subject}</span>
                  <span className="text-aegis-accent">—{t.predicate}→</span>
                  <span className="font-medium">{t.object}</span>
                  <span className="text-[10px] text-aegis-500">
                    (conf {t.confidence.toFixed(2)})
                  </span>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </div>
  );
}

function StatCard({ label, value }: { label: string; value: number | string }) {
  return (
    <div className="rounded-xl bg-white dark:bg-aegis-night-200 border border-aegis-200 dark:border-aegis-night-50 p-3 shadow-soft">
      <p className="text-[10px] font-semibold uppercase text-aegis-500 dark:text-aegis-400">
        {label}
      </p>
      <p className="text-xl font-semibold text-aegis-900 dark:text-aegis-100 mt-1">
        {value}
      </p>
    </div>
  );
}

// ============================================================================
// Tasks Panel
// ============================================================================

function TasksPanel() {
  const [tasks, setTasks] = useState<Task[]>([]);

  const refresh = () => {
    tasksList()
      .then(setTasks)
      .catch(() => setTasks([]));
  };

  useEffect(() => {
    refresh();
    const interval = setInterval(refresh, 2000);
    return () => clearInterval(interval);
  }, []);

  const cancel = async (id: string) => {
    await tasksCancel(id);
    refresh();
  };

  return (
    <div className="space-y-4">
      <div className="rounded-xl bg-white dark:bg-aegis-night-200 border border-aegis-200 dark:border-aegis-night-50 p-4 shadow-soft">
        <p className="text-xs text-aegis-500 dark:text-aegis-400 leading-relaxed">
          The background task queue tracks long-running operations spawned by
          the orchestrator, workflow engine, and the agent loop. Each task
          has a stable id, progress (0→1), and cooperative cancellation via
          the <code className="px-1 py-0.5 rounded bg-aegis-100 dark:bg-aegis-night-50 text-aegis-700 dark:text-aegis-300">CancelFlag</code>{" "}
          poll.
        </p>
      </div>

      <div className="rounded-xl bg-white dark:bg-aegis-night-200 border border-aegis-200 dark:border-aegis-night-50 shadow-soft">
        <div className="flex items-center justify-between px-4 py-2 border-b border-aegis-200 dark:border-aegis-night-50">
          <h3 className="text-sm font-semibold text-aegis-900 dark:text-aegis-100">
            Tasks ({tasks.length})
          </h3>
          <button
            onClick={refresh}
            className="p-1 rounded hover:bg-aegis-100 dark:hover:bg-aegis-night-50 text-aegis-500"
          >
            <RefreshCw className="h-3 w-3" />
          </button>
        </div>
        {tasks.length === 0 ? (
          <p className="px-4 py-6 text-xs text-aegis-500 dark:text-aegis-400 text-center">
            No tasks. Run a plan or workflow to populate the queue.
          </p>
        ) : (
          <ul className="divide-y divide-aegis-200 dark:divide-aegis-night-50">
            {tasks.map((task) => (
              <li key={task.id} className="px-4 py-3">
                <div className="flex items-start gap-3">
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium text-aegis-900 dark:text-aegis-100 truncate">
                      {task.label ?? task.kind}
                      <span className="ml-2 text-[10px] text-aegis-500">
                        ({task.kind})
                      </span>
                    </p>
                    <div className="mt-1 flex items-center gap-2">
                      <div className="flex-1 h-1 rounded-full bg-aegis-100 dark:bg-aegis-night-50 overflow-hidden">
                        <div
                          className="h-full bg-gradient-accent transition-all"
                          style={{ width: `${Math.round(task.progress * 100)}%` }}
                        />
                      </div>
                      <StatusBadge status={task.status} />
                      <span className="text-[10px] text-aegis-500">
                        {Math.round(task.progress * 100)}%
                      </span>
                    </div>
                    {task.error && (
                      <p className="mt-1 text-[11px] text-red-600 dark:text-red-400 truncate">
                        {task.error}
                      </p>
                    )}
                  </div>
                  {task.status === "running" && (
                    <button
                      onClick={() => cancel(task.id)}
                      className="inline-flex items-center gap-1 px-2 py-1 rounded text-[11px] font-medium text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20"
                    >
                      <Square className="h-3 w-3" /> Cancel
                    </button>
                  )}
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>

      <ProactiveBanner />
    </div>
  );
}

// ============================================================================
// Proactive Intelligence Banner (shown in Tasks panel as a footer hint)
// ============================================================================

function ProactiveBanner() {
  const [enabled, setEnabled] = useState(false);
  const [insights, setInsights] = useState<Insight[]>([]);

  useEffect(() => {
    proactiveEnabled().then(setEnabled).catch(() => setEnabled(false));
    const refresh = () => {
      proactiveInsights().then(setInsights).catch(() => setInsights([]));
    };
    refresh();
    const interval = setInterval(refresh, 5000);
    return () => clearInterval(interval);
  }, []);

  const toggle = async () => {
    if (enabled) {
      await proactiveDisable();
      setEnabled(false);
    } else {
      await proactiveEnable();
      setEnabled(true);
    }
  };

  const dismiss = async (id: string) => {
    await proactiveDismiss(id);
    setInsights((prev) => prev.filter((i) => i.id !== id));
  };

  return (
    <div className="rounded-xl bg-gradient-accent-soft border border-aegis-200 dark:border-aegis-night-50 p-4">
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2">
          <Zap className="h-4 w-4 text-aegis-accent" />
          <h3 className="text-sm font-semibold text-aegis-900 dark:text-aegis-100">
            Proactive Intelligence
          </h3>
        </div>
        <button
          onClick={toggle}
          className={`relative inline-flex h-5 w-9 items-center rounded-full transition-colors ${
            enabled ? "bg-aegis-accent" : "bg-aegis-200 dark:bg-aegis-night-50"
          }`}
        >
          <span
            className={`inline-block h-3.5 w-3.5 transform rounded-full bg-white transition-transform ${
              enabled ? "translate-x-4" : "translate-x-1"
            }`}
          />
        </button>
      </div>
      {enabled && insights.length === 0 && (
        <p className="text-xs text-aegis-500 dark:text-aegis-400">
          Engine enabled. Insights will surface here as patterns emerge in
          your activity log.
        </p>
      )}
      {enabled && insights.length > 0 && (
        <ul className="space-y-2">
          {insights.map((i) => (
            <li
              key={i.id}
              className="rounded-lg bg-white dark:bg-aegis-night-200 p-2.5 border border-aegis-200 dark:border-aegis-night-50"
            >
              <div className="flex items-start gap-2">
                <div className="flex-1">
                  <p className="text-xs font-semibold text-aegis-900 dark:text-aegis-100">
                    {i.title}
                  </p>
                  <p className="text-[11px] text-aegis-500 dark:text-aegis-400 mt-0.5 leading-relaxed">
                    {i.detail}
                  </p>
                </div>
                <button
                  onClick={() => dismiss(i.id)}
                  className="text-aegis-400 hover:text-aegis-700 dark:hover:text-aegis-200"
                >
                  ×
                </button>
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
