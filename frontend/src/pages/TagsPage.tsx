import { Link } from 'react-router-dom'
import { IconTag } from '@tabler/icons-react'
import { useTags } from '@/hooks/useTags'
import { Skeleton } from '@/components/ui/Skeleton'
import { EmptyState } from '@/components/ui/EmptyState'

export function TagsPage() {
  const { data, isLoading, error } = useTags()

  if (error) {
    return (
      <div className="card p-6 text-center">
        <p className="text-error">Failed to load tags</p>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold text-text-primary">Tags</h1>
        <p className="text-sm text-text-secondary mt-1">
          Organize your notes by topic
        </p>
      </div>

      {isLoading ? (
        <div className="flex flex-wrap gap-2">
          {Array.from({ length: 8 }).map((_, i) => (
            <Skeleton key={i} className="h-8 w-24" />
          ))}
        </div>
      ) : data?.tags.length === 0 ? (
        <EmptyState
          title="No tags yet"
          description="Add tags to your notes using YAML frontmatter"
        />
      ) : (
        <div className="flex flex-wrap gap-2">
          {data?.tags.map((tag) => (
            <Link
              key={tag}
              to={`/tags/${tag}`}
              className="flex items-center gap-2 px-3 py-2 bg-bg-surface border border-border rounded-lg
                         text-text-secondary hover:text-primary hover:border-primary/50 transition-colors"
            >
              <IconTag size={16} />
              <span>{tag}</span>
            </Link>
          ))}
        </div>
      )}
    </div>
  )
}
