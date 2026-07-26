import { getCoreRowModel, useReactTable, type ColumnDef } from '@tanstack/react-table'
import { useMutation, useQuery } from '@tanstack/react-query'
import {
  IconAlertTriangle,
  IconSearch,
  IconShieldExclamation,
  IconShieldCheck,
  IconSparkles,
} from '@tabler/icons-react'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'

import {
  analyzeAuditEvents,
  fetchAuditEvent,
  fetchAuditEvents,
  type AuditAnalysis,
  type AuditEventRecord,
} from '@/api/audit'
import { getApiErrorMessage } from '@/api/http'
import { DataTable } from '@/components/data-table/DataTable'
import { DataTablePagination } from '@/components/data-table/DataTablePagination'
import { PageHeader } from '@/components/PageHeader'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/Button'
import { Card, CardContent } from '@/components/ui/card'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger } from '@/components/ui/select'
import { Sheet, SheetContent, SheetHeader, SheetTitle } from '@/components/ui/sheet'
import { cn } from '@/lib/utils'

type ResultKind = AuditEventRecord['result']
const PAGE_SIZE = 10
const EMPTY_FILTERS = { reqId: '', actor: '', action: '', resourceType: '', result: '' }

function resultBadge(result: ResultKind) {
  switch (result) {
    case 'succeeded':
      return { label: 'Succeeded', className: 'border-success/30 bg-success/10 text-success' }
    case 'denied':
      return { label: 'Denied', className: 'border-destructive/30 bg-destructive/10 text-destructive' }
    case 'failed':
      return { label: 'Failed', className: 'border-warn/30 bg-warn/10 text-warn' }
  }
}

export function AuditPage() {
  const { t } = useTranslation()
  const [page, setPage] = useState(1)
  const [draft, setDraft] = useState(EMPTY_FILTERS)
  const [filters, setFilters] = useState(draft)
  const [selectedId, setSelectedId] = useState<number | null>(null)
  const list = useQuery({
    queryKey: ['audit-events', page, filters],
    queryFn: () => fetchAuditEvents({ page, pageSize: PAGE_SIZE, ...filters }),
  })
  const detail = useQuery({
    queryKey: ['audit-event', selectedId],
    queryFn: () => fetchAuditEvent(selectedId as number),
    enabled: selectedId !== null,
  })
  const analysis = useMutation({ mutationFn: analyzeAuditEvents })
  const pageCount = Math.max(1, Math.ceil((list.data?.total ?? 0) / PAGE_SIZE))
  const selected = detail.data

  const columns = useMemo<ColumnDef<AuditEventRecord>[]>(
    () => [
      {
        accessorKey: 'createdAt',
        header: t('Time'),
        cell: ({ row }) => <span className="text-muted-foreground">{row.original.createdAt}</span>,
      },
      {
        accessorKey: 'actorLabel',
        header: t('Actor'),
        cell: ({ row }) => row.original.actorLabel,
      },
      {
        accessorKey: 'reqId',
        header: t('Request ID'),
        cell: ({ row }) => <code className="text-xs">{row.original.reqId}</code>,
      },
      {
        accessorKey: 'action',
        header: t('Action'),
        cell: ({ row }) => <code className="rounded bg-muted px-1 py-0.5 text-xs">{row.original.action}</code>,
      },
      {
        id: 'resource',
        header: t('Resource'),
        cell: ({ row }) => (
          <div className="flex flex-col">
            <code className="text-xs">{row.original.resourceType}</code>
            <span className="text-xs text-muted-foreground">{row.original.resourceId || '—'}</span>
          </div>
        ),
      },
      {
        accessorKey: 'result',
        header: t('Result'),
        cell: ({ row }) => {
          const badge = resultBadge(row.original.result)
          return (
            <Badge className={cn(badge?.className)} variant="outline">
              {t(badge?.label ?? row.original.result)}
            </Badge>
          )
        },
      },
      {
        accessorKey: 'sourceIp',
        header: t('Source IP'),
        cell: ({ row }) => <span className="text-muted-foreground">{row.original.sourceIp || '—'}</span>,
      },
      {
        id: 'actions',
        header: '',
        enableHiding: false,
        cell: ({ row }) => (
          <Button onClick={() => setSelectedId(row.original.id)} size="sm" variant="ghost">
            {t('View detail')}
          </Button>
        ),
      },
    ],
    [t],
  )

  const table = useReactTable({
    data: list.data?.list ?? [],
    columns,
    pageCount,
    manualPagination: true,
    getCoreRowModel: getCoreRowModel(),
    state: {
      pagination: { pageIndex: page - 1, pageSize: PAGE_SIZE },
    },
  })

  return (
    <div className="space-y-4">
      <PageHeader
        description={
          <h1 className="text-base font-semibold text-foreground">
            {t('Inspect administrative actions, outcomes, and structured changes.')}
          </h1>
        }
      />
      <Card>
        <CardContent className="space-y-3">
          <div className="flex flex-wrap items-center gap-2">
            <Input
              aria-label="Filter by request ID"
              className="w-48"
              onChange={(event) => setDraft((current) => ({ ...current, reqId: event.target.value }))}
              placeholder={t('Request ID')}
              value={draft.reqId}
            />
            <Input
              aria-label="Filter by actor"
              className="w-40"
              onChange={(event) => setDraft((current) => ({ ...current, actor: event.target.value }))}
              placeholder={t('Actor')}
              value={draft.actor}
            />
            <Input
              aria-label="Filter by action"
              className="w-40"
              onChange={(event) => setDraft((current) => ({ ...current, action: event.target.value }))}
              placeholder={t('Action')}
              value={draft.action}
            />
            <Input
              aria-label="Filter by resource"
              className="w-40"
              onChange={(event) => setDraft((current) => ({ ...current, resourceType: event.target.value }))}
              placeholder={t('Resource')}
              value={draft.resourceType}
            />
            <Select
              onValueChange={(value) =>
                setDraft((current) => ({ ...current, result: !value || value === 'all' ? '' : value }))
              }
              value={draft.result || 'all'}
            >
              <SelectTrigger aria-label="Filter by result" className="w-36">
                <span className="flex flex-1 text-left">
                  {draft.result ? t(resultBadge(draft.result as ResultKind).label) : t('All results')}
                </span>
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">{t('All results')}</SelectItem>
                <SelectItem value="succeeded">{t('Succeeded')}</SelectItem>
                <SelectItem value="denied">{t('Denied')}</SelectItem>
                <SelectItem value="failed">{t('Failed')}</SelectItem>
              </SelectContent>
            </Select>
            <Button
              onClick={() => {
                setPage(1)
                setFilters(draft)
              }}
            >
              <IconSearch size={16} />
              {t('Search')}
            </Button>
            <Button disabled={analysis.isPending} onClick={() => analysis.mutate(filters)} variant="secondary">
              <IconSparkles size={16} />
              {analysis.isPending ? t('Analyzing…') : t('Analyze with AI')}
            </Button>
          </div>
          {analysis.isError && (
            <p className="text-sm text-destructive">{t(getApiErrorMessage(analysis.error, 'Audit analysis failed'))}</p>
          )}
          <DataTable
            emptyLabel={t('No audit events')}
            errorLabel={t('Failed to load data')}
            isError={list.isError}
            isLoading={list.isLoading}
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
            totalText={t('Record total', { count: list.data?.total ?? 0 })}
          />
        </CardContent>
      </Card>

      <Dialog
        onOpenChange={(open) => {
          if (!open) analysis.reset()
        }}
        open={Boolean(analysis.data)}
      >
        <DialogContent className="sm:max-w-2xl">
          <DialogHeader>
            <DialogTitle>{t('Audit analysis')}</DialogTitle>
          </DialogHeader>
          {analysis.data && (
            <AuditAnalysisReport
              analysis={analysis.data}
              onOpenEvent={(eventId) => {
                analysis.reset()
                setSelectedId(eventId)
              }}
            />
          )}
        </DialogContent>
      </Dialog>

      <Sheet
        onOpenChange={(open) => {
          if (!open) setSelectedId(null)
        }}
        open={selectedId !== null}
      >
        <SheetContent className="w-full gap-0 sm:max-w-xl" side="right">
          <SheetHeader className="border-b pb-3">
            <SheetTitle>{t('Audit event detail')}</SheetTitle>
          </SheetHeader>
          <div className="flex-1 overflow-auto px-4 pb-4">
            {selected ? (
              <div className="space-y-4">
                <dl className="grid grid-cols-2 gap-3 text-sm">
                  <div>
                    <dt className="text-muted-foreground">{t('Actor')}</dt>
                    <dd className="font-medium">{selected.actorLabel}</dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">{t('Request ID')}</dt>
                    <dd className="break-all font-mono text-xs">{selected.reqId}</dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">{t('Action')}</dt>
                    <dd className="font-medium">{selected.action}</dd>
                  </div>
                  <div className="col-span-2">
                    <dt className="text-muted-foreground">{t('Resource')}</dt>
                    <dd className="font-medium">
                      <code className="rounded bg-muted px-1 py-0.5 text-xs">{selected.resourceType}</code>
                      {' / '}
                      {selected.resourceId || '—'}
                    </dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">{t('Result')}</dt>
                    <dd className="font-medium">
                      {t(
                        selected.result === 'succeeded'
                          ? 'Succeeded'
                          : selected.result === 'denied'
                            ? 'Denied'
                            : 'Failed',
                      )}
                    </dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">{t('Reason code')}</dt>
                    <dd className="font-medium">{selected.reasonCode || '—'}</dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">{t('Source IP')}</dt>
                    <dd className="font-medium">{selected.sourceIp || '—'}</dd>
                  </div>
                  <div className="col-span-2">
                    <dt className="text-muted-foreground">{t('User agent')}</dt>
                    <dd className="font-medium break-all">{selected.userAgent}</dd>
                  </div>
                  <div className="col-span-2">
                    <dt className="text-muted-foreground">{t('Created at')}</dt>
                    <dd className="font-medium">{selected.createdAt}</dd>
                  </div>
                </dl>
                <section className="space-y-1.5">
                  <h3 className="text-sm font-semibold">{t('Changes')}</h3>
                  <pre className="overflow-auto rounded-lg border bg-muted/50 p-3 text-xs">
                    {JSON.stringify(selected.changes, null, 2)}
                  </pre>
                </section>
              </div>
            ) : (
              <p className="text-sm text-muted-foreground">{t('Loading event…')}</p>
            )}
          </div>
        </SheetContent>
      </Sheet>
    </div>
  )
}

function AuditAnalysisReport({
  analysis,
  onOpenEvent,
}: {
  analysis: AuditAnalysis
  onOpenEvent: (eventId: number) => void
}) {
  const { t } = useTranslation()
  const riskLabel = analysis.riskLevel === 'high' ? t('High') : analysis.riskLevel === 'medium' ? t('Medium') : t('Low')
  const RiskIcon =
    analysis.riskLevel === 'high'
      ? IconShieldExclamation
      : analysis.riskLevel === 'medium'
        ? IconAlertTriangle
        : IconShieldCheck
  const riskClassName =
    analysis.riskLevel === 'high'
      ? 'border-destructive/30 bg-destructive/10 text-destructive'
      : analysis.riskLevel === 'medium'
        ? 'border-warn/30 bg-warn/10 text-warn'
        : 'border-success/30 bg-success/10 text-success'

  return (
    <div className="space-y-4">
      <div className="space-y-2 rounded-lg border p-3">
        <div className="flex items-center gap-2">
          <span className="text-xs font-medium text-muted-foreground uppercase tracking-wide">{t('Risk level')}</span>
          <Badge className={riskClassName} variant="outline">
            <RiskIcon aria-hidden size={14} />
            {riskLabel}
          </Badge>
        </div>
        <p className="text-sm">{analysis.summary}</p>
      </div>
      {analysis.findings.length > 0 && (
        <section className="space-y-2">
          <div className="flex items-center gap-2">
            <h3 className="text-sm font-semibold">{t('Findings')}</h3>
            <Badge variant="secondary">{analysis.findings.length}</Badge>
          </div>
          <ol className="space-y-2">
            {analysis.findings.map((finding, index) => (
              <li className="flex gap-3 rounded-lg border p-3" key={`${finding.title}-${finding.eventIds.join('-')}`}>
                <div
                  aria-hidden
                  className="flex size-6 shrink-0 items-center justify-center rounded-full bg-muted text-xs font-medium"
                >
                  {index + 1}
                </div>
                <div className="min-w-0 flex-1 space-y-1">
                  <h4 className="text-sm font-medium">{finding.title}</h4>
                  <p className="text-sm text-muted-foreground">{finding.explanation}</p>
                  {finding.eventIds.length > 0 && (
                    <div className="flex flex-wrap gap-1.5 pt-1">
                      {finding.eventIds.map((eventId) => (
                        <Button key={eventId} onClick={() => onOpenEvent(eventId)} size="sm" variant="outline">
                          {t('Event')} {eventId}
                        </Button>
                      ))}
                    </div>
                  )}
                </div>
              </li>
            ))}
          </ol>
        </section>
      )}
    </div>
  )
}
