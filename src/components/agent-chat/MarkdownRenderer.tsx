import React, { useMemo } from "react";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeHighlight from "rehype-highlight";
import "highlight.js/styles/atom-one-dark.css";

interface MarkdownRendererProps {
  content: string;
  isStreaming?: boolean;
}

const HIGHLIGHT_MAX_CONTENT_LENGTH = 12_000;

// Hoisted outside component — single allocation, never triggers re-render
const MARKDOWN_COMPONENTS: React.ComponentProps<typeof Markdown>["components"] = {
  a: ({ node, ...props }) => (
    <a
      {...props}
      target="_blank"
      rel="noopener noreferrer"
      className="text-primary hover:text-primary/80 hover:underline transition-colors"
    />
  ),
  pre: ({ node, ...props }) => (
    <pre
      {...props}
      className="max-w-full overflow-x-auto rounded-lg border border-border/50 bg-muted p-0 text-[13px]"
    />
  ),
  code: ({ node, className, children, ...props }: any) => {
    const match = /language-(\w+)/.exec(className || "");
    const isInline = !match && !String(children).includes("\n");

    return isInline ? (
      <code
        {...props}
        className={`${className} bg-primary/10 text-primary font-semibold rounded px-1.5 py-0.5 text-[0.9em]`}
      >
        {children}
      </code>
    ) : (
      <code
        {...props}
        className={`${className} block min-w-0 p-4 text-sm font-mono text-foreground/90`}
      >
        {children}
      </code>
    );
  },
  table: ({ node, ...props }) => (
    <div className="my-4 w-full max-w-full overflow-x-auto rounded-lg border border-border/40">
      <table
        {...props}
        className="w-full min-w-[640px] border-collapse text-left text-xs"
      />
    </div>
  ),
  thead: ({ node, ...props }) => (
    <thead {...props} className="bg-background/80" />
  ),
  th: ({ node, ...props }) => (
    <th
      {...props}
      className="border-b border-r border-border/30 px-3 py-2 align-top font-semibold text-foreground last:border-r-0"
    />
  ),
  td: ({ node, ...props }) => (
    <td
      {...props}
      className="border-b border-r border-border/20 px-3 py-2 align-top text-foreground/90 last:border-r-0"
    />
  ),
  tr: ({ node, ...props }) => (
    <tr {...props} className="odd:bg-background/30" />
  ),
  ul: ({ node, ...props }) => (
    <ul {...props} className="list-disc pl-5 space-y-1 mb-4" />
  ),
  ol: ({ node, ...props }) => (
    <ol {...props} className="list-decimal pl-5 space-y-1 mb-4" />
  ),
  li: ({ node, ...props }) => (
    <li {...props} className="marker:text-muted-foreground/50 text-foreground/90 leading-relaxed" />
  ),
};

const REMARK_PLUGINS = [remarkGfm];
const REHYPE_PLUGINS_HIGHLIGHT = [rehypeHighlight];
const REHYPE_PLUGINS_NONE: [] = [];

export const MarkdownRenderer = React.memo(function MarkdownRenderer({
  content,
  isStreaming = false,
}: MarkdownRendererProps) {
  const shouldHighlight = !isStreaming && content.length <= HIGHLIGHT_MAX_CONTENT_LENGTH;
  const rehypePlugins = shouldHighlight ? REHYPE_PLUGINS_HIGHLIGHT : REHYPE_PLUGINS_NONE;

  const style = useMemo(
    () =>
      isStreaming
        ? undefined
        : { contentVisibility: "auto" as const, containIntrinsicSize: "800px" },
    [isStreaming],
  );

  return (
    <div
      className="prose prose-sm dark:prose-invert max-w-none min-w-0 break-words text-foreground"
      style={style}
    >
      <Markdown
        remarkPlugins={REMARK_PLUGINS}
        rehypePlugins={rehypePlugins}
        components={MARKDOWN_COMPONENTS}
      >
        {content}
      </Markdown>
    </div>
  );
},
(prev, next) =>
  prev.content === next.content && prev.isStreaming === next.isStreaming);
