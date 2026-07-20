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

  it('translates every role workbench section', async () => {
    await i18n.changeLanguage('zh-CN')

    expect([
      i18n.t('Basic Info'),
      i18n.t('Menu Authorization'),
      i18n.t('Data Scope'),
      i18n.t('Assigned Users'),
    ]).toEqual(['基础信息', '菜单授权', '数据范围', '分配用户'])
  })
})
