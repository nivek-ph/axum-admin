import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { IconPlus, IconSearch } from '@tabler/icons-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'

import {
  createStorage,
  deleteStorage,
  fetchStorages,
  setDefaultStorage,
  setStorageStatus,
  updateStorage,
  type StorageDriver,
  type StoragePayload,
  type StorageRecord,
} from '@/api/storages'
import { useConfirm } from '@/components/ConfirmProvider'
import { PageHeader } from '@/components/PageHeader'
import { Button } from '@/components/ui/Button'
import { Card, CardContent } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { useAuthStore } from '@/stores/auth'

import { StorageCard } from './StorageCard'
import { StorageFormDialog } from './StorageFormDialog'

type DriverFilter = 'all' | StorageDriver

function driverFilterFrom(value: string): DriverFilter {
  if (value === 'local' || value === 's3') return value
  return 'all'
}

export function StoragesPage() {
  const { t } = useTranslation()
  const confirmAction = useConfirm()
  const queryClient = useQueryClient()
  const can = useAuthStore((state) => state.can)
  const [driverFilter, setDriverFilter] = useState<DriverFilter>('all')
  const [draftKeyword, setDraftKeyword] = useState('')
  const [keyword, setKeyword] = useState('')
  const [editing, setEditing] = useState<StorageRecord | null>(null)
  const [dialogOpen, setDialogOpen] = useState(false)

  const query = useQuery({
    queryKey: ['storages', keyword, driverFilter],
    queryFn: () => fetchStorages({ keyword, driver: driverFilter === 'all' ? undefined : driverFilter }),
  })
  const invalidate = () => queryClient.invalidateQueries({ queryKey: ['storages'] })
  const saveMutation = useMutation({
    mutationFn: (payload: StoragePayload) => (editing ? updateStorage(editing.id, payload) : createStorage(payload)),
    onSuccess: async () => {
      toast.success(t(editing ? 'Storage updated' : 'Storage created'))
      setDialogOpen(false)
      await invalidate()
    },
    onError: (error) => toast.error(error.message),
  })
  const statusMutation = useMutation({
    mutationFn: ({ id, enabled }: { id: number; enabled: boolean }) => setStorageStatus(id, enabled),
    onSuccess: invalidate,
    onError: (error) => toast.error(error.message),
  })
  const defaultMutation = useMutation({
    mutationFn: setDefaultStorage,
    onSuccess: async () => {
      toast.success(t('Default storage updated'))
      await invalidate()
    },
    onError: (error) => toast.error(error.message),
  })
  const deleteMutation = useMutation({
    mutationFn: deleteStorage,
    onSuccess: async () => {
      toast.success(t('Storage deleted'))
      await invalidate()
    },
    onError: (error) => toast.error(error.message),
  })

  function openCreate() {
    setEditing(null)
    setDialogOpen(true)
  }

  function openEdit(item: StorageRecord) {
    setEditing(item)
    setDialogOpen(true)
  }

  return (
    <div className="flex flex-col gap-3">
      <PageHeader
        description={<h1 className="text-base font-semibold">{t('Manage local and object storage backends.')}</h1>}
        actions={
          can('system:storage:create') ? (
            <Button onClick={openCreate}>
              <IconPlus size={16} />
              {t('New storage')}
            </Button>
          ) : null
        }
      />

      <div className="flex flex-wrap items-center justify-between gap-2">
        <Tabs onValueChange={(value) => setDriverFilter(driverFilterFrom(value))} value={driverFilter}>
          <TabsList>
            <TabsTrigger value="all">{t('All')}</TabsTrigger>
            <TabsTrigger value="local">{t('Local')}</TabsTrigger>
            <TabsTrigger value="s3">{t('Object storage')}</TabsTrigger>
          </TabsList>
        </Tabs>
        <div className="flex gap-2">
          <Input
            aria-label={t('Search storage')}
            className="w-56"
            onChange={(event) => setDraftKeyword(event.target.value)}
            placeholder={t('Name or code')}
            value={draftKeyword}
          />
          <Button onClick={() => setKeyword(draftKeyword.trim())} variant="outline">
            <IconSearch size={16} />
            {t('Search')}
          </Button>
        </div>
      </div>

      {query.isLoading ? <p className="text-sm text-muted-foreground">{t('Loading…')}</p> : null}
      {query.isError ? <p className="text-sm text-destructive">{t('Failed to load data')}</p> : null}
      {!query.isLoading && !query.isError && query.data?.length === 0 ? (
        <Card>
          <CardContent className="py-8 text-center text-muted-foreground">{t('No storages')}</CardContent>
        </Card>
      ) : null}
      <div className="grid gap-3 lg:grid-cols-2 2xl:grid-cols-3">
        {query.data?.map((item) => (
          <StorageCard
            canDelete={can('system:storage:delete')}
            canSetDefault={can('system:storage:set-default')}
            canUpdate={can('system:storage:update')}
            canUpdateStatus={can('system:storage:update-status')}
            item={item}
            key={item.id}
            onDelete={() =>
              void confirmAction(t('Delete storage "{{name}}"?', { name: item.name })).then(
                (yes) => yes && deleteMutation.mutate(item.id),
              )
            }
            onEdit={() => openEdit(item)}
            onSetDefault={() => defaultMutation.mutate(item.id)}
            onToggleStatus={() => statusMutation.mutate({ id: item.id, enabled: !item.enabled })}
          />
        ))}
      </div>

      <StorageFormDialog
        editing={editing}
        onOpenChange={setDialogOpen}
        onSave={(payload) => saveMutation.mutate(payload)}
        open={dialogOpen}
        saving={saveMutation.isPending}
      />
    </div>
  )
}
