import React from "react";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeHighlight from "rehype-highlight";
import { Check, Copy } from "lucide-react";
import { Button } from "@heroui/react";
import { cn } from "../../lib/utils";

interface MarkdownRendererProps {
  content: string;
  isStreaming?: boolean;
  useContentVisibility?: boolean;
  tone?: "assistant" | "user";
}

const HIGHLIGHT_MAX_CONTENT_LENGTH = 12_000;
const COPY_RESET_DELAY_MS = 1_800;

function extractNodeText(node: React.ReactNode): string {
  if (typeof node === "string") return node;
  if (typeof node === "number") return String(node);
  if (Array.isArray(node)) return node.map(extractNodeText).join("");
  if (React.isValidElement(node)) {
    return extractNodeText((node.props as { children?: React.ReactNode }).children);
  }
  return "";
}

function normalizeCodeText(value: string): string {
  return value.replace(/\n$/, "");
}

function getCodeLanguage(className?: string): string | null {
  const match = /language-([\w-]+)/.exec(className || "");
  return match?.[1] ?? null;
}

interface CodeBlockFrameProps {
  children: React.ReactNode;
  className?: string;
  tone: "assistant" | "user";
}

const CodeBlockFrame = React.memo(function CodeBlockFrame({
  children,
  className,
  tone,
}: CodeBlockFrameProps) {
  const [isCopied, setIsCopied] = React.useState(false);
  const codeText = React.useMemo(() => normalizeCodeText(extractNodeText(children)), [children]);
  const language = React.useMemo(() => {
    if (!React.isValidElement(children)) return null;
    return getCodeLanguage((children.props as { className?: string }).className);
  }, [children]);

  React.useEffect(() => {
    if (!isCopied) return;

    const timeoutId = window.setTimeout(() => {
      setIsCopied(false);
    }, COPY_RESET_DELAY_MS);

    return () => window.clearTimeout(timeoutId);
  }, [isCopied]);

  const handleCopy = React.useCallback(async () => {
    if (!codeText) return;

    try {
      await navigator.clipboard.writeText(codeText);
      setIsCopied(true);
    } catch (error) {
      console.error("Failed to copy code block", error);
    }
  }, [codeText]);

  const isUser = tone === "user";

  return (
    <div
      data-tone={tone}
      className={cn(
        "code-block-shell not-prose my-4 overflow-hidden rounded-[22px] text-card-foreground backdrop-blur-md",
        isUser ? "text-inherit" : "text-card-foreground",
      )}
    >
      <div
        className={cn(
          "code-block-toolbar flex items-center justify-between gap-3 px-3 py-2",
        )}
      >
        <div className="flex min-w-0 items-center gap-2">
          <span
            className={cn(
              "code-block-language rounded-full px-2 py-0.5 font-mono text-[11px] uppercase tracking-[0.18em]",
            )}
          >
            {language ?? "text"}
          </span>
          <span
            className={cn(
              "code-block-label truncate text-[11px] font-medium tracking-[0.08em]",
            )}
          >
            Code
          </span>
        </div>

        <Button
          type="button"
          size="sm"
          variant="ghost"
          className={cn(
            "code-block-copy h-6 w-6 shrink-0 p-0 transition-colors",
          )}
          onClick={handleCopy}
          aria-label={isCopied ? "Code copied" : "Copy code"}
        >
          {isCopied ? <Check className="size-3.5" /> : <Copy className="size-3.5" />}
        </Button>
      </div>

      <pre className="code-block-content m-0 max-w-full overflow-x-auto p-0">
        <code
          className={cn(
            className,
            "block min-w-full px-4 py-4 font-mono text-[13px] leading-6 font-normal",
            isUser ? "text-inherit" : "text-foreground/92",
          )}
        >
          {children}
        </code>
      </pre>
    </div>
  );
});

function createMarkdownComponents(
  tone: "assistant" | "user",
): React.ComponentProps<typeof Markdown>["components"] {
  const isUser = tone === "user";

  return {
  a: ({ node, ...props }) => (
    <a
      {...props}
      target="_blank"
      rel="noopener noreferrer"
      className={cn(
        "font-medium underline decoration-from-font underline-offset-4 transition-colors",
        isUser
          ? "text-inherit decoration-current/40 hover:decoration-current/70"
          : "text-primary hover:text-primary/80",
      )}
    />
  ),
  pre: ({ children, className }) => (
    <CodeBlockFrame className={className} tone={tone}>
      {children}
    </CodeBlockFrame>
  ),
  code: ({ node, className, children, ...props }: any) => {
    const isInline =
      !getCodeLanguage(className) && !extractNodeText(children).includes("\n");

    return isInline ? (
      <code
        {...props}
        className={cn(
          className,
          "rounded-md px-1.5 py-0.5 text-[0.9em] font-semibold",
          isUser
            ? "bg-black/12 text-inherit dark:bg-white/12"
            : "bg-primary/10 text-primary",
        )}
      >
        {children}
      </code>
    ) : (
      <code
        {...props}
        className={cn(
          className,
          "hljs bg-transparent p-0 text-inherit",
          isUser ? "text-inherit" : "text-foreground/90",
        )}
      >
        {children}
      </code>
    );
  },
  table: ({ node, ...props }) => (
    <div
      className={cn(
        "chat-table-shell not-prose my-5 w-full max-w-full overflow-hidden rounded-[24px]",
        isUser ? "chat-table-shell-user" : "chat-table-shell-assistant",
      )}
    >
      <div className="chat-table-scroll overflow-x-auto">
        <table
          {...props}
          className="chat-table w-full min-w-[640px] border-separate border-spacing-0 text-left text-xs"
        />
      </div>
    </div>
  ),
  thead: ({ node, ...props }) => (
    <thead
      {...props}
      className={cn("chat-table-head", isUser ? "chat-table-head-user" : "chat-table-head-assistant")}
    />
  ),
  th: ({ node, ...props }) => (
    <th
      {...props}
      className={cn(
        "chat-table-th px-3 py-2.5 align-top font-semibold",
        isUser ? "chat-table-th-user text-inherit" : "chat-table-th-assistant text-foreground",
      )}
    />
  ),
  td: ({ node, ...props }) => (
    <td
      {...props}
      className={cn(
        "chat-table-td px-3 py-2.5 align-top",
        isUser ? "chat-table-td-user text-inherit/90" : "chat-table-td-assistant text-foreground/90",
      )}
    />
  ),
  tr: ({ node, ...props }) => (
    <tr
      {...props}
      className={cn("chat-table-row", isUser ? "chat-table-row-user" : "chat-table-row-assistant")}
    />
  ),
  ul: ({ node, ...props }) => (
    <ul {...props} className="list-disc pl-5 space-y-1 mb-4" />
  ),
  ol: ({ node, ...props }) => (
    <ol {...props} className="list-decimal pl-5 space-y-1 mb-4" />
  ),
  li: ({ node, ...props }) => (
    <li
      {...props}
      className={cn(
        "leading-relaxed",
        isUser
          ? "text-inherit/95 marker:text-primary-foreground/45"
          : "text-foreground/90 marker:text-muted-foreground/50",
      )}
    />
  ),
  blockquote: ({ node, ...props }) => (
    <blockquote
      {...props}
      className={cn(
        "my-4 rounded-r-xl border-l-2 pl-4 italic",
        isUser
          ? "border-primary-foreground/30 bg-black/8 text-inherit/85 dark:bg-white/8"
          : "border-primary/30 bg-primary/5 text-foreground/80",
      )}
    />
  ),
};
}

const MARKDOWN_COMPONENTS_ASSISTANT = createMarkdownComponents("assistant");
const MARKDOWN_COMPONENTS_USER = createMarkdownComponents("user");

const REMARK_PLUGINS = [remarkGfm];
const REHYPE_PLUGINS_HIGHLIGHT = [rehypeHighlight];
const REHYPE_PLUGINS_NONE: [] = [];

export const MarkdownRenderer = React.memo(function MarkdownRenderer({
  content,
  isStreaming = false,
  useContentVisibility = true,
  tone = "assistant",
}: MarkdownRendererProps) {
  const shouldHighlight = !isStreaming && content.length <= HIGHLIGHT_MAX_CONTENT_LENGTH;
  const rehypePlugins = shouldHighlight ? REHYPE_PLUGINS_HIGHLIGHT : REHYPE_PLUGINS_NONE;
  const components =
    tone === "user" ? MARKDOWN_COMPONENTS_USER : MARKDOWN_COMPONENTS_ASSISTANT;
  const style =
    !isStreaming && useContentVisibility
      ? { contentVisibility: "auto" as const, containIntrinsicSize: "800px" }
      : undefined;

  return (
    <div
      className={cn(
        "prose prose-sm max-w-none min-w-0 break-words",
        tone === "user"
          ? "chat-markdown-user text-slate-900 dark:text-slate-50"
          : "chat-markdown-assistant text-foreground dark:prose-invert",
      )}
      style={style}
    >
      <Markdown
        remarkPlugins={REMARK_PLUGINS}
        rehypePlugins={rehypePlugins}
        components={components}
      >
        {content}
      </Markdown>
    </div>
  );
},
(prev, next) =>
  prev.content === next.content &&
  prev.isStreaming === next.isStreaming &&
  prev.useContentVisibility === next.useContentVisibility &&
  prev.tone === next.tone);
