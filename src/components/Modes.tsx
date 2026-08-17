import { useEffect, useState } from "react";
import { Sun, Moon, Loader2 } from "lucide-react";
import { t } from "../i18n";
import { modesGetActive, modesSetMode } from "../lib/tauri";

type Mode = "continuous" | "ondemand";

export function Modes() {
  const [active, setActive] = useState<Mode>("ondemand");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    (async () => {
      try {
        setActive(await modesGetActive());
      } catch {
        // dev mode without Tauri
      } finally {
        setLoading(false);
      }
    })();
  }, []);

  const choose = async (m: Mode) => {
    setSaving(true);
    try {
      await modesSetMode(m);
      setActive(m);
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div className="flex-1 flex justify-center items-center">
        <Loader2 className="h-6 w-6 text-aegis-400 animate-spin" />
      </div>
    );
  }

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden">
      <header className="aegis-section-header">
        <div>
          <h2 className="text-lg font-semibold text-aegis-900 dark:text-aegis-100">{t("modes.title")}</h2>
          <p className="text-xs text-aegis-500 dark:text-aegis-400 mt-0.5">
            Cost optimization: continuous mode keeps the AI warm; on-demand wakes it only when needed.
          </p>
        </div>
      </header>

      <div className="flex-1 overflow-y-auto px-6 py-6">
        <div className="max-w-2xl mx-auto grid grid-cols-1 md:grid-cols-2 gap-4">
          <ModeCard
            active={active === "continuous"}
            onClick={() => choose("continuous")}
            icon={Sun}
            title={t("modes.continuous.title")}
            desc={t("modes.continuous.desc")}
            cost="Higher"
            accent="amber"
          />
          <ModeCard
            active={active === "ondemand"}
            onClick={() => choose("ondemand")}
            icon={Moon}
            title={t("modes.ondemand.title")}
            desc={t("modes.ondemand.desc")}
            cost="Lowest"
            accent="blue"
          />
        </div>

        {saving && (
          <div className="flex justify-center mt-6">
            <Loader2 className="h-4 w-4 text-aegis-400 animate-spin" />
          </div>
        )}

        <div className="max-w-2xl mx-auto mt-6 p-4 rounded-xl bg-aegis-50 dark:bg-aegis-night-100 border border-aegis-200 dark:border-aegis-night-50 text-xs text-aegis-600 dark:text-aegis-300 leading-relaxed">
          <p className="font-semibold text-aegis-700 dark:text-aegis-200 mb-1">How cost saving works</p>
          <ul className="list-disc list-inside space-y-1">
            <li>
              <b>On-demand mode</b> never calls the AI unless you explicitly ask. The security
              monitor keeps running (no AI tokens spent).
            </li>
            <li>
              <b>Continuous mode</b> ticks every 60s and can react to events; pair with a local
              provider (Ollama, LM Studio) to spend $0.
            </li>
            <li>
              When the security monitor escalates a critical threat, the AI is woken briefly to
              draft an explanation even in on-demand mode.
            </li>
          </ul>
        </div>
      </div>
    </div>
  );
}

function ModeCard({
  active,
  onClick,
  icon: Icon,
  title,
  desc,
  cost,
  accent,
}: {
  active: boolean;
  onClick: () => void;
  icon: typeof Sun;
  title: string;
  desc: string;
  cost: string;
  accent: "amber" | "blue";
}) {
  const accentBg = accent === "amber" ? "bg-amber-50 dark:bg-amber-950/40" : "bg-blue-50 dark:bg-blue-950/40";
  const accentText = accent === "amber" ? "text-amber-600 dark:text-amber-400" : "text-blue-600 dark:text-blue-400";
  const accentBorder = active
    ? accent === "amber"
      ? "border-amber-400 ring-2 ring-amber-200 dark:ring-amber-900"
      : "border-blue-400 ring-2 ring-blue-200 dark:ring-blue-900"
    : "border-aegis-200 dark:border-aegis-night-50 hover:border-aegis-300 dark:hover:border-aegis-700";

  return (
    <button
      onClick={onClick}
      className={`text-left p-5 rounded-2xl border-2 transition-all bg-white dark:bg-aegis-night-100 ${accentBorder}`}
    >
      <div className="flex items-center gap-3 mb-3">
        <div className={`h-10 w-10 rounded-xl ${accentBg} flex items-center justify-center`}>
          <Icon className={`h-5 w-5 ${accentText}`} />
        </div>
        <h3 className="text-base font-semibold text-aegis-900 dark:text-aegis-100">{title}</h3>
        {active && (
          <span className={`ml-auto aegis-badge ${accent === "amber" ? "aegis-badge-medium" : "aegis-badge-info"}`}>
            active
          </span>
        )}
      </div>
      <p className="text-xs text-aegis-600 dark:text-aegis-300 leading-relaxed mb-3">{desc}</p>
      <div className="text-[11px] text-aegis-500 dark:text-aegis-400">
        Cost: <span className="font-semibold text-aegis-700 dark:text-aegis-200">{cost}</span>
      </div>
    </button>
  );
}
