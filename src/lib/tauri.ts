import { invoke } from "@tauri-apps/api/core";
import type {
  AppDescriptor,
  ChatResponseDto,
  Conversation,
  ExecResult,
  FileReadResult,
  MemoryStats,
  Message,
  ProviderDto,
  ScanResult,
  SecurityStatus,
  SettingsDto,
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

// ===== Memory =====
export const memoryListConversations = (limit = 50) =>
  invoke<Conversation[]>("memory_list_conversations", { limit });
export const memoryGetConversation = (conversation_id: string) =>
  invoke<Message[]>("memory_get_conversation", { conversationId: conversation_id });
export const memoryClearAll = () => invoke<void>("memory_clear_all");
export const memorySearch = (query: string, limit = 50) =>
  invoke<Message[]>("memory_search", { query, limit });
export const memoryStats = () => invoke<MemoryStats>("memory_stats");

// ===== Security =====
export const securityStatus = () => invoke<SecurityStatus>("security_status");
export const securityScan = (path: string, max_depth = 5) =>
  invoke<ScanResult[]>("security_scan", { path, maxDepth: max_depth });
export const securitySetAutoDefense = (enabled: boolean) =>
  invoke<void>("security_set_auto_defense", { enabled });

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

// ===== System =====
export const appVersion = () => invoke<string>("app_version");
export const appQuit = () => invoke("app_quit");
