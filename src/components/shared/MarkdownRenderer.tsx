import React from "react";
import ReactMarkdown, { Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeSanitize from "rehype-sanitize";
import rehypeHighlight from "rehype-highlight";
import { Copy, Check } from "lucide-react";
import { useState } from "react";
import { Button } from "@heroui/react";
import "highlight.js/styles/github-dark.css";

interface MarkdownRendererProps {
  content: string;
  className?: string;
}

const remarkPlugins = [remarkGfm];
const rehypePlugins = [rehypeSanitize, rehypeHighlight];

const components: Components = {
  // Custom Code Block with Copy Button
  pre: ({ node, children, ...props }: any) => {
    return (
      <div className="relative group my-4 rounded-xl overflow-hidden border border-border/50 bg-neutral-900/90 shadow-sm">
        <div className="flex items-center justify-between px-3 py-1.5 bg-white/5 border-b border-white/5">
          <div className="flex gap-1.5">
            <span className="w-2.5 h-2.5 rounded-full bg-red-500/20" />
            <span className="w-2.5 h-2.5 rounded-full bg-yellow-500/20" />
            <span className="w-2.5 h-2.5 rounded-full bg-green-500/20" />
          </div>
          <CodeCopyBtn>{children}</CodeCopyBtn>
        </div>
        <pre
          {...props}
          className="p-4 overflow-x-auto text-[13px] leading-relaxed font-mono custom-scrollbar"
        >
          {children}
        </pre>
      </div>
    );
  }, // Styling other elements
  table: ({ node, ...props }: any) => (
    <div className="overflow-x-auto my-4 rounded-lg border border-border/50">
      <table className="w-full text-left text-sm" {...props} />
    </div>
  ),
  thead: ({ node, ...props }: any) => (
    <thead
      className="bg-muted/50 text-muted-foreground font-medium"
      {...props}
    />
  ),
  th: ({ node, ...props }: any) => (
    <th className="px-4 py-2 font-medium" {...props} />
  ),
  td: ({ node, ...props }: any) => (
    <td className="px-4 py-2 border-t border-border/50" {...props} />
  ),
  blockquote: ({ node, ...props }: any) => (
    <blockquote
      className="border-l-4 border-primary/50 pl-4 py-1 italic bg-primary/5 rounded-r-lg my-4"
      {...props}
    />
  ),
};

export const MarkdownRenderer = React.memo(function MarkdownRenderer({
  content,
  className = "",
}: MarkdownRendererProps) {
  return (
    <div
      className={`prose dark:prose-invert prose-sm max-w-none 
        prose-headings:font-semibold prose-headings:tracking-tight 
        prose-a:text-primary prose-a:no-underline hover:prose-a:underline
        prose-code:text-primary prose-code:bg-primary/10 prose-code:px-1 prose-code:py-0.5 prose-code:rounded-md prose-code:before:content-none prose-code:after:content-none
        prose-pre:bg-neutral-950/50 prose-pre:border prose-pre:border-white/10 prose-pre:p-0 prose-pre:rounded-xl
        ${className}`}
    >
      <ReactMarkdown
        remarkPlugins={remarkPlugins as any}
        rehypePlugins={rehypePlugins as any}
        components={components}
      >
        {content}
      </ReactMarkdown>
    </div>
  );
});




// Helper component for Copy functionality
const CodeCopyBtn = ({ children }: { children: any }) => {
  const [isCopied, setIsCopied] = useState(false);
  
  // Extract text from React node children
  const extractText = (node: any): string => {
    if (typeof node === 'string') return node;
    if (Array.isArray(node)) return node.map(extractText).join('');
    if (node?.props?.children) return extractText(node.props.children);
    return '';
  };

  const handleCopy = () => {
    const text = extractText(children);
    navigator.clipboard.writeText(text);
    setIsCopied(true);
    setTimeout(() => setIsCopied(false), 2000);
  };

  return (
    <Button
      size="sm"
      variant="ghost" 
      isIconOnly
      className="h-6 w-6 text-neutral-400 hover:text-white data-[hover=true]:bg-white/10"
      onPress={handleCopy}
    >
      {isCopied ? <Check className="w-3.5 h-3.5 text-green-400" /> : <Copy className="w-3.5 h-3.5" />}
    </Button>
  );
};
