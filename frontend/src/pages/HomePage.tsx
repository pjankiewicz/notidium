import { Link } from 'react-router-dom'
import { IconNotes, IconSearch, IconTags, IconChartBar } from '@tabler/icons-react'

const quickActions = [
  { to: '/notes', icon: IconNotes, label: 'View Notes', description: 'Browse your knowledge base' },
  { to: '/search', icon: IconSearch, label: 'Search', description: 'Find notes with fulltext or semantic search' },
  { to: '/tags', icon: IconTags, label: 'Tags', description: 'Organize notes by topic' },
  { to: '/stats', icon: IconChartBar, label: 'Statistics', description: 'View vault analytics' },
]

export function HomePage() {
  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-3xl font-bold text-text-primary">Welcome to Notidium</h1>
        <p className="mt-2 text-text-secondary">
          Your developer knowledge, elementarily organized.
        </p>
      </div>

      <div className="grid grid-cols-2 gap-4">
        {quickActions.map(({ to, icon: Icon, label, description }) => (
          <Link
            key={to}
            to={to}
            className="card p-6 hover:border-primary/50 transition-colors group"
          >
            <Icon size={24} className="text-primary mb-3" />
            <h3 className="font-medium text-text-primary group-hover:text-primary">
              {label}
            </h3>
            <p className="text-sm text-text-secondary mt-1">{description}</p>
          </Link>
        ))}
      </div>

      <div className="card p-6">
        <h2 className="font-medium text-text-primary mb-3">Quick Tips</h2>
        <ul className="space-y-2 text-sm text-text-secondary">
          <li>Press <kbd className="kbd">Cmd+K</kbd> to open the command palette</li>
          <li>Use <kbd className="kbd">Cmd+N</kbd> to create a new note</li>
          <li>Toggle between fulltext and semantic search with <kbd className="kbd">Cmd+/</kbd></li>
        </ul>
      </div>
    </div>
  )
}
