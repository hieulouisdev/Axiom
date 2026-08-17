import { create } from "zustand";
import type { Locale } from "../i18n";

export type ViewId =
  | "chat"
  | "providers"
  | "memory"
  | "security"
  | "modes"
  | "settings"
  | "web";

export type Theme = "light" | "dark";

interface AppState {
  view: ViewId;
  locale: Locale;
  theme: Theme;
  activeConversationId: string | null;
  sidebarCollapsed: boolean;
  setView: (v: ViewId) => void;
  setLocale: (l: Locale) => void;
  setTheme: (t: Theme) => void;
  toggleTheme: () => void;
  setActiveConversation: (id: string | null) => void;
  toggleSidebar: () => void;
}

const THEME_KEY = "aegis-theme";
const SIDEBAR_KEY = "aegis-sidebar-collapsed";

function readStoredTheme(): Theme {
  if (typeof window === "undefined") return "light";
  const stored = window.localStorage.getItem(THEME_KEY);
  if (stored === "dark" || stored === "light") return stored;
  // Default to light — explicit user choice rather than following prefers-color-scheme.
  return "light";
}

function readStoredSidebar(): boolean {
  if (typeof window === "undefined") return false;
  return window.localStorage.getItem(SIDEBAR_KEY) === "1";
}

export const useStore = create<AppState>((set, get) => ({
  view: "chat",
  locale: "en",
  theme: readStoredTheme(),
  activeConversationId: null,
  sidebarCollapsed: readStoredSidebar(),
  setView: (v) => set({ view: v }),
  setLocale: (l) => set({ locale: l }),
  setTheme: (t) => {
    if (typeof window !== "undefined") {
      window.localStorage.setItem(THEME_KEY, t);
      // Toggle the class on <html> so Tailwind's `dark:` variant picks it up.
      const root = document.documentElement;
      if (t === "dark") root.classList.add("dark");
      else root.classList.remove("dark");
    }
    set({ theme: t });
  },
  toggleTheme: () => {
    const next = get().theme === "dark" ? "light" : "dark";
    get().setTheme(next);
  },
  setActiveConversation: (id) => set({ activeConversationId: id }),
  toggleSidebar: () => {
    const next = !get().sidebarCollapsed;
    if (typeof window !== "undefined") {
      window.localStorage.setItem(SIDEBAR_KEY, next ? "1" : "0");
    }
    set({ sidebarCollapsed: next });
  },
}));

// Apply the theme class on <html> as soon as the store is imported.
if (typeof window !== "undefined") {
  const theme = readStoredTheme();
  const root = document.documentElement;
  if (theme === "dark") root.classList.add("dark");
  else root.classList.remove("dark");
}
