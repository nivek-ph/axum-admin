import { getCoreRowModel, useReactTable, type ColumnDef } from '@tanstack/react-table'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { IconCopy, IconLink, IconPencil, IconSearch, IconTrash, IconUpload } from '@tabler/icons-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'

import {
  deleteFile,
  fetchFiles,
  importFileUrl,
  renameFile,
  type FileRecord,
} from '@/api/files'
import { DataTable } from '@/components/data-table/DataTable'
import { DataTablePagination } from '@/components/data-table/DataTablePagination'
import { PageHeader } from '@/components/PageHeader'
import { Button } from '@/components/ui/Button'
import { Card, CardContent } from '@/components/ui/card'
import { useConfirm } from '@/components/ConfirmProvider'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { FileUploadDialog } from '@/pages/files/FileUploadDialog'

const PAGE_SIZE = 10

export function FilesPage() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const confirmAction = useConfirm()
  const [page, setPage] = useState(1)
  const [draftFilters, setDraftFilters] = useState({ keyword: '', category: '' })
  const [filters, setFilters] = useState(draftFilters)
  const [uploadOpen, setUploadOpen] = useState(false)
  const [importOpen, setImportOpen] = useState(false)
  const [importForm, setImportForm] = useState({ name: '', url: '', tag: '', category: '' })
  const [renameTarget, setRenameTarget] = useState<FileRecord | null>(null)
  const [renameName, setRenameName] = useState('')
  const query = useQuery({
    queryKey: ['files', page, filters],
    queryFn: () => fetchFiles({ page, pageSize: PAGE_SIZE, ...filters }),
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
  const pageCount = Math.max(1, Math.ceil((query.data?.total ?? 0) / PAGE_SIZE))

  async function copyUrl(url: string) {
    try {
      await navigator.clipboard.writeText(url)
      toast.success(t('URL copied'))
    } catch {
      toast.error(t('Failed to copy URL'))
    }
  }

  const columns: ColumnDef<FileRecord>[] = [
    {
      accessorKey: 'name',
      header: t('Name'),
      cell: ({ row }) => <strong className="font-medium">{row.original.name}</strong>,
    },
    {
      accessorKey: 'url',
      header: t('URL'),
      cell: ({ row }) => {
        const item = row.original
        return (
          <div className="flex max-w-64 items-center gap-1">
            <a
              className="truncate text-primary underline-offset-2 hover:underline"
              href={item.url}
              rel="noreferrer"
              target="_blank"
              title={item.url}
            >
              {item.url}
            </a>
            <Button aria-label={t('Copy URL')} onClick={() => void copyUrl(item.url)} size="icon-sm" variant="ghost">
              <IconCopy size={14} />
            </Button>
          </div>
        )
      },
    },
    {
      accessorKey: 'ext',
      header: t('Ext'),
      cell: ({ row }) => row.original.ext,
    },
    {
      accessorKey: 'tag',
      header: t('Tag'),
      cell: ({ row }) => row.original.tag || '—',
    },
    {
      accessorKey: 'category',
      header: t('Category'),
      cell: ({ row }) => row.original.category || '—',
    },
    {
      accessorKey: 'updatedAt',
      header: t('Updated at'),
      cell: ({ row }) => row.original.updatedAt,
    },
    {
      id: 'actions',
      header: t('Actions'),
      enableHiding: false,
      cell: ({ row }) => {
        const item = row.original
        return (
          <div className="flex flex-wrap gap-1">
            <Button
              onClick={() => {
                setRenameTarget(item)
                setRenameName(item.name)
              }}
              variant="ghost"
            >
              <IconPencil size={14} />
              {t('Rename')}
            </Button>
            <Button
              onClick={() =>
                void confirmAction(t('Delete file "{{name}}"?', { name: item.name })).then((yes) => {
                  if (yes) deleteMutation.mutate(item.id)
                })
              }
              variant="ghost"
            >
              <IconTrash size={14} />
              {t('Delete')}
            </Button>
          </div>
        )
      },
    },
  ]

  const table = useReactTable({
    data: query.data?.list ?? [],
    columns,
    pageCount,
    manualPagination: true,
    getCoreRowModel: getCoreRowModel(),
    state: {
      pagination: { pageIndex: page - 1, pageSize: PAGE_SIZE },
    },
  })

  return (
    <div className="flex flex-col gap-3">
      <PageHeader
        description={
          <h1 className="text-base font-semibold text-foreground">
            {t('Manage uploads and external file URLs with flat metadata.')}
          </h1>
        }
        actions={
          <>
            <Button onClick={() => setUploadOpen(true)} variant="outline">
              <IconUpload size={16} />
              {t('Upload')}
            </Button>
            <Button
              onClick={() => {
                setImportForm({ name: '', url: '', tag: '', category: filters.category })
                setImportOpen(true)
              }}
            >
              <IconLink size={16} />
              {t('Import URL')}
            </Button>
          </>
        }
      />
      <Card>
        <CardContent className="flex flex-col gap-3">
          <div className="flex flex-wrap items-center gap-2">
            <Input
              aria-label="Filter by name or URL"
              className="w-40"
              onChange={(event) => setDraftFilters((current) => ({ ...current, keyword: event.target.value }))}
              placeholder={t('Name')}
              value={draftFilters.keyword}
            />
            <Input
              aria-label="Filter by category"
              className="w-40"
              onChange={(event) => setDraftFilters((current) => ({ ...current, category: event.target.value }))}
              placeholder={t('Category')}
              value={draftFilters.category}
            />
            <Button
              onClick={() => {
                setPage(1)
                setFilters(draftFilters)
              }}
              variant="outline"
            >
              <IconSearch size={16} />
              {t('Search')}
            </Button>
            <Button
              onClick={() => {
                const empty = { keyword: '', category: '' }
                setDraftFilters(empty)
                setFilters(empty)
                setPage(1)
              }}
              variant="outline"
            >
              {t('Reset')}
            </Button>
          </div>
          <DataTable
            cellClassName="py-1.5"
            emptyLabel={t('No files')}
            errorLabel={t('Failed to load data')}
            isError={query.isError}
            isLoading={query.isLoading}
            loadingLabel={t('Loading…')}
            table={table}
          />
          <DataTablePagination
            nextLabel={t('Next')}
            onPageChange={setPage}
            page={page}
            pageCount={pageCount}
            pageLabel={t('Page')}
            previousLabel={t('Previous')}
            totalText={t('Record total', { count: query.data?.total ?? 0 })}
          />
        </CardContent>
      </Card>
      <FileUploadDialog
        category={filters.category}
        onOpenChange={setUploadOpen}
        onUploaded={invalidate}
        open={uploadOpen}
      />
      <Dialog onOpenChange={setImportOpen} open={importOpen}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>{t('Import URL')}</DialogTitle>
          </DialogHeader>
          <div className="grid grid-cols-2 gap-3">
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="file-import-name">{t('Name')}</Label>
              <Input
                id="file-import-name"
                onChange={(event) => setImportForm((current) => ({ ...current, name: event.target.value }))}
                value={importForm.name}
              />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="file-import-url">{t('URL')}</Label>
              <Input
                id="file-import-url"
                onChange={(event) => setImportForm((current) => ({ ...current, url: event.target.value }))}
                value={importForm.url}
              />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="file-import-tag">{t('Tag')}</Label>
              <Input
                id="file-import-tag"
                onChange={(event) => setImportForm((current) => ({ ...current, tag: event.target.value }))}
                value={importForm.tag}
              />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="file-import-category">{t('Category')}</Label>
              <Input
                id="file-import-category"
                onChange={(event) => setImportForm((current) => ({ ...current, category: event.target.value }))}
                value={importForm.category}
              />
            </div>
          </div>
          <DialogFooter>
            <Button onClick={() => setImportOpen(false)} variant="outline">
              {t('Cancel')}
            </Button>
            <Button
              disabled={importMutation.isPending}
              onClick={() => {
                if (!importForm.name.trim() || !importForm.url.trim()) {
                  toast.error(t('Name and URL are required'))
                  return
                }
                importMutation.mutate(importForm)
              }}
            >
              {t('Import')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      <Dialog onOpenChange={(open) => !open && setRenameTarget(null)} open={Boolean(renameTarget)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('Rename file')}</DialogTitle>
          </DialogHeader>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="file-rename-name">{t('Name')}</Label>
            <Input id="file-rename-name" onChange={(event) => setRenameName(event.target.value)} value={renameName} />
          </div>
          <DialogFooter>
            <Button onClick={() => setRenameTarget(null)} variant="outline">
              {t('Cancel')}
            </Button>
            <Button
              disabled={!renameName.trim() || renameMutation.isPending}
              onClick={() => renameTarget && renameMutation.mutate({ id: renameTarget.id, name: renameName.trim() })}
            >
              {t('Save')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
