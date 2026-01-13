import { useState, useEffect } from 'react'
import { Link, useNavigate } from 'react-router-dom'
import { IconSearch, IconSparkles, IconFileText } from '@tabler/icons-react'
import { useSearchStore } from '@/stores/searchStore'
import { useSettingsStore } from '@/stores/settingsStore'
import { useSearch } from '@/hooks/useSearch'
import { Skeleton } from '@/components/ui/Skeleton'
import { Tooltip } from '@/components/ui/Tooltip'
import { cn } from '@/utils/cn'

export function SearchPage() {
  const navigate = useNavigate()
  const { query, setQuery } = useSearchStore()
  const { searchMode: mode, setSearchMode: setMode } = useSettingsStore()
  const [localQuery, setLocalQuery] = useState(query)
  const { data, isLoading } = useSearch(query, mode)

  // Debounce search
  useEffect(() => {
    const timer = setTimeout(() => {
      setQuery(localQuery)
    }, 300)
    return () => clearTimeout(timer)
  }, [localQuery, setQuery])

  const handleTagClick = (e: React.MouseEvent, tagName: string) => {
    e.preventDefault()
    e.stopPropagation()
    navigate(`/tags/${tagName}`)
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold text-text-primary">Search</h1>
        <p className="text-sm text-text-secondary mt-1">
          Find notes using fulltext or semantic search
        </p>
      </div>

      {/* Search Input */}
      <div className="space-y-3">
        <div className="relative">
          <IconSearch size={18} className="absolute left-3 top-1/2 -translate-y-1/2 text-text-muted" />
          <input
            type="text"
            value={localQuery}
            onChange={(e) => setLocalQuery(e.target.value)}
            placeholder="Search your notes..."
            className="input pl-10 pr-32"
            autoFocus
          />
          {/* Mode Toggle */}
          <div className="absolute right-2 top-1/2 -translate-y-1/2 flex bg-bg-elevated rounded-lg p-0.5">
            <Tooltip content="Search by exact text matches">
              <button
                onClick={() => setMode('fulltext')}
                className={cn(
                  'flex items-center gap-1 px-2 py-1 rounded text-xs transition-colors',
                  mode === 'fulltext'
                    ? 'bg-fulltext/20 text-fulltext'
                    : 'text-text-muted hover:text-text-secondary'
                )}
              >
                <IconFileText size={14} />
                Text
              </button>
            </Tooltip>
            <Tooltip content="Search by meaning and concepts">
              <button
                onClick={() => setMode('semantic')}
                className={cn(
                  'flex items-center gap-1 px-2 py-1 rounded text-xs transition-colors',
                  mode === 'semantic'
                    ? 'bg-semantic/20 text-semantic'
                    : 'text-text-muted hover:text-text-secondary'
                )}
              >
                <IconSparkles size={14} />
                Semantic
              </button>
            </Tooltip>
          </div>
        </div>
      </div>

      {/* Results */}
      {!query ? (
        <div className="card p-12 text-center">
          <IconSearch size={48} className="mx-auto text-text-muted mb-4" />
          <p className="text-text-secondary">
            Enter a search query to find notes
          </p>
          <p className="text-sm text-text-muted mt-2">
            {mode === 'semantic'
              ? 'Semantic search finds conceptually similar content'
              : 'Fulltext search matches exact words and phrases'}
          </p>
        </div>
      ) : isLoading ? (
        <div className="space-y-3">
          {Array.from({ length: 3 }).map((_, i) => (
            <Skeleton key={i} className="h-24" />
          ))}
        </div>
      ) : data?.results.length === 0 ? (
        <div className="card p-12 text-center">
          <p className="text-text-secondary">No results found for "{query}"</p>
          <p className="text-sm text-text-muted mt-2">
            Try {mode === 'fulltext' ? 'semantic' : 'fulltext'} search for different results
          </p>
        </div>
      ) : (
        <div className="space-y-2">
          <p className="text-sm text-text-muted">
            {data?.total} result{data?.total !== 1 ? 's' : ''}
          </p>
          {data?.results.map((result) => (
            <Link
              key={result.note_id}
              to={`/notes/${result.note_id}`}
              className="card p-4 block hover:border-primary/50 transition-colors"
            >
              <div className="flex items-start justify-between gap-4">
                <h3 className="font-medium text-text-primary flex-1">
                  {result.title || 'Untitled'}
                </h3>
                <span className={cn(
                  'text-xs px-2 py-0.5 rounded-full whitespace-nowrap',
                  mode === 'semantic'
                    ? 'bg-semantic/10 text-semantic'
                    : 'bg-fulltext/10 text-fulltext'
                )}>
                  {(result.score * 100).toFixed(0)}% match
                </span>
              </div>

              {/* Snippet */}
              <p className="text-sm text-text-secondary mt-2 line-clamp-2">
                {result.snippet}
              </p>

              {/* Metadata row - same style as notes list */}
              <div className="flex items-center gap-4 mt-3 text-sm text-text-muted">
                {result.updated_at && (
                  <span>Updated {new Date(result.updated_at).toLocaleDateString()}</span>
                )}
                {result.tags && result.tags.length > 0 && (
                  <div className="flex gap-1 flex-wrap">
                    {result.tags.map((tag) => (
                      <button
                        key={tag}
                        onClick={(e) => handleTagClick(e, tag)}
                        className="px-1.5 py-0.5 bg-primary/10 text-primary hover:bg-primary/20 rounded text-xs transition-colors"
                      >
                        {tag}
                      </button>
                    ))}
                  </div>
                )}
                {result.chunk_type && mode === 'semantic' && (
                  <span className="text-xs text-text-muted">
                    {result.chunk_type}
                  </span>
                )}
              </div>
            </Link>
          ))}
        </div>
      )}
    </div>
  )
}
