export type Locale = "en" | "vi" | "es" | "fr" | "de" | "ja" | "zh-CN";

export type ViewId =
  | "chat"
  | "providers"
  | "memory"
  | "security"
  | "modes"
  | "settings"
  | "web"
  | "guide";

export interface ProviderDto {
  id: string;
  name: string;
  description: string;
  homepage: string;
  category: string;
  requires_api_key: boolean;
  local: boolean;
  default_base_url: string | null;
  default_model: string;
  known_models: string[];
  implemented: boolean;
  enabled: boolean;
  is_active: boolean;
  configured: boolean;
}

export interface Conversation {
  id: string;
  title: string;
  provider_id: string | null;
  created_at_ms: number;
  updated_at_ms: number;
}

export interface Message {
  id: string;
  conversation_id: string;
  role: string;
  content: string;
  created_at_ms: number;
  metadata_json: string | null;
}

export interface ChatResponseDto {
  conversation_id: string;
  content: string;
  model: string;
  usage: { prompt_tokens: number; completion_tokens: number; total_tokens: number } | null;
}

export interface ExecResult {
  command: string;
  stdout: string;
  stderr: string;
  exit_code: number;
  duration_ms: number;
}

export interface AppDescriptor {
  name: string;
  path: string | null;
  icon: string | null;
}

export interface FileReadResult {
  path: string;
  bytes: number;
  content: string;
  truncated: boolean;
}

export interface MemoryStats {
  conversations: number;
  messages: number;
  activities: number;
  knowledge: number;
}

export interface SettingsDto {
  language: string;
  mode: string;
  allow_autonomous: boolean;
  bypass_mode: boolean;
  auto_defense: boolean;
  monitor: boolean;
  scanner_enabled: boolean;
  quarantine_auto_delete_days: number;
}

export interface Threat {
  id: string;
  timestamp_ms: number;
  pid: number;
  process_name: string;
  command_line: string;
  signature_id: string;
  signature_name: string;
  severity: "info" | "low" | "medium" | "high" | "critical";
}

export interface ScanResult {
  path: string;
  scanned: boolean;
  infected: boolean;
  signature_name: string | null;
  hash_sha256: string;
  size_bytes: number;
  error: string | null;
}

export interface ChatStreamStartDto {
  stream_id: string;
  conversation_id: string;
}

export interface NetworkAnomaly {
  kind: string;
  detail: string;
  severity: "info" | "low" | "medium" | "high" | "critical";
  timestamp_ms: number;
}

export interface IntegrityEvent {
  path: string;
  event_kind: string;
  expected_hash: string | null;
  actual_hash: string | null;
  timestamp_ms: number;
}

export interface SecurityStatus {
  auto_defense: boolean;
  monitor: boolean;
  scanner_enabled: boolean;
  recent_threats: Threat[];
  recent_events: unknown[];
  network_anomalies: NetworkAnomaly[];
}

// ===== v0.6 — Web access =====
export interface SearchResult {
  title: string;
  url: string;
  snippet: string;
}

export interface WebFetchRawResult {
  status: number;
  body: string;
  len: number;
  url: string;
}

// ===== v0.6 — Memory / entities / encryption =====
export interface EncryptionStatus {
  status: "not_supported" | "disabled" | "enabled";
  supported: boolean;
}

export interface YaraRule {
  name: string;
  tags: string[];
  strings: string[];
  source: string;
}
