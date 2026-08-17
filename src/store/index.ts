import { create } from "zustand";
import type { Locale } from "../i18n";

export type ViewId =
  | "chat"
  | "providers"
  | "memory"
  | "security"
  | "modes"
  | "settings";

interface AppState {
  view: ViewId;
  locale: Locale;
  activeConversationId: string | null;
  setView: (v: ViewId) => void;
  setLocale: (l: Locale) => void;
  setActiveConversation: (id: string | null) => void;
}

export const useStore = create<AppState>((set) => ({
  view: "chat",
  locale: "en",
  activeConversationId: null,
  setView: (v) => set({ view: v }),
  setLocale: (l) => set({ locale: l }),
  setActiveConversation: (id) => set({ activeConversationId: id }),
}));
