import { Link } from 'react-router-dom'
import { IconInbox } from '@tabler/icons-react'

interface EmptyStateProps {
  title: string
  description?: string
  action?: {
    label: string
    to: string
  }
}

export function EmptyState({ title, description, action }: EmptyStateProps) {
  return (
    <div className="card p-12 text-center">
      <IconInbox size={48} className="mx-auto text-text-muted mb-4" />
      <h3 className="font-medium text-text-primary">{title}</h3>
      {description && (
        <p className="text-sm text-text-secondary mt-2">{description}</p>
      )}
      {action && (
        <Link to={action.to} className="btn btn-primary mt-4 inline-block">
          {action.label}
        </Link>
      )}
    </div>
  )
}
