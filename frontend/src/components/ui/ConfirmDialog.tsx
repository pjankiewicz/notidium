import * as Dialog from '@radix-ui/react-dialog'
import { IconX } from '@tabler/icons-react'

interface ConfirmDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  title: string
  description: string
  confirmLabel?: string
  confirmVariant?: 'primary' | 'danger'
  onConfirm: () => void
  isLoading?: boolean
}

export function ConfirmDialog({
  open,
  onOpenChange,
  title,
  description,
  confirmLabel = 'Confirm',
  confirmVariant = 'primary',
  onConfirm,
  isLoading = false,
}: ConfirmDialogProps) {
  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0" />
        <Dialog.Content className="fixed left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 z-50 w-full max-w-md bg-bg-surface border border-border rounded-lg shadow-xl p-6 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95">
          <div className="flex items-center justify-between mb-4">
            <Dialog.Title className="text-lg font-semibold text-text-primary">
              {title}
            </Dialog.Title>
            <Dialog.Close className="p-1 rounded text-text-secondary hover:text-text-primary hover:bg-bg-hover">
              <IconX size={18} />
            </Dialog.Close>
          </div>

          <Dialog.Description className="text-text-secondary mb-6">
            {description}
          </Dialog.Description>

          <div className="flex justify-end gap-3">
            <Dialog.Close className="btn btn-secondary" disabled={isLoading}>
              Cancel
            </Dialog.Close>
            <button
              onClick={onConfirm}
              disabled={isLoading}
              className={
                confirmVariant === 'danger'
                  ? 'btn bg-error hover:bg-error/80 text-white'
                  : 'btn btn-primary'
              }
            >
              {isLoading ? 'Loading...' : confirmLabel}
            </button>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  )
}
