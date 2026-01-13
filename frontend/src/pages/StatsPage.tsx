import { IconNotes, IconPuzzle, IconTags } from '@tabler/icons-react'
import { useStats } from '@/hooks/useStats'
import { Skeleton } from '@/components/ui/Skeleton'

export function StatsPage() {
  const { data, isLoading, error } = useStats()

  if (error) {
    return (
      <div className="card p-6 text-center">
        <p className="text-error">Failed to load statistics</p>
      </div>
    )
  }

  const stats = [
    {
      label: 'Total Notes',
      value: data?.note_count ?? 0,
      icon: IconNotes,
      color: 'text-primary',
    },
    {
      label: 'Indexed Chunks',
      value: data?.chunk_count ?? 0,
      icon: IconPuzzle,
      color: 'text-semantic',
    },
    {
      label: 'Unique Tags',
      value: data?.tag_count ?? 0,
      icon: IconTags,
      color: 'text-fulltext',
    },
  ]

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold text-text-primary">Statistics</h1>
        <p className="text-sm text-text-secondary mt-1">
          Overview of your knowledge base
        </p>
      </div>

      <div className="grid grid-cols-3 gap-4">
        {stats.map(({ label, value, icon: Icon, color }) => (
          <div key={label} className="card p-6">
            <Icon size={24} className={color} />
            {isLoading ? (
              <Skeleton className="h-10 w-16 mt-3" />
            ) : (
              <p className="text-3xl font-bold text-text-primary mt-3">
                {value.toLocaleString()}
              </p>
            )}
            <p className="text-sm text-text-secondary mt-1">{label}</p>
          </div>
        ))}
      </div>

      <div className="card p-6">
        <h2 className="font-medium text-text-primary mb-4">About Indexing</h2>
        <div className="space-y-3 text-sm text-text-secondary">
          <p>
            <strong className="text-text-primary">Chunks</strong> are segments of your notes
            that have been processed for semantic search. Each note is split into meaningful
            pieces (headings, paragraphs, code blocks) to improve search accuracy.
          </p>
          <p>
            <strong className="text-text-primary">Semantic search</strong> uses AI embeddings
            to find conceptually similar content, even if the exact words don't match.
          </p>
        </div>
      </div>
    </div>
  )
}
