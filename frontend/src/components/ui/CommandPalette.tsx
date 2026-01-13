import { useEffect, useCallback, useRef, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { Command } from 'cmdk'
import {
  IconNotes,
  IconSearch,
  IconTags,
  IconChartBar,
  IconPlus,
  IconFile,
  IconSparkles,
  IconFileText,
} from '@tabler/icons-react'
import { useUIStore } from '@/stores/uiStore'
import { useSettingsStore } from '@/stores/settingsStore'
import { useSearch } from '@/hooks/useSearch'
import { cn } from '@/utils/cn'

export function CommandPalette() {
  const navigate = useNavigate()
  const { commandPaletteOpen, setCommandPaletteOpen } = useUIStore()
  const { searchMode, setSearchMode } = useSettingsStore()
  const inputRef = useRef<HTMLInputElement>(null)

  // Local query state for the palette (separate from global search)
  const [query, setQuery] = useState('')

  // Debounced search query
  const [debouncedQuery, setDebouncedQuery] = useState('')

  // Selected item value for controlled selection
  const [selectedValue, setSelectedValue] = useState('')

  // Debounce the search
  useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedQuery(query)
    }, 200)
    return () => clearTimeout(timer)
  }, [query])

  // Fetch search results when there's a query
  const { data: searchResults, isLoading } = useSearch(
    debouncedQuery.length >= 2 ? debouncedQuery : '',
    searchMode
  )

  // Auto-select first search result when results arrive
  useEffect(() => {
    if (searchResults?.results && searchResults.results.length > 0) {
      setSelectedValue(`note-${searchResults.results[0].note_id}`)
    } else if (debouncedQuery.length < 2) {
      // Reset to default when no query
      setSelectedValue('')
    }
  }, [searchResults, debouncedQuery])

  // Focus input when palette opens
  useEffect(() => {
    if (commandPaletteOpen) {
      const timer = setTimeout(() => {
        inputRef.current?.focus()
      }, 0)
      return () => clearTimeout(timer)
    } else {
      // Clear state when closing
      setQuery('')
      setDebouncedQuery('')
      setSelectedValue('')
    }
  }, [commandPaletteOpen])

  // Keyboard shortcut
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault()
        setCommandPaletteOpen(!commandPaletteOpen)
      }
      if (e.key === 'Escape' && commandPaletteOpen) {
        setCommandPaletteOpen(false)
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [commandPaletteOpen, setCommandPaletteOpen])

  const runCommand = useCallback((command: () => void) => {
    setCommandPaletteOpen(false)
    command()
  }, [setCommandPaletteOpen])

  const toggleSearchMode = useCallback(() => {
    setSearchMode(searchMode === 'fulltext' ? 'semantic' : 'fulltext')
  }, [searchMode, setSearchMode])

  if (!commandPaletteOpen) return null

  const hasQuery = query.length >= 2
  const hasResults = searchResults?.results && searchResults.results.length > 0

  return (
    <div className="fixed inset-0 z-50">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/50"
        onClick={() => setCommandPaletteOpen(false)}
      />

      {/* Dialog */}
      <div className="absolute top-[20%] left-1/2 -translate-x-1/2 w-full max-w-xl">
        <Command
          className="bg-bg-elevated border border-border rounded-xl shadow-2xl overflow-hidden"
          loop
          shouldFilter={!hasQuery} // Disable filtering when showing search results
          value={selectedValue}
          onValueChange={setSelectedValue}
        >
          <div className="flex items-center border-b border-border">
            <Command.Input
              ref={inputRef}
              value={query}
              onValueChange={setQuery}
              placeholder="Search notes or type a command..."
              className="flex-1 px-4 py-3 bg-transparent
                         text-text-primary placeholder:text-text-muted
                         focus:outline-none"
            />
            {/* Search mode indicator */}
            <button
              onClick={toggleSearchMode}
              className={cn(
                'mr-2 px-2 py-1 rounded text-xs flex items-center gap-1 transition-colors',
                searchMode === 'semantic'
                  ? 'bg-semantic/20 text-semantic'
                  : 'bg-fulltext/20 text-fulltext'
              )}
              title={`Using ${searchMode} search. Click to toggle.`}
            >
              {searchMode === 'semantic' ? (
                <IconSparkles size={12} />
              ) : (
                <IconFileText size={12} />
              )}
              {searchMode}
            </button>
          </div>

          <Command.List className="max-h-96 overflow-auto p-2">
            {/* Loading state */}
            {isLoading && hasQuery && (
              <div className="py-4 text-center text-text-muted text-sm">
                Searching...
              </div>
            )}

            {/* Search Results */}
            {hasQuery && hasResults && (
              <Command.Group
                heading={`Notes (${searchResults.total})`}
                className="text-xs text-text-muted px-2 py-1"
              >
                {searchResults.results.slice(0, 8).map((result) => (
                  <Command.Item
                    key={result.note_id}
                    value={`note-${result.note_id}`}
                    onSelect={() => runCommand(() => navigate(`/notes/${result.note_id}`))}
                    className="flex items-start gap-3 px-3 py-2 rounded-lg cursor-pointer
                              text-text-secondary hover:text-text-primary hover:bg-bg-hover
                              data-[selected=true]:bg-bg-hover data-[selected=true]:text-text-primary"
                  >
                    <IconFile size={16} className="mt-0.5 flex-shrink-0" />
                    <div className="flex-1 min-w-0">
                      <div className="font-medium text-text-primary truncate">
                        {result.title || 'Untitled'}
                      </div>
                      <div className="text-xs text-text-muted line-clamp-1 mt-0.5">
                        {result.snippet}
                      </div>
                    </div>
                    <span className={cn(
                      'text-xs px-1.5 py-0.5 rounded flex-shrink-0',
                      searchMode === 'semantic'
                        ? 'bg-semantic/10 text-semantic'
                        : 'bg-fulltext/10 text-fulltext'
                    )}>
                      {(result.score * 100).toFixed(0)}%
                    </span>
                  </Command.Item>
                ))}
                {searchResults.total > 8 && (
                  <Command.Item
                    value="view-all-results"
                    onSelect={() => runCommand(() => {
                      // Set the global search query and navigate
                      navigate('/search')
                    })}
                    className="flex items-center justify-center gap-2 px-3 py-2 rounded-lg cursor-pointer
                              text-primary hover:bg-primary-muted
                              data-[selected=true]:bg-primary-muted"
                  >
                    <span className="text-sm">View all {searchResults.total} results</span>
                  </Command.Item>
                )}
              </Command.Group>
            )}

            {/* No results message */}
            {hasQuery && !isLoading && !hasResults && debouncedQuery.length >= 2 && (
              <div className="py-6 text-center text-text-muted">
                No notes found for "{debouncedQuery}"
              </div>
            )}

            {/* Actions - always show */}
            <Command.Group
              heading="Actions"
              className={cn(
                "text-xs text-text-muted px-2 py-1",
                hasQuery && hasResults && "mt-2"
              )}
            >
              <Command.Item
                value="new-note"
                onSelect={() => runCommand(() => navigate('/notes/new'))}
                className="flex items-center gap-3 px-3 py-2 rounded-lg cursor-pointer
                          text-text-secondary hover:text-text-primary hover:bg-bg-hover
                          data-[selected=true]:bg-bg-hover data-[selected=true]:text-text-primary"
              >
                <IconPlus size={16} />
                <span>New Note</span>
                <kbd className="kbd ml-auto">⌘N</kbd>
              </Command.Item>
              <Command.Item
                value="toggle-search-mode"
                onSelect={() => toggleSearchMode()}
                className="flex items-center gap-3 px-3 py-2 rounded-lg cursor-pointer
                          text-text-secondary hover:text-text-primary hover:bg-bg-hover
                          data-[selected=true]:bg-bg-hover data-[selected=true]:text-text-primary"
              >
                <IconSearch size={16} />
                <span>Toggle Search Mode</span>
                <span className={cn(
                  'ml-auto text-xs px-1.5 py-0.5 rounded',
                  searchMode === 'semantic'
                    ? 'bg-semantic/20 text-semantic'
                    : 'bg-fulltext/20 text-fulltext'
                )}>
                  {searchMode}
                </span>
              </Command.Item>
            </Command.Group>

            {/* Navigation - show when no query or as secondary */}
            <Command.Group
              heading="Navigation"
              className={cn(
                "text-xs text-text-muted px-2 py-1 mt-2",
                hasQuery && "opacity-70"
              )}
            >
              <Command.Item
                value="go-to-notes"
                onSelect={() => runCommand(() => navigate('/notes'))}
                className="flex items-center gap-3 px-3 py-2 rounded-lg cursor-pointer
                          text-text-secondary hover:text-text-primary hover:bg-bg-hover
                          data-[selected=true]:bg-bg-hover data-[selected=true]:text-text-primary"
              >
                <IconNotes size={16} />
                <span>Notes</span>
              </Command.Item>
              <Command.Item
                value="go-to-search"
                onSelect={() => runCommand(() => navigate('/search'))}
                className="flex items-center gap-3 px-3 py-2 rounded-lg cursor-pointer
                          text-text-secondary hover:text-text-primary hover:bg-bg-hover
                          data-[selected=true]:bg-bg-hover data-[selected=true]:text-text-primary"
              >
                <IconSearch size={16} />
                <span>Search Page</span>
              </Command.Item>
              <Command.Item
                value="go-to-tags"
                onSelect={() => runCommand(() => navigate('/tags'))}
                className="flex items-center gap-3 px-3 py-2 rounded-lg cursor-pointer
                          text-text-secondary hover:text-text-primary hover:bg-bg-hover
                          data-[selected=true]:bg-bg-hover data-[selected=true]:text-text-primary"
              >
                <IconTags size={16} />
                <span>Tags</span>
              </Command.Item>
              <Command.Item
                value="go-to-stats"
                onSelect={() => runCommand(() => navigate('/stats'))}
                className="flex items-center gap-3 px-3 py-2 rounded-lg cursor-pointer
                          text-text-secondary hover:text-text-primary hover:bg-bg-hover
                          data-[selected=true]:bg-bg-hover data-[selected=true]:text-text-primary"
              >
                <IconChartBar size={16} />
                <span>Stats</span>
              </Command.Item>
            </Command.Group>
          </Command.List>
        </Command>
      </div>
    </div>
  )
}
