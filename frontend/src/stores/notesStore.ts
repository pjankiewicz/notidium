import { create } from 'zustand'

interface NotesFilter {
  tag?: string
  archived?: boolean
}

interface NotesState {
  selectedNoteId: string | null
  filter: NotesFilter

  setSelectedNoteId: (id: string | null) => void
  setFilter: (filter: Partial<NotesFilter>) => void
  clearFilter: () => void
}

export const useNotesStore = create<NotesState>((set) => ({
  selectedNoteId: null,
  filter: {},

  setSelectedNoteId: (id) => set({ selectedNoteId: id }),
  setFilter: (filter) => set((state) => ({ filter: { ...state.filter, ...filter } })),
  clearFilter: () => set({ filter: {} }),
}))
