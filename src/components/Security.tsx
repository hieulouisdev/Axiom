import { useEffect, useState } from "react";
import {
  Shield,
  ShieldCheck,
  Loader2,
  RefreshCw,
  Activity,
  Bug,
  Lock,
  FileSearch,
  FolderOpen,
} from "lucide-react";
import { t } from "../i18n";
import {
  securityStatus,
  securityScan,
  securitySetAutoDefense,
  yaraList,
  yaraEnsureDir,
} from "../lib/tauri";
import type { SecurityStatus as SecurityStatusDto, Threat, YaraRule } from "../types";

export function Security() {
  const [status, setStatus] = useState<SecurityStatusDto | null>(null);
  const [loading, setLoading] = useState(true);
  const [scanPath, setScanPath] = useState("");
  const [scanning, setScanning] = useState(false);
  const [scanResults, setScanResults] = useState<{ total: number; infected: number } | null>(null);
  const [yaraRules, setYaraRules] = useState<YaraRule[]>([]);
  const [yaraLoading, setYaraLoading] = useState(false);

  const refresh = async () => {
    setLoading(true);
    try {
      setStatus(await securityStatus());
    } finally {
      setLoading(false);
    }
  };

  const refreshYara = async () => {
    setYaraLoading(true);
    try {
      await yaraEnsureDir();
      const rules = await yaraList();
      setYaraRules(rules);
    } finally {
      setYaraLoading(false);
    }
  };

  useEffect(() => {
    refresh();
    refreshYara();
    const id = setInterval(refresh, 5000);
    return () => clearInterval(id);
  }, []);

  const toggleAutoDefense = async (enabled: boolean) => {
    await securitySetAutoDefense(enabled);
    await refresh();
  };

  const scan = async () => {
    if (!scanPath) return;
    setScanning(true);
    setScanResults(null);
    try {
      const results = await securityScan(scanPath);
      setScanResults({
        total: results.length,
        infected: results.filter((r) => r.infected).length,
      });
    } finally {
      setScanning(false);
    }
  };

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden">
      <header className="aegis-section-header">
        <div>
          <h2 className="text-lg font-semibold text-aegis-900 dark:text-aegis-100">
            {t("security.title")}
          </h2>
          <p className="text-xs text-aegis-500 dark:text-aegis-400 mt-0.5">
            Passive monitoring + active defense — stays on even in on-demand mode.
          </p>
        </div>
        <button onClick={refresh} className="aegis-btn">
          <RefreshCw className="h-4 w-4" />
        </button>
      </header>

      <div className="flex-1 overflow-y-auto px-6 py-5">
        {loading || !status ? (
          <div className="flex justify-center py-12">
            <Loader2 className="h-6 w-6 text-aegis-400 animate-spin" />
          </div>
        ) : (
          <>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-3 mb-6">
              <ToggleCard
                icon={Activity}
                label={t("security.monitor")}
                enabled={status.monitor}
                onToggle={() => {}}
                disabled
              />
              <ToggleCard
                icon={Shield}
                label={t("security.auto_defense")}
                enabled={status.auto_defense}
                onToggle={toggleAutoDefense}
              />
              <ToggleCard
                icon={Bug}
                label={t("security.scanner")}
                enabled={status.scanner_enabled}
                onToggle={() => {}}
                disabled
              />
            </div>

            <div className="aegis-card p-4 mb-6">
              <h3 className="text-sm font-semibold mb-3 flex items-center gap-2 text-aegis-900 dark:text-aegis-100">
                <Bug className="h-4 w-4 text-aegis-accent" />
                {t("security.scanner")}
              </h3>
              <div className="flex gap-2 mb-3">
                <input
                  value={scanPath}
                  onChange={(e) => setScanPath(e.target.value)}
                  placeholder="/home/user/Downloads or C:\\Users\\..."
                  className="aegis-input flex-1 text-xs font-mono"
                />
                <button
                  onClick={scan}
                  disabled={!scanPath || scanning}
                  className="aegis-btn-primary !text-xs"
                >
                  {scanning ? (
                    <Loader2 className="h-3.5 w-3.5 animate-spin" />
                  ) : (
                    t("security.scan_now")
                  )}
                </button>
              </div>
              {scanResults && (
                <div className="text-xs px-3 py-2 rounded-lg bg-aegis-50 dark:bg-aegis-night-300 text-aegis-700 dark:text-aegis-300">
                  Scanned <b>{scanResults.total}</b> files ·{" "}
                  <span
                    className={
                      scanResults.infected > 0
                        ? "text-red-600 dark:text-red-400 font-semibold"
                        : "text-emerald-600 dark:text-emerald-400"
                    }
                  >
                    {scanResults.infected} infected
                  </span>
                </div>
              )}
            </div>

            {/* YARA rules panel */}
            <div className="aegis-card p-4 mb-6">
              <div className="flex items-center justify-between mb-3">
                <h3 className="text-sm font-semibold flex items-center gap-2 text-aegis-900 dark:text-aegis-100">
                  <FileSearch className="h-4 w-4 text-aegis-accent" />
                  {t("security.yara")}
                </h3>
                <button onClick={refreshYara} className="aegis-btn-ghost text-xs">
                  <RefreshCw className={`h-3 w-3 ${yaraLoading ? "animate-spin" : ""}`} />
                  {t("common.refresh")}
                </button>
              </div>
              {yaraRules.length === 0 ? (
                <div className="flex flex-col items-center py-6 text-center">
                  <FolderOpen className="h-8 w-8 text-aegis-300 dark:text-aegis-600 mb-2" />
                  <p className="text-xs text-aegis-400 dark:text-aegis-500 mb-1">
                    {t("security.yara.empty")}
                  </p>
                  <p className="text-[10px] text-aegis-400 dark:text-aegis-500 font-mono">
                    {`~/.local/share/aegis-ai/yara/`}
                  </p>
                </div>
              ) : (
                <div className="space-y-2">
                  <div className="text-xs text-aegis-500 dark:text-aegis-400 mb-1">
                    {t("security.yara.loaded", { n: yaraRules.length })}
                  </div>
                  {yaraRules.map((r, i) => (
                    <div
                      key={i}
                      className="px-3 py-2 rounded-lg bg-aegis-50 dark:bg-aegis-night-300 border border-aegis-200 dark:border-aegis-night-50"
                    >
                      <div className="flex items-center gap-2">
                        <span className="text-xs font-mono font-medium text-aegis-800 dark:text-aegis-200">
                          {r.name}
                        </span>
                        {r.tags.map((tag) => (
                          <span
                            key={tag}
                            className="text-[9px] uppercase font-bold px-1.5 py-0.5 rounded bg-aegis-accent/10 text-aegis-accent dark:bg-aegis-accent/20"
                          >
                            {tag}
                          </span>
                        ))}
                      </div>
                      <div className="text-[10px] text-aegis-400 dark:text-aegis-500 mt-0.5 font-mono truncate">
                        {r.source.split(/[\\/]/).pop()}
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>

            <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
              <div className="aegis-card p-4">
                <h3 className="text-sm font-semibold mb-3 flex items-center gap-2 text-aegis-900 dark:text-aegis-100">
                  <Shield className="h-4 w-4 text-red-500" />
                  {t("security.threats.recent")}
                </h3>
                {status.recent_threats.length === 0 ? (
                  <div className="flex flex-col items-center py-6 text-center">
                    <ShieldCheck className="h-8 w-8 text-emerald-500 mb-2" />
                    <p className="text-xs text-aegis-500 dark:text-aegis-400">
                      No threats detected.
                    </p>
                  </div>
                ) : (
                  <div className="space-y-2">
                    {status.recent_threats.slice(0, 10).map((t) => (
                      <ThreatRow key={t.id} threat={t} />
                    ))}
                  </div>
                )}
              </div>

              <div className="aegis-card p-4">
                <h3 className="text-sm font-semibold mb-3 flex items-center gap-2 text-aegis-900 dark:text-aegis-100">
                  <Lock className="h-4 w-4 text-aegis-accent" />
                  {t("security.quarantine")}
                </h3>
                <p className="text-xs text-aegis-400 dark:text-aegis-500 px-1">
                  Quarantined files appear here. (Phase 2: full integration with auto-defense.)
                </p>
              </div>
            </div>
          </>
        )}
      </div>
    </div>
  );
}

function ToggleCard({
  icon: Icon,
  label,
  enabled,
  onToggle,
  disabled,
}: {
  icon: typeof Shield;
  label: string;
  enabled: boolean;
  onToggle: (v: boolean) => void;
  disabled?: boolean;
}) {
  return (
    <div className="aegis-card p-4 flex items-center gap-3">
      <div
        className={`h-10 w-10 rounded-lg flex items-center justify-center ${
          enabled
            ? "bg-emerald-50 dark:bg-emerald-950/50"
            : "bg-aegis-100 dark:bg-aegis-night-300"
        }`}
      >
        <Icon
          className={`h-5 w-5 ${
            enabled
              ? "text-emerald-600 dark:text-emerald-400"
              : "text-aegis-400 dark:text-aegis-500"
          }`}
        />
      </div>
      <div className="flex-1">
        <div className="text-sm font-medium text-aegis-900 dark:text-aegis-100">{label}</div>
        <div className="text-[11px] text-aegis-500 dark:text-aegis-400">
          {enabled ? "Enabled" : "Disabled"}
        </div>
      </div>
      <button
        type="button"
        className="aegis-toggle"
        data-checked={enabled ? "true" : "false"}
        onClick={() => !disabled && onToggle(!enabled)}
        disabled={disabled}
      >
        <span className="aegis-toggle-knob" />
      </button>
    </div>
  );
}

function ThreatRow({ threat }: { threat: Threat }) {
  const sevClass = `aegis-badge aegis-badge-${threat.severity}`;
  return (
    <div className="px-3 py-2 rounded-lg bg-aegis-50 dark:bg-aegis-night-300 border border-aegis-200 dark:border-aegis-night-50">
      <div className="flex items-start justify-between gap-2">
        <div className="text-xs font-medium text-aegis-800 dark:text-aegis-200 truncate flex-1">
          {threat.process_name}{" "}
          <span className="text-aegis-400 dark:text-aegis-500">· pid {threat.pid}</span>
        </div>
        <span className={sevClass}>{t(`severity.${threat.severity}`)}</span>
      </div>
      <div className="text-[11px] text-aegis-500 dark:text-aegis-400 mt-1 font-mono truncate">
        {threat.command_line}
      </div>
      <div className="text-[10px] text-aegis-400 dark:text-aegis-500 mt-0.5">
        {threat.signature_name} · {new Date(threat.timestamp_ms).toLocaleString()}
      </div>
    </div>
  );
}
