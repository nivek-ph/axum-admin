import { useMutation, useQuery } from '@tanstack/react-query'
import { AlertTriangle, Search, ShieldAlert, ShieldCheck, Sparkles } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import {
  analyzeAuditEvents,
  fetchAuditEvent,
  fetchAuditEvents,
  type AuditAnalysis,
  type AuditEventRecord,
} from '@/api/audit'
import { getApiErrorMessage } from '@/api/http'
import { Button } from '@/components/ui/Button'
import { Modal } from '@/components/ui/Modal'

export function AuditPage() {
  const { t } = useTranslation()
  const [page, setPage] = useState(1)
  const [draft, setDraft] = useState({ actor: '', action: '', resourceType: '', result: '' })
  const [filters, setFilters] = useState(draft)
  const [selectedId, setSelectedId] = useState<number | null>(null)
  const list = useQuery({
    queryKey: ['audit-events', page, filters],
    queryFn: () => fetchAuditEvents({ page, pageSize: 10, ...filters }),
  })
  const detail = useQuery({
    queryKey: ['audit-event', selectedId],
    queryFn: () => fetchAuditEvent(selectedId as number),
    enabled: selectedId !== null,
  })
  const analysis = useMutation({ mutationFn: analyzeAuditEvents })
  const pages = Math.max(1, Math.ceil((list.data?.total ?? 0) / 10))
  const selected = detail.data

  return (
    <div className="page-stack">
      <header className="page-header">
        <div>
          <p className="eyebrow">{t('Operational trace')}</p>
          <h1>{t('Audit events')}</h1>
          <p>{t('Inspect administrative actions, outcomes, and structured changes.')}</p>
        </div>
      </header>
      <section className="panel">
        <div className="toolbar audit-filters">
          <input
            aria-label="Filter by actor"
            onChange={(event) => setDraft((current) => ({ ...current, actor: event.target.value }))}
            placeholder={t('Actor')}
            value={draft.actor}
          />
          <input
            aria-label="Filter by action"
            onChange={(event) => setDraft((current) => ({ ...current, action: event.target.value }))}
            placeholder={t('Action')}
            value={draft.action}
          />
          <input
            aria-label="Filter by resource"
            onChange={(event) => setDraft((current) => ({ ...current, resourceType: event.target.value }))}
            placeholder={t('Resource')}
            value={draft.resourceType}
          />
          <select
            aria-label="Filter by result"
            onChange={(event) => setDraft((current) => ({ ...current, result: event.target.value }))}
            value={draft.result}
          >
            <option value="">{t('All results')}</option>
            <option value="succeeded">{t('Succeeded')}</option>
            <option value="denied">{t('Denied')}</option>
            <option value="failed">{t('Failed')}</option>
          </select>
          <Button
            onClick={() => {
              setPage(1)
              setFilters(draft)
            }}
          >
            <Search size={16} />
            {t('Search')}
          </Button>
          <Button disabled={analysis.isPending} onClick={() => analysis.mutate(filters)} variant="secondary">
            <Sparkles size={16} />
            {analysis.isPending ? t('Analyzing…') : t('Analyze with AI')}
          </Button>
        </div>
        {analysis.isError && (
          <p className="form-error">{getApiErrorMessage(analysis.error, t('Audit analysis failed'))}</p>
        )}
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th>{t('Time')}</th>
                <th>{t('Actor')}</th>
                <th>{t('Action')}</th>
                <th>{t('Resource')}</th>
                <th>{t('Result')}</th>
                <th>{t('Source IP')}</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {list.data?.list.map((item) => (
                <AuditRow item={item} key={item.id} onOpen={() => setSelectedId(item.id)} />
              ))}
              {!list.isLoading && !list.data?.list.length && (
                <tr>
                  <td className="empty-cell" colSpan={7}>
                    {t('No audit events')}
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
        className="audit-analysis-modal"
        onOpenChange={(open) => {
          if (!open) analysis.reset()
        }}
        open={Boolean(analysis.data)}
        title={t('Audit analysis')}
      >
        {analysis.data && (
          <AuditAnalysisReport
            analysis={analysis.data}
            onOpenEvent={(eventId) => {
              analysis.reset()
              setSelectedId(eventId)
            }}
          />
        )}
      </Modal>
      <Modal
        onOpenChange={(open) => {
          if (!open) setSelectedId(null)
        }}
        open={selectedId !== null}
        title={t('Audit event detail')}
      >
        {selected ? (
          <div className="audit-detail">
            <dl>
              <dt>{t('Actor')}</dt>
              <dd>{selected.actorLabel}</dd>
              <dt>{t('Action')}</dt>
              <dd>{selected.action}</dd>
              <dt>{t('Resource')}</dt>
              <dd>
                {selected.resourceType} / {selected.resourceId || '—'}
              </dd>
              <dt>{t('Result')}</dt>
              <dd>
                {t(selected.result === 'succeeded' ? 'Succeeded' : selected.result === 'denied' ? 'Denied' : 'Failed')}
              </dd>
              <dt>{t('Reason code')}</dt>
              <dd>{selected.reasonCode || '—'}</dd>
              <dt>{t('Source IP')}</dt>
              <dd>{selected.sourceIp || '—'}</dd>
              <dt>{t('User agent')}</dt>
              <dd>{selected.userAgent}</dd>
              <dt>{t('Created at')}</dt>
              <dd>{selected.createdAt}</dd>
            </dl>
            <h3>{t('Changes')}</h3>
            <pre>{JSON.stringify(selected.changes, null, 2)}</pre>
          </div>
        ) : (
          <p>{t('Loading event…')}</p>
        )}
      </Modal>
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
    analysis.riskLevel === 'high' ? ShieldAlert : analysis.riskLevel === 'medium' ? AlertTriangle : ShieldCheck

  return (
    <div className="audit-analysis">
      <div className={`audit-analysis-hero risk-${analysis.riskLevel}`}>
        <div className="audit-analysis-risk">
          <span className="eyebrow">{t('Risk level')}</span>
          <span className={`status risk-${analysis.riskLevel}`}>
            <RiskIcon aria-hidden size={14} />
            {riskLabel}
          </span>
        </div>
        <p>{analysis.summary}</p>
      </div>
      {analysis.findings.length > 0 && (
        <section className="audit-analysis-findings">
          <div className="audit-analysis-findings-head">
            <h3>{t('Findings')}</h3>
            <span>{analysis.findings.length}</span>
          </div>
          <ol className="audit-finding-list">
            {analysis.findings.map((finding, index) => (
              <li className="audit-finding" key={`${finding.title}-${finding.eventIds.join('-')}`}>
                <div className="audit-finding-index" aria-hidden>
                  {index + 1}
                </div>
                <div className="audit-finding-body">
                  <h4>{finding.title}</h4>
                  <p>{finding.explanation}</p>
                  {finding.eventIds.length > 0 && (
                    <div className="audit-finding-events">
                      {finding.eventIds.map((eventId) => (
                        <Button
                          className="audit-event-chip"
                          key={eventId}
                          onClick={() => onOpenEvent(eventId)}
                          variant="ghost"
                        >
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

function AuditRow({ item, onOpen }: { item: AuditEventRecord; onOpen: () => void }) {
  const { t } = useTranslation()
  return (
    <tr>
      <td>{item.createdAt}</td>
      <td>{item.actorLabel}</td>
      <td>
        <code>{item.action}</code>
      </td>
      <td>
        {item.resourceType}
        <small>{item.resourceId || '—'}</small>
      </td>
      <td>
        <span className={`status ${item.result}`}>
          {t(item.result === 'succeeded' ? 'Succeeded' : item.result === 'denied' ? 'Denied' : 'Failed')}
        </span>
      </td>
      <td>{item.sourceIp || '—'}</td>
      <td>
        <Button onClick={onOpen} variant="ghost">
          {t('View detail')}
        </Button>
      </td>
    </tr>
  )
}
