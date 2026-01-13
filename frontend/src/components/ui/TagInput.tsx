import { useState, KeyboardEvent } from 'react'
import { Link } from 'react-router-dom'
import { IconX, IconPlus } from '@tabler/icons-react'
import { cn } from '@/utils/cn'

interface TagInputProps {
  tags: string[]
  onChange: (tags: string[]) => void
  readOnly?: boolean
  className?: string
}

export function TagInput({ tags, onChange, readOnly = false, className }: TagInputProps) {
  const [inputValue, setInputValue] = useState('')
  const [isAdding, setIsAdding] = useState(false)

  const addTag = (tag: string) => {
    const normalizedTag = tag.trim().toLowerCase()
    if (normalizedTag && !tags.includes(normalizedTag)) {
      onChange([...tags, normalizedTag])
    }
    setInputValue('')
    setIsAdding(false)
  }

  const removeTag = (tagToRemove: string) => {
    onChange(tags.filter((t) => t !== tagToRemove))
  }

  const handleKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      e.preventDefault()
      addTag(inputValue)
    } else if (e.key === 'Escape') {
      setInputValue('')
      setIsAdding(false)
    } else if (e.key === 'Backspace' && !inputValue && tags.length > 0) {
      removeTag(tags[tags.length - 1])
    }
  }

  if (readOnly) {
    return (
      <div className={cn('flex flex-wrap gap-2', className)}>
        {tags.map((tag) => (
          <Link
            key={tag}
            to={`/tags/${tag}`}
            className="px-2 py-1 bg-primary/10 text-primary hover:bg-primary/20 rounded-lg text-sm transition-colors"
          >
            {tag}
          </Link>
        ))}
        {tags.length === 0 && (
          <span className="text-text-muted text-sm">No tags</span>
        )}
      </div>
    )
  }

  return (
    <div className={cn('flex flex-wrap gap-2 items-center', className)}>
      {tags.map((tag) => (
        <span
          key={tag}
          className="inline-flex items-center gap-1 px-2 py-1 bg-primary/10 text-primary rounded-lg text-sm"
        >
          {tag}
          <button
            type="button"
            onClick={() => removeTag(tag)}
            className="hover:text-error transition-colors"
          >
            <IconX size={14} />
          </button>
        </span>
      ))}
      {isAdding ? (
        <input
          type="text"
          value={inputValue}
          onChange={(e) => setInputValue(e.target.value)}
          onKeyDown={handleKeyDown}
          onBlur={() => {
            if (inputValue) addTag(inputValue)
            else setIsAdding(false)
          }}
          placeholder="Add tag..."
          className="px-2 py-1 bg-bg-elevated border border-border rounded text-sm text-text-primary focus:outline-none focus:border-primary w-24"
          autoFocus
        />
      ) : (
        <button
          type="button"
          onClick={() => setIsAdding(true)}
          className="inline-flex items-center gap-1 px-2 py-1 text-text-muted hover:text-primary hover:bg-bg-hover rounded-lg text-sm transition-colors"
        >
          <IconPlus size={14} />
          Add tag
        </button>
      )}
    </div>
  )
}

interface TagBadgeProps {
  tag: string
  onClick?: () => void
  onRemove?: () => void
  className?: string
}

export function TagBadge({ tag, onClick, onRemove, className }: TagBadgeProps) {
  const TagComponent = onClick ? 'button' : 'span'

  return (
    <span
      className={cn(
        'inline-flex items-center gap-1 px-2 py-1 bg-primary/10 text-primary rounded-lg text-sm',
        onClick && 'hover:bg-primary/20 cursor-pointer transition-colors',
        className
      )}
    >
      <TagComponent onClick={onClick}>{tag}</TagComponent>
      {onRemove && (
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation()
            onRemove()
          }}
          className="hover:text-error transition-colors ml-0.5"
        >
          <IconX size={14} />
        </button>
      )}
    </span>
  )
}
