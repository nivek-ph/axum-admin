import { getCoreRowModel, useReactTable, type ColumnDef } from '@tanstack/react-table'
import { useQuery } from '@tanstack/react-query'
import { IconCircleMinus, IconCirclePlus, IconRefresh } from '@tabler/icons-react'
import { useCallback, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { fetchMenuTree, type MenuRecord } from '@/api/menus'
import { DataTable } from '@/components/data-table/DataTable'
import { PageHeader } from '@/components/PageHeader'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/Button'
import { Card, CardContent } from '@/components/ui/card'

type MenuRow = MenuRecord & {
  hasChildren: boolean
  level: number
}

function treeChildren(item: MenuRecord): MenuRecord[] {
  const children = item.children ?? []
  if (item.menuType === 'page') return children.filter((child) => child.menuType === 'action')
  return children.filter((child) => child.menuType !== 'action')
}

function flattenVisible(items: MenuRecord[], collapsedIds: Set<number>, level = 0): MenuRow[] {
  return items.flatMap((item) => {
    const children = treeChildren(item)
    const row: MenuRow = {
      ...item,
      hasChildren: children.length > 0,
      level,
    }
    return collapsedIds.has(item.id) ? [row] : [row, ...flattenVisible(children, collapsedIds, level + 1)]
  })
}

function collectExpandableIds(items: MenuRecord[]): Set<number> {
  const ids = new Set<number>()
  function visit(nodes: MenuRecord[]) {
    for (const item of nodes) {
      const children = treeChildren(item)
      if (children.length > 0) ids.add(item.id)
      visit(children)
    }
  }
  visit(items)
  return ids
}

export function MenusPage() {
  const { t } = useTranslation()
  const query = useQuery({ queryKey: ['menu-catalog'], queryFn: fetchMenuTree })
  const tree = useMemo(() => query.data ?? [], [query.data])
  const [collapsedIds, setCollapsedIds] = useState<Set<number>>(() => new Set())
  const rows = useMemo(() => flattenVisible(tree, collapsedIds), [collapsedIds, tree])
  const total = useMemo(() => flattenVisible(tree, new Set()).length, [tree])

  const toggleCollapsed = useCallback((id: number) => {
    setCollapsedIds((current) => {
      const next = new Set(current)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })
  }, [])

  const expandAll = useCallback(() => {
    setCollapsedIds(new Set())
  }, [])

  const collapseAll = useCallback(() => {
    setCollapsedIds(collectExpandableIds(tree))
  }, [tree])

  const columns = useMemo<ColumnDef<MenuRow>[]>(
    () => [
      {
        id: 'name',
        header: t('Name'),
        cell: ({ row }) => {
          const item = row.original
          const title = t(item.meta?.title || item.name)
          const isCollapsed = collapsedIds.has(item.id)
          return (
            <div className="flex min-w-48 items-center" style={{ paddingLeft: item.level * 18 }}>
              {item.level > 0 ? (
                <span className="mr-1 h-4 w-3 shrink-0 rounded-bl border-b border-l border-border" aria-hidden="true" />
              ) : null}
              {item.hasChildren ? (
                <Button
                  aria-expanded={!isCollapsed}
                  aria-label={t(isCollapsed ? 'Expand {{name}}' : 'Collapse {{name}}', { name: title })}
                  className="mr-1"
                  onClick={() => toggleCollapsed(item.id)}
                  size="icon-xs"
                  variant="ghost"
                >
                  {isCollapsed ? <IconCirclePlus /> : <IconCircleMinus />}
                </Button>
              ) : (
                <span className="mr-1 size-6 shrink-0" />
              )}
              <div className="flex min-w-0 flex-col">
                <strong className="font-medium text-foreground">{title}</strong>
                {item.path ? <span className="text-xs text-muted-foreground">{item.path}</span> : null}
              </div>
            </div>
          )
        },
      },
      {
        accessorKey: 'menuType',
        header: t('Type'),
        cell: ({ row }) => (
          <Badge className="h-6 rounded-md px-2 text-sm" variant="secondary">
            {t(row.original.menuType || 'directory')}
          </Badge>
        ),
      },
      {
        accessorKey: 'permission',
        header: t('Permission code'),
        cell: ({ row }) => (
          <code className="rounded bg-muted px-1.5 py-0.5 text-sm">{row.original.permission || '—'}</code>
        ),
      },
    ],
    [collapsedIds, t, toggleCollapsed],
  )

  const table = useReactTable({
    data: rows,
    columns,
    getCoreRowModel: getCoreRowModel(),
    getRowId: (row) => String(row.id),
  })

  return (
    <div className="flex flex-col gap-3">
      <PageHeader
        description={
          <div>
            <h1 className="text-base font-semibold text-foreground">{t('Access catalog')}</h1>
            <p className="mt-1 text-sm text-muted-foreground">
              {t('Definitions are managed by database migrations and are read-only here.')}
            </p>
          </div>
        }
        actions={
          <>
            <Button onClick={() => void query.refetch()} variant="outline">
              <IconRefresh size={16} />
              {t('Refresh')}
            </Button>
            <Button onClick={expandAll} size="sm" variant="outline">
              <IconCirclePlus size={16} />
              {t('Expand all')}
            </Button>
            <Button onClick={collapseAll} size="sm" variant="outline">
              <IconCircleMinus size={16} />
              {t('Collapse all')}
            </Button>
          </>
        }
      />
      <Card>
        <CardContent>
          <DataTable
            cellClassName="py-2 align-top text-sm whitespace-normal"
            emptyLabel={t('No menu data')}
            errorLabel={t('Failed to load data')}
            isError={query.isError}
            isLoading={query.isLoading}
            loadingLabel={t('Loading…')}
            rowClassName={(row) => {
              if (row.original.menuType === 'directory') return 'bg-muted/45 hover:bg-muted/60'
              if (row.original.menuType === 'action') return 'bg-background'
              return 'bg-muted/15'
            }}
            summary={t('Record total', { count: total })}
            table={table}
            tableClassName="min-w-[44rem]"
          />
        </CardContent>
      </Card>
    </div>
  )
}
