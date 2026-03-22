import React, { useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeSanitize from "rehype-sanitize";
import rehypeHighlight from "rehype-highlight";
import { Copy, Check } from "lucide-react";
import { Button } from "@heroui/react";
import { cn } from "../../lib/utils";

// Extract plugins to prevent re-creation on every render
const remarkPlugins = [remarkGfm];
const rehypePlugins = [rehypeSanitize, rehypeHighlight];

function extractText(node: any): string {
  if (typeof node === "string") return node;
  if (typeof node === "number") return String(node);
  if (Array.isArray(node)) return node.map(extractText).join("");
  if (node?.props?.children) return extractText(node.props.children);
  return "";
}

function getCodeLanguage(node: any): string | null {
  const className = node?.props?.className || "";
  const match = /language-([\w-]+)/.exec(className);
  return match?.[1] ?? null;
}

function CodeCopyBtn({ children }: { children: any }) {
  const [isCopied, setIsCopied] = useState(false);

  const handleCopy = async () => {
    const text = extractText(children).replace(/\n$/, "");
    if (!text) return;

    try {
      await navigator.clipboard.writeText(text);
      setIsCopied(true);
      window.setTimeout(() => setIsCopied(false), 1800);
    } catch (error) {
      console.error("Failed to copy code block", error);
    }
  };

  return (
    <Button
      type="button"
      size="sm"
      variant="ghost"
      className="docs-code-copy h-6 w-6 p-0 text-muted-foreground hover:text-foreground"
      onClick={handleCopy}
      aria-label={isCopied ? "Code copied" : "Copy code"}
    >
      {isCopied ? <Check className="size-3.5" /> : <Copy className="size-3.5" />}
    </Button>
  );
}

// Extract components outside to maintain referential equality across renders
const markdownComponents: any = {
  // Custom Code Block with Copy Button
  pre: ({ node, children, ...props }: any) => {
    const language = getCodeLanguage(Array.isArray(children) ? children[0] : children);

    return (
      <div className="docs-code-block not-prose my-5 overflow-hidden rounded-[22px]">
        <div className="docs-code-toolbar flex items-center justify-between gap-3 px-3 py-2">
          <div className="flex min-w-0 items-center gap-2">
            <span className="docs-code-language rounded-full px-2 py-0.5 font-mono text-[11px] uppercase tracking-[0.18em]">
              {language ?? "text"}
            </span>
            <span className="docs-code-label truncate text-[11px] font-medium tracking-[0.08em]">
              Code
            </span>
          </div>
          <CodeCopyBtn>{children}</CodeCopyBtn>
        </div>
        <pre
          {...props}
          className="docs-code-content custom-scrollbar overflow-x-auto p-0"
        >
          {children}
        </pre>
      </div>
    );
  }, // Styling other elements
  table: ({ node, ...props }: any) => (
    <div className="docs-table-shell not-prose my-5 overflow-hidden rounded-[24px]">
      <div className="docs-table-scroll overflow-x-auto">
        <table className="docs-table w-full min-w-[640px] border-separate border-spacing-0 text-left text-sm" {...props} />
      </div>
    </div>
  ),
  thead: ({ node, ...props }: any) => (
    <thead
      className="docs-table-head text-muted-foreground font-medium"
      {...props}
    />
  ),
  th: ({ node, ...props }: any) => (
    <th className="docs-table-th px-4 py-3 font-medium" {...props} />
  ),
  td: ({ node, ...props }: any) => (
    <td className="docs-table-td px-4 py-3" {...props} />
  ),
  blockquote: ({ node, ...props }: any) => (
    <blockquote
      className="border-l-4 border-primary/50 pl-4 py-1 italic bg-primary/5 rounded-r-lg my-4"
      {...props}
    />
  ),
};

interface MarkdownRendererProps {
  content: string;
  className?: string;
}

// Use React.memo to prevent unnecessary re-renders of the entire Markdown AST
export const MarkdownRenderer = React.memo(function MarkdownRenderer({
  content,
  className = "",
}: MarkdownRendererProps) {
  return (
    <div
      className={cn(
        "docs-markdown prose prose-sm max-w-none dark:prose-invert",
        "prose-headings:font-semibold prose-headings:tracking-tight",
        "prose-a:text-primary prose-a:no-underline hover:prose-a:underline",
        "prose-code:text-primary prose-code:bg-primary/10 prose-code:px-1 prose-code:py-0.5 prose-code:rounded-md",
        "prose-code:before:content-none prose-code:after:content-none",
        "prose-pre:border-0 prose-pre:bg-transparent prose-pre:p-0 prose-pre:shadow-none",
        className,
      )}
    >
      <ReactMarkdown
        remarkPlugins={remarkPlugins as any}
        rehypePlugins={rehypePlugins as any}
        components={markdownComponents}
      >
        {content}
      </ReactMarkdown>
    </div>
  );
});
