import { IconCheck, IconCloud, IconDatabase, IconFolder, IconPencil, IconTrash } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'

import type { StorageRecord } from '@/api/storages'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/Button'
import { Card, CardAction, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card'

interface StorageCardProps {
  item: StorageRecord
  canDelete: boolean
  canSetDefault: boolean
  canUpdate: boolean
  canUpdateStatus: boolean
  onDelete: () => void
  onEdit: () => void
  onSetDefault: () => void
  onToggleStatus: () => void
}

function detailRows(item: StorageRecord) {
  if (item.driver === 'local') return [{ label: 'Root directory', value: item.root }]
  return [
    { label: 'Bucket', value: item.bucket },
    { label: 'Root path', value: item.root ?? '—' },
    { label: 'Region', value: item.region },
    { label: 'Endpoint', value: item.endpoint ?? 'AWS' },
    { label: 'Public URL', value: item.publicBaseUrl },
  ]
}

export function StorageCard({
  item,
  canDelete,
  canSetDefault,
  canUpdate,
  canUpdateStatus,
  onDelete,
  onEdit,
  onSetDefault,
  onToggleStatus,
}: StorageCardProps) {
  const { t } = useTranslation()
  const DriverIcon = item.driver === 'local' ? IconFolder : IconCloud

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <DriverIcon className="size-4" />
          {item.name}
          {item.isDefault ? <Badge>{t('Default')}</Badge> : null}
          <Badge variant={item.enabled ? 'secondary' : 'outline'}>{t(item.enabled ? 'Enabled' : 'Disabled')}</Badge>
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
        {canSetDefault && !item.isDefault ? (
          <Button disabled={!item.enabled} onClick={onSetDefault} size="sm" variant="outline">
            <IconCheck size={14} />
            {t('Set default')}
          </Button>
        ) : null}
        {canUpdateStatus && !item.isDefault ? (
          <Button onClick={onToggleStatus} size="sm" variant="outline">
            {t(item.enabled ? 'Disable' : 'Enable')}
          </Button>
        ) : null}
        {canUpdate ? (
          <Button onClick={onEdit} size="sm" variant="ghost">
            <IconPencil size={14} />
            {t('Edit')}
          </Button>
        ) : null}
        {canDelete && !item.isDefault ? (
          <Button onClick={onDelete} size="sm" variant="ghost">
            <IconTrash size={14} />
            {t('Delete')}
          </Button>
        ) : null}
      </CardFooter>
    </Card>
  )
}
