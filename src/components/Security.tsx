import { useEffect, useState } from "react";
import { Shield, ShieldCheck, Loader2, RefreshCw, Activity, Bug, Lock } from "lucide-react";
import { t } from "../i18n";
import { securityStatus, securityScan, securitySetAutoDefense } from "../lib/tauri";
import type { SecurityStatus as SecurityStatusDto, Threat } from "../types";

export function Security() {
  const [status, setStatus] = useState<SecurityStatusDto | null>(null);
  const [loading, setLoading] = useState(true);
  const [scanPath, setScanPath] = useState("");
  const [scanning, setScanning] = useState(false);
  const [scanResults, setScanResults] = useState<{ total: number; infected: number } | null>(null);

  const refresh = async () => {
    setLoading(true);
    try {
      setStatus(await securityStatus());
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    refresh();
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
      <header className="px-6 py-4 border-b border-aegis-200 bg-white flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold">{t("security.title")}</h2>
          <p className="text-xs text-aegis-500 mt-0.5">
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
              <h3 className="text-sm font-semibold mb-3 flex items-center gap-2">
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
                <div className="text-xs px-3 py-2 rounded-lg bg-aegis-50">
                  Scanned <b>{scanResults.total}</b> files ·{" "}
                  <span
                    className={
                      scanResults.infected > 0 ? "text-red-600 font-semibold" : "text-emerald-600"
                    }
                  >
                    {scanResults.infected} infected
                  </span>
                </div>
              )}
            </div>

            <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
              <div className="aegis-card p-4">
                <h3 className="text-sm font-semibold mb-3 flex items-center gap-2">
                  <Shield className="h-4 w-4 text-red-500" />
                  {t("security.threats.recent")}
                </h3>
                {status.recent_threats.length === 0 ? (
                  <div className="flex flex-col items-center py-6 text-center">
                    <ShieldCheck className="h-8 w-8 text-emerald-500 mb-2" />
                    <p className="text-xs text-aegis-500">No threats detected.</p>
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
                <h3 className="text-sm font-semibold mb-3 flex items-center gap-2">
                  <Lock className="h-4 w-4 text-aegis-accent" />
                  {t("security.quarantine")}
                </h3>
                <p className="text-xs text-aegis-400 px-1">
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
          enabled ? "bg-emerald-50" : "bg-aegis-100"
        }`}
      >
        <Icon className={`h-5 w-5 ${enabled ? "text-emerald-600" : "text-aegis-400"}`} />
      </div>
      <div className="flex-1">
        <div className="text-sm font-medium">{label}</div>
        <div className="text-[11px] text-aegis-500">{enabled ? "Enabled" : "Disabled"}</div>
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
    <div className="px-3 py-2 rounded-lg bg-aegis-50 border border-aegis-200">
      <div className="flex items-start justify-between gap-2">
        <div className="text-xs font-medium text-aegis-800 truncate flex-1">
          {threat.process_name} <span className="text-aegis-400">· pid {threat.pid}</span>
        </div>
        <span className={sevClass}>{t(`severity.${threat.severity}`)}</span>
      </div>
      <div className="text-[11px] text-aegis-500 mt-1 font-mono truncate">{threat.command_line}</div>
      <div className="text-[10px] text-aegis-400 mt-0.5">
        {threat.signature_name} · {new Date(threat.timestamp_ms).toLocaleString()}
      </div>
    </div>
  );
}
