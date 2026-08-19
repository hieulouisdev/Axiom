import { useEffect, useRef, useState, useCallback } from "react";
import { Send, Plus, AlertTriangle, Loader2, Square, Copy, Check, Sparkles } from "lucide-react";
import { t } from "../i18n";
import { aiChat, aiChatStream, aiChatCancel } from "../lib/tauri";
import { listen } from "@tauri-apps/api/event";
import { Markdown } from "./Markdown";

interface ChatMessage {
  role: "user" | "assistant" | "system";
  content: string;
  model?: string;
}

export function Chat() {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);
  const [streaming, setStreaming] = useState(false);
  const [streamId, setStreamId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [conversationId, setConversationId] = useState<string | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const unlistenRefs = useRef<Array<() => void>>([]);

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight, behavior: "smooth" });
  }, [messages, loading]);

  // Auto-resize textarea
  useEffect(() => {
    const el = inputRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = Math.min(el.scrollHeight, 160) + "px";
  }, [input]);

  // Set up event listeners for streaming
  useEffect(() => {
    const setupListeners = async () => {
      const unlistenChunk = await listen<{ stream_id: string; delta: string; done: boolean }>(
        "chat://chunk",
        (event) => {
          if (event.payload.done) return;
          setMessages((prev) => {
            const last = prev[prev.length - 1];
            if (last && last.role === "assistant") {
              return [...prev.slice(0, -1), { ...last, content: last.content + event.payload.delta }];
            }
            return [...prev, { role: "assistant", content: event.payload.delta }];
          });
        }
      );

      const unlistenDone = await listen<{ stream_id: string }>(
        "chat://done",
        () => {
          setStreaming(false);
          setLoading(false);
          setStreamId(null);
        }
      );

      const unlistenError = await listen<{ stream_id: string; error: string }>(
        "chat://error",
        (event) => {
          setError(event.payload.error);
          setStreaming(false);
          setLoading(false);
          setStreamId(null);
        }
      );

      const unlistenCancelled = await listen<{ stream_id: string }>(
        "chat://cancelled",
        () => {
          setStreaming(false);
          setLoading(false);
          setStreamId(null);
        }
      );

      unlistenRefs.current = [unlistenChunk, unlistenDone, unlistenError, unlistenCancelled];
    };

    setupListeners();

    return () => {
      unlistenRefs.current.forEach((unlisten) => unlisten());
    };
  }, []);

  const send = async () => {
    const text = input.trim();
    if (!text || loading) return;
    setInput("");
    setError(null);
    setMessages((m) => [...m, { role: "user", content: text }]);
    setLoading(true);

    try {
      // Try streaming first, fall back to regular chat
      try {
        const result = await aiChatStream({
          conversation_id: conversationId,
          user_message: text,
        });
        setConversationId(result.conversation_id);
        setStreamId(result.stream_id);
        setStreaming(true);
        setMessages((m) => [...m, { role: "assistant", content: "" }]);
      } catch {
        // Fallback to non-streaming
        const resp = await aiChat({
          conversation_id: conversationId,
          user_message: text,
        });
        setConversationId(resp.conversation_id);
        setMessages((m) => [
          ...m,
          { role: "assistant", content: resp.content, model: resp.model },
        ]);
        setLoading(false);
      }
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      setError(msg);
      setMessages((m) => [
        ...m,
        {
          role: "assistant",
          content: `⚠️ ${t("common.error")}: ${msg}`,
        },
      ]);
      setLoading(false);
    }
  };

  const stopGeneration = useCallback(async () => {
    if (streamId) {
      try {
        await aiChatCancel(streamId);
      } catch {
        // Stream may already be done
      }
      setStreaming(false);
      setLoading(false);
      setStreamId(null);
    }
  }, [streamId]);

  const newConversation = () => {
    setMessages([]);
    setConversationId(null);
    setError(null);
    setStreamId(null);
    setStreaming(false);
    setLoading(false);
  };

  return (
    <div className="flex-1 flex flex-col h-full">
      <header className="aegis-section-header">
        <div>
          <h2 className="text-lg font-semibold text-aegis-900 dark:text-aegis-100">
            {t("nav.chat")}
          </h2>
          <p className="text-xs text-aegis-500 dark:text-aegis-400 mt-0.5">
            {conversationId
              ? `Conversation: ${conversationId.slice(0, 8)}…`
              : t("app.tagline")}
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
              <Bubble
                key={`${m.role}-${i}-${m.content.length}`}
                message={m}
              />
            ))}
            {loading && !streaming && (
              <div className="flex items-center gap-2 text-sm text-aegis-500 dark:text-aegis-400 px-2">
                <Loader2 className="h-4 w-4 animate-spin" />
                {t("chat.thinking")}
                <span className="inline-flex gap-1">
                  <span className="h-1 w-1 rounded-full bg-aegis-accent animate-pulse-soft" />
                  <span className="h-1 w-1 rounded-full bg-aegis-accent animate-pulse-soft" style={{ animationDelay: "0.2s" }} />
                  <span className="h-1 w-1 rounded-full bg-aegis-accent animate-pulse-soft" style={{ animationDelay: "0.4s" }} />
                </span>
              </div>
            )}
          </div>
        )}
      </div>

      {error && (
        <div className="mx-6 mb-2 p-3 rounded-lg bg-amber-50 dark:bg-amber-950/40 border border-amber-200 dark:border-amber-900 text-amber-800 dark:text-amber-300 text-sm flex items-start gap-2 animate-slide-up">
          <AlertTriangle className="h-4 w-4 flex-shrink-0 mt-0.5" />
          <span>{error}</span>
        </div>
      )}

      <div className="px-6 pb-4">
        <div className="max-w-3xl mx-auto flex items-end gap-2 bg-white dark:bg-aegis-night-100 border border-aegis-200 dark:border-aegis-night-50 rounded-xl p-2 shadow-card focus-within:ring-2 focus-within:ring-aegis-accent/20 focus-within:border-aegis-accent transition-all">
          <textarea
            ref={inputRef}
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
            className="flex-1 resize-none px-2 py-1.5 text-sm bg-transparent focus:outline-hidden placeholder:text-aegis-400 dark:placeholder:text-aegis-500 text-aegis-800 dark:text-aegis-100 max-h-40"
          />
          {streaming ? (
            <button
              onClick={stopGeneration}
              className="aegis-btn-danger !p-2"
              aria-label="Stop generation"
            >
              <Square className="h-4 w-4" />
            </button>
          ) : (
            <button
              onClick={send}
              disabled={!input.trim() || loading}
              className="aegis-btn-primary !p-2"
              aria-label={t("chat.send")}
            >
              <Send className="h-4 w-4" />
            </button>
          )}
        </div>
        <p className="text-[11px] text-aegis-400 dark:text-aegis-500 mt-1.5 text-center">
          Press Enter to send · Shift+Enter for newline · AI can search the web & use tools
        </p>
      </div>
    </div>
  );
}

function EmptyState() {
  return (
    <div className="h-full flex flex-col items-center justify-center text-center px-6 animate-fade-in">
      <div className="relative mb-6">
        <div className="absolute inset-0 bg-gradient-accent blur-2xl opacity-20 rounded-full" />
        <div className="relative h-16 w-16 rounded-2xl bg-gradient-accent flex items-center justify-center shadow-glow animate-bounce-in">
          <Sparkles className="h-7 w-7 text-white" />
        </div>
      </div>
      <h3 className="text-xl font-semibold text-aegis-900 dark:text-aegis-100 mb-2">
        {t("chat.empty.title")}
      </h3>
      <p className="text-sm text-aegis-500 dark:text-aegis-400 max-w-md mb-6">
        {t("chat.empty.subtitle")}
      </p>
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-2 max-w-2xl w-full">
        <FeaturePill text={t("chat.empty.feature1")} />
        <FeaturePill text={t("chat.empty.feature2")} />
        <FeaturePill text={t("chat.empty.feature3")} />
      </div>
    </div>
  );
}

function FeaturePill({ text }: { text: string }) {
  return (
    <div className="px-3 py-2 rounded-lg bg-aegis-50 dark:bg-aegis-night-100 border border-aegis-200 dark:border-aegis-night-50 text-xs text-aegis-700 dark:text-aegis-300">
      {text}
    </div>
  );
}

function Bubble({ message }: { message: ChatMessage }) {
  const isUser = message.role === "user";
  const [copied, setCopied] = useState(false);
  const copy = () => {
    navigator.clipboard?.writeText(message.content).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    });
  };
  return (
    <div className={`flex ${isUser ? "justify-end" : "justify-start"} animate-slide-up group`}>
      <div
        className={`max-w-[80%] px-4 py-2.5 rounded-2xl text-sm leading-relaxed break-words
          ${
            isUser
              ? "bg-gradient-accent text-white rounded-br-sm shadow-soft"
              : "bg-white dark:bg-aegis-night-100 border border-aegis-200 dark:border-aegis-night-50 text-aegis-800 dark:text-aegis-200 rounded-bl-sm shadow-card"
          }`}
      >
        {isUser ? (
          <div className="whitespace-pre-wrap">{message.content}</div>
        ) : (
          <Markdown text={message.content} />
        )}
        <div className="flex items-center justify-between mt-1.5 -mx-1">
          {message.model && !isUser && (
            <div className="text-[10px] opacity-60 px-1">{message.model}</div>
          )}
          {!isUser && !message.content.startsWith("⚠️") && (
            <button
              onClick={copy}
              className="ml-auto opacity-0 group-hover:opacity-100 transition-opacity p-1 rounded hover:bg-black/5 dark:hover:bg-white/10"
              title={t("chat.copy")}
            >
              {copied ? (
                <Check className="h-3 w-3 text-aegis-success" />
              ) : (
                <Copy className="h-3 w-3 opacity-60" />
              )}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
