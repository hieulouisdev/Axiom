import { useEffect, useState } from "react";
import {
  Save,
  Loader2,
  AlertTriangle,
  Sun,
  Moon,
  Download,
  Trash2,
  Shield,
  Database,
  FileText,
  Loader,
  FolderPlus,
  FolderMinus,
  BarChart3,
} from "lucide-react";
import { t } from "../i18n";
import {
  settingsGet,
  settingsSet,
  memoryEncryptionStatus,
  auditExport,
  memoryExportAll,
  memoryForgetAll,
  sandboxStatus,
  sandboxSetEnabled,
  sandboxAddDir,
  sandboxRemoveDir,
  telemetryStatus,
  telemetryOptIn,
  telemetryOptOut,
} from "../lib/tauri";
import { useStore } from "../store";
import { setLocale as setI18nLocale } from "../i18n";
import type { EncryptionStatus, SettingsDto } from "../types";

export function Settings() {
  const [dto, setDto] = useState<SettingsDto | null>(null);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [encryption, setEncryption] = useState<EncryptionStatus | null>(null);
  const [exportingAudit, setExportingAudit] = useState(false);
  const [exportingData, setExportingData] = useState(false);
  const [forgetting, setForgetting] = useState(false);
  const [sandbox, setSandbox] = useState<{ enabled: boolean; allowed_dirs: string[]; allow_home_subdirs: boolean } | null>(null);
  const [sandboxDir, setSandboxDir] = useState("");
  const [tele, setTele] = useState<{ enabled: boolean; prompted: boolean; pending_count: number; install_id: string } | null>(null);
  const setLocale = useStore((s) => s.setLocale);
  const theme = useStore((s) => s.theme);
  const setTheme = useStore((s) => s.setTheme);

  useEffect(() => {
    settingsGet().then(setDto).catch(() => {});
    memoryEncryptionStatus().then(setEncryption).catch(() => {});
    sandboxStatus().then(setSandbox).catch(() => {});
    telemetryStatus().then(setTele).catch(() => {});
  }, []);

  if (!dto) {
    return (
      <div className="flex-1 flex justify-center items-center">
        <Loader2 className="h-6 w-6 text-aegis-400 animate-spin" />
      </div>
    );
  }

  const save = async () => {
    setSaving(true);
    try {
      await settingsSet(dto);
      const l = dto.language === "vi" ? "vi" : "en";
      setLocale(l);
      setI18nLocale(l);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } finally {
      setSaving(false);
    }
  };

  const downloadJson = (data: unknown, filename: string) => {
    const blob = new Blob([JSON.stringify(data, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  };

  const downloadText = (text: string, filename: string, mime = "text/plain") => {
    const blob = new Blob([text], { type: mime });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  };

  const exportAudit = async (format: "json" | "csv") => {
    setExportingAudit(true);
    try {
      const result = await auditExport(10_000, format);
      const ts = new Date().toISOString().replace(/[:.]/g, "-");
      if (format === "json") {
        downloadJson(result, `aegis-audit-${ts}.json`);
      } else {
        const csv = (result as { csv?: string }).csv ?? "";
        downloadText(csv, `aegis-audit-${ts}.csv`, "text/csv");
      }
    } catch (e) {
      alert(`Export failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setExportingAudit(false);
    }
  };

  const exportAll = async () => {
    setExportingData(true);
    try {
      const result = await memoryExportAll();
      const ts = new Date().toISOString().replace(/[:.]/g, "-");
      downloadJson(result, `aegis-export-${ts}.json`);
    } catch (e) {
      alert(`Export failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setExportingData(false);
    }
  };

  const forgetAll = async () => {
    if (!window.confirm(t("settings.data.forget.confirm"))) return;
    setForgetting(true);
    try {
      await memoryForgetAll();
      alert("All data wiped.");
    } catch (e) {
      alert(`Wipe failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setForgetting(false);
    }
  };

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden">
      <header className="aegis-section-header">
        <div>
          <h2 className="text-lg font-semibold text-aegis-900 dark:text-aegis-100">
            {t("settings.title")}
          </h2>
          <p className="text-xs text-aegis-500 dark:text-aegis-400 mt-0.5">
            Persisted to <code className="font-mono">config.toml</code> in your data dir.
          </p>
        </div>
        <button onClick={save} disabled={saving} className="aegis-btn-primary">
          {saving ? <Loader2 className="h-4 w-4 animate-spin" /> : <Save className="h-4 w-4" />}
          {saved ? t("common.success") : t("common.save")}
        </button>
      </header>

      <div className="flex-1 overflow-y-auto px-6 py-5">
        <div className="max-w-2xl mx-auto space-y-5">
          {/* Appearance */}
          <div className="aegis-card p-5">
            <h3 className="text-sm font-semibold mb-3 text-aegis-900 dark:text-aegis-100">
              {t("settings.theme")}
            </h3>
            <div className="flex gap-2">
              <button
                onClick={() => setTheme("light")}
                className={`flex-1 flex items-center justify-center gap-2 px-4 py-2.5 rounded-lg text-sm font-medium transition-all
                  ${
                    theme === "light"
                      ? "bg-gradient-accent text-white shadow-soft"
                      : "bg-white dark:bg-aegis-night-50 border border-aegis-200 dark:border-aegis-night-50 text-aegis-700 dark:text-aegis-300 hover:bg-aegis-50 dark:hover:bg-aegis-night-300"
                  }`}
              >
                <Sun className="h-4 w-4" />
                {t("settings.theme.light")}
              </button>
              <button
                onClick={() => setTheme("dark")}
                className={`flex-1 flex items-center justify-center gap-2 px-4 py-2.5 rounded-lg text-sm font-medium transition-all
                  ${
                    theme === "dark"
                      ? "bg-gradient-accent text-white shadow-soft"
                      : "bg-white dark:bg-aegis-night-50 border border-aegis-200 dark:border-aegis-night-50 text-aegis-700 dark:text-aegis-300 hover:bg-aegis-50 dark:hover:bg-aegis-night-300"
                  }`}
              >
                <Moon className="h-4 w-4" />
                {t("settings.theme.dark")}
              </button>
            </div>
          </div>

          {/* Language */}
          <div className="aegis-card p-5">
            <h3 className="text-sm font-semibold mb-3 text-aegis-900 dark:text-aegis-100">
              {t("settings.language")}
            </h3>
            <div className="flex gap-2">
              {[
                { id: "en", label: "English" },
                { id: "vi", label: "Tiếng Việt" },
              ].map((l) => (
                <button
                  key={l.id}
                  onClick={() => setDto({ ...dto, language: l.id })}
                  className={`flex-1 px-4 py-2 rounded-lg text-sm font-medium transition-all
                    ${
                      dto.language === l.id
                        ? "bg-gradient-accent text-white"
                        : "bg-white dark:bg-aegis-night-50 border border-aegis-200 dark:border-aegis-night-50 text-aegis-700 dark:text-aegis-300 hover:bg-aegis-50 dark:hover:bg-aegis-night-300"
                    }`}
                >
                  {l.label}
                </button>
              ))}
            </div>
          </div>

          {/* Mode */}
          <div className="aegis-card p-5">
            <h3 className="text-sm font-semibold mb-3 text-aegis-900 dark:text-aegis-100">
              {t("settings.mode")}
            </h3>
            <div className="space-y-2">
              <label className="flex items-start gap-3 cursor-pointer p-3 rounded-lg hover:bg-aegis-50 dark:hover:bg-aegis-night-50 transition-colors">
                <input
                  type="radio"
                  checked={dto.mode === "ondemand"}
                  onChange={() => setDto({ ...dto, mode: "ondemand" })}
                  className="mt-1"
                />
                <div>
                  <div className="text-sm font-medium text-aegis-900 dark:text-aegis-100">
                    {t("settings.mode.ondemand")}
                  </div>
                  <div className="text-xs text-aegis-500 dark:text-aegis-400 mt-0.5">
                    {t("modes.ondemand.desc")}
                  </div>
                </div>
              </label>
              <label className="flex items-start gap-3 cursor-pointer p-3 rounded-lg hover:bg-aegis-50 dark:hover:bg-aegis-night-50 transition-colors">
                <input
                  type="radio"
                  checked={dto.mode === "continuous"}
                  onChange={() => setDto({ ...dto, mode: "continuous" })}
                  className="mt-1"
                />
                <div>
                  <div className="text-sm font-medium text-aegis-900 dark:text-aegis-100">
                    {t("settings.mode.continuous")}
                  </div>
                  <div className="text-xs text-aegis-500 dark:text-aegis-400 mt-0.5">
                    {t("modes.continuous.desc")}
                  </div>
                </div>
              </label>
            </div>
          </div>

          {/* Security toggles */}
          <div className="aegis-card p-5">
            <div className="flex items-center gap-2 mb-3">
              <Shield className="h-4 w-4 text-aegis-accent" />
              <h3 className="text-sm font-semibold text-aegis-900 dark:text-aegis-100">
                {t("security.title")}
              </h3>
            </div>
            <div className="space-y-3">
              <ToggleRow
                label={t("security.monitor")}
                desc="Polls running processes every 15s."
                checked={dto.monitor}
                onChange={(v) => setDto({ ...dto, monitor: v })}
              />
              <ToggleRow
                label={t("security.auto_defense")}
                desc="Quarantines files and kills processes that match threat signatures."
                checked={dto.auto_defense}
                onChange={(v) => setDto({ ...dto, auto_defense: v })}
              />
              <ToggleRow
                label={t("security.scanner")}
                desc="On-demand file hash scanner (EICAR + custom sigs)."
                checked={dto.scanner_enabled}
                onChange={(v) => setDto({ ...dto, scanner_enabled: v })}
              />
              <div>
                <label className="text-xs text-aegis-700 dark:text-aegis-300">
                  Quarantine auto-delete after (days)
                </label>
                <input
                  type="number"
                  min={1}
                  max={365}
                  value={dto.quarantine_auto_delete_days}
                  onChange={(e) =>
                    setDto({ ...dto, quarantine_auto_delete_days: parseInt(e.target.value) || 30 })
                  }
                  className="aegis-input mt-1 w-32"
                />
              </div>
            </div>
          </div>

          {/* Bypass Mode */}
          <div className="aegis-card p-5 border-amber-200 dark:border-amber-900/50 bg-amber-50/30 dark:bg-amber-950/10">
            <div className="flex items-start gap-2 mb-3">
              <AlertTriangle className="h-5 w-5 text-amber-600 dark:text-amber-400 flex-shrink-0 mt-0.5" />
              <div>
                <h3 className="text-sm font-semibold text-amber-900 dark:text-amber-300">
                  {t("settings.bypass_mode")}
                </h3>
                <p className="text-xs text-amber-700 dark:text-amber-400 mt-0.5">
                  {t("settings.bypass_mode.hint")}
                </p>
              </div>
            </div>
            <ToggleRow
              label="Enable bypass mode"
              desc="Skip confirmation for medium/high-risk actions except hard-deny list."
              checked={dto.bypass_mode}
              onChange={(v) => setDto({ ...dto, bypass_mode: v })}
              danger
            />
          </div>

          {/* Autonomous (dangerous) */}
          <div className="aegis-card p-5 border-red-200 dark:border-red-900/50 bg-red-50/30 dark:bg-red-950/10">
            <div className="flex items-start gap-2 mb-3">
              <AlertTriangle className="h-5 w-5 text-red-600 dark:text-red-400 flex-shrink-0 mt-0.5" />
              <div>
                <h3 className="text-sm font-semibold text-red-900 dark:text-red-300">
                  {t("settings.allow_autonomous")}
                </h3>
                <p className="text-xs text-red-700 dark:text-red-400 mt-0.5">
                  {t("settings.allow_autonomous.hint")}
                </p>
              </div>
            </div>
            <ToggleRow
              label="Enable autonomous mode"
              desc="Skip safety confirmation for ALL actions."
              checked={dto.allow_autonomous}
              onChange={(v) => setDto({ ...dto, allow_autonomous: v })}
              danger
            />
          </div>

          {/* Database encryption (Phase 2.5) */}
          <div className="aegis-card p-5">
            <div className="flex items-center gap-2 mb-3">
              <Database className="h-4 w-4 text-aegis-accent" />
              <h3 className="text-sm font-semibold text-aegis-900 dark:text-aegis-100">
                {t("settings.encryption")}
              </h3>
            </div>
            {encryption && (
              <div className="text-sm">
                {encryption.status === "not_supported" && (
                  <p className="text-aegis-500 dark:text-aegis-400">
                    {t("settings.encryption.not_supported")}
                  </p>
                )}
                {encryption.status === "disabled" && (
                  <p className="text-aegis-500 dark:text-aegis-400">
                    {t("settings.encryption.disabled")}
                  </p>
                )}
                {encryption.status === "enabled" && (
                  <p className="text-emerald-600 dark:text-emerald-400">
                    {t("settings.encryption.enabled")}
                  </p>
                )}
              </div>
            )}
          </div>

          {/* AI Sandbox */}
          <div className="aegis-card p-5">
            <div className="flex items-center gap-2 mb-3">
              <Shield className="h-4 w-4 text-aegis-accent" />
              <h3 className="text-sm font-semibold text-aegis-900 dark:text-aegis-100">
                {t("settings.sandbox")}
              </h3>
            </div>
            <p className="text-xs text-aegis-500 dark:text-aegis-400 mb-3">
              {t("settings.sandbox.hint")}
            </p>
            {sandbox && (
              <div className="space-y-3">
                <ToggleRow
                  label={t("settings.sandbox.enabled")}
                  desc={sandbox.enabled ? "Active — AI writes restricted" : "Disabled — AI can write anywhere"}
                  checked={sandbox.enabled}
                  onChange={async (v) => {
                    await sandboxSetEnabled(v);
                    setSandbox({ ...sandbox, enabled: v });
                  }}
                />
                <div>
                  <div className="text-xs text-aegis-500 dark:text-aegis-400 mb-1.5">
                    Allowed directories
                  </div>
                  <div className="space-y-1 mb-2">
                    {sandbox.allowed_dirs.length === 0 && (
                      <p className="text-xs text-aegis-400 dark:text-aegis-500 italic">
                        No directories allowed yet
                      </p>
                    )}
                    {sandbox.allowed_dirs.map((dir) => (
                      <div
                        key={dir}
                        className="flex items-center gap-2 text-xs font-mono bg-aegis-50 dark:bg-aegis-night-50 px-2 py-1.5 rounded"
                      >
                        <span className="flex-1 truncate text-aegis-700 dark:text-aegis-300">
                          {dir}
                        </span>
                        <button
                          onClick={async () => {
                            await sandboxRemoveDir(dir);
                            setSandbox({ ...sandbox, allowed_dirs: sandbox.allowed_dirs.filter((d) => d !== dir) });
                          }}
                          className="p-0.5 rounded text-aegis-400 hover:text-red-500 transition-colors"
                          title="Remove directory"
                        >
                          <FolderMinus className="h-3.5 w-3.5" />
                        </button>
                      </div>
                    ))}
                  </div>
                  <div className="flex gap-2">
                    <input
                      type="text"
                      value={sandboxDir}
                      onChange={(e) => setSandboxDir(e.target.value)}
                      placeholder="/path/to/directory"
                      className="aegis-input flex-1 text-xs"
                    />
                    <button
                      onClick={async () => {
                        if (!sandboxDir.trim()) return;
                        await sandboxAddDir(sandboxDir.trim());
                        setSandbox({ ...sandbox, allowed_dirs: [...sandbox.allowed_dirs, sandboxDir.trim()] });
                        setSandboxDir("");
                      }}
                      className="aegis-btn"
                    >
                      <FolderPlus className="h-4 w-4" />
                    </button>
                  </div>
                </div>
                <div className="text-xs text-aegis-500 dark:text-aegis-400">
                  Home sub-dirs: {sandbox.allow_home_subdirs ? "Allowed" : "Blocked"}
                </div>
              </div>
            )}
          </div>

          {/* Telemetry */}
          <div className="aegis-card p-5">
            <div className="flex items-center gap-2 mb-3">
              <BarChart3 className="h-4 w-4 text-aegis-accent" />
              <h3 className="text-sm font-semibold text-aegis-900 dark:text-aegis-100">
                {t("settings.telemetry")}
              </h3>
            </div>
            <p className="text-xs text-aegis-500 dark:text-aegis-400 mb-3">
              {t("settings.telemetry.hint")}
            </p>
            {tele && (
              <div className="space-y-3">
                <ToggleRow
                  label={t("settings.telemetry.enabled")}
                  desc={tele.enabled ? "Anonymous metrics are being sent" : "No metrics are being collected"}
                  checked={tele.enabled}
                  onChange={async () => {
                    if (tele.enabled) {
                      await telemetryOptOut();
                    } else {
                      await telemetryOptIn();
                    }
                    setTele({ ...tele, enabled: !tele.enabled });
                  }}
                />
                <div className="text-xs text-aegis-500 dark:text-aegis-400 space-y-1">
                  <p>Install ID: <code className="font-mono">{tele.install_id}</code></p>
                  {tele.pending_count > 0 && (
                    <p>Pending events: {tele.pending_count}</p>
                  )}
                </div>
              </div>
            )}
          </div>

          {/* Data & Privacy */}
          <div className="aegis-card p-5">
            <div className="flex items-center gap-2 mb-3">
              <FileText className="h-4 w-4 text-aegis-accent" />
              <h3 className="text-sm font-semibold text-aegis-900 dark:text-aegis-100">
                {t("settings.data_privacy")}
              </h3>
            </div>
            <div className="space-y-2">
              <div>
                <div className="text-xs text-aegis-500 dark:text-aegis-400 mb-1.5">
                  {t("settings.audit.export")}
                </div>
                <div className="flex gap-2">
                  <button
                    onClick={() => exportAudit("json")}
                    disabled={exportingAudit}
                    className="aegis-btn flex-1"
                  >
                    {exportingAudit ? (
                      <Loader className="h-4 w-4 animate-spin" />
                    ) : (
                      <Download className="h-4 w-4" />
                    )}
                    {t("settings.audit.export.json")}
                  </button>
                  <button
                    onClick={() => exportAudit("csv")}
                    disabled={exportingAudit}
                    className="aegis-btn flex-1"
                  >
                    <Download className="h-4 w-4" />
                    {t("settings.audit.export.csv")}
                  </button>
                </div>
              </div>

              <button
                onClick={exportAll}
                disabled={exportingData}
                className="aegis-btn w-full"
              >
                {exportingData ? (
                  <Loader className="h-4 w-4 animate-spin" />
                ) : (
                  <Download className="h-4 w-4" />
                )}
                {t("settings.data.export")}
              </button>

              <button
                onClick={forgetAll}
                disabled={forgetting}
                className="aegis-btn-danger w-full"
              >
                {forgetting ? (
                  <Loader className="h-4 w-4 animate-spin" />
                ) : (
                  <Trash2 className="h-4 w-4" />
                )}
                {t("settings.data.forget")}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

function ToggleRow({
  label,
  desc,
  checked,
  onChange,
  danger,
}: {
  label: string;
  desc: string;
  checked: boolean;
  onChange: (v: boolean) => void;
  danger?: boolean;
}) {
  return (
    <div className="flex items-center justify-between gap-3 py-1">
      <div>
        <div
          className={`text-sm font-medium ${
            danger
              ? "text-amber-900 dark:text-amber-300"
              : "text-aegis-800 dark:text-aegis-200"
          }`}
        >
          {label}
        </div>
        <div className="text-xs text-aegis-500 dark:text-aegis-400">{desc}</div>
      </div>
      <button
        type="button"
        className="aegis-toggle"
        data-checked={checked ? "true" : "false"}
        onClick={() => onChange(!checked)}
      >
        <span className="aegis-toggle-knob" />
      </button>
    </div>
  );
}
