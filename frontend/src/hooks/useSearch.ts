import { useQuery } from '@tanstack/react-query'

export interface SearchResult {
  note_id: string
  title: string
  snippet: string
  score: number
  chunk_type?: string
  tags: string[]
  updated_at?: string
}

interface SearchResponse {
  results: SearchResult[]
  total: number
}

export type SearchMode = 'fulltext' | 'semantic'

export function useSearch(query: string, mode: SearchMode, limit = 20) {
  const endpoint = mode === 'semantic' ? '/api/search/semantic' : '/api/search'
  const url = `${endpoint}?q=${encodeURIComponent(query)}&limit=${limit}`

  return useQuery<SearchResponse>({
    queryKey: ['search', mode, query, limit],
    queryFn: async () => {
      const res = await fetch(url)
      if (!res.ok) throw new Error('Search failed')
      return res.json()
    },
    enabled: query.length > 0,
    staleTime: 10_000,
  })
}
