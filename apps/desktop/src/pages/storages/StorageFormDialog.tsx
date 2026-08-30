import { useEffect, useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'

import type { StoragePayload, StorageRecord } from '@/api/storages'
import { Button } from '@/components/ui/Button'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'

import { emptyStoragePayload, storagePayloadFor, storagePayloadWithDriver, validateStoragePayload } from './storageForm'

interface StorageFormDialogProps {
  editing: StorageRecord | null
  open: boolean
  onOpenChange: (open: boolean) => void
  onSave: (payload: StoragePayload) => void
  saving: boolean
}

function Field({ children, label, htmlFor }: { children: ReactNode; label: string; htmlFor: string }) {
  return (
    <div className="flex flex-col gap-1.5">
      <Label htmlFor={htmlFor}>{label}</Label>
      {children}
    </div>
  )
}

export function StorageFormDialog({ editing, open, onOpenChange, onSave, saving }: StorageFormDialogProps) {
  const { t } = useTranslation()
  const [form, setForm] = useState<StoragePayload>(() => emptyStoragePayload())

  useEffect(() => {
    if (open) setForm(editing ? storagePayloadFor(editing) : emptyStoragePayload())
  }, [editing, open])

  function save() {
    const error = validateStoragePayload(form)
    if (error) {
      toast.error(t(error))
      return
    }
    onSave(form)
  }

  return (
    <Dialog onOpenChange={onOpenChange} open={open}>
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
              onChange={(event) =>
                setForm(storagePayloadWithDriver(form, event.target.value === 's3' ? 's3' : 'local'))
              }
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
                  placeholder={editing?.driver === 's3' && editing.hasAccessKey ? t('Leave blank to keep current') : ''}
                  value={form.accessKey}
                />
              </Field>
              <Field htmlFor="storage-secret-key" label={t('Secret key')}>
                <Input
                  autoComplete="new-password"
                  id="storage-secret-key"
                  onChange={(event) => setForm({ ...form, secretKey: event.target.value })}
                  placeholder={editing?.driver === 's3' && editing.hasSecretKey ? t('Leave blank to keep current') : ''}
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
          <Button onClick={() => onOpenChange(false)} variant="outline">
            {t('Cancel')}
          </Button>
          <Button disabled={saving} onClick={save}>
            {t('Save')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
