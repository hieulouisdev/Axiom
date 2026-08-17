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
  MobileCapabilities,
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

// ===== v0.6 — Mobile capabilities =====
export const mobileCapabilities = () =>
  invoke<MobileCapabilities>("mobile_capabilities");

// ===== System =====
export const appVersion = () => invoke<string>("app_version");
export const appQuit = () => invoke("app_quit");
