import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Plus, RefreshCw } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'

import { listRoles } from '@/api/roles'
import {
  assignUserRoles,
  createUser,
  deleteUser,
  fetchUsers,
  resetUserPassword,
  type CreateUserForm,
  type UserRecord,
} from '@/api/users'
import { Button } from '@/components/ui/Button'
import { Modal } from '@/components/ui/Modal'
import { useConfirm } from '@/components/ui/ConfirmProvider'
import { useAuthStore } from '@/stores/auth'

const emptyForm: CreateUserForm = {
  userName: '',
  nickName: '',
  password: '123456',
  phone: '',
  email: '',
  enable: 1,
  roleIds: [],
}

export function UsersPage() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const can = useAuthStore((state) => state.can)
  const confirmAction = useConfirm()
  const [page, setPage] = useState(1)
  const [createOpen, setCreateOpen] = useState(false)
  const [roleUser, setRoleUser] = useState<UserRecord | null>(null)
  const [form, setForm] = useState<CreateUserForm>(emptyForm)
  const [selectedRoles, setSelectedRoles] = useState<number[]>([])
  const users = useQuery({ queryKey: ['users', page, 10], queryFn: () => fetchUsers(page, 10) })
  const roles = useQuery({ queryKey: ['roles'], queryFn: listRoles })
  const invalidate = () => queryClient.invalidateQueries({ queryKey: ['users'] })
  const createMutation = useMutation({
    mutationFn: createUser,
    onSuccess: async (response) => {
      if (response.code !== 'OK') throw new Error(response.message)
      toast.success(t('User created'))
      setCreateOpen(false)
      setForm(emptyForm)
      await invalidate()
    },
    onError: (error) => toast.error(error.message || t('Create failed')),
  })
  const roleMutation = useMutation({
    mutationFn: ({ id, roleIds }: { id: number; roleIds: number[] }) => assignUserRoles(id, roleIds),
    onSuccess: async () => {
      toast.success(t('User role updated'))
      setRoleUser(null)
      await invalidate()
    },
  })
  const deleteMutation = useMutation({
    mutationFn: deleteUser,
    onSuccess: async () => {
      toast.success(t('User deleted'))
      await invalidate()
    },
  })
  const resetMutation = useMutation({
    mutationFn: (id: number) => resetUserPassword(id),
    onSuccess: () => toast.success(t('Password reset to 123456')),
  })
  const pages = Math.max(1, Math.ceil((users.data?.total ?? 0) / 10))

  function openCreate() {
    const firstRole = roles.data?.[0]?.id
    setForm({ ...emptyForm, roleIds: firstRole ? [firstRole] : [] })
    setCreateOpen(true)
  }

  function update<K extends keyof CreateUserForm>(key: K, value: CreateUserForm[K]) {
    setForm((current) => ({ ...current, [key]: value }))
  }

  function submitCreate() {
    if (!form.userName.trim() || !form.nickName.trim() || !form.password || form.roleIds.length === 0) {
      toast.error(t('Username, nickname, password, and role are required'))
      return
    }
    createMutation.mutate(form)
  }

  return (
    <div className="page-stack">
      <header className="page-header">
        <div>
          <p className="eyebrow">{t('Identity management')}</p>
          <h1>{t('Users')}</h1>
          <p>{t('Manage operator accounts, roles, and account recovery.')}</p>
        </div>
        <div className="header-actions">
          <Button onClick={() => void users.refetch()}>
            <RefreshCw size={16} />
            {t('Refresh')}
          </Button>
          {can('system:user:create') && (
            <Button onClick={openCreate} variant="primary">
              <Plus size={16} />
              {t('New user')}
            </Button>
          )}
        </div>
      </header>
      <section className="panel">
        <div className="panel-summary">
          <span>
            {users.data?.total ?? 0} {t('users')}
          </span>
          <span>
            {users.data?.list.filter((item) => item.enable === 1).length ?? 0} {t('enabled')}
          </span>
        </div>
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th>ID</th>
                <th>{t('User')}</th>
                <th>{t('Department')}</th>
                <th>{t('Roles')}</th>
                <th>{t('Status')}</th>
                <th>{t('Actions')}</th>
              </tr>
            </thead>
            <tbody>
              {users.data?.list.map((item) => (
                <tr key={item.id}>
                  <td>{item.id}</td>
                  <td>
                    <strong>{item.nickName}</strong>
                    <small>
                      {item.userName}
                      <br />
                      {item.email}
                    </small>
                  </td>
                  <td>{item.deptName || '—'}</td>
                  <td>
                    <div className="tag-list">
                      {item.roles?.map((role) => (
                        <span className="tag" key={role.id}>
                          {role.name}
                        </span>
                      ))}
                    </div>
                  </td>
                  <td>
                    <span className={item.enable === 1 ? 'status enabled' : 'status'}>
                      {t(item.enable === 1 ? 'Enabled' : 'Disabled')}
                    </span>
                  </td>
                  <td>
                    <div className="row-actions">
                      {can('system:user:assign-roles') && (
                        <Button
                          onClick={() => {
                            setRoleUser(item)
                            setSelectedRoles(
                              item.roleIds?.length ? item.roleIds : (item.roles?.map((role) => role.id) ?? []),
                            )
                          }}
                          variant="ghost"
                        >
                          {t('Change role')}
                        </Button>
                      )}
                      {can('system:user:reset-password') && (
                        <Button onClick={() => resetMutation.mutate(item.id)} variant="ghost">
                          {t('Reset password')}
                        </Button>
                      )}
                      {can('system:user:delete') && (
                        <Button
                          onClick={() =>
                            void confirmAction(t('Delete this user?')).then((confirmed) => {
                              if (confirmed) deleteMutation.mutate(item.id)
                            })
                          }
                          variant="ghost"
                        >
                          {t('Delete')}
                        </Button>
                      )}
                    </div>
                  </td>
                </tr>
              ))}
              {!users.isLoading && users.data?.list.length === 0 && (
                <tr>
                  <td className="empty-cell" colSpan={6}>
                    {t('No users')}
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
            <Button onClick={() => setCreateOpen(false)}>{t('Cancel')}</Button>
            <Button disabled={createMutation.isPending} onClick={submitCreate} variant="primary">
              {t('Create user')}
            </Button>
          </>
        }
        onOpenChange={setCreateOpen}
        open={createOpen}
        title={t('New user')}
      >
        <div className="form-grid">
          <label>
            {t('Username')}
            <input onChange={(event) => update('userName', event.target.value)} value={form.userName} />
          </label>
          <label>
            {t('Nickname')}
            <input onChange={(event) => update('nickName', event.target.value)} value={form.nickName} />
          </label>
          <label>
            {t('Password')}
            <input onChange={(event) => update('password', event.target.value)} type="password" value={form.password} />
          </label>
          <label>
            {t('Role')}
            <select
              aria-label="Role"
              multiple
              onChange={(event) =>
                update(
                  'roleIds',
                  Array.from(event.target.selectedOptions, (option) => Number(option.value)),
                )
              }
              value={form.roleIds.map(String)}
            >
              {roles.data?.map((role) => (
                <option key={role.id} value={role.id}>
                  {role.name}
                </option>
              ))}
            </select>
          </label>
          <label>
            {t('Phone')}
            <input onChange={(event) => update('phone', event.target.value)} value={form.phone} />
          </label>
          <label>
            {t('Email')}
            <input onChange={(event) => update('email', event.target.value)} value={form.email} />
          </label>
          <label>
            {t('Status')}
            <select
              aria-label="Status"
              onChange={(event) => update('enable', Number(event.target.value))}
              value={form.enable}
            >
              <option value={1}>{t('Enabled')}</option>
              <option value={0}>{t('Disabled')}</option>
            </select>
          </label>
        </div>
      </Modal>

      <Modal
        footer={
          <>
            <Button onClick={() => setRoleUser(null)}>{t('Cancel')}</Button>
            <Button
              disabled={selectedRoles.length === 0}
              onClick={() => roleUser && roleMutation.mutate({ id: roleUser.id, roleIds: selectedRoles })}
              variant="primary"
            >
              {t('Save roles')}
            </Button>
          </>
        }
        onOpenChange={(open) => !open && setRoleUser(null)}
        open={Boolean(roleUser)}
        title={t('Change user role')}
      >
        <p>
          {roleUser?.nickName} · {roleUser?.userName}
        </p>
        <div className="check-list">
          {roles.data?.map((role) => (
            <label key={role.id}>
              <input
                checked={selectedRoles.includes(role.id)}
                onChange={() =>
                  setSelectedRoles((current) =>
                    current.includes(role.id) ? current.filter((id) => id !== role.id) : [...current, role.id],
                  )
                }
                type="checkbox"
              />
              {role.name}
            </label>
          ))}
        </div>
      </Modal>
    </div>
  )
}
