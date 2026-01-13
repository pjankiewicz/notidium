import { Outlet } from 'react-router-dom'
import { Sidebar } from './Sidebar'
import { CommandPalette } from '../ui/CommandPalette'
import { useUIStore } from '@/stores/uiStore'
import { cn } from '@/utils/cn'

export function Layout() {
  const sidebarOpen = useUIStore((s) => s.sidebarOpen)

  return (
    <div className="flex h-screen bg-bg-base text-text-primary">
      <Sidebar />
      <main
        className={cn(
          'flex-1 flex flex-col overflow-hidden transition-all duration-200',
          sidebarOpen ? 'ml-64' : 'ml-16'
        )}
      >
        <div className="flex-1 overflow-auto p-4 lg:p-6">
          <div className="h-full">
            <Outlet />
          </div>
        </div>
      </main>
      <CommandPalette />
    </div>
  )
}
