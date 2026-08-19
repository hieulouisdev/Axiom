import { useEffect, useState } from "react";
import {
  Cpu,
  Check,
  X,
  Loader2,
  Star,
  Globe,
  HardDrive,
  Settings2,
} from "lucide-react";
import { t } from "../i18n";
import {
  aiListProviders,
  aiConfigureProvider,
  aiTestProvider,
  aiSetActiveProvider,
} from "../lib/tauri";
import type { ProviderDto } from "../types";

export function Providers() {
  const [providers, setProviders] = useState<ProviderDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [filter, setFilter] = useState<string>("all");
  const [editing, setEditing] = useState<string | null>(null);

  const refresh = async () => {
    setLoading(true);
    try {
      setProviders(await aiListProviders());
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    refresh();
  }, []);

  const categories = [
    { id: "all", label: "All", icon: Cpu },
    { id: "cloudmajor", label: t("providers.category.cloud_major"), icon: Globe },
    { id: "cloudother", label: t("providers.category.cloud_other"), icon: Globe },
    { id: "local", label: t("providers.category.local"), icon: HardDrive },
    { id: "custom", label: t("providers.category.custom"), icon: Settings2 },
  ];

  const filtered =
    filter === "all"
      ? providers
      : providers.filter((p) => p.category.toLowerCase().replace("_", "").includes(filter));

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden">
      <header className="aegis-section-header">
        <div>
          <h2 className="text-lg font-semibold text-aegis-900 dark:text-aegis-100">{t("providers.title")}</h2>
          <p className="text-xs text-aegis-500 dark:text-aegis-400 mt-0.5">
            {providers.filter((p) => p.configured).length} of {providers.length} configured
          </p>
        </div>
      </header>

      <div className="flex-1 overflow-y-auto px-6 py-5">
        <div className="flex gap-2 mb-5 flex-wrap">
          {categories.map((c) => (
            <button
              key={c.id}
              onClick={() => setFilter(c.id)}
              className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-all
                ${
                  filter === c.id
                    ? "bg-gradient-accent text-white shadow-soft"
                    : "bg-white dark:bg-aegis-night-100 border border-aegis-200 dark:border-aegis-night-50 text-aegis-700 dark:text-aegis-300 hover:bg-aegis-50 dark:hover:bg-aegis-night-300"
                }`}
            >
              {c.label}
            </button>
          ))}
        </div>

        {loading ? (
          <div className="flex justify-center py-12">
            <Loader2 className="h-6 w-6 text-aegis-400 animate-spin" />
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
            {filtered.map((p) => (
              <ProviderCard
                key={p.id}
                provider={p}
                onEdit={() => setEditing(p.id)}
                onChange={refresh}
              />
            ))}
          </div>
        )}
      </div>

      {editing && (
        <ProviderEditor
          provider={providers.find((p) => p.id === editing)!}
          onClose={() => setEditing(null)}
          onSaved={refresh}
        />
      )}
    </div>
  );
}

function ProviderCard({
  provider,
  onEdit,
  onChange,
}: {
  provider: ProviderDto;
  onEdit: () => void;
  onChange: () => void;
}) {
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<"idle" | "ok" | "fail">("idle");

  const test = async () => {
    setTesting(true);
    setTestResult("idle");
    try {
      await aiTestProvider(provider.id);
      setTestResult("ok");
    } catch {
      setTestResult("fail");
    } finally {
      setTesting(false);
    }
  };

  const activate = async () => {
    await aiSetActiveProvider(provider.id);
    onChange();
  };

  return (
    <div className="aegis-card p-4 flex flex-col">
      <div className="flex items-start justify-between mb-2">
        <div className="flex-1 min-w-0">
          <h3 className="text-sm font-semibold text-aegis-900 dark:text-aegis-100 truncate">{provider.name}</h3>
          <p className="text-[11px] text-aegis-500 dark:text-aegis-400 mt-0.5">{provider.id}</p>
        </div>
        {provider.is_active && (
          <span className="aegis-badge aegis-badge-low">
            <Star className="h-3 w-3" /> active
          </span>
        )}
      </div>

      <p className="text-xs text-aegis-600 dark:text-aegis-300 mb-3 line-clamp-2">{provider.description}</p>

      <div className="flex flex-wrap gap-1 mb-3">
        {!provider.implemented && (
          <span className="aegis-badge aegis-badge-medium">Phase 2</span>
        )}
        {provider.local && <span className="aegis-badge aegis-badge-info">local</span>}
        {provider.configured && (
          <span className="aegis-badge aegis-badge-low">
            <Check className="h-3 w-3" /> configured
          </span>
        )}
      </div>

      <div className="mt-auto flex gap-1.5">
        <button onClick={onEdit} className="aegis-btn !py-1.5 !px-2.5 text-xs flex-1">
          {t("providers.configure")}
        </button>
        <button
          onClick={test}
          disabled={!provider.implemented || testing}
          className="aegis-btn !py-1.5 !px-2.5 text-xs"
        >
          {testing ? (
            <Loader2 className="h-3 w-3 animate-spin" />
          ) : testResult === "ok" ? (
            <Check className="h-3 w-3 text-emerald-600" />
          ) : testResult === "fail" ? (
            <X className="h-3 w-3 text-red-600" />
          ) : (
            t("providers.test")
          )}
        </button>
        {!provider.is_active && provider.configured && (
          <button
            onClick={activate}
            className="aegis-btn-primary !py-1.5 !px-2.5 text-xs"
          >
            {t("providers.activate")}
          </button>
        )}
      </div>
    </div>
  );
}

function ProviderEditor({
  provider,
  onClose,
  onSaved,
}: {
  provider: ProviderDto;
  onClose: () => void;
  onSaved: () => void;
}) {
  const [apiKey, setApiKey] = useState("");
  const [baseUrl, setBaseUrl] = useState(provider.default_base_url ?? "");
  const [model, setModel] = useState(provider.default_model);
  const [enabled, setEnabled] = useState(provider.enabled);
  const [saving, setSaving] = useState(false);

  const save = async () => {
    setSaving(true);
    try {
      await aiConfigureProvider({
        provider_id: provider.id,
        api_key: apiKey || null,
        base_url: baseUrl || null,
        model: model || null,
        enabled,
      });
      onSaved();
      onClose();
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-aegis-900/40 dark:bg-aegis-950/60 backdrop-blur-xs animate-fade-in">
      <div className="bg-white dark:bg-aegis-night-200 rounded-2xl shadow-elevated w-full max-w-md p-6 animate-slide-up border border-aegis-200 dark:border-aegis-night-50">
        <div className="flex items-start justify-between mb-4">
          <div>
            <h3 className="text-base font-semibold text-aegis-900 dark:text-aegis-100">{provider.name}</h3>
            <p className="text-xs text-aegis-500 dark:text-aegis-400">{provider.description}</p>
          </div>
          <button onClick={onClose} className="text-aegis-400 hover:text-aegis-700 dark:hover:text-aegis-200">
            <X className="h-5 w-5" />
          </button>
        </div>

        <div className="space-y-3">
          {provider.requires_api_key && (
            <div>
              <label className="text-xs font-medium text-aegis-700 dark:text-aegis-300">
                {t("providers.api_key")}
              </label>
              <input
                type="password"
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
                placeholder="sk-…"
                className="aegis-input mt-1"
              />
            </div>
          )}

          <div>
            <label className="text-xs font-medium text-aegis-700 dark:text-aegis-300">
              {t("providers.base_url")}
            </label>
            <input
              value={baseUrl}
              onChange={(e) => setBaseUrl(e.target.value)}
              placeholder="https://…"
              className="aegis-input mt-1 font-mono text-xs"
            />
          </div>

          <div>
            <label className="text-xs font-medium text-aegis-700 dark:text-aegis-300">
              {t("providers.model")}
            </label>
            <input
              value={model}
              onChange={(e) => setModel(e.target.value)}
              list={`models-${provider.id}`}
              className="aegis-input mt-1 font-mono text-xs"
            />
            <datalist id={`models-${provider.id}`}>
              {provider.known_models.map((m) => (
                <option key={m} value={m} />
              ))}
            </datalist>
          </div>

          <label className="flex items-center gap-2 cursor-pointer">
            <button
              type="button"
              className="aegis-toggle"
              data-checked={enabled ? "true" : "false"}
              onClick={() => setEnabled((v) => !v)}
            >
              <span className="aegis-toggle-knob" />
            </button>
            <span className="text-xs text-aegis-700 dark:text-aegis-300">{t("providers.enabled")}</span>
          </label>
        </div>

        <div className="flex justify-end gap-2 mt-5">
          <button onClick={onClose} className="aegis-btn">
            {t("common.cancel")}
          </button>
          <button onClick={save} disabled={saving} className="aegis-btn-primary">
            {saving ? t("common.loading") : t("common.save")}
          </button>
        </div>
      </div>
    </div>
  );
}
