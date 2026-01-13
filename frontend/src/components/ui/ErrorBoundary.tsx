import { ErrorBoundary as ReactErrorBoundary } from 'react-error-boundary'
import { IconAlertTriangle, IconRefresh, IconHome } from '@tabler/icons-react'
import { Link } from 'react-router-dom'

interface ErrorFallbackProps {
  error: Error
  resetErrorBoundary: () => void
}

function ErrorFallback({ error, resetErrorBoundary }: ErrorFallbackProps) {
  return (
    <div className="min-h-[400px] flex items-center justify-center p-6">
      <div className="card p-8 max-w-md text-center">
        <IconAlertTriangle size={48} className="mx-auto text-error mb-4" />
        <h2 className="text-xl font-bold text-text-primary mb-2">Something went wrong</h2>
        <p className="text-text-secondary mb-4">
          An unexpected error occurred. Please try again.
        </p>
        <details className="text-left mb-6 bg-bg-elevated rounded-lg p-3">
          <summary className="text-text-muted text-sm cursor-pointer hover:text-text-secondary">
            Error details
          </summary>
          <pre className="mt-2 text-xs text-error overflow-auto whitespace-pre-wrap font-mono">
            {error.message}
          </pre>
        </details>
        <div className="flex justify-center gap-3">
          <Link to="/" className="btn btn-secondary flex items-center gap-2">
            <IconHome size={16} />
            Go Home
          </Link>
          <button
            onClick={resetErrorBoundary}
            className="btn btn-primary flex items-center gap-2"
          >
            <IconRefresh size={16} />
            Try Again
          </button>
        </div>
      </div>
    </div>
  )
}

interface ErrorBoundaryProps {
  children: React.ReactNode
}

export function ErrorBoundary({ children }: ErrorBoundaryProps) {
  return (
    <ReactErrorBoundary
      FallbackComponent={ErrorFallback}
      onReset={() => {
        // Reset any state that might have caused the error
        window.location.reload()
      }}
    >
      {children}
    </ReactErrorBoundary>
  )
}
