import { NavLink } from 'react-router-dom'
import {
  IconNotes,
  IconSearch,
  IconTags,
  IconChartBar,
  IconChevronLeft,
  IconChevronRight,
  IconCommand,
} from '@tabler/icons-react'
import { useUIStore } from '@/stores/uiStore'
import { cn } from '@/utils/cn'

const navItems = [
  { to: '/notes', icon: IconNotes, label: 'Notes' },
  { to: '/search', icon: IconSearch, label: 'Search' },
  { to: '/tags', icon: IconTags, label: 'Tags' },
  { to: '/stats', icon: IconChartBar, label: 'Stats' },
]

export function Sidebar() {
  const { sidebarOpen, toggleSidebar, setCommandPaletteOpen } = useUIStore()

  return (
    <aside
      className={cn(
        'fixed left-0 top-0 h-full bg-bg-surface border-r border-border',
        'flex flex-col transition-all duration-200 z-40',
        sidebarOpen ? 'w-64' : 'w-16'
      )}
    >
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b border-border">
        {sidebarOpen && (
          <h1 className="text-lg font-semibold text-primary">Notidium</h1>
        )}
        <button
          onClick={toggleSidebar}
          className="p-1.5 rounded-lg text-text-secondary hover:text-text-primary hover:bg-bg-hover"
        >
          {sidebarOpen ? <IconChevronLeft size={18} /> : <IconChevronRight size={18} />}
        </button>
      </div>

      {/* Quick Search */}
      <div className="p-3">
        <button
          onClick={() => setCommandPaletteOpen(true)}
          className={cn(
            'w-full flex items-center gap-2 px-3 py-2 rounded-lg',
            'bg-bg-elevated border border-border text-text-secondary',
            'hover:border-border-focus hover:text-text-primary transition-colors'
          )}
        >
          <IconSearch size={16} />
          {sidebarOpen && (
            <>
              <span className="flex-1 text-left text-sm">Search...</span>
              <kbd className="kbd">
                <IconCommand size={12} />K
              </kbd>
            </>
          )}
        </button>
      </div>

      {/* Navigation */}
      <nav className="flex-1 p-3 space-y-1">
        {navItems.map(({ to, icon: Icon, label }) => (
          <NavLink
            key={to}
            to={to}
            className={({ isActive }) =>
              cn(
                'flex items-center gap-3 px-3 py-2 rounded-lg transition-colors',
                isActive
                  ? 'bg-primary/10 text-primary'
                  : 'text-text-secondary hover:text-text-primary hover:bg-bg-hover'
              )
            }
          >
            <Icon size={20} />
            {sidebarOpen && <span>{label}</span>}
          </NavLink>
        ))}
      </nav>

      {/* Footer */}
      <div className="p-4 border-t border-border">
        {sidebarOpen && (
          <p className="text-xs text-text-muted">
            v0.1.0
          </p>
        )}
      </div>
    </aside>
  )
}
