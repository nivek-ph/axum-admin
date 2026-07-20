import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Plus, Search } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'

import { createParam, deleteParam, fetchParams, updateParam, type ParamRecord } from '@/api/params'
import { Button } from '@/components/ui/Button'
import { useConfirm } from '@/components/ui/ConfirmProvider'
import { Modal } from '@/components/ui/Modal'

const emptyForm: ParamRecord = { id: 0, name: '', key: '', value: '', desc: '' }

export function ParamsPage() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const confirmAction = useConfirm()
  const [page, setPage] = useState(1)
  const [draftFilters, setDraftFilters] = useState({ name: '', key: '' })
  const [filters, setFilters] = useState(draftFilters)
  const [open, setOpen] = useState(false)
  const [form, setForm] = useState<ParamRecord>(emptyForm)
  const query = useQuery({
    queryKey: ['params', page, filters],
    queryFn: () => fetchParams({ page, pageSize: 10, ...filters }),
  })
  const invalidate = () => queryClient.invalidateQueries({ queryKey: ['params'] })
  const saveMutation = useMutation({
    mutationFn: () =>
      form.id ? updateParam(form) : createParam({ name: form.name, key: form.key, value: form.value, desc: form.desc }),
    onSuccess: async () => {
      toast.success(t('Param saved'))
      setOpen(false)
      await invalidate()
    },
    onError: () => toast.error(t('Failed to save param')),
  })
  const deleteMutation = useMutation({
    mutationFn: deleteParam,
    onSuccess: async () => {
      toast.success(t('Param deleted'))
      await invalidate()
    },
  })
  const pages = Math.max(1, Math.ceil((query.data?.total ?? 0) / 10))
  function save() {
    if (!form.name.trim() || !form.key.trim()) {
      toast.error(t('Name and key are required'))
      return
    }
    saveMutation.mutate()
  }

  return (
    <div className="page-stack">
      <header className="page-header">
        <div>
          <p className="eyebrow">{t('System configuration')}</p>
          <h1>{t('Params')}</h1>
          <p>{t('Manage runtime key-value configuration.')}</p>
        </div>
        <Button
          onClick={() => {
            setForm(emptyForm)
            setOpen(true)
          }}
          variant="primary"
        >
          <Plus size={16} />
          {t('New param')}
        </Button>
      </header>
      <section className="panel">
        <div className="toolbar">
          <input
            aria-label="Filter by name"
            onChange={(event) => setDraftFilters((current) => ({ ...current, name: event.target.value }))}
            placeholder={t('Name')}
            value={draftFilters.name}
          />
          <input
            aria-label="Filter by key"
            onChange={(event) => setDraftFilters((current) => ({ ...current, key: event.target.value }))}
            placeholder={t('Key')}
            value={draftFilters.key}
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
        </div>
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th>{t('Name')}</th>
                <th>{t('Key')}</th>
                <th>{t('Value')}</th>
                <th>{t('Description')}</th>
                <th>{t('Actions')}</th>
              </tr>
            </thead>
            <tbody>
              {query.data?.list.map((item) => (
                <tr key={item.id}>
                  <td>{item.name}</td>
                  <td>
                    <code>{item.key}</code>
                  </td>
                  <td>{item.value}</td>
                  <td>{item.desc || '—'}</td>
                  <td>
                    <div className="row-actions">
                      <Button
                        onClick={() => {
                          setForm(item)
                          setOpen(true)
                        }}
                        variant="ghost"
                      >
                        {t('Edit')}
                      </Button>
                      <Button
                        onClick={() =>
                          void confirmAction(t('Delete this param?')).then((yes) => {
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
                  <td className="empty-cell" colSpan={5}>
                    {t('No params')}
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
            <Button onClick={() => setOpen(false)}>{t('Cancel')}</Button>
            <Button disabled={saveMutation.isPending} onClick={save} variant="primary">
              {t('Save')}
            </Button>
          </>
        }
        onOpenChange={setOpen}
        open={open}
        title={t(form.id ? 'Edit param' : 'New param')}
      >
        <div className="form-grid">
          <label>
            {t('Name')}
            <input
              onChange={(event) => setForm((current) => ({ ...current, name: event.target.value }))}
              value={form.name}
            />
          </label>
          <label>
            {t('Key')}
            <input
              onChange={(event) => setForm((current) => ({ ...current, key: event.target.value }))}
              value={form.key}
            />
          </label>
          <label>
            {t('Value')}
            <input
              onChange={(event) => setForm((current) => ({ ...current, value: event.target.value }))}
              value={form.value}
            />
          </label>
          <label>
            {t('Description')}
            <input
              onChange={(event) => setForm((current) => ({ ...current, desc: event.target.value }))}
              value={form.desc}
            />
          </label>
        </div>
      </Modal>
    </div>
  )
}
