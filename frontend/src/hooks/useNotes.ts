import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'

// Types matching the backend API
export interface NoteMeta {
  id: string
  title: string
  slug: string
  created_at: string
  updated_at: string
  tags: string[]
  is_pinned: boolean
  is_archived: boolean
}

export interface NoteResponse {
  id: string
  title: string
  slug: string
  content: string
  tags: string[]
  created_at: string
  updated_at: string
  is_pinned: boolean
  is_archived: boolean
}

interface ListResponse {
  notes: NoteMeta[]
  total: number
  offset: number
  limit: number
}

interface UseNotesParams {
  tag?: string
  limit?: number
  offset?: number
}

export interface CreateNoteRequest {
  title: string
  content: string
  tags?: string[]
}

export interface UpdateNoteRequest {
  title?: string
  content?: string
  tags?: string[]
  is_pinned?: boolean
  is_archived?: boolean
}

async function apiRequest<T>(url: string, options?: RequestInit): Promise<T> {
  const res = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
  })

  if (!res.ok) {
    const error = await res.text()
    throw new Error(error || `Request failed: ${res.status}`)
  }

  // Handle 204 No Content
  if (res.status === 204) {
    return undefined as T
  }

  return res.json()
}

export function useNotes(params?: UseNotesParams) {
  const searchParams = new URLSearchParams()
  if (params?.tag) searchParams.set('tag', params.tag)
  if (params?.limit) searchParams.set('limit', params.limit.toString())
  if (params?.offset) searchParams.set('offset', params.offset.toString())

  const queryString = searchParams.toString()
  const url = `/api/notes${queryString ? `?${queryString}` : ''}`

  return useQuery<ListResponse>({
    queryKey: ['notes', params],
    queryFn: () => apiRequest<ListResponse>(url),
    staleTime: 30_000,
  })
}

export function useNote(noteId: string | undefined) {
  return useQuery<NoteResponse>({
    queryKey: ['notes', noteId],
    queryFn: () => apiRequest<NoteResponse>(`/api/notes/${noteId}`),
    enabled: !!noteId && noteId !== 'new',
  })
}

export function useCreateNote() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: (data: CreateNoteRequest) =>
      apiRequest<NoteResponse>('/api/notes', {
        method: 'POST',
        body: JSON.stringify(data),
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['notes'] })
    },
  })
}

export function useUpdateNote() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: ({ id, data }: { id: string; data: UpdateNoteRequest }) =>
      apiRequest<NoteResponse>(`/api/notes/${id}`, {
        method: 'PUT',
        body: JSON.stringify(data),
      }),
    onSuccess: (response, variables) => {
      // Invalidate list query for sidebar updates
      queryClient.invalidateQueries({ queryKey: ['notes'] })
      // Update the cache directly instead of refetching to avoid
      // overwriting local changes made during the save operation
      queryClient.setQueryData(['notes', variables.id], response)
    },
  })
}

export function useDeleteNote() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: (id: string) =>
      apiRequest<void>(`/api/notes/${id}`, {
        method: 'DELETE',
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['notes'] })
    },
  })
}
