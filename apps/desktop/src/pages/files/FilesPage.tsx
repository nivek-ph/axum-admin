import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Copy, Link2, Search, Upload } from 'lucide-react'
import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'

import { deleteFile, fetchFiles, importFileUrl, renameFile, uploadFile, type FileRecord } from '@/api/files'
import { Button } from '@/components/ui/Button'
import { useConfirm } from '@/components/ui/ConfirmProvider'
import { Modal } from '@/components/ui/Modal'

export function FilesPage() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const confirmAction = useConfirm()
  const fileInput = useRef<HTMLInputElement>(null)
  const [page, setPage] = useState(1)
  const [draftFilters, setDraftFilters] = useState({ keyword: '', category: '' })
  const [filters, setFilters] = useState(draftFilters)
  const [uploading, setUploading] = useState(false)
  const [importOpen, setImportOpen] = useState(false)
  const [importForm, setImportForm] = useState({ name: '', url: '', tag: '', category: '' })
  const [renameTarget, setRenameTarget] = useState<FileRecord | null>(null)
  const [renameName, setRenameName] = useState('')
  const query = useQuery({
    queryKey: ['files', page, filters],
    queryFn: () => fetchFiles({ page, pageSize: 10, ...filters }),
  })
  const invalidate = () => queryClient.invalidateQueries({ queryKey: ['files'] })
  const importMutation = useMutation({
    mutationFn: importFileUrl,
    onSuccess: async () => {
      toast.success(t('File imported'))
      setImportOpen(false)
      await invalidate()
    },
    onError: () => toast.error(t('Failed to import file')),
  })
  const renameMutation = useMutation({
    mutationFn: renameFile,
    onSuccess: async () => {
      toast.success(t('File renamed'))
      setRenameTarget(null)
      await invalidate()
    },
    onError: () => toast.error(t('Failed to rename file')),
  })
  const deleteMutation = useMutation({
    mutationFn: deleteFile,
    onSuccess: async () => {
      toast.success(t('File deleted'))
      await invalidate()
    },
    onError: () => toast.error(t('Failed to delete file')),
  })
  const pages = Math.max(1, Math.ceil((query.data?.total ?? 0) / 10))

  async function selectFile(file?: File) {
    if (!file) return
    setUploading(true)
    try {
      await uploadFile(file, { category: filters.category })
      toast.success(t('File uploaded'))
      await invalidate()
    } catch {
      toast.error(t('Failed to upload file'))
    } finally {
      setUploading(false)
      if (fileInput.current) fileInput.current.value = ''
    }
  }

  async function copyUrl(url: string) {
    try {
      await navigator.clipboard.writeText(url)
      toast.success(t('URL copied'))
    } catch {
      toast.error(t('Failed to copy URL'))
    }
  }

  return (
    <div className="page-stack">
      <header className="page-header">
        <div>
          <p className="eyebrow">{t('File library')}</p>
          <h1>{t('Files')}</h1>
          <p>{t('Manage uploads and external file URLs with flat metadata.')}</p>
        </div>
        <div className="header-actions">
          <input
            className="hidden-input"
            onChange={(event) => void selectFile(event.target.files?.[0])}
            ref={fileInput}
            type="file"
          />
          <Button disabled={uploading} onClick={() => fileInput.current?.click()}>
            <Upload size={16} />
            {t(uploading ? 'Uploading…' : 'Upload')}
          </Button>
          <Button
            onClick={() => {
              setImportForm({ name: '', url: '', tag: '', category: filters.category })
              setImportOpen(true)
            }}
            variant="primary"
          >
            <Link2 size={16} />
            {t('Import URL')}
          </Button>
        </div>
      </header>
      <section className="panel">
        <div className="toolbar">
          <input
            aria-label="Filter by name or URL"
            onChange={(event) => setDraftFilters((current) => ({ ...current, keyword: event.target.value }))}
            placeholder={t('Name')}
            value={draftFilters.keyword}
          />
          <input
            aria-label="Filter by category"
            onChange={(event) => setDraftFilters((current) => ({ ...current, category: event.target.value }))}
            placeholder={t('Category')}
            value={draftFilters.category}
          />
          <Button
            onClick={() => {
              setPage(1)
              setFilters(draftFilters)
            }}
          >
            <Search size={16} />
            {t('Search')}
          </Button>
          <Button
            onClick={() => {
              const empty = { keyword: '', category: '' }
              setDraftFilters(empty)
              setFilters(empty)
              setPage(1)
            }}
          >
            {t('Reset')}
          </Button>
        </div>
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th>{t('Name')}</th>
                <th>{t('URL')}</th>
                <th>{t('Ext')}</th>
                <th>{t('Tag')}</th>
                <th>{t('Category')}</th>
                <th>{t('Updated at')}</th>
                <th>{t('Actions')}</th>
              </tr>
            </thead>
            <tbody>
              {query.data?.list.map((item) => (
                <tr key={item.id}>
                  <td>
                    <strong>{item.name}</strong>
                  </td>
                  <td>
                    <div className="file-url-cell">
                      <a className="file-url" href={item.url} rel="noreferrer" target="_blank" title={item.url}>
                        {item.url}
                      </a>
                      <Button aria-label={t('Copy URL')} onClick={() => void copyUrl(item.url)} variant="ghost">
                        <Copy size={14} />
                      </Button>
                    </div>
                  </td>
                  <td>{item.ext}</td>
                  <td>{item.tag || '—'}</td>
                  <td>{item.category || '—'}</td>
                  <td>{item.updatedAt}</td>
                  <td>
                    <div className="row-actions">
                      <Button
                        onClick={() => {
                          setRenameTarget(item)
                          setRenameName(item.name)
                        }}
                        variant="ghost"
                      >
                        {t('Rename')}
                      </Button>
                      <Button
                        onClick={() =>
                          void confirmAction(t('Delete this file?')).then((yes) => {
                            if (yes) deleteMutation.mutate(item.id)
                          })
                        }
                        variant="ghost"
                      >
                        {t('Delete')}
                      </Button>
                    </div>
                  </td>
                </tr>
              ))}
              {!query.isLoading && !query.data?.list.length && (
                <tr>
                  <td className="empty-cell" colSpan={7}>
                    {t('No files')}
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
        <div className="pagination">
          <Button disabled={page <= 1} onClick={() => setPage((current) => current - 1)}>
            {t('Previous')}
          </Button>
          <span>
            {t('Page')} {page} / {pages}
          </span>
          <Button disabled={page >= pages} onClick={() => setPage((current) => current + 1)}>
            {t('Next')}
          </Button>
        </div>
      </section>
      <Modal
        footer={
          <>
            <Button onClick={() => setImportOpen(false)}>{t('Cancel')}</Button>
            <Button
              disabled={importMutation.isPending}
              onClick={() => {
                if (!importForm.name.trim() || !importForm.url.trim()) {
                  toast.error(t('Name and URL are required'))
                  return
                }
                importMutation.mutate(importForm)
              }}
              variant="primary"
            >
              {t('Import')}
            </Button>
          </>
        }
        onOpenChange={setImportOpen}
        open={importOpen}
        title={t('Import URL')}
      >
        <div className="form-grid">
          <label>
            {t('Name')}
            <input
              onChange={(event) => setImportForm((current) => ({ ...current, name: event.target.value }))}
              value={importForm.name}
            />
          </label>
          <label>
            {t('URL')}
            <input
              onChange={(event) => setImportForm((current) => ({ ...current, url: event.target.value }))}
              value={importForm.url}
            />
          </label>
          <label>
            {t('Tag')}
            <input
              onChange={(event) => setImportForm((current) => ({ ...current, tag: event.target.value }))}
              value={importForm.tag}
            />
          </label>
          <label>
            {t('Category')}
            <input
              onChange={(event) => setImportForm((current) => ({ ...current, category: event.target.value }))}
              value={importForm.category}
            />
          </label>
        </div>
      </Modal>
      <Modal
        footer={
          <>
            <Button onClick={() => setRenameTarget(null)}>{t('Cancel')}</Button>
            <Button
              disabled={!renameName.trim() || renameMutation.isPending}
              onClick={() => renameTarget && renameMutation.mutate({ id: renameTarget.id, name: renameName.trim() })}
              variant="primary"
            >
              {t('Save')}
            </Button>
          </>
        }
        onOpenChange={(open) => {
          if (!open) setRenameTarget(null)
        }}
        open={Boolean(renameTarget)}
        title={t('Rename file')}
      >
        <div className="form-grid">
          <label>
            {t('Name')}
            <input onChange={(event) => setRenameName(event.target.value)} value={renameName} />
          </label>
        </div>
      </Modal>
    </div>
  )
}
