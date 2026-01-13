import { useMutation } from '@tanstack/react-query'

interface UploadAttachmentRequest {
  data: string // Base64-encoded image data
  mime_type: string
  filename?: string
}

interface AttachmentResponse {
  filename: string
  url: string
  markdown: string
}

async function uploadAttachment(req: UploadAttachmentRequest): Promise<AttachmentResponse> {
  const res = await fetch('/api/attachments', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(req),
  })

  if (!res.ok) {
    const error = await res.text()
    throw new Error(error || `Upload failed: ${res.status}`)
  }

  return res.json()
}

export function useUploadAttachment() {
  return useMutation({
    mutationFn: uploadAttachment,
  })
}

/**
 * Convert a Blob to a base64 string
 */
export async function blobToBase64(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onloadend = () => {
      const dataUrl = reader.result as string
      // Remove the data:image/xxx;base64, prefix
      const base64 = dataUrl.split(',')[1]
      resolve(base64)
    }
    reader.onerror = reject
    reader.readAsDataURL(blob)
  })
}
