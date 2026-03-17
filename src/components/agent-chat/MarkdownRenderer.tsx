import React from "react";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeHighlight from "rehype-highlight";
import "highlight.js/styles/atom-one-dark.css";
import { cn } from "../../lib/utils";

interface MarkdownRendererProps {
  content: string;
  isStreaming?: boolean;
  useContentVisibility?: boolean;
  tone?: "assistant" | "user";
}

const HIGHLIGHT_MAX_CONTENT_LENGTH = 12_000;

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
  pre: ({ node, ...props }) => (
    <pre
      {...props}
      className={cn(
        "max-w-full overflow-x-auto rounded-2xl border p-0 text-[13px] shadow-sm",
        isUser
          ? "border-primary-foreground/15 bg-black/10 text-inherit dark:bg-white/10"
          : "border-border/50 bg-muted",
      )}
    />
  ),
  code: ({ node, className, children, ...props }: any) => {
    const match = /language-(\w+)/.exec(className || "");
    const isInline = !match && !String(children).includes("\n");

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
          "block min-w-0 p-4 text-sm font-mono",
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
        "my-4 w-full max-w-full overflow-x-auto rounded-2xl border",
        isUser ? "border-primary-foreground/15 bg-black/6 dark:bg-white/6" : "border-border/40",
      )}
    >
      <table
        {...props}
        className="w-full min-w-[640px] border-collapse text-left text-xs"
      />
    </div>
  ),
  thead: ({ node, ...props }) => (
    <thead
      {...props}
      className={cn(isUser ? "bg-black/10 dark:bg-white/10" : "bg-background/80")}
    />
  ),
  th: ({ node, ...props }) => (
    <th
      {...props}
      className={cn(
        "border-b border-r px-3 py-2 align-top font-semibold last:border-r-0",
        isUser
          ? "border-primary-foreground/15 text-inherit"
          : "border-border/30 text-foreground",
      )}
    />
  ),
  td: ({ node, ...props }) => (
    <td
      {...props}
      className={cn(
        "border-b border-r px-3 py-2 align-top last:border-r-0",
        isUser
          ? "border-primary-foreground/12 text-inherit/90"
          : "border-border/20 text-foreground/90",
      )}
    />
  ),
  tr: ({ node, ...props }) => (
    <tr
      {...props}
      className={cn(isUser ? "odd:bg-black/6 dark:odd:bg-white/6" : "odd:bg-background/30")}
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
