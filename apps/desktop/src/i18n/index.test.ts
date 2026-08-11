import { afterEach, describe, expect, it } from 'vitest'

import i18n from './index'

describe('Chinese admin terminology', () => {
  afterEach(async () => {
    await i18n.changeLanguage('en-US')
  })

  it('preserves the Vue navigation labels', async () => {
    await i18n.changeLanguage('zh-CN')

    expect([
      i18n.t('Dashboard'),
      i18n.t('Users'),
      i18n.t('Roles'),
      i18n.t('Departments'),
      i18n.t('Access catalog'),
      i18n.t('Params'),
      i18n.t('Dictionaries'),
      i18n.t('Files'),
      i18n.t('Audit events'),
      i18n.t('Profile'),
    ]).toEqual([
      '控制台',
      '用户管理',
      '角色管理',
      '部门管理',
      '权限目录',
      '参数配置',
      '数据字典',
      '文件管理',
      '审计事件',
      '个人中心',
    ])
  })

  it('translates the simplified access workbenches', async () => {
    await i18n.changeLanguage('zh-CN')

    expect([
      i18n.t('Basic Info'),
      i18n.t('Page Access'),
      i18n.t('Direct Permissions'),
      i18n.t('Effective Permissions'),
    ]).toEqual(['基础信息', '页面访问', '直接权限', '生效权限'])
  })

  it('translates the rate-limit error shown by the shared toast', async () => {
    await i18n.changeLanguage('zh-CN')

    expect(i18n.t('too many requests')).toBe('请求过于频繁，请稍后再试')
  })
})
