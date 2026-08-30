import { IconFile, IconRefresh, IconTrash, IconUpload } from '@tabler/icons-react'
import { useRef, useState, type DragEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'

import { MAX_UPLOAD_BYTES, uploadFile } from '@/api/files'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/Button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'

type UploadStatus = 'pending' | 'uploading' | 'success' | 'error'

interface UploadItem {
  id: string
  file: File
  progress: number
  status: UploadStatus
}

interface FileUploadDialogProps {
  category: string
  onOpenChange: (open: boolean) => void
  onUploaded: () => Promise<unknown>
  open: boolean
}

function formatFileSize(bytes: number) {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KiB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MiB`
}

function fileType(file: File) {
  return file.name.includes('.') ? file.name.split('.').pop()?.toUpperCase() || '—' : '—'
}

export function FileUploadDialog({ category, onOpenChange, onUploaded, open }: FileUploadDialogProps) {
  const { t } = useTranslation()
  const inputRef = useRef<HTMLInputElement>(null)
  const sequence = useRef(0)
  const [items, setItems] = useState<UploadItem[]>([])
  const [uploading, setUploading] = useState(false)

  function addFiles(files: FileList | File[]) {
    const selected = Array.from(files)
    const accepted = selected.filter((file) => file.size <= MAX_UPLOAD_BYTES)
    if (accepted.length !== selected.length) toast.error(t('File is too large (maximum 1 GiB)'))

    setItems((current) => {
      const keys = new Set(current.map((item) => `${item.file.name}:${item.file.size}:${item.file.lastModified}`))
      const additions = accepted
        .filter((file) => !keys.has(`${file.name}:${file.size}:${file.lastModified}`))
        .map((file) => ({
          id: `upload-${sequence.current++}`,
          file,
          progress: 0,
          status: 'pending' as const,
        }))
      return [...current, ...additions]
    })
    if (inputRef.current) inputRef.current.value = ''
  }

  function updateItem(id: string, change: Partial<Pick<UploadItem, 'progress' | 'status'>>) {
    setItems((current) => current.map((item) => (item.id === id ? { ...item, ...change } : item)))
  }

  async function startUploads(ids?: string[]) {
    if (uploading) return
    const selectedIds = ids ? new Set(ids) : null
    const targets = items.filter(
      (item) => (!selectedIds || selectedIds.has(item.id)) && (item.status === 'pending' || item.status === 'error'),
    )
    if (!targets.length) return

    setUploading(true)
    let succeeded = 0
    let failed = 0
    for (const item of targets) {
      updateItem(item.id, { progress: 0, status: 'uploading' })
      try {
        await uploadFile(item.file, { category }, (progress) => updateItem(item.id, { progress }))
        updateItem(item.id, { progress: 100, status: 'success' })
        succeeded += 1
      } catch {
        updateItem(item.id, { status: 'error' })
        failed += 1
      }
    }
    setUploading(false)
    if (succeeded) {
      await onUploaded()
      toast.success(t('Upload completed'))
    }
    if (failed) toast.error(t('Some files failed to upload'))
  }

  function statusBadge(status: UploadStatus) {
    if (status === 'uploading') return <Badge>{t('Uploading')}</Badge>
    if (status === 'success') return <Badge className="bg-emerald-500/10 text-emerald-700">{t('Completed')}</Badge>
    if (status === 'error') return <Badge variant="destructive">{t('Failed')}</Badge>
    return <Badge variant="secondary">{t('Pending')}</Badge>
  }

  function handleDrop(event: DragEvent<HTMLDivElement>) {
    event.preventDefault()
    if (!uploading) addFiles(event.dataTransfer.files)
  }

  return (
    <Dialog
      onOpenChange={(nextOpen) => {
        if (!nextOpen && uploading) return
        if (!nextOpen) setItems([])
        onOpenChange(nextOpen)
      }}
      open={open}
    >
      <DialogContent className="max-h-[85vh] overflow-hidden sm:max-w-5xl">
        <DialogHeader>
          <DialogTitle>{t('Upload files')}</DialogTitle>
          <DialogDescription>{t('Select or drag files here, then start the upload when ready.')}</DialogDescription>
        </DialogHeader>

        <div className="flex min-h-0 flex-col gap-3 overflow-y-auto pr-1">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <div className="flex items-center gap-2">
              <input
                aria-label={t('Select files')}
                className="hidden"
                multiple
                onChange={(event) => event.target.files && addFiles(event.target.files)}
                ref={inputRef}
                type="file"
              />
              <Button disabled={uploading} onClick={() => inputRef.current?.click()} variant="outline">
                <IconFile />
                {t('Select files')}
              </Button>
              <Button disabled={uploading || items.length === 0} onClick={() => setItems([])} variant="destructive">
                <IconTrash />
                {t('Clear')}
              </Button>
            </div>
            <Button
              disabled={uploading || !items.some((item) => item.status === 'pending' || item.status === 'error')}
              onClick={() => void startUploads()}
            >
              <IconUpload />
              {uploading ? t('Uploading') : t('Start upload')}
            </Button>
          </div>

          <div className="rounded-lg bg-muted/60 px-4 py-3">
            <div className="font-medium">{t('Upload details')}</div>
            <div className="mt-1 flex flex-wrap gap-x-6 gap-y-1 text-sm text-muted-foreground">
              <span>{t('Maximum file size: 1 GiB')}</span>
              <span>{t('Chunk size: 4 MiB')}</span>
              <span>{t('Interrupted uploads can be resumed within one hour')}</span>
            </div>
          </div>

          <div
            className="rounded-lg border border-dashed px-4 py-5 text-center transition-colors hover:bg-muted/30"
            onDragOver={(event) => event.preventDefault()}
            onDrop={handleDrop}
          >
            <IconUpload className="mx-auto mb-2 text-muted-foreground" size={26} />
            <div className="font-medium">{t('Drag files here')}</div>
            <div className="mt-1 text-sm text-muted-foreground">{t('You can select multiple files at once')}</div>
          </div>

          <div className="overflow-hidden rounded-lg border">
            <Table>
              <TableHeader className="bg-muted/50">
                <TableRow>
                  <TableHead>{t('Name')}</TableHead>
                  <TableHead>{t('File type')}</TableHead>
                  <TableHead>{t('File size')}</TableHead>
                  <TableHead className="min-w-40">{t('Progress')}</TableHead>
                  <TableHead>{t('Status')}</TableHead>
                  <TableHead className="text-right">{t('Actions')}</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {items.length === 0 ? (
                  <TableRow>
                    <TableCell className="h-28 text-center text-muted-foreground" colSpan={6}>
                      {t('No files selected')}
                    </TableCell>
                  </TableRow>
                ) : (
                  items.map((item) => (
                    <TableRow key={item.id}>
                      <TableCell className="max-w-64 truncate font-medium" title={item.file.name}>
                        {item.file.name}
                      </TableCell>
                      <TableCell>{fileType(item.file)}</TableCell>
                      <TableCell>{formatFileSize(item.file.size)}</TableCell>
                      <TableCell>
                        <div className="flex items-center gap-2">
                          <div
                            aria-label={item.file.name}
                            aria-valuemax={100}
                            aria-valuemin={0}
                            aria-valuenow={item.progress}
                            className="h-1.5 flex-1 overflow-hidden rounded-full bg-muted"
                            role="progressbar"
                          >
                            <div
                              className="h-full bg-primary transition-[width]"
                              style={{ width: `${item.progress}%` }}
                            />
                          </div>
                          <span className="w-9 text-right text-xs text-muted-foreground">{item.progress}%</span>
                        </div>
                      </TableCell>
                      <TableCell>{statusBadge(item.status)}</TableCell>
                      <TableCell>
                        <div className="flex justify-end gap-1">
                          {item.status === 'error' && (
                            <Button onClick={() => void startUploads([item.id])} size="sm" variant="ghost">
                              <IconRefresh />
                              {t('Retry')}
                            </Button>
                          )}
                          {item.status !== 'uploading' && (
                            <Button
                              aria-label={t('Remove {{name}}', { name: item.file.name })}
                              disabled={uploading}
                              onClick={() =>
                                setItems((current) => current.filter((currentItem) => currentItem.id !== item.id))
                              }
                              size="icon-sm"
                              variant="ghost"
                            >
                              <IconTrash />
                            </Button>
                          )}
                        </div>
                      </TableCell>
                    </TableRow>
                  ))
                )}
              </TableBody>
            </Table>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}
