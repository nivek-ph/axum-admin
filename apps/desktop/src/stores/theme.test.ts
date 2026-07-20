import { afterEach, describe, expect, it } from 'vitest'

import { applyTheme, bootstrapTheme, useThemeStore } from './theme'

describe('theme store', () => {
  afterEach(() => {
    window.localStorage.removeItem('ava.themeMode')
    useThemeStore.setState({ mode: 'light' })
    applyTheme('light')
  })

  it('persists mode on the document element with fixed indigo palette', () => {
    useThemeStore.getState().setMode('dark')
    expect(document.documentElement.dataset.palette).toBe('indigo')
    expect(document.documentElement.dataset.mode).toBe('dark')
    expect(window.localStorage.getItem('ava.themeMode')).toBe('dark')
  })

  it('bootstraps saved mode', () => {
    window.localStorage.setItem('ava.themeMode', 'dark')
    bootstrapTheme()
    expect(useThemeStore.getState().mode).toBe('dark')
    expect(document.documentElement.dataset.mode).toBe('dark')
    expect(document.documentElement.dataset.palette).toBe('indigo')
  })
})
