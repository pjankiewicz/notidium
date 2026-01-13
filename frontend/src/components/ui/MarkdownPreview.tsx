import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import rehypeHighlight from 'rehype-highlight'
import { cn } from '@/utils/cn'
import { CodeBlock } from './CodeBlock'

interface MarkdownPreviewProps {
  content: string
  className?: string
}

// Helper to extract source line from node position
function getSourceLine(node: { position?: { start?: { line?: number } } } | undefined): number | undefined {
  return node?.position?.start?.line
}

function getSourceEndLine(node: { position?: { end?: { line?: number } } } | undefined): number | undefined {
  return node?.position?.end?.line
}

export function MarkdownPreview({ content, className }: MarkdownPreviewProps) {
  return (
    <div className={cn('prose prose-invert max-w-none', className)}>
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        rehypePlugins={[rehypeHighlight]}
        components={{
          // Custom renderers with source line tracking for scroll sync
          h1: ({ children, node }) => (
            <h1
              data-source-line={getSourceLine(node)}
              data-source-end-line={getSourceEndLine(node)}
              className="text-2xl font-bold text-text-primary border-b border-border pb-2 mb-4"
            >
              {children}
            </h1>
          ),
          h2: ({ children, node }) => (
            <h2
              data-source-line={getSourceLine(node)}
              data-source-end-line={getSourceEndLine(node)}
              className="text-xl font-bold text-text-primary border-b border-border pb-1 mb-3 mt-6"
            >
              {children}
            </h2>
          ),
          h3: ({ children, node }) => (
            <h3
              data-source-line={getSourceLine(node)}
              data-source-end-line={getSourceEndLine(node)}
              className="text-lg font-semibold text-text-primary mb-2 mt-4"
            >
              {children}
            </h3>
          ),
          h4: ({ children, node }) => (
            <h4
              data-source-line={getSourceLine(node)}
              data-source-end-line={getSourceEndLine(node)}
              className="text-base font-semibold text-text-primary mb-2 mt-3"
            >
              {children}
            </h4>
          ),
          h5: ({ children, node }) => (
            <h5
              data-source-line={getSourceLine(node)}
              data-source-end-line={getSourceEndLine(node)}
              className="text-sm font-semibold text-text-primary mb-1 mt-2"
            >
              {children}
            </h5>
          ),
          h6: ({ children, node }) => (
            <h6
              data-source-line={getSourceLine(node)}
              data-source-end-line={getSourceEndLine(node)}
              className="text-sm font-medium text-text-secondary mb-1 mt-2"
            >
              {children}
            </h6>
          ),
          p: ({ children, node }) => (
            <p
              data-source-line={getSourceLine(node)}
              data-source-end-line={getSourceEndLine(node)}
              className="text-text-primary leading-relaxed mb-4"
            >
              {children}
            </p>
          ),
          a: ({ href, children }) => (
            <a
              href={href}
              className="text-primary hover:text-primary-hover underline"
              target="_blank"
              rel="noopener noreferrer"
            >
              {children}
            </a>
          ),
          ul: ({ children, node }) => (
            <ul
              data-source-line={getSourceLine(node)}
              data-source-end-line={getSourceEndLine(node)}
              className="list-disc list-inside mb-4 space-y-1 text-text-primary"
            >
              {children}
            </ul>
          ),
          ol: ({ children, node }) => (
            <ol
              data-source-line={getSourceLine(node)}
              data-source-end-line={getSourceEndLine(node)}
              className="list-decimal list-inside mb-4 space-y-1 text-text-primary"
            >
              {children}
            </ol>
          ),
          li: ({ children }) => (
            <li className="text-text-primary">{children}</li>
          ),
          blockquote: ({ children, node }) => (
            <blockquote
              data-source-line={getSourceLine(node)}
              data-source-end-line={getSourceEndLine(node)}
              className="border-l-4 border-primary pl-4 italic text-text-secondary my-4"
            >
              {children}
            </blockquote>
          ),
          code: ({ className, children, ...props }) => {
            const isInline = !className
            if (isInline) {
              return (
                <code
                  className="bg-bg-elevated px-1.5 py-0.5 rounded text-sm font-mono text-primary"
                  {...props}
                >
                  {children}
                </code>
              )
            }
            return (
              <code className={cn('font-mono', className)} {...props}>
                {children}
              </code>
            )
          },
          pre: ({ children, node }) => (
            <CodeBlock data-source-line={getSourceLine(node)} data-source-end-line={getSourceEndLine(node)}>
              {children}
            </CodeBlock>
          ),
          table: ({ children, node }) => (
            <div
              data-source-line={getSourceLine(node)}
              data-source-end-line={getSourceEndLine(node)}
              className="overflow-x-auto mb-4"
            >
              <table className="min-w-full border border-border rounded-lg">
                {children}
              </table>
            </div>
          ),
          th: ({ children }) => (
            <th className="px-4 py-2 bg-bg-elevated border-b border-border text-left font-semibold text-text-primary">
              {children}
            </th>
          ),
          td: ({ children }) => (
            <td className="px-4 py-2 border-b border-border text-text-primary">
              {children}
            </td>
          ),
          hr: ({ node }) => (
            <hr
              data-source-line={getSourceLine(node)}
              data-source-end-line={getSourceEndLine(node)}
              className="border-border my-6"
            />
          ),
          img: ({ src, alt, node }) => (
            <img
              data-source-line={getSourceLine(node)}
              data-source-end-line={getSourceEndLine(node)}
              src={src}
              alt={alt}
              className="max-w-full rounded-lg border border-border"
            />
          ),
        }}
      >
        {content}
      </ReactMarkdown>
    </div>
  )
}
