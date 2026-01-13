import { useState, useRef } from 'react'
import { IconCopy, IconCheck } from '@tabler/icons-react'
import { cn } from '@/utils/cn'

interface CodeBlockProps {
  children: React.ReactNode
  className?: string
}

export function CodeBlock({ children, className }: CodeBlockProps) {
  const [copied, setCopied] = useState(false)
  const preRef = useRef<HTMLPreElement>(null)

  const handleCopy = async () => {
    if (!preRef.current) return

    // Extract text content from the code element
    const code = preRef.current.querySelector('code')
    const text = code?.textContent || preRef.current.textContent || ''

    try {
      await navigator.clipboard.writeText(text)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch (err) {
      console.error('Failed to copy:', err)
    }
  }

  return (
    <div className="relative group">
      <pre
        ref={preRef}
        className={cn(
          'bg-bg-elevated rounded-lg p-4 overflow-x-auto mb-4 border border-border',
          className
        )}
      >
        {children}
      </pre>
      <button
        onClick={handleCopy}
        className={cn(
          'absolute top-2 right-2 p-1.5 rounded-md transition-all',
          'bg-bg-surface/80 border border-border/50',
          'text-text-muted hover:text-text-primary hover:bg-bg-hover',
          'opacity-0 group-hover:opacity-100',
          copied && 'text-success opacity-100'
        )}
        aria-label={copied ? 'Copied!' : 'Copy code'}
      >
        {copied ? <IconCheck size={14} /> : <IconCopy size={14} />}
      </button>
    </div>
  )
}
