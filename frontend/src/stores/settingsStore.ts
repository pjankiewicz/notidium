import { create } from 'zustand'
import { persist } from 'zustand/middleware'

type ViewMode = 'edit' | 'preview' | 'split'
type SearchMode = 'fulltext' | 'semantic'

interface SettingsState {
  // Note editor settings
  viewMode: ViewMode
  setViewMode: (mode: ViewMode) => void

  // Search settings
  searchMode: SearchMode
  setSearchMode: (mode: SearchMode) => void
}

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      viewMode: 'edit',
      setViewMode: (viewMode) => set({ viewMode }),

      searchMode: 'fulltext',
      setSearchMode: (searchMode) => set({ searchMode }),
    }),
    {
      name: 'notidium-settings',
    }
  )
)
