import { useEffect, useRef, useState } from "react";
import { Send, Plus, AlertTriangle, Loader2 } from "lucide-react";
import { useStore } from "../store";
import { t } from "../i18n";
import { aiChat, memoryGetConversation } from "../lib/tauri";
import type { Message } from "../types";

interface ChatMessage {
  role: "user" | "assistant" | "system";
  content: string;
  model?: string;
}

export function Chat() {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [conversationId, setConversationId] = useState<string | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight, behavior: "smooth" });
  }, [messages, loading]);

  const send = async () => {
    const text = input.trim();
    if (!text || loading) return;
    setInput("");
    setError(null);
    setMessages((m) => [...m, { role: "user", content: text }]);
    setLoading(true);

    try {
      const resp = await aiChat({
        conversation_id: conversationId,
        user_message: text,
      });
      setConversationId(resp.conversation_id);
      setMessages((m) => [
        ...m,
        { role: "assistant", content: resp.content, model: resp.model },
      ]);
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      setError(msg);
      setMessages((m) => [
        ...m,
        {
          role: "assistant",
          content: `⚠️ ${t("chat.error_no_provider")}\n\nTechnical detail: ${msg}`,
        },
      ]);
    } finally {
      setLoading(false);
    }
  };

  const newConversation = () => {
    setMessages([]);
    setConversationId(null);
    setError(null);
  };

  return (
    <div className="flex-1 flex flex-col h-full">
      <header className="flex items-center justify-between px-6 py-4 border-b border-aegis-200 bg-white">
        <div>
          <h2 className="text-lg font-semibold text-aegis-900">{t("nav.chat")}</h2>
          <p className="text-xs text-aegis-500">
            {conversationId ? `Conversation: ${conversationId.slice(0, 8)}…` : t("app.tagline")}
          </p>
        </div>
        <button onClick={newConversation} className="aegis-btn">
          <Plus className="h-4 w-4" />
          {t("chat.new_conversation")}
        </button>
      </header>

      <div ref={scrollRef} className="flex-1 overflow-y-auto px-6 py-6">
        {messages.length === 0 ? (
          <EmptyState />
        ) : (
          <div className="max-w-3xl mx-auto space-y-4">
            {messages.map((m, i) => (
              <Bubble key={i} message={m} />
            ))}
            {loading && (
              <div className="flex items-center gap-2 text-sm text-aegis-500 px-2">
                <Loader2 className="h-4 w-4 animate-spin" />
                {t("chat.thinking")}
              </div>
            )}
          </div>
        )}
      </div>

      {error && (
        <div className="mx-6 mb-2 p-3 rounded-lg bg-amber-50 border border-amber-200 text-amber-800 text-sm flex items-start gap-2">
          <AlertTriangle className="h-4 w-4 flex-shrink-0 mt-0.5" />
          <span>{error}</span>
        </div>
      )}

      <div className="px-6 pb-4">
        <div className="max-w-3xl mx-auto flex items-end gap-2 bg-white border border-aegis-200 rounded-xl p-2 shadow-card focus-within:ring-2 focus-within:ring-aegis-accent/20">
          <textarea
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                send();
              }
            }}
            placeholder={t("chat.placeholder")}
            rows={1}
            className="flex-1 resize-none px-2 py-1.5 text-sm bg-transparent focus:outline-none placeholder:text-aegis-400 max-h-40"
          />
          <button
            onClick={send}
            disabled={!input.trim() || loading}
            className="aegis-btn-primary !p-2"
            aria-label={t("chat.send")}
          >
            <Send className="h-4 w-4" />
          </button>
        </div>
        <p className="text-[11px] text-aegis-400 mt-1.5 text-center">
          Press Enter to send · Shift+Enter for newline
        </p>
      </div>
    </div>
  );
}

function EmptyState() {
  return (
    <div className="h-full flex flex-col items-center justify-center text-center px-6">
      <div className="h-14 w-14 rounded-2xl bg-aegis-100 flex items-center justify-center mb-4">
        <Send className="h-6 w-6 text-aegis-400" />
      </div>
      <h3 className="text-base font-semibold text-aegis-900 mb-1">{t("chat.empty.title")}</h3>
      <p className="text-sm text-aegis-500 max-w-sm">{t("chat.empty.subtitle")}</p>
    </div>
  );
}

function Bubble({ message }: { message: ChatMessage }) {
  const isUser = message.role === "user";
  return (
    <div className={`flex ${isUser ? "justify-end" : "justify-start"} animate-fade-in`}>
      <div
        className={`max-w-[80%] px-4 py-2.5 rounded-2xl text-sm leading-relaxed whitespace-pre-wrap break-words
          ${
            isUser
              ? "bg-aegis-accent text-white rounded-br-sm"
              : "bg-white border border-aegis-200 text-aegis-800 rounded-bl-sm shadow-card"
          }`}
      >
        {message.content}
        {message.model && !isUser && (
          <div className="mt-1.5 text-[10px] opacity-60">{message.model}</div>
        )}
      </div>
    </div>
  );
}
