import { create } from 'zustand'

type SearchMode = 'fulltext' | 'semantic'

interface SearchState {
  query: string
  mode: SearchMode
  isSearching: boolean

  setQuery: (query: string) => void
  setMode: (mode: SearchMode) => void
  toggleMode: () => void
  setIsSearching: (isSearching: boolean) => void
  clear: () => void
}

export const useSearchStore = create<SearchState>((set) => ({
  query: '',
  mode: 'fulltext',
  isSearching: false,

  setQuery: (query) => set({ query }),
  setMode: (mode) => set({ mode }),
  toggleMode: () => set((state) => ({
    mode: state.mode === 'fulltext' ? 'semantic' : 'fulltext'
  })),
  setIsSearching: (isSearching) => set({ isSearching }),
  clear: () => set({ query: '', isSearching: false }),
}))
