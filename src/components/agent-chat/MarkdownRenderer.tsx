import { memo } from "react";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeHighlight from "rehype-highlight";
import "highlight.js/styles/atom-one-dark.css"; // Start with a default dark theme

interface MarkdownRendererProps {
  content: string;
}

// ⚡ Bolt: Move plugins and components outside component body
// Defining these inline causes referential inequality on every render.
// In streaming chat interfaces, this forces react-markdown to tear down
// and recreate the entire AST unnecessarily on every token.
const remarkPlugins = [remarkGfm];
const rehypePlugins = [rehypeHighlight];

const markdownComponents: any = {
  a: ({ node, ...props }: any) => (
    <a
      {...props}
      target="_blank"
      rel="noopener noreferrer"
      className="text-primary hover:text-primary/80 hover:underline transition-colors"
    />
  ),
  pre: ({ node, ...props }: any) => (
    <pre
      {...props}
      className="bg-muted border border-border/50 rounded-lg p-0 overflow-x-auto text-[13px]"
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
        className={`${className} block p-4 text-sm font-mono text-foreground/90`}
      >
        {children}
      </code>
    );
  },
  ul: ({ node, ...props }: any) => (
    <ul {...props} className="list-disc pl-5 space-y-1 mb-4" />
  ),
  ol: ({ node, ...props }: any) => (
    <ol {...props} className="list-decimal pl-5 space-y-1 mb-4" />
  ),
  li: ({ node, ...props }: any) => (
    <li {...props} className="marker:text-muted-foreground/50 text-foreground/90 leading-relaxed" />
  ),
};

// ⚡ Bolt: Wrap in React.memo to prevent re-renders when parent streams tokens
export const MarkdownRenderer = memo(function MarkdownRenderer({ content }: MarkdownRendererProps) {
  return (
    <div className="prose prose-sm dark:prose-invert max-w-none break-words text-foreground">
      <Markdown
        remarkPlugins={remarkPlugins as any}
        rehypePlugins={rehypePlugins as any}
        components={markdownComponents}
      >
        {content}
      </Markdown>
    </div>
  );
});
