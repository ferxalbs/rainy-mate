import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeHighlight from "rehype-highlight";
import "highlight.js/styles/atom-one-dark.css"; // Start with a default dark theme

interface MarkdownRendererProps {
  content: string;
}

export function MarkdownRenderer({ content }: MarkdownRendererProps) {
  return (
    <div className="prose prose-sm dark:prose-invert max-w-none break-words">
      <Markdown
        remarkPlugins={[remarkGfm]}
        rehypePlugins={[rehypeHighlight]}
        components={{
          a: ({ node, ...props }) => (
            <a
              {...props}
              target="_blank"
              rel="noopener noreferrer"
              className="text-blue-400 hover:underline"
            />
          ),
          pre: ({ node, ...props }) => (
            <pre
              {...props}
              className="bg-gray-900/50 rounded-lg p-0 overflow-x-auto border border-white/10"
            />
          ),
          code: ({ node, className, children, ...props }: any) => {
            const match = /language-(\w+)/.exec(className || "");
            const isInline = !match && !String(children).includes("\n");

            return isInline ? (
              <code
                {...props}
                className={`${className} bg-white/10 rounded px-1 py-0.5 text-[0.9em]`}
              >
                {children}
              </code>
            ) : (
              <code
                {...props}
                className={`${className} block p-4 text-sm font-mono`}
              >
                {children}
              </code>
            );
          },
          ul: ({ node, ...props }) => (
            <ul {...props} className="list-disc pl-4 space-y-1" />
          ),
          ol: ({ node, ...props }) => (
            <ol {...props} className="list-decimal pl-4 space-y-1" />
          ),
          li: ({ node, ...props }) => (
            <li {...props} className="marker:text-gray-400" />
          ),
        }}
      >
        {content}
      </Markdown>
    </div>
  );
}
