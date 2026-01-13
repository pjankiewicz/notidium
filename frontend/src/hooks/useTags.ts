import { useQuery } from '@tanstack/react-query'

interface TagsResponse {
  tags: string[]
}

export function useTags() {
  return useQuery<TagsResponse>({
    queryKey: ['tags'],
    queryFn: async () => {
      const res = await fetch('/api/tags')
      if (!res.ok) throw new Error('Failed to fetch tags')
      return res.json()
    },
    staleTime: 60_000,
  })
}
