import { useQuery } from '@tanstack/react-query'

interface StatsResponse {
  note_count: number
  chunk_count: number
  tag_count: number
}

export function useStats() {
  return useQuery<StatsResponse>({
    queryKey: ['stats'],
    queryFn: async () => {
      const res = await fetch('/api/stats')
      if (!res.ok) throw new Error('Failed to fetch stats')
      return res.json()
    },
    staleTime: 30_000,
  })
}
