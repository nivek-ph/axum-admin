import { useQuery } from '@tanstack/react-query'
import { ChevronDown, ChevronRight } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { fetchMenuTree, type MenuRecord } from '@/api/menus'

type MenuRow = MenuRecord & { level: number; hasChildren: boolean }

function flatten(items: MenuRecord[], collapsedIds: Set<number>, level = 0): MenuRow[] {
  return items.flatMap((item) => {
    const children = item.children ?? []
    const row = { ...item, level, hasChildren: children.length > 0 }
    return collapsedIds.has(item.id) ? [row] : [row, ...flatten(children, collapsedIds, level + 1)]
  })
}

export function MenusPage() {
  const { t } = useTranslation()
  const query = useQuery({ queryKey: ['menu-catalog'], queryFn: fetchMenuTree })
  const [collapsedIds, setCollapsedIds] = useState<Set<number>>(() => new Set())
  const rows = flatten(query.data ?? [], collapsedIds)

  function toggle(id: number) {
    setCollapsedIds((current) => {
      const next = new Set(current)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })
  }
  return (
    <div className="page-stack">
      <header className="page-header">
        <div>
          <p className="eyebrow">{t('Access catalog')}</p>
          <h1>{t('Menus and permissions')}</h1>
          <p>{t('Definitions are managed by database migrations and are read-only here.')}</p>
        </div>
      </header>
      <section className="panel">
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th>{t('Name')}</th>
                <th>{t('Type')}</th>
                <th>{t('Permission code')}</th>
                <th>{t('API bindings')}</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((row) => {
                const title = t(row.meta?.title || row.name)
                const collapsed = collapsedIds.has(row.id)
                return (
                  <tr key={row.id}>
                    <td style={{ paddingLeft: 14 + row.level * 20 }}>
                      <div className="tree-cell">
                        {row.hasChildren ? (
                          <button
                            aria-expanded={!collapsed}
                            aria-label={`${t(collapsed ? 'Expand' : 'Collapse')} ${title}`}
                            className="tree-toggle"
                            onClick={() => toggle(row.id)}
                            type="button"
                          >
                            {collapsed ? <ChevronRight size={16} /> : <ChevronDown size={16} />}
                          </button>
                        ) : (
                          <span className="tree-toggle-placeholder" />
                        )}
                        <span>
                          <strong>{title}</strong>
                          <small>{row.path}</small>
                        </span>
                      </div>
                    </td>
                    <td>
                      <span className="tag">{t(row.menuType || 'directory')}</span>
                    </td>
                    <td>
                      <code>{row.permission || '—'}</code>
                    </td>
                    <td>
                      {row.apiBindings?.length
                        ? row.apiBindings.map((binding) => (
                            <div className="api-binding" key={`${binding.method}:${binding.pathPattern}`}>
                              <code>{binding.method}</code> {binding.pathPattern}
                            </div>
                          ))
                        : '—'}
                    </td>
                  </tr>
                )
              })}
              {!query.isLoading && rows.length === 0 && (
                <tr>
                  <td className="empty-cell" colSpan={4}>
                    {t('No menu data')}
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </section>
    </div>
  )
}
