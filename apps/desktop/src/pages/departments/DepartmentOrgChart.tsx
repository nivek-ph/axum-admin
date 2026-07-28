import {
  IconArrowsMaximize,
  IconArrowsMinimize,
  IconCircleCheckFilled,
  IconCircleMinus,
  IconCirclePlus,
  IconCircleX,
  IconMinus,
  IconPencil,
  IconPlus,
} from '@tabler/icons-react'
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { DeptRecord } from '@/api/departments'
import { Button } from '@/components/ui/Button'
import { Card, CardContent } from '@/components/ui/card'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'

interface DepartmentOrgChartProps {
  departments: DeptRecord[]
  isError: boolean
  isLoading: boolean
  onAddChild?: (department: DeptRecord) => void
  onEdit?: (department: DeptRecord) => void
}

function countDepartments(items: DeptRecord[]): number {
  return items.reduce((count, item) => count + 1 + countDepartments(item.children ?? []), 0)
}

function collectExpandableIds(items: DeptRecord[]): Set<number> {
  const ids = new Set<number>()
  for (const item of items) {
    if (item.children?.length) {
      ids.add(item.id)
      for (const childId of collectExpandableIds(item.children)) ids.add(childId)
    }
  }
  return ids
}

function visibleChartRoots(items: DeptRecord[]): DeptRecord[] {
  if (items.length <= 1) return items
  return items.filter((item) => item.code !== 'head_office' || (item.children?.length ?? 0) > 0)
}

interface OrgNodeProps {
  collapsed: Set<number>
  department: DeptRecord
  isRoot?: boolean
  onAddChild?: (department: DeptRecord) => void
  onEdit?: (department: DeptRecord) => void
  onToggle: (id: number) => void
  parentName?: string
}

function OrgNode({ collapsed, department, isRoot = false, onAddChild, onEdit, onToggle, parentName }: OrgNodeProps) {
  const { t } = useTranslation()
  const children = department.children ?? []
  const hasChildren = children.length > 0
  const isCollapsed = collapsed.has(department.id)
  const trigger = (
    <div
      aria-label={department.name}
      className={`org-chart-card group ${isRoot ? 'org-chart-card-root' : 'org-chart-card-vertical'}`}
      data-org-chart-root={isRoot || undefined}
      role="group"
      tabIndex={0}
    >
      {hasChildren ? (
        <button
          aria-expanded={!isCollapsed}
          aria-label={t(isCollapsed ? 'Expand {{name}}' : 'Collapse {{name}}', { name: department.name })}
          className="flex size-8 shrink-0 items-center justify-center rounded-full text-muted-foreground transition-colors hover:text-foreground"
          onClick={() => onToggle(department.id)}
          type="button"
        >
          {isCollapsed ? <IconCirclePlus size={20} /> : <IconCircleMinus size={20} />}
        </button>
      ) : (
        <span
          className={
            isRoot
              ? 'mx-3 size-1.5 shrink-0 rounded-full bg-muted-foreground/50'
              : 'my-2 size-1.5 shrink-0 rounded-full bg-muted-foreground/50'
          }
        />
      )}
      <strong
        className={isRoot ? 'min-w-0 flex-1 truncate text-left text-sm font-medium' : 'org-chart-vertical-name'}
        title={department.name}
      >
        {department.name}
      </strong>
      <span
        aria-label={t(department.status === 'enabled' ? 'Enabled' : 'Disabled')}
        className="shrink-0"
        title={t(department.status === 'enabled' ? 'Enabled' : 'Disabled')}
      >
        {department.status === 'enabled' ? (
          <IconCircleCheckFilled className="text-success" size={15} />
        ) : (
          <IconCircleX className="text-muted-foreground" size={15} />
        )}
      </span>
      {(onAddChild || onEdit) && (
        <div className="absolute -right-2 -bottom-3 z-10 flex gap-0.5 opacity-0 transition-opacity group-focus-within:opacity-100 group-hover:opacity-100">
          {onAddChild && (
            <Button
              aria-label={t('Add child to {{name}}', { name: department.name })}
              className="border bg-background shadow-sm"
              onClick={() => onAddChild(department)}
              size="icon-xs"
              variant="ghost"
            >
              <IconPlus />
            </Button>
          )}
          {onEdit && (
            <Button
              aria-label={t('Edit {{name}}', { name: department.name })}
              className="border bg-background shadow-sm"
              onClick={() => onEdit(department)}
              size="icon-xs"
              variant="ghost"
            >
              <IconPencil />
            </Button>
          )}
        </div>
      )}
    </div>
  )

  return (
    <li>
      <Tooltip>
        <TooltipTrigger render={trigger} />
        <TooltipContent align="start" className="w-64 flex-col items-stretch gap-2 p-3" side="right">
          <strong className="text-sm">{t('Department details')}</strong>
          <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-xs">
            <dt className="opacity-70">{t('Name')}</dt>
            <dd className="truncate font-medium">{department.name}</dd>
            <dt className="opacity-70">{t('Code')}</dt>
            <dd className="truncate font-mono">{department.code}</dd>
            <dt className="opacity-70">{t('Parent department')}</dt>
            <dd className="truncate">{parentName ?? t('Root department')}</dd>
            <dt className="opacity-70">{t('Status')}</dt>
            <dd>{t(department.status === 'enabled' ? 'Enabled' : 'Disabled')}</dd>
            <dt className="opacity-70">{t('Direct children')}</dt>
            <dd>{children.length}</dd>
          </dl>
        </TooltipContent>
      </Tooltip>
      {hasChildren && !isCollapsed && (
        <ul>
          {children.map((child) => (
            <OrgNode
              collapsed={collapsed}
              department={child}
              key={child.id}
              onAddChild={onAddChild}
              onEdit={onEdit}
              onToggle={onToggle}
              parentName={department.name}
            />
          ))}
        </ul>
      )}
    </li>
  )
}

export function DepartmentOrgChart({ departments, isError, isLoading, onAddChild, onEdit }: DepartmentOrgChartProps) {
  const { t } = useTranslation()
  const [collapsed, setCollapsed] = useState<Set<number>>(() => new Set())
  const [fullscreen, setFullscreen] = useState(false)
  const [zoom, setZoom] = useState(90)
  const chartViewportRef = useRef<HTMLDivElement>(null)
  const chartRoots = useMemo(() => visibleChartRoots(departments), [departments])
  const departmentCount = useMemo(() => countDepartments(chartRoots), [chartRoots])
  const expandableIds = useMemo(() => collectExpandableIds(chartRoots), [chartRoots])

  useLayoutEffect(() => {
    const currentViewport = chartViewportRef.current
    if (!currentViewport) return
    const chartViewport: HTMLDivElement = currentViewport

    function centerChart() {
      if (chartRoots.length === 1) {
        const root = chartViewport.querySelector<HTMLElement>('[data-org-chart-root="true"]')
        if (!root) return
        const viewportRect = chartViewport.getBoundingClientRect()
        const rootRect = root.getBoundingClientRect()
        const visualOffset = rootRect.left + rootRect.width / 2 - (viewportRect.left + viewportRect.width / 2)
        chartViewport.scrollLeft += visualOffset / (zoom / 100)
        return
      }
      chartViewport.scrollLeft = Math.max(0, (chartViewport.scrollWidth - chartViewport.clientWidth) / 2)
    }

    centerChart()
    const timeoutId = window.setTimeout(centerChart)
    return () => window.clearTimeout(timeoutId)
  }, [chartRoots, fullscreen, zoom])

  useEffect(() => {
    if (!fullscreen) return
    const previousOverflow = document.body.style.overflow
    document.body.style.overflow = 'hidden'
    function exitOnEscape(event: KeyboardEvent) {
      if (event.key === 'Escape') setFullscreen(false)
    }
    window.addEventListener('keydown', exitOnEscape)
    return () => {
      document.body.style.overflow = previousOverflow
      window.removeEventListener('keydown', exitOnEscape)
    }
  }, [fullscreen])

  function toggle(id: number) {
    setCollapsed((current) => {
      const next = new Set(current)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })
  }

  return (
    <>
      {fullscreen && <div aria-hidden="true" className="fixed inset-0 z-40 bg-background/80 backdrop-blur-sm" />}
      <Card className={fullscreen ? 'fixed inset-2 z-50 gap-0 rounded-xl py-0 shadow-2xl' : 'gap-0 py-0'}>
        <div className="flex flex-wrap items-center justify-between gap-2 border-b px-4 py-3">
          <div>
            <h2 className="font-medium">{t('Organization chart')}</h2>
            <p className="text-xs text-muted-foreground">{t('Department count', { count: departmentCount })}</p>
          </div>
          <div className="flex flex-wrap items-center gap-1">
            <Button onClick={() => setCollapsed(new Set())} size="sm" variant="outline">
              {t('Expand all')}
            </Button>
            <Button onClick={() => setCollapsed(new Set(expandableIds))} size="sm" variant="outline">
              {t('Collapse all')}
            </Button>
            <span className="mx-1 h-5 border-l" />
            <Button
              aria-label={t('Zoom out')}
              disabled={zoom <= 80}
              onClick={() => setZoom((current) => Math.max(80, current - 10))}
              size="icon-sm"
              variant="outline"
            >
              <IconMinus />
            </Button>
            <button
              className="min-w-12 text-center text-xs text-muted-foreground hover:text-foreground"
              onClick={() => setZoom(90)}
              type="button"
            >
              {zoom}%
            </button>
            <Button
              aria-label={t('Zoom in')}
              disabled={zoom >= 120}
              onClick={() => setZoom((current) => Math.min(120, current + 10))}
              size="icon-sm"
              variant="outline"
            >
              <IconPlus />
            </Button>
            <span className="mx-1 h-5 border-l" />
            <Button
              aria-label={t(fullscreen ? 'Exit fullscreen' : 'Fullscreen')}
              onClick={() => setFullscreen((current) => !current)}
              size="icon-sm"
              variant="outline"
            >
              {fullscreen ? <IconArrowsMinimize /> : <IconArrowsMaximize />}
            </Button>
          </div>
        </div>
        <CardContent className={fullscreen ? 'min-h-0 flex-1 p-0' : 'p-0'}>
          {isLoading ? (
            <div className="p-6 text-sm text-muted-foreground">{t('Loading…')}</div>
          ) : isError ? (
            <div className="p-6 text-sm text-destructive">{t('Failed to load data')}</div>
          ) : chartRoots.length === 0 ? (
            <div className="p-6 text-sm text-muted-foreground">{t('No departments')}</div>
          ) : (
            <div
              className={
                fullscreen
                  ? 'h-full min-h-0 overflow-auto bg-muted/20 p-8'
                  : 'min-h-[32rem] overflow-auto bg-muted/20 p-8'
              }
              ref={chartViewportRef}
            >
              <div className="org-chart-tree" style={{ zoom: zoom / 100 }}>
                <TooltipProvider delay={250}>
                  <ul className="org-chart-forest">
                    {chartRoots.map((department) => (
                      <OrgNode
                        collapsed={collapsed}
                        department={department}
                        isRoot
                        key={department.id}
                        onAddChild={onAddChild}
                        onEdit={onEdit}
                        onToggle={toggle}
                      />
                    ))}
                  </ul>
                </TooltipProvider>
              </div>
            </div>
          )}
        </CardContent>
      </Card>
    </>
  )
}
