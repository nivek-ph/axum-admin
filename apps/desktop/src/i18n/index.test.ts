import { beforeEach, describe, expect, it, vi } from 'vitest'

import { currentLocale, setLocale, t, toggleLocale } from './index'

describe('i18n', () => {
  beforeEach(() => {
    localStorage.clear()
    setLocale('zh-CN')
  })

  it('defaults to English when no locale is persisted', async () => {
    localStorage.clear()
    vi.resetModules()

    const fresh = await import('./index')

    expect(fresh.currentLocale.value).toBe('en-US')
    expect(fresh.t('Users')).toBe('Users')
  })

  it('uses English source text by default', () => {
    setLocale('en-US')

    expect(t('Users')).toBe('Users')
    expect(t('Refresh list')).toBe('Refresh list')
  })

  it('translates English source text to Chinese', () => {
    setLocale('zh-CN')

    expect(t('Users')).toBe('用户管理')
    expect(t('Role')).toBe('角色')
    expect(t('Roles')).toBe('角色管理')
    expect(t('Menu')).toBe('菜单管理')
    expect(t('Param')).toBe('参数配置')
    expect(t('Dictionary')).toBe('字典管理')
    expect(t('File')).toBe('文件管理')
    expect(t('Access catalog')).toBe('权限目录')
    expect(t('Menus and permissions')).toBe('菜单与权限')
    expect(t('Definitions are managed by database migrations and are read-only here.')).toBe(
      '菜单定义由数据库迁移统一管理，此处仅供查看。'
    )
    expect(t('All {label}', { label: t('Method') })).toBe('全部方法')
    expect(t('Refresh list')).toBe('刷新列表')
  })

  it('falls back to source text for missing translations', () => {
    setLocale('en-US')

    expect(t('Untranslated text')).toBe('Untranslated text')
  })

  it('persists and toggles locale', () => {
    setLocale('en-US')

    expect(currentLocale.value).toBe('en-US')
    expect(localStorage.getItem('axum-vue-admin.locale')).toBe('en-US')

    toggleLocale()

    expect(currentLocale.value).toBe('zh-CN')
  })
})
