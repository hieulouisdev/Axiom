import { useState } from "react";
import {
  BookOpen,
  MessageSquare,
  Cpu,
  Database,
  Lock,
  Mic,
  Globe,
  Keyboard,
  Wrench,
  Rocket,
  Bot,
  ChevronRight,
  type LucideIcon,
} from "lucide-react";
import { t } from "../i18n";

interface GuideSection {
  id: string;
  icon: LucideIcon;
  titleKey: string;
  content: () => JSX.Element;
}

function H2({ children }: { children: React.ReactNode }) {
  return (
    <h2 className="text-lg font-semibold text-aegis-900 dark:text-aegis-100 mb-2">
      {children}
    </h2>
  );
}

function H3({ children }: { children: React.ReactNode }) {
  return (
    <h3 className="text-sm font-semibold text-aegis-800 dark:text-aegis-200 mt-4 mb-1.5">
      {children}
    </h3>
  );
}

function P({ children }: { children: React.ReactNode }) {
  return (
    <p className="text-sm text-aegis-700 dark:text-aegis-300 leading-relaxed mb-2">
      {children}
    </p>
  );
}

function Ul({ children }: { children: React.ReactNode }) {
  return (
    <ul className="list-disc list-outside ml-5 space-y-1 text-sm text-aegis-700 dark:text-aegis-300 mb-2">
      {children}
    </ul>
  );
}

function Li({ children }: { children: React.ReactNode }) {
  return <li>{children}</li>;
}

function Code({ children }: { children: React.ReactNode }) {
  return (
    <code className="font-mono text-xs bg-aegis-100 dark:bg-aegis-night-50 px-1.5 py-0.5 rounded">
      {children}
    </code>
  );
}

function CodeBlock({ children }: { children: React.ReactNode }) {
  return (
    <pre className="font-mono text-xs bg-aegis-100 dark:bg-aegis-night-50 p-3 rounded-lg overflow-x-auto mb-3 text-aegis-800 dark:text-aegis-200">
      {children}
    </pre>
  );
}

const sections: GuideSection[] = [
  {
    id: "overview",
    icon: BookOpen,
    titleKey: "guide.overview",
    content: OverviewContent,
  },
  {
    id: "getting_started",
    icon: Rocket,
    titleKey: "guide.getting_started",
    content: GettingStartedContent,
  },
  {
    id: "chat",
    icon: MessageSquare,
    titleKey: "guide.chat",
    content: ChatContent,
  },
  {
    id: "providers",
    icon: Cpu,
    titleKey: "guide.providers",
    content: ProvidersContent,
  },
  {
    id: "agent",
    icon: Bot,
    titleKey: "guide.agent",
    content: AgentContent,
  },
  {
    id: "memory",
    icon: Database,
    titleKey: "guide.memory",
    content: MemoryContent,
  },
  {
    id: "security",
    icon: Lock,
    titleKey: "guide.security",
    content: SecurityContent,
  },
  {
    id: "voice",
    icon: Mic,
    titleKey: "guide.voice",
    content: VoiceContent,
  },
  {
    id: "web",
    icon: Globe,
    titleKey: "guide.web",
    content: WebContent,
  },
  {
    id: "shortcuts",
    icon: Keyboard,
    titleKey: "guide.shortcuts",
    content: ShortcutsContent,
  },
  {
    id: "troubleshooting",
    icon: Wrench,
    titleKey: "guide.troubleshooting",
    content: TroubleshootingContent,
  },
];

function OverviewContent() {
  return (
    <>
      <H2>{t("guide.overview")}</H2>
      <P>
        Aegis AI is a security-first, cross-platform AI assistant built with
        Tauri 2.0 and Rust. It runs natively on macOS, Linux, and Windows,
        bringing the power of large language models directly to your desktop
        while keeping your data private and your machine safe.
      </P>
      <H3>Key Features</H3>
      <Ul>
        <Li>
          <strong>90+ AI Providers</strong> — Connect to OpenAI, Anthropic,
          Google, Mistral, Cohere, and dozens more. Switch providers instantly
          with zero config friction.
        </Li>
        <Li>
          <strong>Computer-Use Agent</strong> — 28 built-in tools let the AI
          read and write files, execute shell commands, open applications, take
          screenshots, and more — all gated by a safety policy.
        </Li>
        <Li>
          <strong>Real-time Security</strong> — Process monitor, auto-defense,
          virus scanner with YARA rules, network anomaly detection, and file
          integrity monitoring keep your machine protected 24/7.
        </Li>
        <Li>
          <strong>On-device Memory</strong> — Conversations and a knowledge base
          are stored locally with optional SQLCipher encryption. Auto entity
          extraction turns chat into reusable facts.
        </Li>
        <Li>
          <strong>Voice I/O</strong> — Push-to-talk (Ctrl+Space), speech-to-text,
          and text-to-speech for hands-free interaction.
        </Li>
        <Li>
          <strong>Web Search</strong> — Search DuckDuckGo, fetch and extract
          readable content from any web page, all from within the chat.
        </Li>
        <Li>
          <strong>Bilingual UI</strong> — Full English and Vietnamese support
          with more locales coming soon.
        </Li>
      </Ul>
      <H3>Philosophy</H3>
      <P>
        Aegis AI is built on a simple principle: the AI should never do anything
        on your machine without your explicit permission. Every potentially
        destructive action — file writes, command execution, application
        launches — requires confirmation through a safety gate. You can
        customize the strictness level from "on-demand" (confirm everything) to
        "bypass mode" (auto-approve medium-risk actions) to "autonomous" (skip
        all confirmations — use at your own risk).
      </P>
      <P>
        All data stays on your device by default. API keys are stored in the
        system keychain (macOS Keychain, Linux Secret Service, Windows
        Credential Manager). There is no telemetry unless you opt in. Your
        conversations are yours alone.
      </P>
    </>
  );
}

function GettingStartedContent() {
  return (
    <>
      <H2>{t("guide.getting_started")}</H2>
      <H3>First Launch</H3>
      <P>
        When you first open Aegis AI, you'll see the chat view with a welcome
        message. Before you can start chatting with an AI, you need to
        configure at least one provider with a valid API key. The app ships
        with a built-in provider called <Code>Aegis Cloud</Code> that offers
        free-tier access to get you started immediately.
      </P>
      <H3>Configuring a Provider</H3>
      <P>
        Navigate to <strong>AI Providers</strong> in the sidebar. You'll see
        the full catalog organized by category:
      </P>
      <Ul>
        <Li>
          <strong>Cloud — major</strong>: OpenAI, Anthropic, Google Gemini,
          Azure, AWS Bedrock
        </Li>
        <Li>
          <strong>Cloud — other</strong>: Mistral, Cohere, Together AI,
          Groq, Perplexity, DeepSeek, and 80+ more
        </Li>
        <Li>
          <strong>Local</strong>: Ollama, LM Studio, llama.cpp
        </Li>
        <Li>
          <strong>Custom</strong>: Any OpenAI-compatible endpoint
        </Li>
      </Ul>
      <P>
        Click <strong>Configure</strong> on any provider. Enter your API key —
        it will be stored securely in the system keychain, never in a config
        file. Optionally set a custom base URL or model. Click{" "}
        <strong>Test connection</strong> to verify everything works, then{" "}
        <strong>Set as active</strong> to make it your default.
      </P>
      <H3>Your First Chat</H3>
      <P>
        Return to the <strong>Chat</strong> view. Type a message and hit Enter
        (or click Send). The AI will respond using your active provider. If
        the model supports tools, Aegis will automatically enable the
        computer-use agent — you can ask it to read files, run commands, or
        search the web.
      </P>
      <CodeBlock>{`You: What files are in my home directory?\nAI: I'll list the files in your home directory.\n  [Action requested: Run "ls ~"]\n  [You confirm → AI proceeds]\n  AI: Here are the files: Desktop, Documents, Downloads, …`}</CodeBlock>
      <P>
        Congratulations — you're up and running! Explore the other sections of
        this guide to learn about advanced features.
      </P>
    </>
  );
}

function ChatContent() {
  return (
    <>
      <H2>{t("guide.chat")}</H2>
      <H3>Starting a Conversation</H3>
      <P>
        Click <strong>New conversation</strong> or press{" "}
        <Code>Ctrl+N</Code> to start fresh. Each conversation is independent
        — it has its own context window and message history. Conversations are
        automatically saved and can be browsed in the <strong>Memory</strong>{" "}
        view.
      </P>
      <H3>Streaming Responses</H3>
      <P>
        By default, Aegis streams AI responses token-by-token, giving you
        real-time feedback as the model generates text. The streaming
        indicator appears as a pulsing dot next to the model name. If your
        provider doesn't support streaming, the full response will appear at
        once when generation completes.
      </P>
      <H3>Stop Generation</H3>
      <P>
        While the AI is generating, a <strong>Stop</strong> button replaces
        the Send button. Click it or press <Code>Escape</Code> to cancel
        generation immediately. The partial response is preserved in the
        conversation so you don't lose any work.
      </P>
      <H3>Markdown Rendering</H3>
      <P>
        AI responses are rendered as rich Markdown with support for:
      </P>
      <Ul>
        <Li>Headings (h1–h4)</Li>
        <Li>Bold, italic, and strikethrough text</Li>
        <Li>Numbered and bulleted lists</Li>
        <Li>Fenced code blocks with syntax highlighting</Li>
        <Li>Inline code</Li>
        <Li>Tables</Li>
        <Li>Links (clickable, open in system browser)</Li>
        <Li>Blockquotes</Li>
      </Ul>
      <H3>Copying & Regenerating</H3>
      <P>
        Hover over any AI message to reveal action buttons. <strong>Copy</strong>{" "}
        copies the raw Markdown to clipboard. <strong>Regenerate</strong> asks
        the model to produce a new response for the same prompt — useful if
        the first attempt was unsatisfactory.
      </P>
      <H3>Tool Actions in Chat</H3>
      <P>
        When the AI requests a tool action (e.g., running a command or
        writing a file), a confirmation card appears inline in the chat.
        Review the action summary, then click <strong>Confirm</strong> or{" "}
        <strong>Deny</strong>. The AI can only proceed with your approval
        unless you've enabled bypass or autonomous mode.
      </P>
    </>
  );
}

function ProvidersContent() {
  return (
    <>
      <H2>{t("guide.providers")}</H2>
      <H3>Provider Catalog</H3>
      <P>
        Aegis AI ships with 90+ pre-configured providers, each with default
        base URLs and model names. You don't need to manually enter connection
        details — just supply an API key and you're ready to go. The catalog
        is organized into four categories:
      </P>
      <Ul>
        <Li>
          <strong>Cloud — major</strong>: The biggest names in AI — OpenAI
          (GPT-4o, o1, o3), Anthropic (Claude 3.5/4), Google (Gemini 2.0/2.5),
          Azure OpenAI, AWS Bedrock
        </Li>
        <Li>
          <strong>Cloud — other</strong>: Mistral, Cohere, Together AI, Groq,
          Perplexity, DeepSeek, Fireworks, Anyscale, Cerebras, Replicate,
          and 70+ more
        </Li>
        <Li>
          <strong>Local</strong>: Ollama, LM Studio, llama.cpp server — run
          models on your own hardware for zero-cost, fully private inference
        </Li>
        <Li>
          <strong>Custom</strong>: Any OpenAI-compatible API endpoint. Point
          to your own server or a proxy.
        </Li>
      </Ul>
      <H3>Aegis Cloud (Built-in)</H3>
      <P>
        The <Code>Aegis Cloud</Code> provider is included out of the box. It
        offers a free tier with rate limits, perfect for trying out the app
        without any API keys. No configuration required — it's active by
        default on first launch.
      </P>
      <H3>API Key Security</H3>
      <P>
        API keys are never stored in plaintext configuration files. Instead,
        they're saved in your operating system's secure credential store:
      </P>
      <Ul>
        <Li>
          <strong>macOS</strong>: Keychain (Keychain Access → Aegis)
        </Li>
        <Li>
          <strong>Linux</strong>: Secret Service (GNOME Keyring / KDE Wallet)
        </Li>
        <Li>
          <strong>Windows</strong>: Credential Manager
        </Li>
      </Ul>
      <P>
        This means your API keys survive app reinstallation and are protected
        by your OS login. If you need to revoke a key, you can do so from
        the provider configuration screen.
      </P>
      <H3>Switching Providers</H3>
      <P>
        Click <strong>Set as active</strong> on any configured provider to make
        it the default for new conversations. You can also override the
        provider per-conversation by selecting it from the dropdown in the
        chat input area.
      </P>
      <H3>Custom Base URL</H3>
      <P>
        Every provider supports a custom base URL override. This is useful
        for corporate proxies, API gateways, or alternative endpoints. For
        example, set the base URL to your Azure OpenAI deployment endpoint
        instead of the public OpenAI API.
      </P>
    </>
  );
}

function AgentContent() {
  return (
    <>
      <H2>{t("guide.agent")}</H2>
      <H3>28 Built-in Tools</H3>
      <P>
        The computer-use agent equips the AI with 28 tools to interact with
        your machine. These tools are the bridge between the AI's reasoning
        and your operating system. Every tool invocation is logged in the
        audit trail for full accountability.
      </P>
      <Ul>
        <Li>
          <strong>File Operations</strong>: Read file, write file, list
          directory, search files
        </Li>
        <Li>
          <strong>Shell</strong>: Execute command (with timeout and output
          capture)
        </Li>
        <Li>
          <strong>Applications</strong>: Open app, list installed apps
        </Li>
        <Li>
          <strong>Clipboard</strong>: Read clipboard, write clipboard
        </Li>
        <Li>
          <strong>Screen</strong>: Take screenshot, analyze screen content
        </Li>
        <Li>
          <strong>Browser</strong>: Open URL in default browser
        </Li>
        <Li>
          <strong>Web</strong>: Search DuckDuckGo, fetch web pages, extract
          readable content
        </Li>
        <Li>
          <strong>Memory</strong>: Save facts, recall facts, search knowledge
        </Li>
        <Li>
          <strong>System</strong>: Get system info, monitor processes
        </Li>
      </Ul>
      <H3>Safety Policy</H3>
      <P>
        Every tool action is classified by risk level:
      </P>
      <Ul>
        <Li>
          <strong>Low</strong> (auto-approved): Read operations, list
          directory, search
        </Li>
        <Li>
          <strong>Medium</strong> (confirmation required): Write files, open
          URLs, clipboard write
        </Li>
        <Li>
          <strong>High</strong> (confirmation required, highlighted):
          Execute commands, delete files
        </Li>
        <Li>
          <strong>Critical / Hard-deny</strong> (always blocked):{" "}
          <Code>rm -rf /</Code>, format disk, kernel module operations,
          and similar destructive commands
        </Li>
      </Ul>
      <H3>Bypass Mode</H3>
      <P>
        When bypass mode is enabled in Settings, medium and high-risk actions
        are auto-approved (except the hard-deny list). This is useful for
        automation workflows where you trust the AI but want protection
        against catastrophic mistakes. Bypass mode is clearly indicated with
        an amber warning banner.
      </P>
      <H3>Kill Switch</H3>
      <P>
        Press <Code>Ctrl+Shift+K</Code> at any time to immediately kill all
        pending and in-progress tool actions. This is the emergency stop
        button — it cancels streaming, terminates running commands, and
        revokes all pending action tokens. Use it if the AI is doing something
        unexpected.
      </P>
    </>
  );
}

function MemoryContent() {
  return (
    <>
      <H2>{t("guide.memory")}</H2>
      <H3>Conversations</H3>
      <P>
        Every chat message is persisted to a local SQLite database. You can
        browse, search, and resume conversations at any time. The Memory view
        shows all conversations sorted by most recent, with a search bar that
        performs full-text search across all messages.
      </P>
      <H3>Knowledge Base</H3>
      <P>
        Beyond raw conversations, Aegis maintains a structured knowledge base
        of facts extracted from your chats. These are stored as subject-predicate-object
        triples (e.g.,{" "}
        <Code>{"user → prefers_theme → dark"}</Code>). The AI can query
        this knowledge base to personalize responses — it remembers your
        preferences, project details, and any facts you've mentioned.
      </P>
      <H3>Auto Entity Extraction</H3>
      <P>
        After each conversation, Aegis can automatically extract entities and
        relationships from the dialogue. This uses a lightweight NLP pipeline
        (no extra API calls — it runs locally). You can also trigger manual
        extraction from the Memory view. Extracted entities appear in the
        knowledge base and are available for RAG retrieval.
      </P>
      <H3>RAG (Retrieval-Augmented Generation)</H3>
      <P>
        Before sending a prompt to the AI, Aegis automatically searches the
        knowledge base for relevant facts and injects them into the system
        prompt. This means the AI has context from all your previous
        conversations without you having to repeat yourself. The RAG pipeline
        uses semantic similarity scoring to select the most relevant facts.
      </P>
      <H3>Encryption</H3>
      <P>
        If you build Aegis with the <Code>sqlcipher</Code> feature, the
        entire database is encrypted at rest using AES-256. The encryption
        key is derived from your OS keychain. Check Settings → Database
        encryption to see the current status.
      </P>
      <H3>GDPR Compliance</H3>
      <P>
        Aegis provides full data export (JSON) and complete data wipe
        (forget all) functionality. The export includes all conversations,
        knowledge base entries, and audit logs. The wipe permanently deletes
        everything — there is no undo.
      </P>
    </>
  );
}

function SecurityContent() {
  return (
    <>
      <H2>{t("guide.security")}</H2>
      <H3>Process Monitor</H3>
      <P>
        The process monitor polls running processes every 15 seconds
        and compares them against known threat signatures. When a suspicious
        process is detected (e.g., a known crypto miner, reverse shell, or
        malware loader), it's flagged as a threat and logged. You can view
        recent threats in the Security view.
      </P>
      <H3>Auto-Defense</H3>
      <P>
        When auto-defense is enabled, Aegis takes automatic action against
        confirmed threats:
      </P>
      <Ul>
        <Li>Quarantines suspicious files (moved to a secure directory)</Li>
        <Li>Kills malicious processes</Li>
        <Li>Blocks network connections from flagged processes</Li>
        <Li>Emits desktop notifications for critical threats</Li>
      </Ul>
      <P>
        All auto-defense actions are logged in the audit trail. You can review
        and undo any action (e.g., restore a quarantined file) from the
        Security view.
      </P>
      <H3>Virus Scanner</H3>
      <P>
        The on-demand virus scanner checks files against multiple signature
        sources:
      </P>
      <Ul>
        <Li>EICAR test signature</Li>
        <Li>Custom hash-based signatures (SHA-256)</Li>
        <Li>YARA rules (see below)</Li>
      </Ul>
      <P>
        To scan a directory, click <strong>Scan now</strong> and select the
        path. Results show clean and infected files with detection details.
      </P>
      <H3>YARA Rules</H3>
      <P>
        YARA is the industry standard for malware pattern matching. Aegis
        loads YARA rules from a dedicated directory. Drop{" "}
        <Code>.yar</Code> or <Code>.yara</Code> files into the rules
        directory (accessible via <strong>Open rules directory</strong>) and
        they'll be loaded on next scan. The Security view shows how many
        rules are currently loaded.
      </P>
      <H3>Quarantine</H3>
      <P>
        Quarantined files are moved to a secure directory where they can't be
        executed. You can view the quarantine list, restore individual files,
        or let Aegis auto-delete them after a configurable number of days
        (default: 30, configurable in Settings).
      </P>
      <H3>File Integrity Monitoring</H3>
      <P>
        Aegis can maintain a baseline of file hashes for critical paths. If
        a file is modified unexpectedly, an integrity event is raised. Use{" "}
        <strong>Save baseline</strong> to snapshot current hashes, and{" "}
        <strong>Check integrity</strong> to compare against the baseline.
      </P>
      <H3>Network Anomaly Detection</H3>
      <P>
        The network scanner checks for unusual connections — processes
        communicating on unexpected ports, connections to known C2 servers,
        and DNS anomalies. Results appear in the Security view under
        Network Anomalies.
      </P>
    </>
  );
}

function VoiceContent() {
  return (
    <>
      <H2>{t("guide.voice")}</H2>
      <H3>Push-to-Talk</H3>
      <P>
        Hold <Code>Ctrl+Space</Code> to activate the microphone. While held,
        Aegis captures audio from your default input device. Release the key
        to stop recording and send the audio for transcription. The transcribed
        text appears in the chat input and is sent as your message.
      </P>
      <H3>Speech-to-Text (STT)</H3>
      <P>
        Aegis uses OpenAI's Whisper API for speech recognition. This requires
        an active OpenAI provider with a valid API key. Audio is sent as a
        short recording (max 60 seconds per push-to-talk session). The
        transcription supports 50+ languages and handles technical vocabulary
        well.
      </P>
      <P>
        If you don't have an OpenAI key, you can use a local Whisper model
        via Ollama. Configure Ollama as a provider and ensure the{" "}
        <Code>whisper</Code> model is pulled.
      </P>
      <H3>Text-to-Speech (TTS)</H3>
      <P>
        AI responses can be read aloud using TTS. Click the speaker icon on
        any AI message to hear it spoken. Aegis uses OpenAI's TTS API
        (requires OpenAI key) or the system's built-in speech synthesizer as
        a fallback. The voice and speed can be configured in Settings.
      </P>
      <H3>Tips</H3>
      <Ul>
        <Li>
          Speak clearly and at a normal pace for best transcription accuracy
        </Li>
        <Li>
          Use push-to-talk in quiet environments for best results
        </Li>
        <Li>
          If transcription is inaccurate, you can edit the transcribed text
          before sending
        </Li>
        <Li>
          TTS works offline with the system synthesizer; only the OpenAI
          voices require an API call
        </Li>
      </Ul>
    </>
  );
}

function WebContent() {
  return (
    <>
      <H2>{t("guide.web")}</H2>
      <H3>Web Search</H3>
      <P>
        The <strong>Web Search</strong> view lets you search the internet
        directly from Aegis. Results come from DuckDuckGo — no API key
        required. Each result shows the title, URL, and a snippet. Click any
        result to fetch and read the full page content.
      </P>
      <H3>In-Chat Search</H3>
      <P>
        The AI can also search the web during conversations. If you ask a
        question that benefits from current information, the AI will
        automatically invoke the web search tool. Search results are injected
        into the AI's context so it can synthesize an answer grounded in
        real-time data.
      </P>
      <H3>Page Fetching</H3>
      <P>
        Click <strong>Fetch page</strong> on any search result, or provide a
        URL directly. Aegis downloads the page HTML and extracts the main
        content using a readability algorithm (similar to Firefox Reader
        View). The extracted text is displayed in a clean, readable format.
      </P>
      <H3>Raw Fetch</H3>
      <P>
        For programmatic use, the <Code>web_fetch_raw</Code> tool lets the
        AI make arbitrary HTTP requests with custom methods and bodies. This
        is useful for API testing, webhook calls, and fetching non-HTML
        content. Like all tools, it's gated by the safety policy.
      </P>
      <H3>Privacy Notes</H3>
      <Ul>
        <Li>
          DuckDuckGo search does not track queries or create a search profile
        </Li>
        <Li>
          Fetched page content stays local — it's not forwarded to any third
          party beyond the AI provider
        </Li>
        <Li>
          No cookies or tracking headers are sent with fetch requests
        </Li>
      </Ul>
    </>
  );
}

function ShortcutsContent() {
  return (
    <>
      <H2>{t("guide.shortcuts")}</H2>
      <P>
        Aegis AI supports the following keyboard shortcuts:
      </P>
      <div className="space-y-1.5 mb-4">
        <ShortcutRow shortcut="Ctrl+N" description="New conversation" />
        <ShortcutRow shortcut="Ctrl+Enter" description="Send message" />
        <ShortcutRow shortcut="Escape" description="Stop generation / Close dialog" />
        <ShortcutRow shortcut="Ctrl+Space" description="Push-to-talk (hold)" />
        <ShortcutRow shortcut="Ctrl+Shift+K" description="Kill switch — stop all tool actions" />
        <ShortcutRow shortcut="Ctrl+K" description="Focus search" />
        <ShortcutRow shortcut="Ctrl+," description="Open Settings" />
        <ShortcutRow shortcut="Ctrl+B" description="Toggle sidebar" />
        <ShortcutRow shortcut="Ctrl+1–8" description="Switch sidebar views (Chat, Web, Providers, Memory, Security, Modes, Guide, Settings)" />
        <ShortcutRow shortcut="Ctrl+L" description="Clear chat input" />
        <ShortcutRow shortcut="Ctrl+Shift+C" description="Copy last AI response" />
        <ShortcutRow shortcut="Ctrl+R" description="Regenerate last response" />
        <ShortcutRow shortcut="Ctrl+F" description="Search in current view" />
        <ShortcutRow shortcut="Ctrl+D" description="Toggle dark/light theme" />
      </div>
      <P>
        On macOS, use <Code>Cmd</Code> instead of <Code>Ctrl</Code> for all
        shortcuts.
      </P>
    </>
  );
}

function ShortcutRow({ shortcut, description }: { shortcut: string; description: string }) {
  return (
    <div className="flex items-center gap-4 py-1.5 px-3 rounded-lg hover:bg-aegis-50 dark:hover:bg-aegis-night-50">
      <div className="flex gap-1 min-w-[140px]">
        {shortcut.split("+").map((key, i) => (
          <span key={i} className="flex items-center gap-1">
            {i > 0 && <span className="text-aegis-400 text-xs">+</span>}
            <kbd className="px-2 py-0.5 text-xs font-mono bg-aegis-100 dark:bg-aegis-night-50 rounded border border-aegis-200 dark:border-aegis-night-50 text-aegis-800 dark:text-aegis-200">
              {key}
            </kbd>
          </span>
        ))}
      </div>
      <span className="text-sm text-aegis-700 dark:text-aegis-300">
        {description}
      </span>
    </div>
  );
}

function TroubleshootingContent() {
  return (
    <>
      <H2>{t("guide.troubleshooting")}</H2>
      <H3>"No AI provider configured"</H3>
      <P>
        You need at least one provider with a valid API key. Go to{" "}
        <strong>AI Providers</strong>, configure a provider, and set it as
        active. The built-in <Code>Aegis Cloud</Code> provider works without
        any key.
      </P>
      <H3>"Connection failed" when testing a provider</H3>
      <Ul>
        <Li>
          Verify your API key is correct — check for trailing spaces or
          missing characters
        </Li>
        <Li>
          Check your internet connection and any VPN/proxy settings
        </Li>
        <Li>
          If using a custom base URL, ensure it's a valid OpenAI-compatible
          endpoint
        </Li>
        <Li>
          Some providers require regional endpoints (e.g., Azure OpenAI) —
          set the correct base URL
        </Li>
      </Ul>
      <H3>Streaming stops mid-response</H3>
      <P>
        This usually indicates a network interruption or the provider hitting
        a rate limit. Try regenerating the response. If the problem persists,
        switch to a different provider or model with higher rate limits.
      </P>
      <H3>High CPU usage from process monitor</H3>
      <P>
        The process monitor polls every 15 seconds. On systems with thousands
        of processes, this can use noticeable CPU. You can disable the
        process monitor in Settings → Security toggles if you don't need
        real-time threat detection.
      </P>
      <H3>API key not found after reinstall</H3>
      <P>
        API keys are stored in the system keychain, which persists across app
        reinstallations. If keys are missing, you may have cleared your
        keychain. Re-enter the keys in the provider configuration screen.
      </P>
      <H3>Dark mode doesn't apply to some elements</H3>
      <P>
        Aegis uses Tailwind's dark mode with class-based toggling. If some
        elements don't switch, try toggling the theme twice (Light → Dark) to
        force a refresh. Report any persistent issues as a bug.
      </P>
      <H3>Linux: Secret Service not available</H3>
      <P>
        On Linux without GNOME Keyring or KDE Wallet, the keychain backend
        falls back to a plaintext file. Install{" "}
        <Code>gnome-keyring</Code> or <Code>kwallet</Code> for secure key
        storage:
      </P>
      <CodeBlock>{`# Debian/Ubuntu\nsudo apt install gnome-keyring\n\n# Fedora\nsudo dnf install gnome-keyring`}</CodeBlock>
      <H3>Reset to defaults</H3>
      <P>
        If Aegis is in a broken state, you can reset by deleting the config
        and data directories. On most systems, these are in{" "}
        <Code>~/.local/share/aegis-ai/</Code> (Linux) or{" "}
        <Code>~/Library/Application Support/aegis-ai/</Code> (macOS). Back
        up these directories before deleting.
      </P>
    </>
  );
}

export function Guide() {
  const [activeSection, setActiveSection] = useState("overview");
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);

  const current = sections.find((s) => s.id === activeSection) ?? sections[0];

  return (
    <div className="flex-1 flex h-full overflow-hidden">
      {/* Sidebar */}
      <aside
        className={`${
          sidebarCollapsed ? "w-12" : "w-56"
        } h-full bg-white dark:bg-aegis-night-200 border-r border-aegis-200 dark:border-aegis-night-50 flex flex-col transition-all duration-200 overflow-hidden`}
      >
        <div className="p-3 border-b border-aegis-200 dark:border-aegis-night-50">
          <div className="flex items-center justify-between">
            {!sidebarCollapsed && (
              <h2 className="text-sm font-semibold text-aegis-900 dark:text-aegis-100">
                {t("guide.title")}
              </h2>
            )}
            <button
              onClick={() => setSidebarCollapsed(!sidebarCollapsed)}
              className="p-1 rounded text-aegis-500 hover:bg-aegis-100 dark:hover:bg-aegis-night-50 transition-colors"
              aria-label="Toggle guide sidebar"
            >
              <ChevronRight
                className={`h-3.5 w-3.5 transition-transform ${
                  sidebarCollapsed ? "" : "rotate-180"
                }`}
              />
            </button>
          </div>
        </div>
        <nav className="flex-1 p-2 space-y-0.5 overflow-y-auto">
          {sections.map(({ id, icon: Icon, titleKey }) => {
            const active = activeSection === id;
            return (
              <button
                key={id}
                onClick={() => setActiveSection(id)}
                title={sidebarCollapsed ? t(titleKey) : undefined}
                className={`w-full flex items-center gap-2 px-2.5 py-2 rounded-lg text-sm font-medium transition-all
                  ${
                    active
                      ? "bg-gradient-accent text-white shadow-soft"
                      : "text-aegis-700 dark:text-aegis-300 hover:bg-aegis-100 dark:hover:bg-aegis-night-50"
                  }
                  ${sidebarCollapsed ? "justify-center" : ""}
                `}
              >
                <Icon className="h-4 w-4 flex-shrink-0" />
                {!sidebarCollapsed && (
                  <span className="truncate">{t(titleKey)}</span>
                )}
              </button>
            );
          })}
        </nav>
      </aside>

      {/* Content */}
      <main className="flex-1 overflow-y-auto bg-aegis-50 dark:bg-aegis-night-500">
        <div className="max-w-3xl mx-auto px-8 py-6">
          <div className="aegis-card p-6">
            <current.content />
          </div>
        </div>
      </main>
    </div>
  );
}
