import { create } from 'zustand'
import { persist } from 'zustand/middleware'

type ViewMode = 'edit' | 'preview' | 'split'
type SearchMode = 'fulltext' | 'semantic'
type Theme = 'light' | 'dark'

interface SettingsState {
  // Note editor settings
  viewMode: ViewMode
  setViewMode: (mode: ViewMode) => void

  // Search settings
  searchMode: SearchMode
  setSearchMode: (mode: SearchMode) => void

  // Theme settings
  theme: Theme
  setTheme: (theme: Theme) => void
  toggleTheme: () => void
}

// Apply theme to document
const applyTheme = (theme: Theme) => {
  if (theme === 'dark') {
    document.documentElement.setAttribute('data-theme', 'dark')
  } else {
    document.documentElement.removeAttribute('data-theme')
  }
}

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set, get) => ({
      viewMode: 'edit',
      setViewMode: (viewMode) => set({ viewMode }),

      searchMode: 'fulltext',
      setSearchMode: (searchMode) => set({ searchMode }),

      theme: 'dark',
      setTheme: (theme) => {
        applyTheme(theme)
        set({ theme })
      },
      toggleTheme: () => {
        const newTheme = get().theme === 'dark' ? 'light' : 'dark'
        applyTheme(newTheme)
        set({ theme: newTheme })
      },
    }),
    {
      name: 'notidium-settings',
      onRehydrateStorage: () => (state) => {
        // Apply theme on initial load
        if (state?.theme) {
          applyTheme(state.theme)
        }
      },
    }
  )
)
