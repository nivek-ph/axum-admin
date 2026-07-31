import { getCoreRowModel, useReactTable, type ColumnDef } from '@tanstack/react-table'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import {
  IconCircleMinus,
  IconCirclePlus,
  IconDots,
  IconHierarchy,
  IconPencil,
  IconPlus,
  IconRefresh,
  IconTable,
  IconTrash,
} from '@tabler/icons-react'
import { useMemo, useState } from 'react'
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
import { ApiHttpError } from '@/api/http'
import { DataTable } from '@/components/data-table/DataTable'
import { PageHeader } from '@/components/PageHeader'
import { Badge } from '@/components/ui/badge'
import { Button, buttonVariants } from '@/components/ui/Button'
import { Card, CardContent } from '@/components/ui/card'
import { useConfirm } from '@/components/ConfirmProvider'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { useAuthStore } from '@/stores/auth'

import { DepartmentOrgChart } from './DepartmentOrgChart'

type FlatDepartment = DeptRecord & { hasChildren: boolean; level: number }

const emptyForm: DeptPayload = { parent_id: null, name: '', code: '', sort: 0, status: 'enabled' }
const ROOT_PARENT_VALUE = '__root__'

function flatten(items: DeptRecord[], level = 0): FlatDepartment[] {
  return items.flatMap((item) => [
    { ...item, hasChildren: Boolean(item.children?.length), level },
    ...flatten(item.children ?? [], level + 1),
  ])
}

function flattenVisible(items: DeptRecord[], collapsed: Set<number>, level = 0): FlatDepartment[] {
  return items.flatMap((item) => [
    { ...item, hasChildren: Boolean(item.children?.length), level },
    ...(collapsed.has(item.id) ? [] : flattenVisible(item.children ?? [], collapsed, level + 1)),
  ])
}

function countDescendants(item: DeptRecord): number {
  return (item.children ?? []).reduce((sum, child) => sum + 1 + countDescendants(child), 0)
}

function collectSelfAndDescendantIds(item: DeptRecord): Set<number> {
  const ids = new Set<number>([item.id])
  for (const child of item.children ?? []) {
    for (const id of collectSelfAndDescendantIds(child)) ids.add(id)
  }
  return ids
}

function findDepartment(items: DeptRecord[], id: number): DeptRecord | null {
  for (const item of items) {
    if (item.id === id) return item
    const nested = findDepartment(item.children ?? [], id)
    if (nested) return nested
  }
  return null
}

export function DepartmentsPage() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const confirmAction = useConfirm()
  const canCreate = useAuthStore((state) => state.can('system:dept:create'))
  const canUpdate = useAuthStore((state) => state.can('system:dept:update'))
  const canDelete = useAuthStore((state) => state.can('system:dept:delete'))
  const query = useQuery({ queryKey: ['departments'], queryFn: listDepartments })
  const [open, setOpen] = useState(false)
  const [view, setView] = useState<'table' | 'chart'>('table')
  const [editingId, setEditingId] = useState<number | null>(null)
  const [form, setForm] = useState<DeptPayload>(emptyForm)
  const [collapsedTableIds, setCollapsedTableIds] = useState<Set<number>>(() => new Set())
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
    onError: (error) => {
      if (error instanceof ApiHttpError && error.body?.code === 'DEPT_HAS_DESCENDANTS') {
        toast.error(t('Move or delete subordinate departments before deleting this department.'))
        return
      }
      toast.error(t('Failed to delete department'))
    },
  })

  const tree = useMemo(() => query.data ?? [], [query.data])
  const allRows = useMemo(() => flatten(tree), [tree])
  const visibleRows = useMemo(() => flattenVisible(tree, collapsedTableIds), [collapsedTableIds, tree])
  const expandableTableIds = useMemo(
    () => new Set(allRows.filter((item) => item.hasChildren).map((item) => item.id)),
    [allRows],
  )
  const parentOptions = useMemo(() => {
    if (editingId == null) return allRows
    const editing = findDepartment(tree, editingId)
    if (!editing) return allRows
    const blocked = collectSelfAndDescendantIds(editing)
    return allRows.filter((item) => !blocked.has(item.id))
  }, [allRows, editingId, tree])
  const parentSelectItems = useMemo(() => {
    const items: Record<string, string> = { [ROOT_PARENT_VALUE]: t('Root department') }
    for (const item of parentOptions) {
      const prefix = item.level > 0 ? `${'\u00A0'.repeat(item.level * 2)}└ ` : ''
      items[String(item.id)] = `${prefix}${item.name}`
    }
    return items
  }, [parentOptions, t])

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

  async function confirmDelete(item: DeptRecord) {
    const childCount = countDescendants(item)
    if (childCount > 0) {
      toast.error(
        t('Move or delete the {{count}} subordinate departments before deleting "{{name}}".', {
          count: childCount,
          name: item.name,
        }),
      )
      return
    }
    const yes = await confirmAction(t('Delete department "{{name}}"?', { name: item.name }))
    if (yes) deleteMutation.mutate(item.id)
  }

  function toggleTableDepartment(id: number) {
    setCollapsedTableIds((current) => {
      const next = new Set(current)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })
  }

  function save() {
    if (!form.name.trim() || !form.code.trim()) {
      toast.error(t('Name and code are required'))
      return
    }
    saveMutation.mutate()
  }

  const columns: ColumnDef<FlatDepartment>[] = [
    {
      accessorKey: 'name',
      header: t('Name'),
      cell: ({ row }) => {
        const item = row.original
        const isCollapsed = collapsedTableIds.has(item.id)
        return (
          <div className="flex min-w-48 items-center" style={{ paddingLeft: item.level * 18 }}>
            {item.level > 0 ? (
              <span className="mr-1 h-4 w-3 shrink-0 rounded-bl border-b border-l border-border" aria-hidden="true" />
            ) : null}
            {item.hasChildren ? (
              <Button
                aria-expanded={!isCollapsed}
                aria-label={t(isCollapsed ? 'Expand {{name}}' : 'Collapse {{name}}', { name: item.name })}
                className="mr-1"
                onClick={() => toggleTableDepartment(item.id)}
                size="icon-xs"
                variant="ghost"
              >
                {isCollapsed ? <IconCirclePlus /> : <IconCircleMinus />}
              </Button>
            ) : (
              <span className="mr-1 size-6 shrink-0" />
            )}
            <strong className="font-medium">{item.name}</strong>
          </div>
        )
      },
    },
    {
      accessorKey: 'code',
      header: t('Code'),
      cell: ({ row }) => <code className="rounded bg-muted px-1.5 py-0.5 text-xs">{row.original.code}</code>,
    },
    {
      accessorKey: 'sort',
      header: t('Sort'),
      cell: ({ row }) => row.original.sort,
    },
    {
      accessorKey: 'status',
      header: t('Status'),
      cell: ({ row }) => (
        <Badge variant={row.original.status === 'enabled' ? 'default' : 'outline'}>
          {t(row.original.status === 'enabled' ? 'Enabled' : 'Disabled')}
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
          <div className="flex items-center gap-1 whitespace-nowrap">
            {canCreate && (
              <Button onClick={() => beginCreate(item.id)} size="sm" variant="ghost">
                <IconPlus size={14} />
                {t('Add child')}
              </Button>
            )}
            {(canUpdate || canDelete) && (
              <DropdownMenu>
                <DropdownMenuTrigger
                  aria-label={t('More actions for {{name}}', { name: item.name })}
                  className={buttonVariants({ size: 'icon-sm', variant: 'ghost' })}
                >
                  <IconDots />
                </DropdownMenuTrigger>
                <DropdownMenuContent>
                  {canUpdate && (
                    <DropdownMenuItem onClick={() => beginEdit(item)}>
                      <IconPencil />
                      {t('Edit')}
                    </DropdownMenuItem>
                  )}
                  {canUpdate && canDelete && <DropdownMenuSeparator />}
                  {canDelete && (
                    <DropdownMenuItem className="text-destructive" onClick={() => void confirmDelete(item)}>
                      <IconTrash />
                      {t('Delete')}
                    </DropdownMenuItem>
                  )}
                </DropdownMenuContent>
              </DropdownMenu>
            )}
          </div>
        )
      },
    },
  ]

  const table = useReactTable({
    data: visibleRows,
    columns,
    getCoreRowModel: getCoreRowModel(),
    getRowId: (row) => String(row.id),
  })

  return (
    <div className="flex flex-col gap-3">
      <PageHeader
        description={
          <h1 className="text-base font-semibold text-foreground">{t('Manage department hierarchy and status.')}</h1>
        }
        actions={
          <>
            <Button onClick={() => void query.refetch()} variant="outline">
              <IconRefresh size={16} />
              {t('Refresh')}
            </Button>
            {canCreate && (
              <Button onClick={() => beginCreate()}>
                <IconPlus size={16} />
                {t('New department')}
              </Button>
            )}
          </>
        }
      />
      <Tabs onValueChange={(value) => setView(value as 'table' | 'chart')} value={view}>
        <TabsList aria-label={t('Department views')}>
          <TabsTrigger value="table">
            <IconTable />
            {t('Table view')}
          </TabsTrigger>
          <TabsTrigger value="chart">
            <IconHierarchy />
            {t('Organization chart')}
          </TabsTrigger>
        </TabsList>
        <TabsContent value="table">
          {view === 'table' && (
            <Card>
              <CardContent className="flex flex-col gap-2">
                <div className="flex justify-end gap-1 border-b pb-2">
                  <Button onClick={() => setCollapsedTableIds(new Set())} size="sm" variant="ghost">
                    {t('Expand all')}
                  </Button>
                  <Button onClick={() => setCollapsedTableIds(expandableTableIds)} size="sm" variant="ghost">
                    {t('Collapse all')}
                  </Button>
                </div>
                <DataTable
                  cellClassName="py-1.5"
                  emptyLabel={t('No departments')}
                  errorLabel={t('Failed to load data')}
                  isError={query.isError}
                  isLoading={query.isLoading}
                  loadingLabel={t('Loading…')}
                  summary={t('Record total', { count: allRows.length })}
                  table={table}
                />
              </CardContent>
            </Card>
          )}
        </TabsContent>
        <TabsContent value="chart">
          {view === 'chart' && (
            <DepartmentOrgChart
              departments={tree}
              isError={query.isError}
              isLoading={query.isLoading}
              onAddChild={canCreate ? (item) => beginCreate(item.id) : undefined}
              onEdit={canUpdate ? beginEdit : undefined}
            />
          )}
        </TabsContent>
      </Tabs>
      <Dialog onOpenChange={setOpen} open={open}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>{editingId ? t('Edit') : t('New department')}</DialogTitle>
          </DialogHeader>
          <div className="grid grid-cols-2 gap-3">
            <div className="col-span-2 flex flex-col gap-1.5">
              <Label htmlFor="dept-parent">{t('Parent department')}</Label>
              <Select
                items={parentSelectItems}
                onValueChange={(value) => {
                  if (!value) return
                  setForm((current) => ({
                    ...current,
                    parent_id: value === ROOT_PARENT_VALUE ? null : Number(value),
                  }))
                }}
                value={form.parent_id == null ? ROOT_PARENT_VALUE : String(form.parent_id)}
              >
                <SelectTrigger aria-label={t('Parent department')} className="w-full" id="dept-parent">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {Object.entries(parentSelectItems).map(([value, label]) => (
                    <SelectItem key={value} value={value}>
                      {label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="dept-name">{t('Name')}</Label>
              <Input
                id="dept-name"
                onChange={(event) => setForm((current) => ({ ...current, name: event.target.value }))}
                value={form.name}
              />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="dept-code">{t('Code')}</Label>
              <Input
                id="dept-code"
                onChange={(event) => setForm((current) => ({ ...current, code: event.target.value }))}
                value={form.code}
              />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="dept-sort">{t('Sort')}</Label>
              <Input
                id="dept-sort"
                onChange={(event) => setForm((current) => ({ ...current, sort: Number(event.target.value) }))}
                type="number"
                value={form.sort ?? 0}
              />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="dept-status">{t('Status')}</Label>
              <Select
                onValueChange={(value) => {
                  if (value) setForm((current) => ({ ...current, status: value }))
                }}
                value={form.status}
              >
                <SelectTrigger aria-label={t('Status')} className="w-full" id="dept-status">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="enabled">{t('Enabled')}</SelectItem>
                  <SelectItem value="disabled">{t('Disabled')}</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
          <DialogFooter>
            <Button onClick={() => setOpen(false)} variant="outline">
              {t('Cancel')}
            </Button>
            <Button disabled={saveMutation.isPending} onClick={save}>
              {t('Save')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
