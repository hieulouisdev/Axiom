import { useEffect, useState } from "react";
import { Database, Search, Trash2, Loader2, MessageSquare, Activity, BookOpen } from "lucide-react";
import { t } from "../i18n";
import { memoryListConversations, memoryGetConversation, memoryStats, memoryClearAll } from "../lib/tauri";
import type { Conversation, MemoryStats, Message } from "../types";

export function Memory() {
  const [stats, setStats] = useState<MemoryStats | null>(null);
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [activeConv, setActiveConv] = useState<string | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = async () => {
    setLoading(true);
    try {
      const [s, c] = await Promise.all([memoryStats(), memoryListConversations(50)]);
      setStats(s);
      setConversations(c);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    refresh();
  }, []);

  const openConv = async (id: string) => {
    setActiveConv(id);
    setMessages(await memoryGetConversation(id));
  };

  const clearAll = async () => {
    if (!confirm("Clear ALL memory? This cannot be undone.")) return;
    await memoryClearAll();
    await refresh();
    setActiveConv(null);
    setMessages([]);
  };

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden">
      <header className="flex items-center justify-between px-6 py-4 border-b border-aegis-200 bg-white">
        <div>
          <h2 className="text-lg font-semibold">{t("memory.title")}</h2>
          <p className="text-xs text-aegis-500 mt-0.5">
            All data stays on your device — SQLite store.
          </p>
        </div>
        <button onClick={clearAll} className="aegis-btn-danger !text-xs !py-1.5">
          <Trash2 className="h-3.5 w-3.5" />
          {t("memory.clear_all")}
        </button>
      </header>

      {loading ? (
        <div className="flex justify-center py-12">
          <Loader2 className="h-6 w-6 text-aegis-400 animate-spin" />
        </div>
      ) : (
        <div className="flex-1 overflow-y-auto px-6 py-5">
          {stats && (
            <div className="grid grid-cols-2 md:grid-cols-4 gap-3 mb-6">
              <StatCard
                icon={MessageSquare}
                label={t("memory.stats.conversations")}
                value={stats.conversations}
              />
              <StatCard
                icon={BookOpen}
                label={t("memory.stats.messages")}
                value={stats.messages}
              />
              <StatCard
                icon={Activity}
                label={t("memory.stats.activities")}
                value={stats.activities}
              />
              <StatCard
                icon={Database}
                label={t("memory.stats.knowledge")}
                value={stats.knowledge}
              />
            </div>
          )}

          <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
            <div className="lg:col-span-1">
              <h3 className="text-xs font-semibold text-aegis-700 uppercase tracking-wide mb-2">
                {t("memory.conversations")}
              </h3>
              <div className="space-y-1.5">
                {conversations.length === 0 ? (
                  <p className="text-xs text-aegis-400 px-1">No conversations yet.</p>
                ) : (
                  conversations.map((c) => (
                    <button
                      key={c.id}
                      onClick={() => openConv(c.id)}
                      className={`w-full text-left px-3 py-2 rounded-lg text-sm transition-all
                        ${
                          activeConv === c.id
                            ? "bg-aegis-accent text-white"
                            : "bg-white border border-aegis-200 hover:bg-aegis-50"
                        }`}
                    >
                      <div className="truncate text-xs font-medium">{c.title || "(untitled)"}</div>
                      <div
                        className={`text-[10px] mt-0.5 ${
                          activeConv === c.id ? "text-blue-100" : "text-aegis-400"
                        }`}
                      >
                        {new Date(c.updated_at_ms).toLocaleString()}
                      </div>
                    </button>
                  ))
                )}
              </div>
            </div>

            <div className="lg:col-span-2">
              <h3 className="text-xs font-semibold text-aegis-700 uppercase tracking-wide mb-2">
                Messages
              </h3>
              <div className="bg-white border border-aegis-200 rounded-xl p-4 min-h-[300px]">
                {messages.length === 0 ? (
                  <div className="flex flex-col items-center justify-center h-full py-8 text-center">
                    <Search className="h-8 w-8 text-aegis-300 mb-2" />
                    <p className="text-xs text-aegis-400">
                      Select a conversation to view its messages.
                    </p>
                  </div>
                ) : (
                  <div className="space-y-2.5">
                    {messages.map((m) => (
                      <div
                        key={m.id}
                        className={`px-3 py-2 rounded-lg text-sm ${
                          m.role === "user"
                            ? "bg-aegis-50 text-aegis-800"
                            : "bg-white border border-aegis-200"
                        }`}
                      >
                        <div className="text-[10px] uppercase tracking-wide text-aegis-500 mb-0.5">
                          {m.role} · {new Date(m.created_at_ms).toLocaleTimeString()}
                        </div>
                        <div className="whitespace-pre-wrap text-aegis-800">{m.content}</div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function StatCard({
  icon: Icon,
  label,
  value,
}: {
  icon: typeof Database;
  label: string;
  value: number;
}) {
  return (
    <div className="aegis-card p-4 flex items-center gap-3">
      <div className="h-10 w-10 rounded-lg bg-aegis-accent/10 flex items-center justify-center">
        <Icon className="h-5 w-5 text-aegis-accent" />
      </div>
      <div>
        <div className="text-xl font-semibold text-aegis-900">{value.toLocaleString()}</div>
        <div className="text-[11px] text-aegis-500">{label}</div>
      </div>
    </div>
  );
}
