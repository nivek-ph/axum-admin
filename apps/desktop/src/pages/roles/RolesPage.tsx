import { IconPlus, IconRefresh, IconSearch } from '@tabler/icons-react'
import { useCallback, useEffect, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'

import type { MenuRecord } from '@/api/menus'
import {
  createRole,
  deleteRole,
  getRoleAccess,
  listRoles,
  setRoleAccess,
  updateRole,
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

type FlatNode = MenuRecord & { level: number }

function flattenTree(items: MenuRecord[], level = 0): FlatNode[] {
  return items.flatMap((item) => [{ ...item, level }, ...flattenTree(item.children ?? [], level + 1)])
}

export function RolesPage() {
  const { t } = useTranslation()
  const can = useAuthStore((state) => state.can)
  const confirmAction = useConfirm()
  const canViewAccess = can('system:role:access-read')
  const canUpdateAccess = can('system:role:access-update')
  const [roles, setRoles] = useState<RoleResource[]>([])
  const [selectedRoleId, setSelectedRoleId] = useState<number | null>(null)
  const [roleSearch, setRoleSearch] = useState('')
  const [tree, setTree] = useState<MenuRecord[]>([])
  const [permissions, setPermissions] = useState<string[]>([])
  const [protectedRole, setProtectedRole] = useState(false)
  const [loading, setLoading] = useState(true)
  const [loadingAccess, setLoadingAccess] = useState(false)
  const [saving, setSaving] = useState(false)
  const [roleModal, setRoleModal] = useState(false)
  const [editingRole, setEditingRole] = useState<RoleResource | null>(null)
  const [roleForm, setRoleForm] = useState<RolePayload>({ code: '', name: '', status: 'enabled', sort: 0 })

  const selectedRole = roles.find((role) => role.id === selectedRoleId) ?? null
  const nodes = useMemo(() => flattenTree(tree), [tree])
  const nodeById = useMemo(() => new Map(nodes.map((node) => [node.id, node])), [nodes])
  const canEditAccess = canUpdateAccess && !protectedRole

  const loadRoles = useCallback(async () => {
    setLoading(true)
    try {
      const next = await listRoles()
      setRoles(next)
      setSelectedRoleId((current) => (next.some((role) => role.id === current) ? current : (next[0]?.id ?? null)))
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t('Failed to load roles'))
    } finally {
      setLoading(false)
    }
  }, [t])

  useEffect(() => {
    void loadRoles()
  }, [loadRoles])

  useEffect(() => {
    if (!selectedRoleId || !canViewAccess) {
      setTree([])
      setPermissions([])
      setProtectedRole(false)
      return
    }
    let cancelled = false
    setLoadingAccess(true)
    void getRoleAccess(selectedRoleId)
      .then((access) => {
        if (cancelled) return
        setTree(access.tree)
        setPermissions(access.permissions)
        setProtectedRole(access.protected)
      })
      .catch((error) => toast.error(error instanceof Error ? error.message : t('Failed to load role access')))
      .finally(() => {
        if (!cancelled) setLoadingAccess(false)
      })
    return () => {
      cancelled = true
    }
  }, [canViewAccess, selectedRoleId, t])

  function togglePermission(node: FlatNode, checked: boolean) {
    if (!node.permission || node.menuType === 'directory') return
    setPermissions((current) => {
      const next = new Set(current)
      if (checked) {
        next.add(node.permission!)
        if (node.menuType === 'action') {
          const page = nodeById.get(node.parentId)
          if (page?.permission) next.add(page.permission)
        }
      } else {
        next.delete(node.permission!)
        if (node.menuType === 'page') {
          nodes
            .filter((candidate) => candidate.parentId === node.id && candidate.menuType === 'action')
            .forEach((candidate) => candidate.permission && next.delete(candidate.permission))
        }
      }
      return [...next].sort()
    })
  }

  async function saveAccess() {
    if (!selectedRoleId || !canEditAccess) return
    setSaving(true)
    try {
      await setRoleAccess(selectedRoleId, permissions)
      const persisted = await getRoleAccess(selectedRoleId)
      setPermissions(persisted.permissions)
      toast.success(t('Role access updated'))
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t('Failed to save role access'))
    } finally {
      setSaving(false)
    }
  }

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
      await loadRoles()
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
        description={<h1 className="text-base font-semibold">{t('Manage roles and their access in one tree.')}</h1>}
        actions={
          <>
            <Button onClick={() => void loadRoles()} variant="outline"><IconRefresh size={16} />{t('Refresh')}</Button>
            {can('system:role:create') && <Button onClick={() => openRoleModal()}><IconPlus size={16} />{t('New role')}</Button>}
          </>
        }
      />
      <div className="grid grid-cols-1 gap-4 lg:grid-cols-[220px_1fr]">
        <Card className="h-fit">
          <CardContent className="flex flex-col gap-2">
            <div className="relative">
              <IconSearch className="absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground" />
              <Input aria-label="Search roles" className="pl-8" onChange={(event) => setRoleSearch(event.target.value)} placeholder={t('Search role name / code')} value={roleSearch} />
            </div>
            <ScrollArea className="h-[min(60vh,520px)]">
              <div className="flex flex-col gap-1 pr-2">
                {filteredRoles.map((role) => (
                  <button className={cn('flex flex-col rounded-md px-2.5 py-1.5 text-left text-sm hover:bg-muted', selectedRoleId === role.id && 'bg-muted font-medium')} key={role.id} onClick={() => setSelectedRoleId(role.id)} type="button">
                    <strong className="truncate">{role.name}</strong>
                    <small className="text-xs text-muted-foreground">ID {role.id} · {role.code}</small>
                  </button>
                ))}
              </div>
            </ScrollArea>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex-row items-center justify-between gap-3 border-b pb-3">
            <div><p className="text-xs font-medium text-muted-foreground uppercase">{t('Current role')}</p><CardTitle>{selectedRole?.name || (loading ? t('Loading…') : t('Roles'))}</CardTitle></div>
            {selectedRole && <div className="flex gap-2">
              {can('system:role:update') && <Button disabled={protectedRole} onClick={() => openRoleModal(selectedRole)} size="sm" variant="outline">{t('Edit')}</Button>}
              {can('system:role:delete') && <Button disabled={protectedRole} onClick={() => void confirmAction(t('Delete role "{{name}}"?', { name: selectedRole.name })).then((confirmed) => { if (confirmed) void deleteRole(selectedRole.id).then(loadRoles).catch((error) => toast.error(error.message)) })} size="sm" variant="destructive">{t('Delete')}</Button>}
            </div>}
          </CardHeader>
          <CardContent className="pt-4">
            <Tabs defaultValue={canViewAccess ? 'access' : 'basic'}>
              <TabsList aria-label="Role sections"><TabsTrigger value="basic">{t('Basic Info')}</TabsTrigger>{canViewAccess && <TabsTrigger value="access">{t('Role Access')}</TabsTrigger>}</TabsList>
              <TabsContent className="pt-4" value="basic">
                {selectedRole && <dl className="grid grid-cols-2 gap-4 text-sm sm:grid-cols-3"><div><dt className="text-muted-foreground">{t('Role code')}</dt><dd className="font-medium">{selectedRole.code}</dd></div><div><dt className="text-muted-foreground">{t('Status')}</dt><dd>{t(selectedRole.status === 'enabled' ? 'Enabled' : 'Disabled')}</dd></div><div><dt className="text-muted-foreground">{t('Sort')}</dt><dd>{selectedRole.sort}</dd></div></dl>}
              </TabsContent>
              <TabsContent className="space-y-3 pt-4" value="access">
                <div className="flex items-start justify-between gap-3"><p className="text-xs text-muted-foreground">{t('Selecting an action also selects its page. Removing a page removes its actions.')}</p>{canEditAccess && <Button disabled={saving || loadingAccess} onClick={() => void saveAccess()} size="sm">{t('Save role access')}</Button>}</div>
                {protectedRole && <p className="text-xs text-muted-foreground">{t('The protected super_admin access is read-only and maintained by migrations.')}</p>}
                <div className="divide-y rounded-lg border">
                  {nodes.map((node) => <div className="flex items-center gap-3 px-3 py-2" key={node.id} style={{ paddingLeft: 12 + node.level * 18 }}>
                    {node.menuType === 'directory' ? <span className="size-4" /> : <Checkbox aria-label={t('{{title}} access', { title: t(node.meta?.title || node.name) })} checked={Boolean(node.permission && permissions.includes(node.permission))} disabled={!canEditAccess || loadingAccess || node.status !== 'enabled'} onCheckedChange={(checked) => togglePermission(node, checked === true)} />}
                    <div><strong className="text-sm">{t(node.meta?.title || node.name)}</strong>{node.permission && <small className="ml-2 text-xs text-muted-foreground">{node.permission}</small>}</div>
                  </div>)}
                </div>
              </TabsContent>
            </Tabs>
          </CardContent>
        </Card>
      </div>
      <Dialog onOpenChange={setRoleModal} open={roleModal}>
        <DialogContent className="sm:max-w-lg"><DialogHeader><DialogTitle>{t(editingRole ? 'Edit' : 'New role')}</DialogTitle></DialogHeader>
          <div className="grid grid-cols-2 gap-3"><div className="col-span-2 space-y-1.5"><Label htmlFor="role-name">{t('Role name')}</Label><Input id="role-name" onChange={(event) => setRoleForm((current) => ({ ...current, name: event.target.value }))} value={roleForm.name} /></div><div className="col-span-2 space-y-1.5"><Label htmlFor="role-code">{t('Role code')}</Label><Input disabled={Boolean(editingRole)} id="role-code" onChange={(event) => setRoleForm((current) => ({ ...current, code: event.target.value }))} value={roleForm.code} /></div><div className="space-y-1.5"><Label htmlFor="role-status">{t('Status')}</Label><Select onValueChange={(value) => value && setRoleForm((current) => ({ ...current, status: value }))} value={roleForm.status}><SelectTrigger className="w-full" id="role-status"><SelectValue /></SelectTrigger><SelectContent><SelectItem value="enabled">{t('Enabled')}</SelectItem><SelectItem value="disabled">{t('Disabled')}</SelectItem></SelectContent></Select></div><div className="space-y-1.5"><Label htmlFor="role-sort">{t('Sort')}</Label><Input id="role-sort" onChange={(event) => setRoleForm((current) => ({ ...current, sort: Number(event.target.value) }))} type="number" value={roleForm.sort} /></div></div>
          <DialogFooter><Button onClick={() => setRoleModal(false)} variant="outline">{t('Cancel')}</Button><Button disabled={saving} onClick={() => void saveRole()}>{t('Save')}</Button></DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
