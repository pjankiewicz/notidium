import { useState, useEffect, useCallback, useMemo, useRef } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import {
  IconArrowLeft,
  IconTrash,
  IconDeviceFloppy,
  IconEye,
  IconEdit,
  IconPin,
  IconPinnedOff,
  IconArchive,
  IconArchiveOff,
  IconLoader2,
  IconCopy,
  IconCheck,
  IconSearch,
  IconX,
} from '@tabler/icons-react'
import { toast } from 'sonner'
import { useNote, useUpdateNote, useDeleteNote, useCreateNote } from '@/hooks/useNotes'
import { useUploadAttachment, blobToBase64 } from '@/hooks/useUploadAttachment'
import { Skeleton } from '@/components/ui/Skeleton'
import { MarkdownPreview } from '@/components/ui/MarkdownPreview'
import { TagInput } from '@/components/ui/TagInput'
import { ConfirmDialog } from '@/components/ui/ConfirmDialog'
import { Tooltip } from '@/components/ui/Tooltip'
import { cn } from '@/utils/cn'
import { stripFrontmatter } from '@/utils/frontmatter'
import { useSettingsStore } from '@/stores/settingsStore'

export function NotePage() {
  const { noteId } = useParams<{ noteId: string }>()
  const navigate = useNavigate()
  const isNewNote = noteId === 'new'

  // API hooks
  const { data: note, isLoading, error } = useNote(noteId)
  const createNote = useCreateNote()
  const updateNote = useUpdateNote()
  const deleteNote = useDeleteNote()
  const uploadAttachment = useUploadAttachment()

  // Persisted settings
  const { viewMode, setViewMode } = useSettingsStore()

  // Local state
  const [title, setTitle] = useState('')
  const [content, setContent] = useState('')
  const [tags, setTags] = useState<string[]>([])
  const [isPinned, setIsPinned] = useState(false)
  const [isArchived, setIsArchived] = useState(false)
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false)
  const [copied, setCopied] = useState(false)

  // In-note search state
  const [showSearch, setShowSearch] = useState(false)
  const [searchQuery, setSearchQuery] = useState('')
  const searchInputRef = useRef<HTMLInputElement>(null)
  const textareaRef = useRef<HTMLTextAreaElement>(null)
  const previewRef = useRef<HTMLDivElement>(null)
  const isScrollSyncing = useRef(false)

  // Track which note we've initialized local state from
  const initializedNoteId = useRef<string | null>(null)

  // Track if form is dirty
  const isDirty = useMemo(() => {
    if (isNewNote) {
      return title !== '' || content !== ''
    }
    if (!note) return false
    return (
      title !== note.title ||
      content !== stripFrontmatter(note.content) ||
      JSON.stringify(tags.sort()) !== JSON.stringify([...note.tags].sort()) ||
      isPinned !== note.is_pinned ||
      isArchived !== note.is_archived
    )
  }, [isNewNote, note, title, content, tags, isPinned, isArchived])

  // Initialize form from note data - only on initial load or when navigating to different note
  useEffect(() => {
    if (note && note.id !== initializedNoteId.current) {
      setTitle(note.title)
      // Strip frontmatter from content - tags are managed via UI only
      setContent(stripFrontmatter(note.content))
      setTags(note.tags)
      setIsPinned(note.is_pinned)
      setIsArchived(note.is_archived)
      initializedNoteId.current = note.id
    }
  }, [note])

  // Initialize for new note
  useEffect(() => {
    if (isNewNote) {
      setTitle('')
      setContent('')
      setTags([])
      setIsPinned(false)
      setIsArchived(false)
      initializedNoteId.current = null
    }
  }, [isNewNote])

  // Auto-save debouncing
  const [isSaving, setIsSaving] = useState(false)
  const [lastSaved, setLastSaved] = useState<Date | null>(null)

  const save = useCallback(async () => {
    if (isNewNote) {
      if (!title.trim()) {
        toast.error('Title is required')
        return
      }
      setIsSaving(true)
      try {
        const result = await createNote.mutateAsync({
          title: title.trim(),
          content,
          tags,
        })
        toast.success('Note created')
        navigate(`/notes/${result.id}`, { replace: true })
      } catch {
        toast.error('Failed to create note')
      } finally {
        setIsSaving(false)
      }
    } else if (noteId) {
      setIsSaving(true)
      try {
        await updateNote.mutateAsync({
          id: noteId,
          data: {
            title: title.trim() || undefined,
            content,
            tags,
            is_pinned: isPinned,
            is_archived: isArchived,
          },
        })
        setLastSaved(new Date())
        toast.success('Note saved')
      } catch {
        toast.error('Failed to save note')
      } finally {
        setIsSaving(false)
      }
    }
  }, [isNewNote, noteId, title, content, tags, isPinned, isArchived, createNote, updateNote, navigate])

  // Auto-save effect
  const autoSaveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  useEffect(() => {
    if (!isDirty || isNewNote) {
      if (autoSaveTimerRef.current) {
        clearTimeout(autoSaveTimerRef.current)
        autoSaveTimerRef.current = null
      }
      return
    }

    if (autoSaveTimerRef.current) {
      clearTimeout(autoSaveTimerRef.current)
    }

    autoSaveTimerRef.current = setTimeout(() => {
      save()
    }, 2000) // Auto-save after 2 seconds of inactivity

    return () => {
      if (autoSaveTimerRef.current) {
        clearTimeout(autoSaveTimerRef.current)
        autoSaveTimerRef.current = null
      }
    }
  }, [title, content, tags, isPinned, isArchived, isDirty, isNewNote, save])

  const handleDelete = async () => {
    if (!noteId || isNewNote) return

    try {
      await deleteNote.mutateAsync(noteId)
      toast.success('Note deleted')
      navigate('/notes')
    } catch {
      toast.error('Failed to delete note')
    }
    setDeleteDialogOpen(false)
  }

  // Copy entire note to clipboard
  const handleCopyNote = async () => {
    const fullContent = title ? `# ${title}\n\n${content}` : content
    try {
      await navigator.clipboard.writeText(fullContent)
      setCopied(true)
      toast.success('Note copied to clipboard')
      setTimeout(() => setCopied(false), 2000)
    } catch {
      toast.error('Failed to copy note')
    }
  }

  // Handle Tab/Shift+Tab for indentation in textarea
  const handleTextareaKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    const textarea = e.currentTarget
    const { selectionStart, selectionEnd, value } = textarea

    if (e.key === 'Tab') {
      e.preventDefault()

      if (e.shiftKey) {
        // Dedent: remove leading spaces/tab from current line(s)
        const beforeCursor = value.substring(0, selectionStart)
        const lineStart = beforeCursor.lastIndexOf('\n') + 1
        const beforeSelection = value.substring(0, lineStart)
        const selectedLines = value.substring(lineStart, selectionEnd)
        const afterSelection = value.substring(selectionEnd)

        // Remove 2 spaces or 1 tab from start of each line
        const dedented = selectedLines.replace(/^( {2}|\t)/gm, '')
        const removed = selectedLines.length - dedented.length

        const newContent = beforeSelection + dedented + afterSelection
        setContent(newContent)

        // Adjust cursor position
        requestAnimationFrame(() => {
          textarea.selectionStart = Math.max(lineStart, selectionStart - (selectionStart === lineStart ? 0 : Math.min(2, removed)))
          textarea.selectionEnd = selectionEnd - removed
        })
      } else {
        // Indent: add 2 spaces
        if (selectionStart === selectionEnd) {
          // No selection - just insert spaces at cursor
          const newContent = value.substring(0, selectionStart) + '  ' + value.substring(selectionEnd)
          setContent(newContent)
          requestAnimationFrame(() => {
            textarea.selectionStart = textarea.selectionEnd = selectionStart + 2
          })
        } else {
          // Selection - indent all selected lines
          const beforeCursor = value.substring(0, selectionStart)
          const lineStart = beforeCursor.lastIndexOf('\n') + 1
          const beforeSelection = value.substring(0, lineStart)
          const selectedLines = value.substring(lineStart, selectionEnd)
          const afterSelection = value.substring(selectionEnd)

          const indented = selectedLines.replace(/^/gm, '  ')
          const added = indented.length - selectedLines.length

          const newContent = beforeSelection + indented + afterSelection
          setContent(newContent)

          requestAnimationFrame(() => {
            textarea.selectionStart = selectionStart + 2
            textarea.selectionEnd = selectionEnd + added
          })
        }
      }
    }
  }

  // Track image upload state
  const [isUploading, setIsUploading] = useState(false)

  // Synchronized scrolling for split mode
  // Maps the top visible line in one pane to the other pane using Markdown AST source line ranges
  // (data-source-line + data-source-end-line) so the relationship is non-linear.

  const scrollSyncTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const editorMirrorRef = useRef<HTMLDivElement | null>(null)
  const editorMetricsRef = useRef<{
    content: string
    width: number
    lineTops: number[]
    lineHeights: number[]
  } | null>(null)

  useEffect(() => {
    return () => {
      editorMirrorRef.current?.remove()
      editorMirrorRef.current = null
      editorMetricsRef.current = null
    }
  }, [])

  type PreviewBlockRange = {
    startLine: number
    endLine: number
    top: number
    bottom: number
  }

  // Get all block elements with their source line ranges and positions
  const getPreviewBlockRanges = useCallback((): PreviewBlockRange[] => {
    const preview = previewRef.current
    if (!preview) return []

    const elements = Array.from(preview.querySelectorAll<HTMLElement>('[data-source-line][data-source-end-line]'))
    if (elements.length === 0) return []

    const previewPaddingTop = parseFloat(getComputedStyle(preview).paddingTop) || 0
    const previewRect = preview.getBoundingClientRect()

    const ranges = elements
      .map(el => {
        const startLine = parseInt(el.getAttribute('data-source-line') || '0', 10)
        const endLineRaw = parseInt(el.getAttribute('data-source-end-line') || '0', 10)
        if (startLine <= 0 || endLineRaw <= 0) return null
        const endLine = Math.max(startLine, endLineRaw)

        const elRect = el.getBoundingClientRect()
        const top = elRect.top - previewRect.top + preview.scrollTop - previewPaddingTop
        const height = elRect.height

        return {
          startLine,
          endLine,
          top,
          bottom: top + height,
        }
      })
      .filter((r): r is PreviewBlockRange => r !== null)
      .sort((a, b) => a.startLine - b.startLine || a.top - b.top)

    return ranges
  }, [])

  // Calculate editor line height
  const getLineHeight = useCallback(() => {
    const textarea = textareaRef.current
    if (!textarea) return 21

    // Create a temporary element to measure actual line height
    const temp = document.createElement('div')
    temp.style.cssText = window.getComputedStyle(textarea).cssText
    temp.style.height = 'auto'
    temp.style.position = 'absolute'
    temp.style.visibility = 'hidden'
    temp.style.whiteSpace = 'pre'
    temp.textContent = 'X'
    document.body.appendChild(temp)
    const height = temp.offsetHeight
    document.body.removeChild(temp)

    return height || 21
  }, [])

  const ensureEditorMetrics = useCallback(() => {
    const textarea = textareaRef.current
    if (!textarea) return null

    const width = textarea.clientWidth
    const cached = editorMetricsRef.current
    if (cached && cached.content === content && cached.width === width) return cached

    let mirror = editorMirrorRef.current
    if (!mirror) {
      mirror = document.createElement('div')
      mirror.setAttribute('data-editor-mirror', 'true')
      editorMirrorRef.current = mirror
      document.body.appendChild(mirror)
    }

    const style = window.getComputedStyle(textarea)
    mirror.style.position = 'absolute'
    mirror.style.top = '0'
    mirror.style.left = '-99999px'
    mirror.style.visibility = 'hidden'
    mirror.style.pointerEvents = 'none'
    mirror.style.boxSizing = 'border-box'
    mirror.style.width = `${width}px`
    mirror.style.fontFamily = style.fontFamily
    mirror.style.fontSize = style.fontSize
    mirror.style.fontWeight = style.fontWeight
    mirror.style.fontStyle = style.fontStyle
    mirror.style.letterSpacing = style.letterSpacing
    mirror.style.lineHeight = style.lineHeight
    mirror.style.paddingTop = style.paddingTop
    mirror.style.paddingRight = style.paddingRight
    mirror.style.paddingBottom = style.paddingBottom
    mirror.style.paddingLeft = style.paddingLeft
    mirror.style.border = '0'
    mirror.style.margin = '0'

    mirror.innerHTML = ''

    const lines = content.split('\n')
    for (let i = 0; i < lines.length; i++) {
      const lineEl = document.createElement('div')
      lineEl.style.whiteSpace = 'pre-wrap'
      lineEl.style.wordBreak = 'break-word'
      lineEl.style.overflowWrap = 'anywhere'
      lineEl.style.margin = '0'
      lineEl.textContent = lines[i] === '' ? '\u200b' : lines[i]
      mirror.appendChild(lineEl)
    }

    const paddingTop = parseFloat(style.paddingTop) || 0
    const children = Array.from(mirror.children) as HTMLElement[]
    const lineTops = children.map(el => el.offsetTop - paddingTop)
    const lineHeights = children.map(el => el.offsetHeight)

    const metrics = { content, width, lineTops, lineHeights }
    editorMetricsRef.current = metrics
    return metrics
  }, [content])

  const getEditorTopLine = useCallback(() => {
    const textarea = textareaRef.current
    if (!textarea) return { line: 1, progress: 0, lineFloat: 1 }

    const metrics = ensureEditorMetrics()
    if (!metrics || metrics.lineTops.length === 0) {
      const lineHeight = getLineHeight()
      const lineFloat = 1 + textarea.scrollTop / lineHeight
      const line = Math.max(1, Math.floor(lineFloat))
      return { line, progress: lineFloat - line, lineFloat }
    }

    const y = textarea.scrollTop
    const { lineTops, lineHeights } = metrics

    let lo = 0
    let hi = lineTops.length - 1
    let idx = 0
    while (lo <= hi) {
      const mid = (lo + hi) >> 1
      if (lineTops[mid] <= y) {
        idx = mid
        lo = mid + 1
      } else {
        hi = mid - 1
      }
    }

    const top = lineTops[idx]
    const height = Math.max(1, lineHeights[idx] || 1)
    const progress = Math.min(1, Math.max(0, (y - top) / height))
    const line = idx + 1
    const lineFloat = line + progress
    return { line, progress, lineFloat }
  }, [ensureEditorMetrics, getLineHeight])

  const lineFloatToEditorScrollTop = useCallback((lineFloat: number) => {
    const textarea = textareaRef.current
    if (!textarea) return 0

    const metrics = ensureEditorMetrics()
    if (!metrics || metrics.lineTops.length === 0) {
      const lineHeight = getLineHeight()
      return Math.max(0, (lineFloat - 1) * lineHeight)
    }

    const { lineTops, lineHeights } = metrics
    const clamped = Math.min(Math.max(1, lineFloat), lineTops.length + 1)
    const baseLine = Math.min(lineTops.length, Math.max(1, Math.floor(clamped)))
    const frac = clamped - baseLine
    const idx = baseLine - 1

    return Math.max(0, lineTops[idx] + frac * Math.max(1, lineHeights[idx] || 1))
  }, [ensureEditorMetrics, getLineHeight])

  // Editor scroll -> Preview scroll
  const handleEditorScroll = useCallback(() => {
    if (viewMode !== 'split' || isScrollSyncing.current) return

    const textarea = textareaRef.current
    const preview = previewRef.current
    if (!textarea || !preview) return

    // Clear any pending scroll sync
    if (scrollSyncTimeoutRef.current) {
      clearTimeout(scrollSyncTimeoutRef.current)
    }

    isScrollSyncing.current = true

    const ranges = getPreviewBlockRanges()

    if (ranges.length === 0) {
      // Fallback: simple percentage scroll
      const ratio = textarea.scrollTop / Math.max(1, textarea.scrollHeight - textarea.clientHeight)
      preview.scrollTop = ratio * (preview.scrollHeight - preview.clientHeight)
    } else {
      const { progress, lineFloat } = getEditorTopLine()

      let before = ranges[0]
      let after = ranges[ranges.length - 1]

      for (let i = 0; i < ranges.length; i++) {
        if (ranges[i].startLine <= lineFloat) before = ranges[i]
        if (ranges[i].startLine >= lineFloat) {
          after = ranges[i]
          break
        }
      }

      const last = ranges[ranges.length - 1]
      let targetScroll = 0

      if (lineFloat >= last.endLine) {
        targetScroll = preview.scrollHeight - preview.clientHeight
      } else if (before.startLine <= lineFloat && lineFloat <= before.endLine) {
        const lineRange = before.endLine - before.startLine
        const height = Math.max(1, before.bottom - before.top)
        const within = lineRange > 0 ? (lineFloat - before.startLine) / lineRange : progress
        targetScroll = before.top + Math.min(1, Math.max(0, within)) * height
      } else if (before === after) {
        targetScroll = before.top
      } else {
        const gapLines = Math.max(1, after.startLine - before.endLine)
        const gapPixels = after.top - before.bottom
        const within = (lineFloat - before.endLine) / gapLines
        targetScroll = before.bottom + within * gapPixels
      }

      preview.scrollTop = Math.max(0, targetScroll)
    }

    scrollSyncTimeoutRef.current = setTimeout(() => {
      isScrollSyncing.current = false
    }, 16)
  }, [viewMode, getEditorTopLine, getPreviewBlockRanges])

  // Preview scroll -> Editor scroll
  const handlePreviewScroll = useCallback(() => {
    if (viewMode !== 'split' || isScrollSyncing.current) return

    const textarea = textareaRef.current
    const preview = previewRef.current
    if (!textarea || !preview) return

    if (scrollSyncTimeoutRef.current) {
      clearTimeout(scrollSyncTimeoutRef.current)
    }

    isScrollSyncing.current = true

    const ranges = getPreviewBlockRanges()

    if (ranges.length === 0) {
      // Fallback: simple percentage scroll
      const ratio = preview.scrollTop / Math.max(1, preview.scrollHeight - preview.clientHeight)
      textarea.scrollTop = ratio * (textarea.scrollHeight - textarea.clientHeight)
    } else {
      const scrollTop = preview.scrollTop

      const byTop = [...ranges].sort((a, b) => a.top - b.top)
      let before = byTop[0]
      let after = byTop[byTop.length - 1]

      for (let i = 0; i < byTop.length; i++) {
        const r = byTop[i]
        if (r.top <= scrollTop) before = r
        if (r.bottom >= scrollTop) {
          after = r
          break
        }
      }

      const last = byTop[byTop.length - 1]
      let targetLineFloat = 1

      if (scrollTop >= last.bottom) {
        targetLineFloat = last.endLine
      } else if (before.top <= scrollTop && scrollTop <= before.bottom) {
        const height = Math.max(1, before.bottom - before.top)
        const within = (scrollTop - before.top) / height
        const lineRange = before.endLine - before.startLine
        targetLineFloat = before.startLine + within * (lineRange > 0 ? lineRange : 1)
      } else if (before === after) {
        targetLineFloat = scrollTop < before.top ? before.startLine : before.endLine
      } else {
        const gapPixels = Math.max(1, after.top - before.bottom)
        const gapLines = Math.max(1, after.startLine - before.endLine)
        const within = (scrollTop - before.bottom) / gapPixels
        targetLineFloat = before.endLine + within * gapLines
      }

      textarea.scrollTop = lineFloatToEditorScrollTop(targetLineFloat)
    }

    scrollSyncTimeoutRef.current = setTimeout(() => {
      isScrollSyncing.current = false
    }, 16)
  }, [viewMode, getPreviewBlockRanges, lineFloatToEditorScrollTop])

  // Handle paste event for images
  const handlePaste = useCallback(async (e: React.ClipboardEvent<HTMLTextAreaElement>) => {
    const items = e.clipboardData?.items
    if (!items) return

    for (const item of items) {
      // Check if it's an image (item.type may be empty in some browsers/scenarios)
      if (item.type.startsWith('image/') || item.kind === 'file') {
        const file = item.getAsFile()
        if (!file) continue

        // Skip non-image files
        const mimeType = file.type || item.type
        if (!mimeType.startsWith('image/') && !mimeType) {
          // If no mime type, the backend will detect from magic bytes
        }

        e.preventDefault() // Prevent default paste behavior for images

        setIsUploading(true)
        try {
          const base64Data = await blobToBase64(file)
          const result = await uploadAttachment.mutateAsync({
            data: base64Data,
            mime_type: mimeType || '', // Backend will detect from magic bytes if empty
          })

          // Insert markdown at cursor position
          const textarea = textareaRef.current
          if (textarea) {
            const { selectionStart, selectionEnd, value } = textarea
            const before = value.substring(0, selectionStart)
            const after = value.substring(selectionEnd)
            const newContent = before + result.markdown + after
            setContent(newContent)

            // Move cursor after the inserted markdown
            requestAnimationFrame(() => {
              textarea.selectionStart = textarea.selectionEnd = selectionStart + result.markdown.length
              textarea.focus()
            })
          } else {
            // If no textarea ref, just append to content
            setContent(prev => prev + '\n' + result.markdown)
          }

          toast.success('Image uploaded')
        } catch (err) {
          const message = err instanceof Error ? err.message : 'Failed to upload image'
          toast.error(message)
        } finally {
          setIsUploading(false)
        }
        return // Only handle the first image
      }
    }
  }, [uploadAttachment])

  // Find in note
  const findInNote = useCallback(() => {
    if (!searchQuery || !textareaRef.current) return

    const textarea = textareaRef.current
    const searchLower = searchQuery.toLowerCase()
    const contentLower = content.toLowerCase()
    const currentPos = textarea.selectionEnd || 0

    // Find next occurrence after current position
    let index = contentLower.indexOf(searchLower, currentPos)
    if (index === -1) {
      // Wrap around to start
      index = contentLower.indexOf(searchLower)
    }

    if (index !== -1) {
      textarea.focus()
      textarea.setSelectionRange(index, index + searchQuery.length)
      // Scroll into view
      const lineHeight = 20
      const lines = content.substring(0, index).split('\n').length
      textarea.scrollTop = Math.max(0, (lines - 5) * lineHeight)
    } else {
      toast.error('Not found')
    }
  }, [searchQuery, content])

  // Focus search input when shown
  useEffect(() => {
    if (showSearch && searchInputRef.current) {
      searchInputRef.current.focus()
    }
  }, [showSearch])

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 's') {
        e.preventDefault()
        save()
      }
      if ((e.metaKey || e.ctrlKey) && e.key === 'f') {
        e.preventDefault()
        setShowSearch(prev => !prev)
      }
      if (e.key === 'Escape' && showSearch) {
        setShowSearch(false)
        setSearchQuery('')
      }
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [save, showSearch])

  if (isLoading && !isNewNote) {
    return (
      <div className="space-y-4">
        <Skeleton className="h-10 w-64" />
        <Skeleton className="h-8 w-48" />
        <Skeleton className="h-96" />
      </div>
    )
  }

  if (error && !isNewNote) {
    return (
      <div className="card p-6 text-center">
        <p className="text-error mb-2">Note not found</p>
        <p className="text-text-muted text-sm mb-4">The note may have been deleted or the ID is invalid.</p>
        <button onClick={() => navigate('/notes')} className="btn btn-secondary">
          Back to Notes
        </button>
      </div>
    )
  }

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between mb-4 flex-shrink-0">
        <button
          onClick={() => navigate('/notes')}
          className="flex items-center gap-2 text-text-secondary hover:text-text-primary transition-colors"
        >
          <IconArrowLeft size={18} />
          Back
        </button>

        <div className="flex items-center gap-2">
          {/* View mode toggle */}
          <div className="flex bg-bg-elevated rounded-lg p-0.5 border border-border">
            <Tooltip content="Edit mode">
              <button
                onClick={() => setViewMode('edit')}
                className={cn(
                  'px-2 py-1 rounded text-sm transition-colors',
                  viewMode === 'edit' ? 'bg-bg-hover text-text-primary' : 'text-text-muted hover:text-text-secondary'
                )}
              >
                <IconEdit size={16} />
              </button>
            </Tooltip>
            <Tooltip content="Split view">
              <button
                onClick={() => setViewMode('split')}
                className={cn(
                  'px-2 py-1 rounded text-sm transition-colors',
                  viewMode === 'split' ? 'bg-bg-hover text-text-primary' : 'text-text-muted hover:text-text-secondary'
                )}
              >
                Split
              </button>
            </Tooltip>
            <Tooltip content="Preview mode">
              <button
                onClick={() => setViewMode('preview')}
                className={cn(
                  'px-2 py-1 rounded text-sm transition-colors',
                  viewMode === 'preview' ? 'bg-bg-hover text-text-primary' : 'text-text-muted hover:text-text-secondary'
                )}
              >
                <IconEye size={16} />
              </button>
            </Tooltip>
          </div>

          {/* Search in note */}
          <Tooltip content="Find in note (⌘F)">
            <button
              onClick={() => setShowSearch(prev => !prev)}
              className={cn(
                'p-2 rounded-lg transition-colors',
                showSearch ? 'text-primary bg-primary/10' : 'text-text-muted hover:text-text-secondary hover:bg-bg-hover'
              )}
            >
              <IconSearch size={18} />
            </button>
          </Tooltip>

          {/* Copy note */}
          <Tooltip content="Copy note">
            <button
              onClick={handleCopyNote}
              className={cn(
                'p-2 rounded-lg transition-colors',
                copied ? 'text-success' : 'text-text-muted hover:text-text-secondary hover:bg-bg-hover'
              )}
            >
              {copied ? <IconCheck size={18} /> : <IconCopy size={18} />}
            </button>
          </Tooltip>

          {/* Pin/Archive toggles (only for existing notes) */}
          {!isNewNote && (
            <>
              <Tooltip content={isPinned ? 'Unpin note' : 'Pin note'}>
                <button
                  onClick={() => setIsPinned(!isPinned)}
                  className={cn(
                    'p-2 rounded-lg transition-colors',
                    isPinned ? 'text-warning bg-warning/10' : 'text-text-muted hover:text-text-secondary hover:bg-bg-hover'
                  )}
                >
                  {isPinned ? <IconPinnedOff size={18} /> : <IconPin size={18} />}
                </button>
              </Tooltip>
              <Tooltip content={isArchived ? 'Unarchive note' : 'Archive note'}>
                <button
                  onClick={() => setIsArchived(!isArchived)}
                  className={cn(
                    'p-2 rounded-lg transition-colors',
                    isArchived ? 'text-text-secondary bg-bg-elevated' : 'text-text-muted hover:text-text-secondary hover:bg-bg-hover'
                  )}
                >
                  {isArchived ? <IconArchiveOff size={18} /> : <IconArchive size={18} />}
                </button>
              </Tooltip>
            </>
          )}

          {/* Delete button */}
          {!isNewNote && (
            <Tooltip content="Delete note">
              <button
                onClick={() => setDeleteDialogOpen(true)}
                className="p-2 rounded-lg text-text-muted hover:text-error hover:bg-error/10 transition-colors"
              >
                <IconTrash size={18} />
              </button>
            </Tooltip>
          )}

          {/* Save button */}
          <Tooltip content={isNewNote ? 'Create note' : 'Save note (⌘S)'}>
            <button
              onClick={save}
              disabled={isSaving || (!isDirty && !isNewNote)}
              className={cn(
                'btn btn-primary flex items-center gap-2',
                (!isDirty && !isNewNote) && 'opacity-50 cursor-not-allowed'
              )}
            >
              {isSaving ? (
                <IconLoader2 size={18} className="animate-spin" />
              ) : (
                <IconDeviceFloppy size={18} />
              )}
              {isNewNote ? 'Create' : 'Save'}
            </button>
          </Tooltip>
        </div>
      </div>

      {/* In-note search bar */}
      {showSearch && (
        <div className="flex items-center gap-2 mb-4 p-2 bg-bg-surface rounded-lg border border-border">
          <IconSearch size={16} className="text-text-muted" />
          <input
            ref={searchInputRef}
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                findInNote()
              }
            }}
            placeholder="Find in note..."
            className="flex-1 bg-transparent border-none outline-none text-sm text-text-primary placeholder:text-text-muted"
          />
          <button
            onClick={findInNote}
            className="px-2 py-1 text-xs text-text-secondary hover:text-text-primary"
          >
            Find
          </button>
          <button
            onClick={() => {
              setShowSearch(false)
              setSearchQuery('')
            }}
            className="p-1 text-text-muted hover:text-text-primary"
          >
            <IconX size={14} />
          </button>
        </div>
      )}

      {/* Title */}
      <input
        type="text"
        value={title}
        onChange={(e) => setTitle(e.target.value)}
        className="text-2xl font-bold bg-transparent border-none outline-none w-full text-text-primary mb-3 placeholder:text-text-muted"
        placeholder="Note title..."
      />

      {/* Tags */}
      <div className="mb-4">
        <TagInput tags={tags} onChange={setTags} />
      </div>

      {/* Content area */}
      <div className="flex-1 min-h-[400px] flex gap-4">
        {viewMode !== 'preview' && (
          <div className={cn('flex-1 min-w-0 flex flex-col', viewMode === 'split' && 'max-w-[50%]')}>
            <textarea
              ref={textareaRef}
              value={content}
              onChange={(e) => setContent(e.target.value)}
              onKeyDown={handleTextareaKeyDown}
              onPaste={handlePaste}
              onScroll={handleEditorScroll}
              className="w-full flex-1 min-h-[400px] bg-bg-surface border border-border rounded-lg p-4
                       text-text-primary font-mono text-sm resize-none
                       focus:outline-none focus:border-border-focus"
              placeholder="Write your note in Markdown..."
              disabled={isUploading}
            />
          </div>
        )}
        {viewMode !== 'edit' && (
          <div className={cn('flex-1 min-w-0 flex flex-col', viewMode === 'split' && 'max-w-[50%]')}>
            <div
              ref={previewRef}
              onScroll={handlePreviewScroll}
              className="flex-1 min-h-[400px] bg-bg-surface border border-border rounded-lg p-4 overflow-auto"
            >
              {content ? (
                <MarkdownPreview content={content} />
              ) : (
                <p className="text-text-muted italic">Preview will appear here...</p>
              )}
            </div>
          </div>
        )}
      </div>

      {/* Meta footer */}
      <div className="flex items-center justify-between mt-4 pt-3 border-t border-border text-xs text-text-muted flex-shrink-0">
        <div className="flex items-center gap-4">
          {!isNewNote && note && (
            <>
              <span>Created {new Date(note.created_at).toLocaleString()}</span>
              <span>·</span>
              <span>Updated {new Date(note.updated_at).toLocaleString()}</span>
            </>
          )}
        </div>
        <div className="flex items-center gap-4">
          {lastSaved && <span>Last saved {lastSaved.toLocaleTimeString()}</span>}
          {isDirty && !isSaving && <span className="text-warning">Unsaved changes</span>}
          {isSaving && <span className="text-primary">Saving...</span>}
          {isUploading && <span className="text-primary">Uploading image...</span>}
          <span className="text-text-muted">{content.length} characters</span>
          <kbd className="kbd">⌘S</kbd>
        </div>
      </div>

      {/* Delete confirmation dialog */}
      <ConfirmDialog
        open={deleteDialogOpen}
        onOpenChange={setDeleteDialogOpen}
        title="Delete Note"
        description="Are you sure you want to delete this note? This action cannot be undone."
        confirmLabel="Delete"
        confirmVariant="danger"
        onConfirm={handleDelete}
        isLoading={deleteNote.isPending}
      />
    </div>
  )
}
