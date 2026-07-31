import { IconPlus, IconRefresh, IconSearch } from '@tabler/icons-react'
import { useCallback, useEffect, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'

import { fetchMenuTree, type MenuRecord } from '@/api/menus'
import {
  createRole,
  deleteRole,
  getRolePageAccess,
  getRolePermissions,
  listRoles,
  setRolePageAccess,
  setRolePermissions,
  updateRole,
  type PermissionCatalogItem,
  type RolePayload,
  type RoleResource,
} from '@/api/roles'
import { useConfirm } from '@/components/ConfirmProvider'
import { PageHeader } from '@/components/PageHeader'
import { Button } from '@/components/ui/Button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { cn } from '@/lib/utils'
import { useAuthStore } from '@/stores/auth'

type Tab = 'basic' | 'page_access' | 'permissions'
type FlatMenu = MenuRecord & { level: number }

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

export function RolesPage() {
  const { t } = useTranslation()
  const can = useAuthStore((state) => state.can)
  const confirmAction = useConfirm()
  const canViewPageAccess = can('system:role:menus-read') && can('system:menu:list')
  const canViewPermissions = can('system:role:permissions-read')
  const [roles, setRoles] = useState<RoleResource[]>([])
  const [menus, setMenus] = useState<MenuRecord[]>([])
  const [selectedRoleId, setSelectedRoleId] = useState<number | null>(null)
  const [tab, setTab] = useState<Tab>(canViewPageAccess ? 'page_access' : canViewPermissions ? 'permissions' : 'basic')
  const [roleSearch, setRoleSearch] = useState('')
  const [selectedMenuIds, setSelectedMenuIds] = useState<number[]>([])
  const [selectedPermissions, setSelectedPermissions] = useState<string[]>([])
  const [permissionCatalog, setPermissionCatalog] = useState<PermissionCatalogItem[]>([])
  const [protectedRole, setProtectedRole] = useState(false)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [roleModal, setRoleModal] = useState(false)
  const [editingRole, setEditingRole] = useState<RoleResource | null>(null)
  const [roleForm, setRoleForm] = useState<RolePayload>({
    code: '',
    name: '',
    status: 'enabled',
    sort: 0,
  })

  const selectedRole = roles.find((role) => role.id === selectedRoleId) ?? null
  const pageMenus = useMemo(() => flattenPageMenus(menus), [menus])
  const canEditPageAccess = can('system:role:update-permission') && !protectedRole
  const canEditPermissions = can('system:role:permissions-update') && !protectedRole

  const loadWorkbench = useCallback(async () => {
    setLoading(true)
    try {
      const nextRoles = await listRoles()
      setRoles(nextRoles)
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
    let cancelled = false
    if (tab === 'page_access' && canViewPageAccess) {
      void Promise.all([fetchMenuTree(), getRolePageAccess(selectedRoleId)])
        .then(([nextMenus, access]) => {
          if (cancelled) return
          setMenus(nextMenus)
          setSelectedMenuIds(access.menuIds)
          setProtectedRole(access.protected)
        })
        .catch(() => toast.error(t('Failed to load page access')))
    }
    if (tab === 'permissions' && canViewPermissions) {
      void getRolePermissions(selectedRoleId)
        .then((result) => {
          if (cancelled) return
          setSelectedPermissions(result.permissions)
          setPermissionCatalog(result.catalog)
          setProtectedRole(result.protected)
        })
        .catch(() => toast.error(t('Failed to load role permissions')))
    }
    return () => {
      cancelled = true
    }
  }, [canViewPageAccess, canViewPermissions, selectedRoleId, tab, t])

  function setMenuAccess(menuId: number, enabled: boolean) {
    const current = new Set(selectedMenuIds)
    const byId = new Map(pageMenus.map((menu) => [menu.id, menu]))
    if (enabled) {
      let node = byId.get(menuId)
      while (node) {
        current.add(node.id)
        node = node.parentId ? byId.get(node.parentId) : undefined
      }
      const addChildren = (id: number) =>
        pageMenus
          .filter((item) => item.parentId === id)
          .forEach((item) => {
            current.add(item.id)
            addChildren(item.id)
          })
      addChildren(menuId)
    } else {
      current.delete(menuId)
      const removeChildren = (id: number) =>
        pageMenus
          .filter((item) => item.parentId === id)
          .forEach((item) => {
            current.delete(item.id)
            removeChildren(item.id)
          })
      removeChildren(menuId)
    }
    setSelectedMenuIds([...current].sort((a, b) => a - b))
  }

  async function savePageAccess() {
    if (!selectedRoleId || !canEditPageAccess) return
    setSaving(true)
    try {
      await setRolePageAccess(selectedRoleId, selectedMenuIds)
      toast.success(t('Page access updated'))
    } catch {
      toast.error(t('Failed to save page access'))
    } finally {
      setSaving(false)
    }
  }

  async function savePermissions() {
    if (!selectedRoleId || !canEditPermissions) return
    setSaving(true)
    try {
      await setRolePermissions(selectedRoleId, selectedPermissions)
      toast.success(t('Role permissions updated'))
    } catch {
      toast.error(t('Failed to save permissions'))
    } finally {
      setSaving(false)
    }
  }

  const permissionGroups = useMemo(() => {
    const groups = new Map<number, { title: string; items: PermissionCatalogItem[] }>()
    for (const item of permissionCatalog.filter((entry) => entry.menuType === 'action')) {
      const group = groups.get(item.owningPageId) ?? { title: item.owningPageTitle, items: [] }
      group.items.push(item)
      groups.set(item.owningPageId, group)
    }
    return [...groups.entries()]
  }, [permissionCatalog])

  function openRoleModal(role?: RoleResource) {
    setEditingRole(role ?? null)
    setRoleForm(
      role
        ? { code: role.code, name: role.name, status: role.status, sort: role.sort }
        : { code: '', name: '', status: 'enabled', sort: 0 },
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
      await loadWorkbench()
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

  return (
    <div className="space-y-4">
      <PageHeader
        description={
          <h1 className="text-base font-semibold text-foreground">
            {t('Manage reusable page access and operation permissions.')}
          </h1>
        }
        actions={
          <>
            <Button onClick={() => void loadWorkbench()} variant="outline">
              <IconRefresh size={16} />
              {t('Refresh')}
            </Button>
            {can('system:role:create') && (
              <Button onClick={() => openRoleModal()}>
                <IconPlus size={16} />
                {t('New role')}
              </Button>
            )}
          </>
        }
      />

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-[220px_1fr]">
        <Card className="h-fit">
          <CardContent className="flex flex-col gap-2">
            <div className="relative">
              <IconSearch className="absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground" />
              <Input
                aria-label="Search roles"
                className="pl-8"
                onChange={(event) => setRoleSearch(event.target.value)}
                placeholder={t('Search role name / code')}
                value={roleSearch}
              />
            </div>
            <ScrollArea className="h-[min(60vh,520px)]">
              <div className="flex flex-col gap-1 pr-2">
                {filteredRoles.map((role) => (
                  <button
                    className={cn(
                      'flex flex-col rounded-md px-2.5 py-1.5 text-left text-sm transition-colors hover:bg-muted',
                      selectedRoleId === role.id && 'bg-muted font-medium',
                    )}
                    key={role.id}
                    onClick={() => {
                      setSelectedRoleId(role.id)
                      setProtectedRole(role.code === 'super_admin')
                    }}
                    type="button"
                  >
                    <strong className="truncate">{role.name}</strong>
                    <small className="text-xs text-muted-foreground">
                      ID {role.id} · {role.code}
                    </small>
                  </button>
                ))}
              </div>
            </ScrollArea>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex-row items-center justify-between gap-3 space-y-0 border-b pb-3">
            <div>
              <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">{t('Current role')}</p>
              <CardTitle>{selectedRole?.name || (loading ? t('Loading…') : t('Roles'))}</CardTitle>
            </div>
            {selectedRole && (
              <div className="flex items-center gap-2">
                {can('system:role:update') && (
                  <Button onClick={() => openRoleModal(selectedRole)} size="sm" variant="outline">
                    {t('Edit')}
                  </Button>
                )}
                {can('system:role:delete') && (
                  <Button
                    disabled={protectedRole}
                    onClick={() =>
                      void confirmAction(t('Delete role "{{name}}"?', { name: selectedRole.name })).then((confirmed) => {
                        if (confirmed) void deleteRole(selectedRole.id).then(loadWorkbench)
                      })
                    }
                    size="sm"
                    variant="destructive"
                  >
                    {t('Delete')}
                  </Button>
                )}
              </div>
            )}
          </CardHeader>
          <CardContent className="pt-4">
            <Tabs onValueChange={(value) => setTab(value as Tab)} value={tab}>
              <TabsList aria-label="Role sections">
                <TabsTrigger value="basic">{t('Basic Info')}</TabsTrigger>
                {canViewPageAccess && <TabsTrigger value="page_access">{t('Page Access')}</TabsTrigger>}
                {canViewPermissions && <TabsTrigger value="permissions">{t('Operation Permissions')}</TabsTrigger>}
              </TabsList>

              <TabsContent className="pt-4" value="basic">
                {selectedRole && (
                  <dl className="grid grid-cols-2 gap-4 text-sm sm:grid-cols-3">
                    <div>
                      <dt className="text-muted-foreground">{t('Role code')}</dt>
                      <dd className="font-medium">{selectedRole.code}</dd>
                    </div>
                    <div>
                      <dt className="text-muted-foreground">{t('Status')}</dt>
                      <dd className="font-medium">{t(selectedRole.status === 'enabled' ? 'Enabled' : 'Disabled')}</dd>
                    </div>
                    <div>
                      <dt className="text-muted-foreground">{t('Sort')}</dt>
                      <dd className="font-medium">{selectedRole.sort}</dd>
                    </div>
                  </dl>
                )}
              </TabsContent>

              <TabsContent className="pt-4" value="page_access">
                {selectedRole && (
                  <div className="space-y-3">
                    <div className="flex flex-wrap items-start justify-between gap-3">
                      <p className="text-xs text-muted-foreground">
                        {t('Choose which directories and pages this role can navigate.')}
                      </p>
                      {canEditPageAccess && (
                        <Button disabled={saving} onClick={() => void savePageAccess()} size="sm">
                          {t('Save page access')}
                        </Button>
                      )}
                    </div>
                    {protectedRole && (
                      <p className="text-xs text-muted-foreground">
                        {t('The protected super_admin grants are maintained by migrations.')}
                      </p>
                    )}
                    <div className="divide-y divide-border rounded-lg border">
                      {pageMenus.map((menu) => (
                        <div className="flex items-center gap-4 px-3 py-2" key={menu.id}>
                          <div className="min-w-48" style={{ paddingLeft: menu.level * 18 }}>
                            <strong className="block text-sm">{t(menu.meta?.title || menu.name)}</strong>
                            <small className="text-xs text-muted-foreground">{menu.path}</small>
                          </div>
                          <Checkbox
                            aria-label={t('{{title}} page access', { title: t(menu.meta?.title || menu.name) })}
                            checked={selectedMenuIds.includes(menu.id)}
                            disabled={!canEditPageAccess}
                            onCheckedChange={(checked) => setMenuAccess(menu.id, checked === true)}
                          />
                        </div>
                      ))}
                    </div>
                  </div>
                )}
              </TabsContent>

              <TabsContent className="pt-4" value="permissions">
                {selectedRole && canViewPermissions && (
                  <div className="space-y-3">
                    <div className="flex flex-wrap items-start justify-between gap-3">
                      <p className="text-xs text-muted-foreground">
                        {t('Page access includes its entry permission; choose additional actions here.')}
                      </p>
                      {canEditPermissions && (
                        <Button disabled={saving} onClick={() => void savePermissions()} size="sm">
                          {t('Save permissions')}
                        </Button>
                      )}
                    </div>
                    <div className="divide-y divide-border rounded-lg border">
                      {permissionGroups.map(([pageId, group]) => (
                        <div className="space-y-2 px-3 py-3" key={pageId}>
                          <div className="flex items-center gap-2">
                            <strong className="text-sm">{t(group.title)}</strong>
                            {!group.items.some((item) => item.pageVisible) && (
                              <span className="text-xs text-amber-600">{t('Page not visible')}</span>
                            )}
                          </div>
                          <div className="flex flex-wrap gap-3">
                            {group.items.map((item) => (
                              <label className="inline-flex items-center gap-1.5 text-xs" key={item.permission}>
                                <Checkbox
                                  checked={selectedPermissions.includes(item.permission)}
                                  disabled={!canEditPermissions}
                                  onCheckedChange={(checked) =>
                                    setSelectedPermissions((current) =>
                                      checked === true
                                        ? [...new Set([...current, item.permission])].sort()
                                        : current.filter((permission) => permission !== item.permission),
                                    )
                                  }
                                />
                                <span>{t(item.title)}</span>
                                {!item.effectivelyEnabled && (
                                  <span className="text-muted-foreground">{t('Dormant')}</span>
                                )}
                              </label>
                            ))}
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>
                )}
              </TabsContent>
            </Tabs>
          </CardContent>
        </Card>
      </div>

      <Dialog onOpenChange={setRoleModal} open={roleModal}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>{t(editingRole ? 'Edit' : 'New role')}</DialogTitle>
          </DialogHeader>
          <div className="grid grid-cols-2 gap-3">
            <div className="col-span-2 space-y-1.5">
              <Label htmlFor="role-name">{t('Role name')}</Label>
              <Input
                id="role-name"
                onChange={(event) => setRoleForm((current) => ({ ...current, name: event.target.value }))}
                value={roleForm.name}
              />
            </div>
            <div className="col-span-2 space-y-1.5">
              <Label htmlFor="role-code">{t('Role code')}</Label>
              <Input
                disabled={Boolean(editingRole)}
                id="role-code"
                onChange={(event) => setRoleForm((current) => ({ ...current, code: event.target.value }))}
                value={roleForm.code}
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="role-status">{t('Status')}</Label>
              <Select
                disabled={editingRole?.code === 'super_admin'}
                onValueChange={(value) => value && setRoleForm((current) => ({ ...current, status: value }))}
                value={roleForm.status}
              >
                <SelectTrigger className="w-full" id="role-status">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="enabled">{t('Enabled')}</SelectItem>
                  <SelectItem value="disabled">{t('Disabled')}</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="role-sort">{t('Sort')}</Label>
              <Input
                id="role-sort"
                onChange={(event) => setRoleForm((current) => ({ ...current, sort: Number(event.target.value) }))}
                type="number"
                value={roleForm.sort}
              />
            </div>
          </div>
          <DialogFooter>
            <Button onClick={() => setRoleModal(false)} variant="outline">
              {t('Cancel')}
            </Button>
            <Button disabled={saving} onClick={() => void saveRole()}>
              {t('Save role')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
