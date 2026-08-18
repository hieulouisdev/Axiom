import { useEffect } from "react";
import { Sidebar } from "./components/Sidebar";
import { Chat } from "./components/Chat";
import { Providers } from "./components/Providers";
import { Memory } from "./components/Memory";
import { Security } from "./components/Security";
import { Settings } from "./components/Settings";
import { Modes } from "./components/Modes";
import { Web } from "./components/Web";
import { Guide } from "./components/Guide";
import { useStore } from "./store";
import { i18nGetLocale, i18nSetLocale, settingsGet } from "./lib/tauri";
import { setLocale as setI18nLocale } from "./i18n";

export default function App() {
  const view = useStore((s) => s.view);
  const setLocale = useStore((s) => s.setLocale);

  useEffect(() => {
    (async () => {
      try {
        const settings = await settingsGet();
        const l = settings.language === "vi" ? "vi" : "en";
        setLocale(l);
        setI18nLocale(l);
        await i18nSetLocale(l);
      } catch {
        // Backend not ready (likely dev mode without Tauri).
        const l = (await i18nGetLocale().catch(() => "en")) as "en" | "vi";
        setLocale(l);
        setI18nLocale(l);
      }
    })();
  }, [setLocale]);

  return (
    <div className="flex h-screen w-screen bg-aegis-50 dark:bg-aegis-night-500 text-aegis-900 dark:text-aegis-100 overflow-hidden">
      <Sidebar />
      <main className="flex-1 flex flex-col overflow-hidden bg-aegis-50 dark:bg-aegis-night-500">
        {view === "chat" && <Chat />}
        {view === "web" && <Web />}
        {view === "providers" && <Providers />}
        {view === "memory" && <Memory />}
        {view === "security" && <Security />}
        {view === "modes" && <Modes />}
        {view === "settings" && <Settings />}
        {view === "guide" && <Guide />}
      </main>
    </div>
  );
}
