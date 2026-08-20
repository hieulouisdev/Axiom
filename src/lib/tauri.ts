import { invoke } from "@tauri-apps/api/core";
import type {
  AppDescriptor,
  ChatResponseDto,
  ChatStreamStartDto,
  Conversation,
  EncryptionStatus,
  ExecResult,
  FileReadResult,
  MemoryStats,
  Message,
  ProviderDto,
  ScanResult,
  SearchResult,
  SecurityStatus,
  SettingsDto,
  WebFetchRawResult,
  YaraRule,
} from "../types";

// ===== AI =====
export const aiChat = (params: {
  conversation_id?: string | null;
  user_message: string;
  model?: string | null;
  temperature?: number | null;
  max_tokens?: number | null;
}) =>
  invoke<ChatResponseDto>("ai_chat", {
    params: {
      conversation_id: params.conversation_id ?? null,
      user_message: params.user_message,
      model: params.model ?? null,
      temperature: params.temperature ?? null,
      max_tokens: params.max_tokens ?? null,
    },
  });

export const aiChatStream = (params: {
  conversation_id?: string | null;
  user_message: string;
  model?: string | null;
  temperature?: number | null;
  max_tokens?: number | null;
}) =>
  invoke<ChatStreamStartDto>("ai_chat_stream", {
    params: {
      conversation_id: params.conversation_id ?? null,
      user_message: params.user_message,
      model: params.model ?? null,
      temperature: params.temperature ?? null,
      max_tokens: params.max_tokens ?? null,
    },
  });

export const aiChatCancel = (stream_id: string) =>
  invoke<void>("ai_chat_cancel", { streamId: stream_id });

export const aiListProviders = () => invoke<ProviderDto[]>("ai_list_providers");
export const aiSetActiveProvider = (provider_id: string | null) =>
  invoke("ai_set_active_provider", { providerId: provider_id });
export const aiConfigureProvider = (cfg: {
  provider_id: string;
  api_key?: string | null;
  base_url?: string | null;
  model?: string | null;
  enabled: boolean;
}) => invoke("ai_configure_provider", { cfg });
export const aiTestProvider = (provider_id: string) =>
  invoke<void>("ai_test_provider", { providerId: provider_id });

// ===== Computer use =====
export const computerExecCommand = (command: string, authorized = false) =>
  invoke<ExecResult>("computer_exec_command", {
    params: { command, authorized },
  });
export const computerOpenApp = (name: string, authorized = false) =>
  invoke<void>("computer_open_app", { name, authorized });
export const computerListApps = () => invoke<AppDescriptor[]>("computer_list_apps");
export const computerFileRead = (path: string) =>
  invoke<FileReadResult>("computer_file_read", { path });
export const computerFileWrite = (path: string, content: string, authorized = false) =>
  invoke<void>("computer_file_write", {
    params: { path, content, authorized },
  });
export const computerScreenshot = () => invoke("computer_screenshot");
export const computerRequestAction = (action: string, summary: string) =>
  invoke<string>("computer_request_action", { action, summary });
export const computerConfirmAction = (token: string, action: string) =>
  invoke<void>("computer_confirm_action", { params: { token, action } });

// ===== Clipboard =====
export const clipboardRead = () => invoke("clipboard_read_cmd");
export const clipboardWrite = (text: string) =>
  invoke<void>("clipboard_write_cmd", { text });
export const clipboardWatchStart = () => invoke<void>("clipboard_watch_start_cmd");
export const clipboardWatchStop = () => invoke<void>("clipboard_watch_stop_cmd");

// ===== Memory =====
export const memoryListConversations = (limit = 50) =>
  invoke<Conversation[]>("memory_list_conversations", { limit });
export const memoryGetConversation = (conversation_id: string) =>
  invoke<Message[]>("memory_get_conversation", { conversationId: conversation_id });
export const memoryClearAll = () => invoke<void>("memory_clear_all");
export const memorySearch = (query: string, limit = 50) =>
  invoke<Message[]>("memory_search", { query, limit });
export const memoryStats = () => invoke<MemoryStats>("memory_stats");
export const memorySummarize = (conversation_id: string) =>
  invoke<string>("memory_summarize", { conversationId: conversation_id });

// v0.6 — entity extraction
export const memoryExtractEntities = (conversation_id: string, limit?: number) =>
  invoke<number>("memory_extract_entities", { conversationId: conversation_id, limit: limit ?? null });

// v0.6 — encryption status
export const memoryEncryptionStatus = () =>
  invoke<EncryptionStatus>("memory_encryption_status");

// v0.6 — GDPR data export / wipe
export const memoryExportAll = () =>
  invoke<unknown>("memory_export_all");
export const memoryForgetAll = () =>
  invoke<void>("memory_forget_all");

// ===== Security =====
export const securityStatus = () => invoke<SecurityStatus>("security_status");
export const securityScan = (path: string, max_depth = 5) =>
  invoke<ScanResult[]>("security_scan", { path, maxDepth: max_depth });
export const securityQuarantineList = () =>
  invoke("security_quarantine_list");
export const securityRestoreFile = (id: string) =>
  invoke<void>("security_restore_file", { id });
export const securitySetAutoDefense = (enabled: boolean) =>
  invoke<void>("security_set_auto_defense", { enabled });
export const securityIntegrityCheck = () =>
  invoke("security_integrity_check");
export const securityIntegritySaveBaseline = () =>
  invoke("security_integrity_save_baseline");
export const securityNetworkScan = () =>
  invoke("security_network_scan");

// v0.6 — YARA rules
export const yaraList = () => invoke<YaraRule[]>("yara_list");
export const yaraEnsureDir = () => invoke<void>("yara_ensure_dir");

// v0.6 — audit log export
export const auditExport = (limit?: number, format?: "json" | "csv") =>
  invoke<unknown>("audit_export", { limit: limit ?? null, format: format ?? null });

// ===== Modes =====
export const modesGetActive = () => invoke<"continuous" | "ondemand">("modes_get_active");
export const modesSetMode = (mode: "continuous" | "ondemand") =>
  invoke<void>("modes_set_mode", { mode });

// ===== Settings / i18n =====
export const settingsGet = () => invoke<SettingsDto>("settings_get");
export const settingsSet = (dto: SettingsDto) => invoke<void>("settings_set", { dto });
export const i18nGetLocale = () => invoke<string>("i18n_get_locale");
export const i18nSetLocale = (locale: string) =>
  invoke<void>("i18n_set_locale", { locale });
export const i18nTranslate = (key: string) => invoke<string>("i18n_translate", { key });

// ===== v0.6 — Web access =====
export const webSearch = (query: string) =>
  invoke<SearchResult[]>("web_search", { query });
export const webFetch = (url: string) =>
  invoke<string>("web_fetch", { url });
export const webFetchRaw = (url: string, method?: string, body?: string) =>
  invoke<WebFetchRawResult>("web_fetch_raw", { url, method: method ?? null, body: body ?? null });

// ===== System =====
export const appVersion = () => invoke<string>("app_version");
export const appQuit = () => invoke("app_quit");

// ===== v0.7 — Phase 4.2: Sandbox =====
export const sandboxStatus = () =>
  invoke<{ enabled: boolean; allowed_dirs: string[]; allow_home_subdirs: boolean }>("sandbox_status");
export const sandboxSetEnabled = (enabled: boolean) =>
  invoke<void>("sandbox_set_enabled", { enabled });
export const sandboxAddDir = (dir: string) =>
  invoke<void>("sandbox_add_dir", { dir });
export const sandboxRemoveDir = (dir: string) =>
  invoke<void>("sandbox_remove_dir", { dir });

// ===== v0.7 — Phase 4.3: Telemetry =====
export const telemetryStatus = () =>
  invoke<{ enabled: boolean; prompted: boolean; pending_count: number; install_id: string }>("telemetry_status");
export const telemetryOptIn = () =>
  invoke<void>("telemetry_opt_in");
export const telemetryOptOut = () =>
  invoke<void>("telemetry_opt_out");

// ===========================================================================
// v1.6.0 — Multi-Agent Orchestrator
// ===========================================================================

export interface PlanStep {
  id: string;
  description: string;
  skill: string | null;
  depends_on: string[];
  parallelizable: boolean;
}

export interface StepResult {
  step_id: string;
  status: "pending" | "running" | "completed" | "failed" | "cancelled";
  output: string | null;
  error: string | null;
  duration_ms: number;
}

export interface Plan {
  id: string;
  goal: string;
  steps: PlanStep[];
  status: "pending" | "running" | "completed" | "failed" | "cancelled";
  created_ms: number;
  updated_ms: number;
  results: Record<string, StepResult>;
}

export const orchestratorRunPlan = (params: {
  goal: string;
  refine_with_ai?: boolean;
}) =>
  invoke<string>("orchestrator_run_plan", {
    params: { goal: params.goal, refine_with_ai: params.refine_with_ai ?? true },
  });

export const orchestratorGetPlan = (planId: string) =>
  invoke<Plan | null>("orchestrator_get_plan", { planId });

export const orchestratorListPlans = () =>
  invoke<Plan[]>("orchestrator_list_plans");

export const orchestratorCancel = (planId: string) =>
  invoke<boolean>("orchestrator_cancel", { planId });

// ===========================================================================
// v1.6.0 — Workflow Engine
// ===========================================================================

export type WorkflowAction =
  | { kind: "ai_call"; prompt: string; model?: string | null }
  | { kind: "shell_command"; command: string }
  | { kind: "web_search"; query: string }
  | { kind: "file_read"; path: string }
  | { kind: "file_write"; path: string; content: string }
  | { kind: "sleep"; ms: number }
  | { kind: "noop"; value: string };

export interface Condition {
  lhs: string;
  op: "eq" | "ne" | "contains" | "gt" | "lt" | "ge" | "le";
  rhs: unknown;
}

export interface WorkflowStep {
  id: string;
  name: string;
  action: WorkflowAction;
  depends_on: string[];
  condition?: Condition | null;
  retries: number;
}

export interface Workflow {
  id: string;
  name: string;
  description?: string | null;
  trigger: "manual" | "cron" | "event";
  steps: WorkflowStep[];
  tags: string[];
}

export interface StepOutput {
  step_id: string;
  value: unknown;
  ran: boolean;
}

export interface WorkflowRunResult {
  run_id: string;
  workflow_id: string;
  status: "pending" | "running" | "completed" | "failed" | "skipped" | "cancelled";
  outputs: Record<string, StepOutput>;
  duration_ms: number;
}

export const workflowUpsert = (workflow: Workflow) =>
  invoke<string>("workflow_upsert", { workflow });
export const workflowDelete = (workflowId: string) =>
  invoke<boolean>("workflow_delete", { workflowId });
export const workflowGet = (workflowId: string) =>
  invoke<Workflow | null>("workflow_get", { workflowId });
export const workflowList = () => invoke<Workflow[]>("workflow_list");
export const workflowRun = (workflowId: string) =>
  invoke<string>("workflow_run", { workflowId });
export const workflowRuns = () =>
  invoke<WorkflowRunResult[]>("workflow_runs");

// ===========================================================================
// v1.6.0 — Knowledge Graph
// ===========================================================================

export interface Triple {
  id: number;
  subject: string;
  predicate: string;
  object: string;
  source: string | null;
  confidence: number;
  created_at_ms: number;
}

export const graphAddTriple = (triple: {
  subject: string;
  predicate: string;
  object: string;
  source?: string | null;
  confidence?: number;
}) =>
  invoke<number>("graph_add_triple", {
    triple: {
      subject: triple.subject,
      predicate: triple.predicate,
      object: triple.object,
      source: triple.source ?? null,
      confidence: triple.confidence ?? 0.7,
    },
  });

export const graphQuery = (params: {
  subject?: string | null;
  predicate?: string | null;
  object?: string | null;
}) =>
  invoke<Triple[]>("graph_query", {
    subject: params.subject ?? null,
    predicate: params.predicate ?? null,
    object: params.object ?? null,
  });

export const graphNeighbors = (subject: string, depth: number) =>
  invoke<Triple[]>("graph_neighbors", { subject, depth });

export const graphPath = (start: string, target: string, maxDepth: number) =>
  invoke<Triple[]>("graph_path", {
    start,
    target,
    maxDepth,
  });

export const graphSubjects = (limit?: number) =>
  invoke<string[]>("graph_subjects", { limit: limit ?? null });

export const graphPredicates = () => invoke<string[]>("graph_predicates");
export const graphCount = () => invoke<number>("graph_count");
export const graphClear = () => invoke<void>("graph_clear");

// ===========================================================================
// v1.6.0 — Proactive Intelligence
// ===========================================================================

export type InsightKind =
  | "activity_pattern"
  | "memory_suggestion"
  | "security"
  | "workflow_suggestion"
  | "efficiency";

export interface Insight {
  id: string;
  kind: InsightKind;
  title: string;
  detail: string;
  suggested_action: string | null;
  severity: number;
  created_ms: number;
  dismissed: boolean;
}

export const proactiveInsights = () => invoke<Insight[]>("proactive_insights");
export const proactiveRecent = (limit: number) =>
  invoke<Insight[]>("proactive_recent", { limit });
export const proactiveDismiss = (insightId: string) =>
  invoke<boolean>("proactive_dismiss", { insightId });
export const proactiveEnable = () => invoke<void>("proactive_enable");
export const proactiveDisable = () => invoke<void>("proactive_disable");
export const proactiveEnabled = () => invoke<boolean>("proactive_enabled");

// ===========================================================================
// v1.6.0 — Background Task Queue
// ===========================================================================

export type TaskStatus = "pending" | "running" | "completed" | "failed" | "cancelled";

export interface Task {
  id: string;
  kind: string;
  status: TaskStatus;
  progress: number;
  label: string | null;
  result: unknown;
  error: string | null;
  created_ms: number;
  updated_ms: number;
}

export const tasksList = () => invoke<Task[]>("tasks_list");
export const tasksActive = () => invoke<Task[]>("tasks_active");
export const tasksGet = (taskId: string) =>
  invoke<Task | null>("tasks_get", { taskId });
export const tasksCancel = (taskId: string) =>
  invoke<boolean>("tasks_cancel", { taskId });

// ===========================================================================
// v1.6.0 — Wire previously-unwired backend commands
// ===========================================================================

export const agentListTools = () =>
  invoke<unknown[]>("agent_list_tools");

export const auditRecent = (limit: number) =>
  invoke<unknown[]>("audit_recent", { limit });
export const auditCount = () => invoke<number>("audit_count");
export const auditWipe = () => invoke<void>("audit_wipe");

export const safetyTripKillSwitch = () =>
  invoke<void>("safety_trip_kill_switch");
export const safetyResetKillSwitch = () =>
  invoke<void>("safety_reset_kill_switch");
export const safetyKillSwitchStatus = () =>
  invoke<boolean>("safety_kill_switch_status");
export const safetyRateLimiterStatus = () =>
  invoke<{ available_tokens: number; refill_per_sec: number; burst_capacity: number }>(
    "safety_rate_limiter_status"
  );
export const safetyRateLimiterReset = () =>
  invoke<void>("safety_rate_limiter_reset");

export const bypassModeStatus = () => invoke<boolean>("bypass_mode_status");
export const bypassModeEnable = () => invoke<void>("bypass_mode_enable");
export const bypassModeDisable = () => invoke<void>("bypass_mode_disable");

export interface CatalogProvider {
  id: string;
  name: string;
}
export interface CatalogModel {
  id: string;
  provider_id: string;
  name: string;
  context_length: number | null;
  max_output_tokens: number | null;
  pricing: { prompt_per_1k: number | null; completion_per_1k: number | null } | null;
  supports_tool_call: boolean;
  supports_vision: boolean;
  supports_structured_output: boolean;
  release_date: string | null;
  knowledge_cutoff: string | null;
  source: string | null;
}
export const aiListModels = () => invoke<CatalogModel[]>("ai_list_models");
export const aiModelsForProvider = (providerId: string) =>
  invoke<CatalogModel[]>("ai_models_for_provider", { providerId });

export interface Skill {
  id: string;
  name: string;
  description: string;
}
export const skillsList = () => invoke<Skill[]>("skills_list");
export const skillsActive = () => invoke<string | null>("skills_active");
export const skillsSet = (id: string) => invoke<void>("skills_set", { id });

export const voiceTranscribe = (audioB64: string, mime: string) =>
  invoke<{ text: string; language: string | null; duration_ms: number; backend: string; wake_word_detected: boolean }>(
    "voice_transcribe",
    { audioB64, mime }
  );
export const voiceSpeak = (text: string, opts?: { voice?: string | null; rate?: number | null }) =>
  invoke<{ path: string; mime: string; duration_ms: number; backend: string }>(
    "voice_speak",
    { text, voice: opts?.voice ?? null, rate: opts?.rate ?? null }
  );
export const voicePttState = () =>
  invoke<"idle" | "recording">("voice_ptt_state");
export const voicePttSetHotkey = (hotkey: string) =>
  invoke<void>("voice_ptt_set_hotkey", { hotkey });
