import { useState, type ReactNode } from "react";
import { Check, Copy } from "lucide-react";

/**
 * Lightweight markdown renderer — handles the common subset used by AI
 * chat responses: headings, bold/italic, inline code, fenced code blocks,
 * bullet/numbered lists, blockquotes, links, and paragraphs.
 *
 * No external dependency — we walk the text line-by-line and build up
 * React nodes. This avoids pulling in react-markdown (and its 100+ KB of
 * transitive deps) for what is a fairly small feature surface.
 */
export function Markdown({ text }: { text: string }) {
  const blocks: ReactNode[] = [];
  const lines = text.split("\n");
  let i = 0;

  while (i < lines.length) {
    const line = lines[i];

    // Fenced code block
    const fenceMatch = line.match(/^```(\w+)?/);
    if (fenceMatch) {
      const lang = fenceMatch[1] || "text";
      const codeLines: string[] = [];
      i++;
      while (i < lines.length && !lines[i].startsWith("```")) {
        codeLines.push(lines[i]);
        i++;
      }
      i++; // skip closing ```
      blocks.push(<CodeBlock key={blocks.length} code={codeLines.join("\n")} lang={lang} />);
      continue;
    }

    // Headings
    const headingMatch = line.match(/^(#{1,6})\s+(.*)$/);
    if (headingMatch) {
      const level = headingMatch[1].length;
      const text = headingMatch[2];
      const sizes = ["text-xl", "text-lg", "text-base", "text-sm", "text-sm", "text-xs"];
      blocks.push(
        <div
          key={blocks.length}
          className={`font-semibold mt-3 mb-1 ${sizes[level - 1]} text-aegis-900 dark:text-aegis-100`}
        >
          {renderInline(text)}
        </div>
      );
      i++;
      continue;
    }

    // Blockquote
    if (line.startsWith("> ")) {
      const quoteLines: string[] = [];
      while (i < lines.length && lines[i].startsWith("> ")) {
        quoteLines.push(lines[i].slice(2));
        i++;
      }
      blocks.push(
        <blockquote
          key={blocks.length}
          className="border-l-2 border-aegis-accent/40 pl-3 my-2 text-aegis-600 dark:text-aegis-400 italic"
        >
          {renderInline(quoteLines.join(" "))}
        </blockquote>
      );
      continue;
    }

    // Unordered list
    if (line.match(/^\s*[-*+]\s+/)) {
      const items: string[] = [];
      while (i < lines.length && lines[i].match(/^\s*[-*+]\s+/)) {
        items.push(lines[i].replace(/^\s*[-*+]\s+/, ""));
        i++;
      }
      blocks.push(
        <ul key={blocks.length} className="list-disc list-inside space-y-1 my-2">
          {items.map((it, idx) => (
            <li key={idx} className="text-aegis-800 dark:text-aegis-200">
              {renderInline(it)}
            </li>
          ))}
        </ul>
      );
      continue;
    }

    // Ordered list
    if (line.match(/^\s*\d+\.\s+/)) {
      const items: string[] = [];
      while (i < lines.length && lines[i].match(/^\s*\d+\.\s+/)) {
        items.push(lines[i].replace(/^\s*\d+\.\s+/, ""));
        i++;
      }
      blocks.push(
        <ol key={blocks.length} className="list-decimal list-inside space-y-1 my-2">
          {items.map((it, idx) => (
            <li key={idx} className="text-aegis-800 dark:text-aegis-200">
              {renderInline(it)}
            </li>
          ))}
        </ol>
      );
      continue;
    }

    // Blank line — paragraph break
    if (line.trim() === "") {
      i++;
      continue;
    }

    // Paragraph (collect until next blank / block)
    const paraLines: string[] = [];
    while (
      i < lines.length &&
      lines[i].trim() !== "" &&
      !lines[i].startsWith("```") &&
      !lines[i].match(/^(#{1,6})\s+/) &&
      !lines[i].startsWith("> ") &&
      !lines[i].match(/^\s*[-*+]\s+/) &&
      !lines[i].match(/^\s*\d+\.\s+/)
    ) {
      paraLines.push(lines[i]);
      i++;
    }
    blocks.push(
      <p key={blocks.length} className="my-1 leading-relaxed">
        {renderInline(paraLines.join(" "))}
      </p>
    );
  }

  return <div className="space-y-0.5">{blocks}</div>;
}

/** Render inline markdown: **bold**, *italic*, `code`, [link](url). */
function renderInline(text: string): ReactNode[] {
  const nodes: ReactNode[] = [];
  let rest = text;
  let key = 0;

  const patterns: { re: RegExp; render: (m: RegExpExecArray) => ReactNode }[] = [
    {
      re: /\*\*([^*]+)\*\*/,
      render: (m) => <strong key={key++} className="font-semibold">{m[1]}</strong>,
    },
    {
      re: /`([^`]+)`/,
      render: (m) => (
        <code
          key={key++}
          className="px-1 py-0.5 rounded bg-aegis-100 dark:bg-aegis-night-300 text-aegis-800 dark:text-aegis-200 text-[0.85em] font-mono"
        >
          {m[1]}
        </code>
      ),
    },
    {
      re: /\*([^*]+)\*/,
      render: (m) => <em key={key++}>{m[1]}</em>,
    },
    {
      re: /\[([^\]]+)\]\(([^)]+)\)/,
      render: (m) => (
        <a
          key={key++}
          href={m[2]}
          target="_blank"
          rel="noopener noreferrer"
          className="text-aegis-accent hover:underline"
        >
          {m[1]}
        </a>
      ),
    },
  ];

  while (rest.length > 0) {
    let earliest: { idx: number; match: RegExpExecArray; render: (m: RegExpExecArray) => ReactNode } | null = null;
    for (const p of patterns) {
      const m = p.re.exec(rest);
      if (m && (earliest === null || m.index < earliest.idx)) {
        earliest = { idx: m.index, match: m, render: p.render };
      }
    }
    if (earliest === null) {
      nodes.push(rest);
      break;
    }
    if (earliest.idx > 0) {
      nodes.push(rest.slice(0, earliest.idx));
    }
    nodes.push(earliest.render(earliest.match));
    rest = rest.slice(earliest.idx + earliest.match[0].length);
  }
  return nodes;
}

function CodeBlock({ code, lang }: { code: string; lang: string }) {
  const [copied, setCopied] = useState(false);
  const copy = () => {
    navigator.clipboard?.writeText(code).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    });
  };
  return (
    <div className="relative group my-2">
      <div className="flex items-center justify-between px-3 py-1.5 bg-aegis-900 text-aegis-300 text-[10px] font-mono uppercase tracking-wide rounded-t-lg">
        <span>{lang}</span>
        <button
          onClick={copy}
          className="flex items-center gap-1 hover:text-white transition-colors"
        >
          {copied ? <Check className="h-3 w-3" /> : <Copy className="h-3 w-3" />}
          {copied ? "Copied" : "Copy"}
        </button>
      </div>
      <pre className="aegis-code-block !mt-0 !rounded-t-none">
        <code>{code}</code>
      </pre>
    </div>
  );
}
