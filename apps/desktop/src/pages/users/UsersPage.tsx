import { getCoreRowModel, useReactTable, type ColumnDef } from '@tanstack/react-table'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { IconKey, IconPlus, IconRefresh, IconSearch, IconShield, IconTrash } from '@tabler/icons-react'
import { useCallback, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'

import { listRoles } from '@/api/roles'
import { listDepartments, type DeptRecord } from '@/api/departments'
import {
  assignUserRoles,
  createUser,
  deleteUser,
  fetchUsers,
  getUserAccess,
  resetUserPassword,
  type CreateUserForm,
  type UserRecord,
} from '@/api/users'
import { useConfirm } from '@/components/ConfirmProvider'
import { DataTable } from '@/components/data-table/DataTable'
import { DataTablePagination } from '@/components/data-table/DataTablePagination'
import { PageHeader } from '@/components/PageHeader'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/Button'
import { Card, CardContent } from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
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
const PAGE_SIZE = 10

function flattenDepartmentOptions(items: DeptRecord[]): Array<{ id: number; label: string }> {
  return items.flatMap((item) => [{ id: item.id, label: item.name }, ...flattenDepartmentOptions(item.children ?? [])])
}

export function UsersPage() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const can = useAuthStore((state) => state.can)
  const currentDepartmentName = useAuthStore((state) => state.userInfo?.deptName)
  const canReadAccess = can('system:user:access-read')
  const canReadRoles = can('system:role:list')
  const canReadDepartments = can('system:dept:list')
  const canAssignRoles = canReadRoles && can('system:user:assign-roles')
  const confirmAction = useConfirm()
  const [page, setPage] = useState(1)
  const [draftKeyword, setDraftKeyword] = useState('')
  const [keyword, setKeyword] = useState('')
  const [createOpen, setCreateOpen] = useState(false)
  const [accessUser, setAccessUser] = useState<UserRecord | null>(null)
  const [form, setForm] = useState<CreateUserForm>(emptyForm)
  const [selectedRoles, setSelectedRoles] = useState<number[]>([])
  const users = useQuery({
    queryKey: ['users', page, PAGE_SIZE, keyword],
    queryFn: () => fetchUsers({ page, pageSize: PAGE_SIZE, keyword }),
  })
  const roles = useQuery({ queryKey: ['roles'], queryFn: listRoles, enabled: canReadRoles })
  const departments = useQuery({
    queryKey: ['departments'],
    queryFn: listDepartments,
    enabled: canReadDepartments && createOpen,
  })
  const departmentOptions = useMemo(() => flattenDepartmentOptions(departments.data ?? []), [departments.data])
  const departmentSelectItems = useMemo(
    () => Object.fromEntries(departmentOptions.map((item) => [String(item.id), item.label])),
    [departmentOptions],
  )
  const access = useQuery({
    queryKey: ['user-access', accessUser?.id],
    queryFn: () => getUserAccess(accessUser!.id),
    enabled: canReadAccess && Boolean(accessUser),
  })
  const invalidateUsers = () => queryClient.invalidateQueries({ queryKey: ['users'] })
  const invalidateAccess = () => queryClient.invalidateQueries({ queryKey: ['user-access', accessUser?.id] })
  const createMutation = useMutation({
    mutationFn: createUser,
    onSuccess: async (response) => {
      if (response.code !== 'OK') throw new Error(response.message)
      toast.success(t('User created'))
      setCreateOpen(false)
      setForm(emptyForm)
      await invalidateUsers()
    },
    onError: (error) => toast.error(error.message || t('Create failed')),
  })
  const roleMutation = useMutation({
    mutationFn: ({ id, roleIds }: { id: number; roleIds: number[] }) => assignUserRoles(id, roleIds),
    onSuccess: async () => {
      toast.success(t('User roles updated'))
      await Promise.all([invalidateUsers(), invalidateAccess()])
    },
    onError: (error) => toast.error(error.message),
  })
  const deleteMutation = useMutation({
    mutationFn: deleteUser,
    onSuccess: async () => {
      toast.success(t('User deleted'))
      await invalidateUsers()
    },
  })
  const resetMutation = useMutation({
    mutationFn: (id: number) => resetUserPassword(id),
    onSuccess: () => toast.success(t('Password reset to 123456')),
  })
  const pageCount = Math.max(1, Math.ceil((users.data?.total ?? 0) / PAGE_SIZE))

  const openAccess = useCallback(
    (item: UserRecord) => {
      setAccessUser(item)
      void queryClient
        .fetchQuery({
          queryKey: ['user-access', item.id],
          queryFn: () => getUserAccess(item.id),
        })
        .then((result) => {
          setSelectedRoles(result.assignedRoles.map((role) => role.id))
        })
    },
    [queryClient],
  )

  const columns = useMemo<ColumnDef<UserRecord>[]>(
    () => [
      { accessorKey: 'id', header: 'ID', cell: ({ row }) => row.original.id },
      {
        id: 'user',
        header: t('User'),
        cell: ({ row }) => (
          <div className="flex flex-col">
            <strong className="font-medium">{row.original.nickName}</strong>
            <span className="text-xs text-muted-foreground">
              {row.original.userName}
              <br />
              {row.original.email}
            </span>
          </div>
        ),
      },
      {
        accessorKey: 'deptName',
        header: t('Department'),
        cell: ({ row }) => row.original.deptName || '—',
      },
      ...(canReadAccess
        ? [
            {
              id: 'roles',
              header: t('Roles'),
              cell: ({ row }: { row: { original: UserRecord } }) => (
                <div className="flex flex-wrap gap-1">
                  {row.original.roles?.map((role) => (
                    <Badge key={role.id} variant="secondary">
                      {role.name}
                    </Badge>
                  ))}
                </div>
              ),
            },
          ]
        : []),
      {
        accessorKey: 'enable',
        header: t('Status'),
        cell: ({ row }) => (
          <Badge variant={row.original.enable === 1 ? 'default' : 'outline'}>
            {t(row.original.enable === 1 ? 'Enabled' : 'Disabled')}
          </Badge>
        ),
      },
      {
        id: 'actions',
        header: t('Actions'),
        enableHiding: false,
        cell: ({ row }) => {
          const item = row.original
          return (
            <div className="flex flex-wrap gap-1">
              {canReadAccess && (
                <Button onClick={() => openAccess(item)} variant="ghost">
                  <IconShield size={14} />
                  {t('Access')}
                </Button>
              )}
              {can('system:user:reset-password') && (
                <Button onClick={() => resetMutation.mutate(item.id)} variant="ghost">
                  <IconKey size={14} />
                  {t('Reset password')}
                </Button>
              )}
              {can('system:user:delete') && (
                <Button
                  onClick={() =>
                    void confirmAction(t('Delete user "{{name}}"?', { name: item.userName })).then((confirmed) => {
                      if (confirmed) deleteMutation.mutate(item.id)
                    })
                  }
                  variant="ghost"
                >
                  <IconTrash size={14} />
                  {t('Delete')}
                </Button>
              )}
            </div>
          )
        },
      },
    ],
    [can, canReadAccess, confirmAction, deleteMutation, openAccess, resetMutation, t],
  )

  const table = useReactTable({
    data: users.data?.list ?? [],
    columns,
    pageCount,
    manualPagination: true,
    getCoreRowModel: getCoreRowModel(),
    state: { pagination: { pageIndex: page - 1, pageSize: PAGE_SIZE } },
  })

  function update<K extends keyof CreateUserForm>(key: K, value: CreateUserForm[K]) {
    setForm((current) => ({ ...current, [key]: value }))
  }

  function submitCreate() {
    if (!form.userName.trim() || !form.nickName.trim() || !form.password) {
      toast.error(t('Username, nickname, and password are required'))
      return
    }
    createMutation.mutate(form)
  }

  return (
    <div className="flex flex-col gap-3">
      <PageHeader
        description={
          <h1 className="text-base font-semibold text-foreground">
            {t('Manage employee accounts and individual access.')}
          </h1>
        }
        actions={
          <>
            <Button onClick={() => void users.refetch()} variant="outline">
              <IconRefresh size={16} />
              {t('Refresh')}
            </Button>
            {can('system:user:create') && (
              <Button
                onClick={() => {
                  setForm(emptyForm)
                  setCreateOpen(true)
                }}
              >
                <IconPlus size={16} />
                {t('New user')}
              </Button>
            )}
          </>
        }
      />
      <Card>
        <CardContent className="flex flex-col gap-3">
          <form
            className="flex flex-wrap items-center gap-2"
            onSubmit={(event) => {
              event.preventDefault()
              setKeyword(draftKeyword.trim())
              setPage(1)
            }}
          >
            <Input
              aria-label={t('Search users')}
              className="w-64"
              onChange={(event) => setDraftKeyword(event.target.value)}
              placeholder={t('Search users')}
              value={draftKeyword}
            />
            <Button type="submit">
              <IconSearch size={16} />
              {t('Search')}
            </Button>
            <Button
              onClick={() => {
                setDraftKeyword('')
                setKeyword('')
                setPage(1)
              }}
              type="button"
              variant="outline"
            >
              {t('Reset')}
            </Button>
          </form>
          <DataTable
            cellClassName="py-1.5"
            emptyLabel={t('No users')}
            errorLabel={t('Failed to load data')}
            isError={users.isError}
            isLoading={users.isLoading}
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
            totalText={t('Record total', { count: users.data?.total ?? 0 })}
          />
        </CardContent>
      </Card>

      <Dialog onOpenChange={setCreateOpen} open={createOpen}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>{t('New user')}</DialogTitle>
          </DialogHeader>
          <div className="grid grid-cols-2 gap-3">
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="user-username">{t('Username')}</Label>
              <Input
                id="user-username"
                onChange={(event) => update('userName', event.target.value)}
                value={form.userName}
              />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="user-nickname">{t('Nickname')}</Label>
              <Input
                id="user-nickname"
                onChange={(event) => update('nickName', event.target.value)}
                value={form.nickName}
              />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="user-password">{t('Password')}</Label>
              <Input
                id="user-password"
                onChange={(event) => update('password', event.target.value)}
                type="password"
                value={form.password}
              />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="user-status">{t('Status')}</Label>
              <Select
                onValueChange={(value) => value != null && update('enable', Number(value))}
                value={String(form.enable)}
              >
                <SelectTrigger aria-label={t('Status')} className="w-full" id="user-status">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="1">{t('Enabled')}</SelectItem>
                  <SelectItem value="0">{t('Disabled')}</SelectItem>
                </SelectContent>
              </Select>
            </div>
            {canReadDepartments ? (
              <div className="col-span-2 flex flex-col gap-1.5">
                <Label htmlFor="user-department">{t('Department')}</Label>
                <Select
                  items={departmentSelectItems}
                  onValueChange={(value) => value != null && update('deptId', Number(value))}
                  value={form.deptId == null ? null : String(form.deptId)}
                >
                  <SelectTrigger aria-label={t('Department')} className="w-full" id="user-department">
                    <SelectValue placeholder={t('Department')} />
                  </SelectTrigger>
                  <SelectContent>
                    {departmentOptions.map((item) => (
                      <SelectItem key={item.id} value={String(item.id)}>
                        {item.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            ) : (
              <div className="col-span-2 flex flex-col gap-1.5">
                <Label htmlFor="user-department">{t('Department')}</Label>
                <Input disabled id="user-department" value={currentDepartmentName || t('Not set')} />
              </div>
            )}
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="user-phone">{t('Phone')}</Label>
              <Input id="user-phone" onChange={(event) => update('phone', event.target.value)} value={form.phone} />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="user-email">{t('Email')}</Label>
              <Input id="user-email" onChange={(event) => update('email', event.target.value)} value={form.email} />
            </div>
            {canAssignRoles && (
              <div className="col-span-2 flex flex-col gap-1.5">
                <Label>{t('Initial roles (optional)')}</Label>
                <div className="flex flex-wrap gap-3 rounded-lg border border-input p-2.5">
                  {roles.data?.map((role) => (
                    <label className="flex items-center gap-1.5 text-sm" key={role.id}>
                      <Checkbox
                        checked={form.roleIds?.includes(role.id)}
                        disabled={role.status !== 'enabled'}
                        onCheckedChange={() =>
                          update(
                            'roleIds',
                            form.roleIds?.includes(role.id)
                              ? form.roleIds.filter((id) => id !== role.id)
                              : [...(form.roleIds ?? []), role.id],
                          )
                        }
                      />
                      <span>{role.name}</span>
                      {role.code === 'super_admin' && <Badge variant="outline">{t('Protected')}</Badge>}
                      {role.status !== 'enabled' && <Badge variant="outline">{t('Dormant')}</Badge>}
                    </label>
                  ))}
                </div>
              </div>
            )}
          </div>
          <DialogFooter>
            <Button onClick={() => setCreateOpen(false)} variant="outline">
              {t('Cancel')}
            </Button>
            <Button disabled={createMutation.isPending} onClick={submitCreate}>
              {t('Create user')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog onOpenChange={(open) => !open && setAccessUser(null)} open={Boolean(accessUser)}>
        <DialogContent className="sm:max-w-3xl">
          <DialogHeader>
            <DialogTitle>{t('Employee access')}</DialogTitle>
          </DialogHeader>
          <p className="text-sm text-muted-foreground">
            {accessUser?.nickName} · {accessUser?.userName}
          </p>
          <Tabs defaultValue="roles">
            <TabsList>
              <TabsTrigger value="roles">{t('Assigned Roles')}</TabsTrigger>
              <TabsTrigger value="effective">{t('Effective Permissions')}</TabsTrigger>
            </TabsList>
            <TabsContent className="space-y-3 pt-3" value="roles">
              <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
                {roles.data?.map((role) => (
                  <label className="flex items-center gap-2 rounded-md border p-2 text-sm" key={role.id}>
                    <Checkbox
                      checked={selectedRoles.includes(role.id)}
                      disabled={!canAssignRoles || (role.status !== 'enabled' && !selectedRoles.includes(role.id))}
                      onCheckedChange={() =>
                        setSelectedRoles((current) =>
                          current.includes(role.id) ? current.filter((id) => id !== role.id) : [...current, role.id],
                        )
                      }
                    />
                    <span>{role.name}</span>
                    <small className="text-muted-foreground">{role.code}</small>
                    {role.code === 'super_admin' && <Badge variant="outline">{t('Protected')}</Badge>}
                    {role.status !== 'enabled' && <Badge variant="outline">{t('Dormant')}</Badge>}
                  </label>
                ))}
              </div>
              {canAssignRoles && (
                <div className="flex justify-end">
                  <Button
                    disabled={roleMutation.isPending || access.isLoading}
                    onClick={() => accessUser && roleMutation.mutate({ id: accessUser.id, roleIds: selectedRoles })}
                  >
                    {t('Save roles')}
                  </Button>
                </div>
              )}
            </TabsContent>
            <TabsContent className="pt-3" value="effective">
              <div className="max-h-[440px] divide-y overflow-y-auto rounded-lg border">
                {access.data?.effectivePermissions.map((item) => (
                  <div className="flex flex-wrap items-center gap-2 px-3 py-2 text-sm" key={item.permission}>
                    <strong>{item.permission}</strong>
                    {item.roles.map((role) => (
                      <Badge key={role.id} variant="secondary">
                        {role.name}
                      </Badge>
                    ))}
                  </div>
                ))}
              </div>
            </TabsContent>
          </Tabs>
        </DialogContent>
      </Dialog>
    </div>
  )
}
