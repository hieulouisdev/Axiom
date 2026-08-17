import { useEffect, useState } from "react";
import {
  Shield,
  MessageSquare,
  Cpu,
  Database,
  Lock,
  Settings as SettingsIcon,
  Sun,
  X,
} from "lucide-react";
import { useStore, type ViewId } from "../store";
import { t } from "../i18n";
import { appVersion } from "../lib/tauri";

const NAV: { id: ViewId; icon: typeof Shield; key: string }[] = [
  { id: "chat", icon: MessageSquare, key: "nav.chat" },
  { id: "providers", icon: Cpu, key: "nav.providers" },
  { id: "memory", icon: Database, key: "nav.memory" },
  { id: "security", icon: Lock, key: "nav.security" },
  { id: "modes", icon: Sun, key: "nav.modes" },
  { id: "settings", icon: SettingsIcon, key: "nav.settings" },
];

export function Sidebar() {
  const view = useStore((s) => s.view);
  const setView = useStore((s) => s.setView);
  const [version, setVersion] = useState("0.2.0");

  useEffect(() => {
    appVersion().then(setVersion).catch(() => {});
  }, []);

  return (
    <aside className="w-64 h-full bg-white border-r border-aegis-200 flex flex-col">
      <div className="p-5 border-b border-aegis-200">
        <div className="flex items-center gap-3">
          <div className="h-9 w-9 rounded-xl overflow-hidden shadow-soft">
            <img src="/logoapp.png" alt="Aegis AI" className="h-full w-full object-cover" />
          </div>
          <div className="flex-1 min-w-0">
            <h1 className="text-base font-semibold text-aegis-900 truncate">Aegis AI</h1>
            <p className="text-[11px] text-aegis-500 truncate">v{version}</p>
          </div>
        </div>
      </div>

      <nav className="flex-1 p-3 space-y-1">
        {NAV.map(({ id, icon: Icon, key }) => {
          const active = view === id;
          return (
            <button
              key={id}
              onClick={() => setView(id)}
              className={`w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium transition-all
                ${
                  active
                    ? "bg-aegis-accent text-white shadow-soft"
                    : "text-aegis-700 hover:bg-aegis-100"
                }`}
            >
              <Icon className="h-4 w-4 flex-shrink-0" />
              <span className="truncate">{t(key)}</span>
            </button>
          );
        })}
      </nav>

      <div className="p-3 border-t border-aegis-200">
        <div className="rounded-lg bg-aegis-50 px-3 py-2.5 text-[11px] text-aegis-500 leading-relaxed">
          <p className="font-medium text-aegis-700 mb-1">Security-first AI</p>
          <p>
            All actions on your machine require explicit confirmation. Your conversations stay
            on-device.
          </p>
        </div>
      </div>
    </aside>
  );
}
