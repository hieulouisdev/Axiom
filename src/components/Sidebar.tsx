import { useEffect, useState } from "react";
import {
  Shield,
  MessageSquare,
  Cpu,
  Database,
  Lock,
  Settings as SettingsIcon,
  Sun,
  Moon,
  PanelLeftClose,
  PanelLeft,
  Globe,
  BookOpen,
  type LucideIcon,
} from "lucide-react";
import { useStore, type ViewId } from "../store";
import { t } from "../i18n";
import { appVersion } from "../lib/tauri";

interface NavItem {
  id: ViewId;
  icon: LucideIcon;
  key: string;
  badge?: string;
}

const NAV: NavItem[] = [
  { id: "chat", icon: MessageSquare, key: "nav.chat" },
  { id: "web", icon: Globe, key: "nav.web", badge: "new" },
  { id: "providers", icon: Cpu, key: "nav.providers" },
  { id: "memory", icon: Database, key: "nav.memory" },
  { id: "security", icon: Lock, key: "nav.security" },
  { id: "modes", icon: Sun, key: "nav.modes" },
  { id: "guide", icon: BookOpen, key: "nav.guide" },
  { id: "settings", icon: SettingsIcon, key: "nav.settings" },
];

export function Sidebar() {
  const view = useStore((s) => s.view);
  const setView = useStore((s) => s.setView);
  const theme = useStore((s) => s.theme);
  const toggleTheme = useStore((s) => s.toggleTheme);
  const collapsed = useStore((s) => s.sidebarCollapsed);
  const toggleSidebar = useStore((s) => s.toggleSidebar);
  const [version, setVersion] = useState("0.7.0");

  useEffect(() => {
    appVersion().then(setVersion).catch(() => {});
  }, []);

  return (
    <aside
      className={`${
        collapsed ? "w-16" : "w-64"
      } h-full bg-white dark:bg-aegis-night-200 border-r border-aegis-200 dark:border-aegis-night-50 flex flex-col transition-all duration-200`}
    >
      {/* Brand header */}
      <div className="p-4 border-b border-aegis-200 dark:border-aegis-night-50">
        <div className="flex items-center gap-3">
          <div className="h-9 w-9 rounded-xl overflow-hidden shadow-soft flex-shrink-0 ring-1 ring-aegis-200/50 dark:ring-aegis-night-50">
            <img src="/logoapp.png" alt="Aegis AI" className="h-full w-full object-cover" />
          </div>
          {!collapsed && (
            <div className="flex-1 min-w-0 animate-fade-in">
              <h1 className="text-base font-semibold aegis-gradient-text truncate">
                Aegis AI
              </h1>
              <p className="text-[11px] text-aegis-500 dark:text-aegis-400 truncate">
                v{version}
              </p>
            </div>
          )}
          <button
            onClick={toggleSidebar}
            className="p-1.5 rounded-lg text-aegis-500 hover:bg-aegis-100 dark:hover:bg-aegis-night-50 transition-colors"
            aria-label={t("nav.sidebar.toggle")}
            title={t("nav.sidebar.toggle")}
          >
            {collapsed ? (
              <PanelLeft className="h-4 w-4" />
            ) : (
              <PanelLeftClose className="h-4 w-4" />
            )}
          </button>
        </div>
      </div>

      {/* Nav */}
      <nav className="flex-1 p-2 space-y-0.5 overflow-y-auto">
        {NAV.map(({ id, icon: Icon, key, badge }) => {
          const active = view === id;
          return (
            <button
              key={id}
              onClick={() => setView(id)}
              title={collapsed ? t(key) : undefined}
              className={`w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium transition-all group relative
                ${
                  active
                    ? "bg-gradient-accent text-white shadow-soft"
                    : "text-aegis-700 dark:text-aegis-300 hover:bg-aegis-100 dark:hover:bg-aegis-night-50"
                }
                ${collapsed ? "justify-center" : ""}
              `}
            >
              <Icon className="h-4 w-4 flex-shrink-0" />
              {!collapsed && <span className="truncate">{t(key)}</span>}
              {badge && !collapsed && (
                <span
                  className={`ml-auto text-[9px] font-bold uppercase px-1.5 py-0.5 rounded-full ${
                    active
                      ? "bg-white/20 text-white"
                      : "bg-aegis-accent/10 text-aegis-accent dark:bg-aegis-accent/20 dark:text-aegis-accentSoft"
                  }`}
                >
                  {badge}
                </span>
              )}
              {badge && collapsed && (
                <span className="absolute top-1 right-1 h-1.5 w-1.5 rounded-full bg-aegis-accent animate-pulse-soft" />
              )}
            </button>
          );
        })}
      </nav>

      {/* Footer */}
      <div className="p-2 border-t border-aegis-200 dark:border-aegis-night-50 space-y-1">
        <button
          onClick={toggleTheme}
          title={t("nav.theme.toggle")}
          className={`w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium transition-all
            text-aegis-700 dark:text-aegis-300 hover:bg-aegis-100 dark:hover:bg-aegis-night-50
            ${collapsed ? "justify-center" : ""}
          `}
        >
          {theme === "dark" ? (
            <Sun className="h-4 w-4 flex-shrink-0" />
          ) : (
            <Moon className="h-4 w-4 flex-shrink-0" />
          )}
          {!collapsed && (
            <span>
              {theme === "dark" ? t("settings.theme.light") : t("settings.theme.dark")}
            </span>
          )}
        </button>

        {!collapsed && (
          <div className="rounded-lg bg-gradient-accent-soft px-3 py-2.5 text-[11px] text-aegis-600 dark:text-aegis-300 leading-relaxed">
            <div className="flex items-center gap-1.5 font-medium text-aegis-700 dark:text-aegis-200 mb-1">
              <Shield className="h-3 w-3" />
              Security-first AI
            </div>
            <p>
              All actions on your machine require explicit confirmation. Your
              conversations stay on-device.
            </p>
          </div>
        )}
      </div>
    </aside>
  );
}
