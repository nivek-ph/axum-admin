import { Plus, RefreshCw, Search } from 'lucide-react'
import { useCallback, useEffect, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'

import { listDepartments, type DeptRecord } from '@/api/departments'
import { fetchMenuTree, type MenuRecord } from '@/api/menus'
import {
  createRole,
  deleteRole,
  getRoleDeptIds,
  getRolePermissionIds,
  getRoleUserIds,
  listRoles,
  setRoleDeptIds,
  setRolePermissionIds,
  setRoleUserIds,
  updateRole,
  type RolePayload,
  type RoleResource,
} from '@/api/roles'
import { fetchUsers, type UserRecord } from '@/api/users'
import { Button } from '@/components/ui/Button'
import { Modal } from '@/components/ui/Modal'
import { useConfirm } from '@/components/ui/ConfirmProvider'
import { useAuthStore } from '@/stores/auth'

type Tab = 'basic' | 'menus' | 'scope' | 'users'
type FlatMenu = MenuRecord & { level: number }
type FlatDept = DeptRecord & { level: number }

const scopeOptions = [
  ['all', 'All data'],
  ['dept', 'Own department'],
  ['dept_and_children', 'Department and children'],
  ['self', 'Self only'],
  ['custom_depts', 'Custom departments'],
] as const

function flattenAllMenus(items: MenuRecord[]): MenuRecord[] {
  return items.flatMap((item) => [item, ...flattenAllMenus(item.children ?? [])])
}
function flattenPageMenus(items: MenuRecord[], level = 0): FlatMenu[] {
  return items
    .filter((item) => item.menuType !== 'action')
    .flatMap((item) => [
      { ...item, level },
      ...flattenPageMenus(
        (item.children ?? []).filter((child) => child.menuType !== 'action'),
        level + 1,
      ),
    ])
}
function flattenDepartments(items: DeptRecord[], level = 0): FlatDept[] {
  return items.flatMap((item) => [{ ...item, level }, ...flattenDepartments(item.children ?? [], level + 1)])
}
function isSystemRole(role: RoleResource) {
  return role.is_system || role.code === 'super_admin'
}

export function RolesPage() {
  const { t } = useTranslation()
  const can = useAuthStore((state) => state.can)
  const confirmAction = useConfirm()
  const canViewMembers = can('system:role:list-users') && can('system:user:list')
  const canAssignMembers = can('system:role:assign-users')
  const [roles, setRoles] = useState<RoleResource[]>([])
  const [menus, setMenus] = useState<MenuRecord[]>([])
  const [selectedRoleId, setSelectedRoleId] = useState<number | null>(null)
  const [tab, setTab] = useState<Tab>('menus')
  const [roleSearch, setRoleSearch] = useState('')
  const [memberSearch, setMemberSearch] = useState('')
  const [selectedMenuIds, setSelectedMenuIds] = useState<number[]>([])
  const [selectedDeptIds, setSelectedDeptIds] = useState<number[]>([])
  const [selectedUserIds, setSelectedUserIds] = useState<number[]>([])
  const [departments, setDepartments] = useState<DeptRecord[]>([])
  const [users, setUsers] = useState<UserRecord[]>([])
  const [scope, setScope] = useState('all')
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [roleModal, setRoleModal] = useState(false)
  const [editingRole, setEditingRole] = useState<RoleResource | null>(null)
  const [roleForm, setRoleForm] = useState<RolePayload>({
    code: '',
    name: '',
    status: 'enabled',
    sort: 0,
    data_scope: 'all',
  })

  const selectedRole = roles.find((role) => role.id === selectedRoleId) ?? null
  const allMenus = useMemo(() => flattenAllMenus(menus), [menus])
  const pageMenus = useMemo(() => flattenPageMenus(menus), [menus])
  const flatDepartments = useMemo(() => flattenDepartments(departments), [departments])
  const systemRole = selectedRole ? isSystemRole(selectedRole) : false
  const canEditPermissions = can('system:role:update-permission') && !systemRole

  const loadWorkbench = useCallback(async () => {
    setLoading(true)
    try {
      const [nextRoles, nextMenus] = await Promise.all([listRoles(), fetchMenuTree()])
      setRoles(nextRoles)
      setMenus(nextMenus)
      setSelectedRoleId((current) =>
        nextRoles.some((role) => role.id === current) ? current : (nextRoles[0]?.id ?? null),
      )
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t('Failed to load roles'))
    } finally {
      setLoading(false)
    }
  }, [t])

  useEffect(() => {
    void loadWorkbench()
  }, [loadWorkbench])

  useEffect(() => {
    if (!selectedRoleId) return
    if (tab === 'menus')
      void getRolePermissionIds(selectedRoleId)
        .then(setSelectedMenuIds)
        .catch(() => toast.error(t('Failed to load role permissions')))
    if (tab === 'scope')
      void Promise.all([listDepartments(), getRoleDeptIds(selectedRoleId)])
        .then(([tree, ids]) => {
          setDepartments(tree)
          setSelectedDeptIds(ids)
          setScope(selectedRole?.data_scope ?? 'all')
        })
        .catch(() => toast.error(t('Failed to load data scope')))
    if (tab === 'users' && canViewMembers)
      void Promise.all([fetchUsers(1, 200), getRoleUserIds(selectedRoleId)])
        .then(([result, ids]) => {
          setUsers(result.list)
          setSelectedUserIds(ids)
        })
        .catch(() => toast.error(t('Failed to load role members')))
  }, [canViewMembers, selectedRoleId, tab, selectedRole?.data_scope, t])

  function setMenuAccess(menuId: number, enabled: boolean, includeDescendants: boolean) {
    const current = new Set(selectedMenuIds)
    const byId = new Map(allMenus.map((menu) => [menu.id, menu]))
    if (enabled) {
      let node = byId.get(menuId)
      while (node) {
        current.add(node.id)
        node = node.parentId ? byId.get(node.parentId) : undefined
      }
      if (includeDescendants) {
        const addChildren = (id: number) =>
          allMenus
            .filter((item) => item.parentId === id)
            .forEach((item) => {
              current.add(item.id)
              addChildren(item.id)
            })
        addChildren(menuId)
      }
    } else {
      current.delete(menuId)
      const removeChildren = (id: number) =>
        allMenus
          .filter((item) => item.parentId === id)
          .forEach((item) => {
            current.delete(item.id)
            removeChildren(item.id)
          })
      removeChildren(menuId)
    }
    setSelectedMenuIds([...current].sort((a, b) => a - b))
  }

  async function savePermissions() {
    if (!selectedRoleId || !canEditPermissions) return
    setSaving(true)
    try {
      await setRolePermissionIds(selectedRoleId, selectedMenuIds)
      toast.success(t('Role permissions updated'))
    } catch {
      toast.error(t('Failed to save permissions'))
    } finally {
      setSaving(false)
    }
  }

  async function saveScope() {
    if (!selectedRole) return
    setSaving(true)
    try {
      await updateRole(selectedRole.id, {
        code: selectedRole.code,
        name: selectedRole.name,
        status: selectedRole.status,
        sort: selectedRole.sort,
        data_scope: scope,
      })
      await setRoleDeptIds(selectedRole.id, scope === 'custom_depts' ? selectedDeptIds : [])
      toast.success(t('Data scope updated'))
      await loadWorkbench()
    } catch {
      toast.error(t('Failed to save data scope'))
    } finally {
      setSaving(false)
    }
  }

  async function saveMembers() {
    if (!selectedRoleId) return
    setSaving(true)
    try {
      await setRoleUserIds(selectedRoleId, selectedUserIds)
      toast.success(t('Role members updated'))
    } catch {
      toast.error(t('Failed to save members'))
    } finally {
      setSaving(false)
    }
  }

  function openRoleModal(role?: RoleResource) {
    setEditingRole(role ?? null)
    setRoleForm(
      role
        ? { code: role.code, name: role.name, status: role.status, sort: role.sort, data_scope: role.data_scope }
        : { code: '', name: '', status: 'enabled', sort: 0, data_scope: 'all' },
    )
    setRoleModal(true)
  }

  async function saveRole() {
    if (!roleForm.name.trim() || !roleForm.code.trim()) {
      toast.error(t('Role name and code are required'))
      return
    }
    setSaving(true)
    try {
      const response = editingRole ? await updateRole(editingRole.id, roleForm) : await createRole(roleForm)
      if (response.code !== 'OK') throw new Error(response.message)
      setRoleModal(false)
      const createdId = editingRole ? undefined : (response.data as { role?: RoleResource } | undefined)?.role?.id
      await loadWorkbench()
      if (createdId) setSelectedRoleId(createdId)
      toast.success(t(editingRole ? 'Role updated' : 'Role created'))
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t('Failed to save role'))
    } finally {
      setSaving(false)
    }
  }

  const filteredRoles = roles.filter((role) =>
    `${role.name} ${role.code} ${role.id}`.toLowerCase().includes(roleSearch.toLowerCase()),
  )
  const filteredUsers = users.filter((user) =>
    `${user.nickName} ${user.userName} ${user.email}`.toLowerCase().includes(memberSearch.toLowerCase()),
  )

  return (
    <div className="page-stack">
      <header className="page-header">
        <div>
          <p className="eyebrow">{t('Access control')}</p>
          <h1>{t('Roles')}</h1>
          <p>{t('Manage page access, action permissions, data scope, and members.')}</p>
        </div>
        <div className="header-actions">
          <Button onClick={() => void loadWorkbench()}>
            <RefreshCw size={16} />
            {t('Refresh')}
          </Button>
          {can('system:role:create') && (
            <Button onClick={() => openRoleModal()} variant="primary">
              <Plus size={16} />
              {t('New role')}
            </Button>
          )}
        </div>
      </header>
      <section className="role-workbench">
        <aside className="role-list">
          <label className="search-input">
            <Search size={15} />
            <input
              aria-label="Search roles"
              onChange={(event) => setRoleSearch(event.target.value)}
              placeholder={t('Search role name / code')}
              value={roleSearch}
            />
          </label>
          {filteredRoles.map((role) => (
            <button
              className={selectedRoleId === role.id ? 'active' : ''}
              key={role.id}
              onClick={() => setSelectedRoleId(role.id)}
              type="button"
            >
              <strong>{role.name}</strong>
              <small>
                ID {role.id} · {role.code}
              </small>
            </button>
          ))}
        </aside>
        <div className="role-panel">
          <div className="role-panel-header">
            <div>
              <p className="eyebrow">{t('Current role')}</p>
              <h2>{selectedRole?.name || (loading ? t('Loading…') : t('Roles'))}</h2>
            </div>
            {selectedRole && (
              <div className="header-actions">
                {can('system:role:update') && <Button onClick={() => openRoleModal(selectedRole)}>{t('Edit')}</Button>}
                {can('system:role:delete') && (
                  <Button
                    disabled={systemRole}
                    onClick={() =>
                      void confirmAction(t('Delete role "{name}"?', { name: selectedRole.name })).then((confirmed) => {
                        if (confirmed) void deleteRole(selectedRole.id).then(loadWorkbench)
                      })
                    }
                    variant="danger"
                  >
                    {t('Delete')}
                  </Button>
                )}
              </div>
            )}
          </div>
          <div aria-label="Role sections" className="tabs" role="tablist">
            {(
              [
                ['basic', 'Basic Info'],
                ['menus', 'Menu Authorization'],
                ['scope', 'Data Scope'],
                ...(canViewMembers ? [['users', 'Assigned Users'] as const] : []),
              ] as const
            ).map(([value, label]) => (
              <button aria-selected={tab === value} key={value} onClick={() => setTab(value)} role="tab" type="button">
                {t(label)}
              </button>
            ))}
          </div>
          {tab === 'basic' && selectedRole && (
            <div className="role-detail-grid">
              <div>
                <span>{t('Role code')}</span>
                <strong>{selectedRole.code}</strong>
              </div>
              <div>
                <span>{t('Status')}</span>
                <strong>{t(selectedRole.status === 'enabled' ? 'Enabled' : 'Disabled')}</strong>
              </div>
              <div>
                <span>{t('Data Scope')}</span>
                <strong>
                  {t(scopeOptions.find(([value]) => value === selectedRole.data_scope)?.[1] ?? selectedRole.data_scope)}
                </strong>
              </div>
              <div>
                <span>{t('Sort')}</span>
                <strong>{selectedRole.sort}</strong>
              </div>
            </div>
          )}
          {tab === 'menus' && selectedRole && (
            <div className="role-content">
              <div className="content-toolbar">
                <div>
                  <h3>{t('Menu Authorization')}</h3>
                  <p>
                    {t(
                      'Select page access to include button permissions under that page and avoid visible pages with 403 APIs.',
                    )}
                  </p>
                </div>
                {canEditPermissions && (
                  <Button disabled={saving} onClick={() => void savePermissions()} variant="primary">
                    {t('Save permissions')}
                  </Button>
                )}
              </div>
              <div className="permission-list">
                {pageMenus.map((menu) => (
                  <div className="permission-row" key={menu.id} style={{ paddingLeft: 14 + menu.level * 18 }}>
                    <div className="permission-resource">
                      <strong>{t(menu.meta?.title || menu.name)}</strong>
                      <small>{menu.path}</small>
                    </div>
                    <div className="permission-checks">
                      <label>
                        <input
                          checked={selectedMenuIds.includes(menu.id)}
                          disabled={!canEditPermissions}
                          onChange={(event) => setMenuAccess(menu.id, event.target.checked, true)}
                          type="checkbox"
                        />
                        {t('Page access')}
                      </label>
                      {(menu.children ?? [])
                        .filter((child) => child.menuType === 'action')
                        .map((action) => (
                          <label key={action.id}>
                            <input
                              aria-label={action.meta?.title || action.permission || action.name}
                              checked={selectedMenuIds.includes(action.id)}
                              disabled={!canEditPermissions}
                              onChange={(event) => setMenuAccess(action.id, event.target.checked, false)}
                              type="checkbox"
                            />
                            {t(action.meta?.title || action.permission || action.name)}
                          </label>
                        ))}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
          {tab === 'scope' && selectedRole && (
            <div className="role-content">
              <div className="content-toolbar">
                <h3>{t('Data Scope')}</h3>
                <Button disabled={saving} onClick={() => void saveScope()} variant="primary">
                  {t('Save data scope')}
                </Button>
              </div>
              <div className="scope-grid">
                <div className="check-list">
                  {scopeOptions.map(([value, label]) => (
                    <label key={value}>
                      <input checked={scope === value} name="scope" onChange={() => setScope(value)} type="radio" />
                      {label}
                    </label>
                  ))}
                </div>
                <div className="check-list">
                  <h4>{t('Custom departments')}</h4>
                  {flatDepartments.map((department) => (
                    <label key={department.id} style={{ paddingLeft: 10 + department.level * 16 }}>
                      <input
                        checked={selectedDeptIds.includes(department.id)}
                        disabled={scope !== 'custom_depts'}
                        onChange={() =>
                          setSelectedDeptIds((current) =>
                            current.includes(department.id)
                              ? current.filter((id) => id !== department.id)
                              : [...current, department.id],
                          )
                        }
                        type="checkbox"
                      />
                      {department.name}
                    </label>
                  ))}
                </div>
              </div>
            </div>
          )}
          {tab === 'users' && selectedRole && canViewMembers && (
            <div className="role-content">
              <div className="content-toolbar">
                <label className="search-input">
                  <Search size={15} />
                  <input
                    aria-label="Search members"
                    onChange={(event) => setMemberSearch(event.target.value)}
                    placeholder={t('Search users')}
                    value={memberSearch}
                  />
                </label>
                {canAssignMembers && (
                  <Button disabled={saving} onClick={() => void saveMembers()} variant="primary">
                    {t('Save members')}
                  </Button>
                )}
              </div>
              <div className="member-grid">
                {filteredUsers.map((user) => (
                  <label
                    className={selectedUserIds.includes(user.id) ? 'member-card selected' : 'member-card'}
                    key={user.id}
                  >
                    <input
                      checked={selectedUserIds.includes(user.id)}
                      disabled={!canAssignMembers}
                      onChange={() =>
                        setSelectedUserIds((current) =>
                          current.includes(user.id) ? current.filter((id) => id !== user.id) : [...current, user.id],
                        )
                      }
                      type="checkbox"
                    />
                    <span className="avatar small-avatar">{(user.nickName || user.userName).slice(0, 1)}</span>
                    <span>
                      <strong>{user.nickName || user.userName}</strong>
                      <small>{user.userName}</small>
                    </span>
                  </label>
                ))}
              </div>
            </div>
          )}
        </div>
      </section>
      <Modal
        footer={
          <>
            <Button onClick={() => setRoleModal(false)}>{t('Cancel')}</Button>
            <Button disabled={saving} onClick={() => void saveRole()} variant="primary">
              {t('Save role')}
            </Button>
          </>
        }
        onOpenChange={setRoleModal}
        open={roleModal}
        title={t(editingRole ? 'Edit' : 'New role')}
      >
        <div className="form-grid">
          <label>
            {t('Role name')}
            <input
              onChange={(event) => setRoleForm((current) => ({ ...current, name: event.target.value }))}
              value={roleForm.name}
            />
          </label>
          <label>
            {t('Role code')}
            <input
              disabled={Boolean(editingRole)}
              onChange={(event) => setRoleForm((current) => ({ ...current, code: event.target.value }))}
              value={roleForm.code}
            />
          </label>
          <label>
            {t('Status')}
            <select
              onChange={(event) => setRoleForm((current) => ({ ...current, status: event.target.value }))}
              value={roleForm.status}
            >
              <option value="enabled">{t('Enabled')}</option>
              <option value="disabled">{t('Disabled')}</option>
            </select>
          </label>
          <label>
            {t('Sort')}
            <input
              onChange={(event) => setRoleForm((current) => ({ ...current, sort: Number(event.target.value) }))}
              type="number"
              value={roleForm.sort}
            />
          </label>
        </div>
      </Modal>
    </div>
  )
}
