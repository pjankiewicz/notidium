import { useState } from 'react'
import { Link, useParams, useNavigate } from 'react-router-dom'
import { IconPlus, IconSearch } from '@tabler/icons-react'
import { useNotes } from '@/hooks/useNotes'
import { Skeleton } from '@/components/ui/Skeleton'
import { EmptyState } from '@/components/ui/EmptyState'

export function NotesPage() {
  const { tag } = useParams<{ tag?: string }>()
  const navigate = useNavigate()
  const [search, setSearch] = useState('')
  const { data, isLoading, error } = useNotes({ tag, limit: 50 })

  if (error) {
    return (
      <div className="card p-6 text-center">
        <p className="text-error">Failed to load notes</p>
        <button
          onClick={() => window.location.reload()}
          className="btn btn-secondary mt-4"
        >
          Retry
        </button>
      </div>
    )
  }

  const handleTagClick = (e: React.MouseEvent, tagName: string) => {
    e.preventDefault()
    e.stopPropagation()
    navigate(`/tags/${tagName}`)
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-text-primary">
            {tag ? `Notes tagged: ${tag}` : 'All Notes'}
          </h1>
          {data && (
            <p className="text-sm text-text-secondary mt-1">
              {data.total} note{data.total !== 1 ? 's' : ''}
            </p>
          )}
        </div>
        <Link to="/notes/new" className="btn btn-primary flex items-center gap-2">
          <IconPlus size={16} />
          New Note
        </Link>
      </div>

      {/* Search */}
      <div className="relative">
        <IconSearch size={18} className="absolute left-3 top-1/2 -translate-y-1/2 text-text-muted" />
        <input
          type="text"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="Filter notes..."
          className="input pl-10"
        />
      </div>

      {/* Notes List */}
      {isLoading ? (
        <div className="space-y-3">
          {Array.from({ length: 5 }).map((_, i) => (
            <Skeleton key={i} className="h-20" />
          ))}
        </div>
      ) : data?.notes.length === 0 ? (
        <EmptyState
          title="No notes yet"
          description="Create your first note to get started"
          action={{ label: 'New Note', to: '/notes/new' }}
        />
      ) : (
        <div className="space-y-2">
          {data?.notes
            .filter((note) =>
              search
                ? note.title.toLowerCase().includes(search.toLowerCase())
                : true
            )
            .map((note) => (
              <Link
                key={note.id}
                to={`/notes/${note.id}`}
                className="card p-4 block hover:border-primary/50 transition-colors"
              >
                <h3 className="font-medium text-text-primary">{note.title}</h3>
                <div className="flex items-center gap-4 mt-2 text-sm text-text-secondary">
                  <span>Updated {new Date(note.updated_at).toLocaleDateString()}</span>
                  {note.tags.length > 0 && (
                    <div className="flex gap-1">
                      {note.tags.map((t) => (
                        <button
                          key={t}
                          onClick={(e) => handleTagClick(e, t)}
                          className="px-1.5 py-0.5 bg-primary/10 text-primary hover:bg-primary/20 rounded text-xs transition-colors"
                        >
                          {t}
                        </button>
                      ))}
                    </div>
                  )}
                  {note.is_pinned && (
                    <span className="text-warning text-xs">Pinned</span>
                  )}
                </div>
              </Link>
            ))}
        </div>
      )}
    </div>
  )
}
