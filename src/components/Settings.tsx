import { useEffect, useState } from "react";
import { Save, Loader2, AlertTriangle } from "lucide-react";
import { t } from "../i18n";
import { settingsGet, settingsSet } from "../lib/tauri";
import { useStore } from "../store";
import { setLocale as setI18nLocale } from "../i18n";
import type { SettingsDto } from "../types";

export function Settings() {
  const [dto, setDto] = useState<SettingsDto | null>(null);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const setLocale = useStore((s) => s.setLocale);

  useEffect(() => {
    settingsGet().then(setDto).catch(() => {});
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

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden">
      <header className="px-6 py-4 border-b border-aegis-200 bg-white flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold">{t("settings.title")}</h2>
          <p className="text-xs text-aegis-500 mt-0.5">
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
          {/* Language */}
          <div className="aegis-card p-5">
            <h3 className="text-sm font-semibold mb-3">{t("settings.language")}</h3>
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
                        ? "bg-aegis-accent text-white"
                        : "bg-white border border-aegis-200 text-aegis-700 hover:bg-aegis-50"
                    }`}
                >
                  {l.label}
                </button>
              ))}
            </div>
            <p className="text-[11px] text-aegis-400 mt-2">
              Default: English. Changes apply immediately on Save.
            </p>
          </div>

          {/* Mode */}
          <div className="aegis-card p-5">
            <h3 className="text-sm font-semibold mb-3">{t("settings.mode")}</h3>
            <div className="space-y-2">
              <label className="flex items-start gap-3 cursor-pointer p-3 rounded-lg hover:bg-aegis-50 transition-colors">
                <input
                  type="radio"
                  checked={dto.mode === "ondemand"}
                  onChange={() => setDto({ ...dto, mode: "ondemand" })}
                  className="mt-1"
                />
                <div>
                  <div className="text-sm font-medium">{t("settings.mode.ondemand")}</div>
                  <div className="text-xs text-aegis-500 mt-0.5">{t("modes.ondemand.desc")}</div>
                </div>
              </label>
              <label className="flex items-start gap-3 cursor-pointer p-3 rounded-lg hover:bg-aegis-50 transition-colors">
                <input
                  type="radio"
                  checked={dto.mode === "continuous"}
                  onChange={() => setDto({ ...dto, mode: "continuous" })}
                  className="mt-1"
                />
                <div>
                  <div className="text-sm font-medium">{t("settings.mode.continuous")}</div>
                  <div className="text-xs text-aegis-500 mt-0.5">{t("modes.continuous.desc")}</div>
                </div>
              </label>
            </div>
          </div>

          {/* Security toggles */}
          <div className="aegis-card p-5">
            <h3 className="text-sm font-semibold mb-3">{t("security.title")}</h3>
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
                <label className="text-xs text-aegis-700">
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

          {/* Autonomous (dangerous) */}
          <div className="aegis-card p-5 border-amber-200 bg-amber-50/30">
            <div className="flex items-start gap-2 mb-3">
              <AlertTriangle className="h-5 w-5 text-amber-600 flex-shrink-0 mt-0.5" />
              <div>
                <h3 className="text-sm font-semibold text-amber-900">
                  {t("settings.allow_autonomous")}
                </h3>
                <p className="text-xs text-amber-700 mt-0.5">{t("settings.allow_autonomous.hint")}</p>
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
            danger ? "text-amber-900" : "text-aegis-800"
          }`}
        >
          {label}
        </div>
        <div className="text-xs text-aegis-500">{desc}</div>
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
