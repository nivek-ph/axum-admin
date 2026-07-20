import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Plus, RefreshCw } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'

import {
  createDepartment,
  deleteDepartment,
  listDepartments,
  updateDepartment,
  type DeptPayload,
  type DeptRecord,
} from '@/api/departments'
import { Button } from '@/components/ui/Button'
import { useConfirm } from '@/components/ui/ConfirmProvider'
import { Modal } from '@/components/ui/Modal'

type FlatDepartment = DeptRecord & { level: number }
const emptyForm: DeptPayload = { parent_id: null, name: '', code: '', sort: 0, status: 'enabled' }
function flatten(items: DeptRecord[], level = 0): FlatDepartment[] {
  return items.flatMap((item) => [{ ...item, level }, ...flatten(item.children ?? [], level + 1)])
}

export function DepartmentsPage() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const confirmAction = useConfirm()
  const query = useQuery({ queryKey: ['departments'], queryFn: listDepartments })
  const [open, setOpen] = useState(false)
  const [editingId, setEditingId] = useState<number | null>(null)
  const [form, setForm] = useState<DeptPayload>(emptyForm)
  const invalidate = () => queryClient.invalidateQueries({ queryKey: ['departments'] })
  const saveMutation = useMutation({
    mutationFn: () => (editingId ? updateDepartment(editingId, form) : createDepartment(form)),
    onSuccess: async () => {
      toast.success(t('Department saved'))
      setOpen(false)
      await invalidate()
    },
    onError: () => toast.error(t('Failed to save department')),
  })
  const deleteMutation = useMutation({
    mutationFn: deleteDepartment,
    onSuccess: async () => {
      toast.success(t('Department deleted'))
      await invalidate()
    },
    onError: () => toast.error(t('Failed to delete department')),
  })

  function beginCreate(parentId: number | null = null) {
    setEditingId(null)
    setForm({ ...emptyForm, parent_id: parentId })
    setOpen(true)
  }
  function beginEdit(item: DeptRecord) {
    setEditingId(item.id)
    setForm({
      parent_id: item.parent_id ?? null,
      name: item.name,
      code: item.code,
      sort: item.sort,
      status: item.status,
    })
    setOpen(true)
  }
  function save() {
    if (!form.name.trim() || !form.code.trim()) {
      toast.error(t('Name and code are required'))
      return
    }
    saveMutation.mutate()
  }

  return (
    <div className="page-stack">
      <header className="page-header">
        <div>
          <p className="eyebrow">{t('Organization')}</p>
          <h1>{t('Departments')}</h1>
          <p>{t('Manage department hierarchy and status.')}</p>
        </div>
        <div className="header-actions">
          <Button onClick={() => void query.refetch()}>
            <RefreshCw size={16} />
            {t('Refresh')}
          </Button>
          <Button onClick={() => beginCreate()} variant="primary">
            <Plus size={16} />
            {t('New department')}
          </Button>
        </div>
      </header>
      <section className="panel">
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th>ID</th>
                <th>{t('Name')}</th>
                <th>{t('Code')}</th>
                <th>{t('Sort')}</th>
                <th>{t('Status')}</th>
                <th>{t('Actions')}</th>
              </tr>
            </thead>
            <tbody>
              {flatten(query.data ?? []).map((item) => (
                <tr key={item.id}>
                  <td>{item.id}</td>
                  <td style={{ paddingLeft: 14 + item.level * 20 }}>
                    <strong>{item.name}</strong>
                  </td>
                  <td>
                    <code>{item.code}</code>
                  </td>
                  <td>{item.sort}</td>
                  <td>
                    <span className={`status ${item.status}`}>
                      {t(item.status === 'enabled' ? 'Enabled' : 'Disabled')}
                    </span>
                  </td>
                  <td>
                    <div className="row-actions">
                      <Button onClick={() => beginCreate(item.id)} variant="ghost">
                        {t('Add child')}
                      </Button>
                      <Button onClick={() => beginEdit(item)} variant="ghost">
                        {t('Edit')}
                      </Button>
                      <Button
                        onClick={() =>
                          void confirmAction(t('Delete department "{name}"?', { name: item.name })).then((yes) => {
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
              {!query.isLoading && !query.data?.length && (
                <tr>
                  <td className="empty-cell" colSpan={6}>
                    {t('No departments')}
                  </td>
                </tr>
              )}
            </tbody>
          </table>
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
        title={editingId ? t('Edit') : t('New department')}
      >
        <div className="form-grid">
          <label>
            {t('Parent ID')}
            <input
              onChange={(event) =>
                setForm((current) => ({
                  ...current,
                  parent_id: event.target.value ? Number(event.target.value) : null,
                }))
              }
              type="number"
              value={form.parent_id ?? ''}
            />
          </label>
          <label>
            {t('Name')}
            <input
              onChange={(event) => setForm((current) => ({ ...current, name: event.target.value }))}
              value={form.name}
            />
          </label>
          <label>
            {t('Code')}
            <input
              onChange={(event) => setForm((current) => ({ ...current, code: event.target.value }))}
              value={form.code}
            />
          </label>
          <label>
            {t('Sort')}
            <input
              onChange={(event) => setForm((current) => ({ ...current, sort: Number(event.target.value) }))}
              type="number"
              value={form.sort ?? 0}
            />
          </label>
          <label>
            {t('Status')}
            <select
              onChange={(event) => setForm((current) => ({ ...current, status: event.target.value }))}
              value={form.status}
            >
              <option value="enabled">{t('Enabled')}</option>
              <option value="disabled">{t('Disabled')}</option>
            </select>
          </label>
        </div>
      </Modal>
    </div>
  )
}
