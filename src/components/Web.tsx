import { useState } from "react";
import { Search, ExternalLink, Loader2, FileText, AlertTriangle } from "lucide-react";
import { useStore } from "../store";
import { t } from "../i18n";
import { webSearch, webFetch } from "../lib/tauri";
import type { SearchResult } from "../types";

export function Web() {
  const [query, setQuery] = useState("");
  const [searching, setSearching] = useState(false);
  const [results, setResults] = useState<SearchResult[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [fetchingUrl, setFetchingUrl] = useState<string | null>(null);
  const [pageContent, setPageContent] = useState<{ url: string; text: string } | null>(null);
  const theme = useStore((s) => s.theme);

  const search = async () => {
    const q = query.trim();
    if (!q || searching) return;
    setSearching(true);
    setError(null);
    setResults([]);
    setPageContent(null);
    try {
      const hits = await webSearch(q);
      setResults(hits);
      if (hits.length === 0) {
        setError(t("web.no_results"));
      }
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSearching(false);
    }
  };

  const fetchPage = async (url: string) => {
    setFetchingUrl(url);
    setPageContent(null);
    setError(null);
    try {
      const text = await webFetch(url);
      setPageContent({ url, text });
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setFetchingUrl(null);
    }
  };

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden">
      <header className="aegis-section-header">
        <div>
          <h2 className="text-lg font-semibold text-aegis-900 dark:text-aegis-100">
            {t("web.title")}
          </h2>
          <p className="text-xs text-aegis-500 dark:text-aegis-400 mt-0.5">
            Powered by DuckDuckGo — no API key required.
          </p>
        </div>
      </header>

      <div className="flex-1 overflow-y-auto px-6 py-5">
        <div className="max-w-3xl mx-auto space-y-5">
          {/* Search bar */}
          <div className="flex gap-2">
            <div className="flex-1 flex items-center gap-2 bg-white dark:bg-aegis-night-100 border border-aegis-200 dark:border-aegis-night-50 rounded-xl px-3 shadow-card focus-within:ring-2 focus-within:ring-aegis-accent/20 focus-within:border-aegis-accent transition-all">
              <Search className="h-4 w-4 text-aegis-400 flex-shrink-0" />
              <input
                type="text"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") search();
                }}
                placeholder={t("web.placeholder")}
                className="flex-1 py-2.5 text-sm bg-transparent focus:outline-hidden placeholder:text-aegis-400 dark:placeholder:text-aegis-500 text-aegis-800 dark:text-aegis-100"
              />
            </div>
            <button
              onClick={search}
              disabled={!query.trim() || searching}
              className="aegis-btn-primary"
            >
              {searching ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <Search className="h-4 w-4" />
              )}
              {searching ? t("web.searching") : t("chat.send")}
            </button>
          </div>

          {error && (
            <div className="p-3 rounded-lg bg-amber-50 dark:bg-amber-950/40 border border-amber-200 dark:border-amber-900 text-amber-800 dark:text-amber-300 text-sm flex items-start gap-2">
              <AlertTriangle className="h-4 w-4 flex-shrink-0 mt-0.5" />
              <span>{error}</span>
            </div>
          )}

          {/* Search results */}
          {results.length > 0 && (
            <div className="space-y-2 animate-slide-up">
              <div className="text-xs text-aegis-500 dark:text-aegis-400 px-1">
                {results.length} results
              </div>
              {results.map((r, i) => (
                <div
                  key={i}
                  className="aegis-card-hover p-3 cursor-pointer"
                  onClick={() => fetchPage(r.url)}
                >
                  <div className="flex items-start justify-between gap-3">
                    <div className="flex-1 min-w-0">
                      <h3 className="text-sm font-medium text-aegis-accent hover:underline truncate">
                        {r.title}
                      </h3>
                      <p className="text-[11px] text-aegis-400 dark:text-aegis-500 truncate mt-0.5">
                        {r.url}
                      </p>
                      {r.snippet && (
                        <p className="text-xs text-aegis-600 dark:text-aegis-300 mt-1.5 line-clamp-2">
                          {r.snippet}
                        </p>
                      )}
                    </div>
                    <div className="flex items-center gap-1 flex-shrink-0">
                      {fetchingUrl === r.url ? (
                        <Loader2 className="h-4 w-4 animate-spin text-aegis-400" />
                      ) : (
                        <>
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              fetchPage(r.url);
                            }}
                            className="p-1.5 rounded-lg hover:bg-aegis-100 dark:hover:bg-aegis-night-50 text-aegis-500"
                            title={t("web.fetch")}
                          >
                            <FileText className="h-3.5 w-3.5" />
                          </button>
                          <a
                            href={r.url}
                            target="_blank"
                            rel="noopener noreferrer"
                            onClick={(e) => e.stopPropagation()}
                            className="p-1.5 rounded-lg hover:bg-aegis-100 dark:hover:bg-aegis-night-50 text-aegis-500"
                            title="Open in browser"
                          >
                            <ExternalLink className="h-3.5 w-3.5" />
                          </a>
                        </>
                      )}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}

          {/* Page content preview */}
          {pageContent && (
            <div className="aegis-card p-4 animate-slide-up">
              <div className="flex items-center justify-between mb-3 pb-2 border-b border-aegis-200 dark:border-aegis-night-50">
                <div className="flex items-center gap-2 min-w-0">
                  <FileText className="h-4 w-4 text-aegis-accent flex-shrink-0" />
                  <div className="min-w-0">
                    <div className="text-sm font-medium text-aegis-900 dark:text-aegis-100 truncate">
                      {t("web.page_content")}
                    </div>
                    <a
                      href={pageContent.url}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-[11px] text-aegis-accent hover:underline truncate block"
                    >
                      {pageContent.url}
                    </a>
                  </div>
                </div>
                <button
                  onClick={() => setPageContent(null)}
                  className="aegis-btn-ghost text-xs"
                >
                  {t("common.close")}
                </button>
              </div>
              <div
                className={`text-sm whitespace-pre-wrap text-aegis-700 dark:text-aegis-300 max-h-[500px] overflow-y-auto font-mono leading-relaxed ${
                  theme === "dark" ? "text-aegis-300" : ""
                }`}
              >
                {pageContent.text}
              </div>
            </div>
          )}

          {/* Empty state */}
          {!searching && results.length === 0 && !pageContent && !error && (
            <div className="flex flex-col items-center justify-center py-12 text-center">
              <div className="h-14 w-14 rounded-2xl bg-gradient-accent-soft flex items-center justify-center mb-4">
                <Search className="h-6 w-6 text-aegis-accent" />
              </div>
              <h3 className="text-base font-semibold text-aegis-900 dark:text-aegis-100 mb-1">
                {t("web.title")}
              </h3>
              <p className="text-sm text-aegis-500 dark:text-aegis-400 max-w-sm">
                {t("web.placeholder")}
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
