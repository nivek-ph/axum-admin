import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import {
  IconCheck,
  IconCloud,
  IconDatabase,
  IconFolder,
  IconPencil,
  IconPlus,
  IconSearch,
  IconTrash,
} from '@tabler/icons-react'
import { useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'

import {
  createStorage,
  deleteStorage,
  fetchStorages,
  setDefaultStorage,
  setStorageStatus,
  updateStorage,
  type StoragePayload,
  type StorageRecord,
  type StorageDriver,
} from '@/api/storages'
import { useConfirm } from '@/components/ConfirmProvider'
import { PageHeader } from '@/components/PageHeader'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/Button'
import { Card, CardAction, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { useAuthStore } from '@/stores/auth'

type DriverFilter = 'all' | StorageDriver

const emptyForm: StoragePayload = {
  name: '',
  code: '',
  driver: 'local',
  root: './uploads',
  bucket: '',
  region: '',
  endpoint: '',
  publicBaseUrl: '',
  accessKey: '',
  secretKey: '',
  virtualHostStyle: false,
  enabled: true,
  sort: 999,
  description: '',
}

function Field({ children, label, htmlFor }: { children: ReactNode; label: string; htmlFor: string }) {
  return (
    <div className="flex flex-col gap-1.5">
      <Label htmlFor={htmlFor}>{label}</Label>
      {children}
    </div>
  )
}

function detailRows(item: StorageRecord) {
  if (item.driver === 'local') return [{ label: 'Root directory', value: item.root || '—' }]
  return [
    { label: 'Bucket', value: item.bucket || '—' },
    { label: 'Root path', value: item.root || '—' },
    { label: 'Region', value: item.region || '—' },
    { label: 'Endpoint', value: item.endpoint || 'AWS' },
    { label: 'Public URL', value: item.publicBaseUrl || '—' },
  ]
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
  const [form, setForm] = useState<StoragePayload>(emptyForm)
  const query = useQuery({
    queryKey: ['storages', keyword, driverFilter],
    queryFn: () => fetchStorages({ keyword, driver: driverFilter === 'all' ? undefined : driverFilter }),
  })
  const invalidate = () => queryClient.invalidateQueries({ queryKey: ['storages'] })
  const saveMutation = useMutation({
    mutationFn: () => (editing ? updateStorage(editing.id, form) : createStorage(form)),
    onSuccess: async () => {
      toast.success(t(editing ? 'Storage updated' : 'Storage created'))
      setDialogOpen(false)
      await invalidate()
    },
    onError: (error) => toast.error(error.message),
  })
  const statusMutation = useMutation({
    mutationFn: ({ id, enabled }: { id: number; enabled: boolean }) => setStorageStatus(id, enabled),
    onSuccess: async () => invalidate(),
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
    setForm({ ...emptyForm })
    setDialogOpen(true)
  }

  function openEdit(item: StorageRecord) {
    setEditing(item)
    setForm({
      name: item.name,
      code: item.code,
      driver: item.driver,
      root: item.root || '',
      bucket: item.bucket || '',
      region: item.region || '',
      endpoint: item.endpoint || '',
      publicBaseUrl: item.publicBaseUrl || '',
      accessKey: '',
      secretKey: '',
      virtualHostStyle: item.virtualHostStyle,
      enabled: item.enabled,
      sort: item.sort,
      description: item.description,
    })
    setDialogOpen(true)
  }

  function save() {
    if (!form.name.trim() || !form.code.trim()) {
      toast.error(t('Name and code are required'))
      return
    }
    if (form.driver === 'local' && !form.root?.trim()) {
      toast.error(t('Root directory is required'))
      return
    }
    if (form.driver === 's3' && (!form.bucket?.trim() || !form.region?.trim() || !form.publicBaseUrl?.trim())) {
      toast.error(t('Bucket, region, and public URL are required'))
      return
    }
    saveMutation.mutate()
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
        <Tabs onValueChange={(value) => setDriverFilter(value as DriverFilter)} value={driverFilter}>
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
        {query.data?.map((item) => {
          const DriverIcon = item.driver === 'local' ? IconFolder : IconCloud
          return (
            <Card key={item.id}>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <DriverIcon className="size-4" />
                  {item.name}
                  {item.isDefault ? <Badge>{t('Default')}</Badge> : null}
                  <Badge variant={item.enabled ? 'secondary' : 'outline'}>
                    {t(item.enabled ? 'Enabled' : 'Disabled')}
                  </Badge>
                </CardTitle>
                <CardDescription>{item.code}</CardDescription>
                <CardAction>
                  <IconDatabase className="size-5 text-muted-foreground" />
                </CardAction>
              </CardHeader>
              <CardContent className="flex flex-col gap-2">
                {detailRows(item).map((row) => (
                  <div className="grid grid-cols-[6rem_1fr] gap-2 text-xs" key={row.label}>
                    <span className="text-muted-foreground">{t(row.label)}</span>
                    <span className="truncate" title={row.value}>
                      {row.value}
                    </span>
                  </div>
                ))}
                {item.description ? <p className="pt-1 text-xs text-muted-foreground">{item.description}</p> : null}
              </CardContent>
              <CardFooter className="flex flex-wrap gap-1.5">
                {can('system:storage:set-default') && !item.isDefault ? (
                  <Button
                    disabled={!item.enabled}
                    onClick={() => defaultMutation.mutate(item.id)}
                    size="sm"
                    variant="outline"
                  >
                    <IconCheck size={14} />
                    {t('Set default')}
                  </Button>
                ) : null}
                {can('system:storage:update-status') && !item.isDefault ? (
                  <Button
                    onClick={() => statusMutation.mutate({ id: item.id, enabled: !item.enabled })}
                    size="sm"
                    variant="outline"
                  >
                    {t(item.enabled ? 'Disable' : 'Enable')}
                  </Button>
                ) : null}
                {can('system:storage:update') ? (
                  <Button onClick={() => openEdit(item)} size="sm" variant="ghost">
                    <IconPencil size={14} />
                    {t('Edit')}
                  </Button>
                ) : null}
                {can('system:storage:delete') && !item.isDefault ? (
                  <Button
                    onClick={() =>
                      void confirmAction(t('Delete storage "{{name}}"?', { name: item.name })).then(
                        (yes) => yes && deleteMutation.mutate(item.id),
                      )
                    }
                    size="sm"
                    variant="ghost"
                  >
                    <IconTrash size={14} />
                    {t('Delete')}
                  </Button>
                ) : null}
              </CardFooter>
            </Card>
          )
        })}
      </div>

      <Dialog onOpenChange={setDialogOpen} open={dialogOpen}>
        <DialogContent className="max-h-[85vh] overflow-y-auto sm:max-w-2xl">
          <DialogHeader>
            <DialogTitle>{t(editing ? 'Edit storage' : 'New storage')}</DialogTitle>
          </DialogHeader>
          <div className="grid gap-3 sm:grid-cols-2">
            <Field htmlFor="storage-name" label={t('Name')}>
              <Input
                id="storage-name"
                onChange={(event) => setForm({ ...form, name: event.target.value })}
                value={form.name}
              />
            </Field>
            <Field htmlFor="storage-code" label={t('Code')}>
              <Input
                disabled={Boolean(editing)}
                id="storage-code"
                onChange={(event) => setForm({ ...form, code: event.target.value })}
                value={form.code}
              />
            </Field>
            <Field htmlFor="storage-driver" label={t('Type')}>
              <select
                className="h-9 rounded-md border border-input bg-transparent px-3 text-sm"
                disabled={Boolean(editing)}
                id="storage-driver"
                onChange={(event) => {
                  const driver = event.target.value as StorageDriver
                  setForm({ ...form, driver, root: driver === 'local' ? './uploads' : '' })
                }}
                value={form.driver}
              >
                <option value="local">{t('Local')}</option>
                <option value="s3">{t('Object storage')}</option>
              </select>
            </Field>
            <Field htmlFor="storage-sort" label={t('Sort')}>
              <Input
                id="storage-sort"
                min={0}
                onChange={(event) => setForm({ ...form, sort: Number(event.target.value) })}
                type="number"
                value={form.sort}
              />
            </Field>
            {form.driver === 'local' ? (
              <div className="sm:col-span-2">
                <Field htmlFor="storage-root" label={t('Root directory')}>
                  <Input
                    disabled={Boolean(editing)}
                    id="storage-root"
                    onChange={(event) => setForm({ ...form, root: event.target.value })}
                    value={form.root}
                  />
                </Field>
              </div>
            ) : (
              <>
                <Field htmlFor="storage-bucket" label={t('Bucket')}>
                  <Input
                    disabled={Boolean(editing)}
                    id="storage-bucket"
                    onChange={(event) => setForm({ ...form, bucket: event.target.value })}
                    value={form.bucket}
                  />
                </Field>
                <Field htmlFor="storage-region" label={t('Region')}>
                  <Input
                    disabled={Boolean(editing)}
                    id="storage-region"
                    onChange={(event) => setForm({ ...form, region: event.target.value })}
                    value={form.region}
                  />
                </Field>
                <Field htmlFor="storage-endpoint" label={t('Endpoint')}>
                  <Input
                    disabled={Boolean(editing)}
                    id="storage-endpoint"
                    onChange={(event) => setForm({ ...form, endpoint: event.target.value })}
                    value={form.endpoint}
                  />
                </Field>
                <Field htmlFor="storage-root" label={t('Root path')}>
                  <Input
                    disabled={Boolean(editing)}
                    id="storage-root"
                    onChange={(event) => setForm({ ...form, root: event.target.value })}
                    placeholder="uploads"
                    value={form.root}
                  />
                </Field>
                <div className="sm:col-span-2">
                  <Field htmlFor="storage-public-url" label={t('Public URL')}>
                    <Input
                      disabled={Boolean(editing)}
                      id="storage-public-url"
                      onChange={(event) => setForm({ ...form, publicBaseUrl: event.target.value })}
                      value={form.publicBaseUrl}
                    />
                  </Field>
                </div>
                <Field htmlFor="storage-access-key" label={t('Access key')}>
                  <Input
                    autoComplete="off"
                    id="storage-access-key"
                    onChange={(event) => setForm({ ...form, accessKey: event.target.value })}
                    placeholder={editing?.hasAccessKey ? t('Leave blank to keep current') : ''}
                    value={form.accessKey}
                  />
                </Field>
                <Field htmlFor="storage-secret-key" label={t('Secret key')}>
                  <Input
                    autoComplete="new-password"
                    id="storage-secret-key"
                    onChange={(event) => setForm({ ...form, secretKey: event.target.value })}
                    placeholder={editing?.hasSecretKey ? t('Leave blank to keep current') : ''}
                    type="password"
                    value={form.secretKey}
                  />
                </Field>
              </>
            )}
            <label className="flex items-center gap-2 text-sm">
              <input
                checked={form.enabled}
                disabled={Boolean(editing?.isDefault)}
                onChange={(event) => setForm({ ...form, enabled: event.target.checked })}
                type="checkbox"
              />
              {t('Enabled')}
            </label>
            {form.driver === 's3' ? (
              <label className="flex items-center gap-2 text-sm">
                <input
                  checked={form.virtualHostStyle}
                  disabled={Boolean(editing)}
                  onChange={(event) => setForm({ ...form, virtualHostStyle: event.target.checked })}
                  type="checkbox"
                />
                {t('Virtual-hosted style')}
              </label>
            ) : null}
            <div className="sm:col-span-2">
              <Field htmlFor="storage-description" label={t('Description')}>
                <Input
                  id="storage-description"
                  onChange={(event) => setForm({ ...form, description: event.target.value })}
                  value={form.description}
                />
              </Field>
            </div>
          </div>
          <DialogFooter>
            <Button onClick={() => setDialogOpen(false)} variant="outline">
              {t('Cancel')}
            </Button>
            <Button disabled={saveMutation.isPending} onClick={save}>
              {t('Save')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
