import { create } from 'zustand'

export const THEME_MODES = ['light', 'dark'] as const
export type ThemeMode = (typeof THEME_MODES)[number]

const MODE_KEY = 'ava.themeMode'
const PALETTE = 'indigo'

function readMode(): ThemeMode {
  const value = window.localStorage.getItem(MODE_KEY)
  return THEME_MODES.includes(value as ThemeMode) ? (value as ThemeMode) : 'light'
}

export function applyTheme(mode: ThemeMode) {
  const root = document.documentElement
  root.dataset.palette = PALETTE
  root.dataset.mode = mode
}

interface ThemeState {
  mode: ThemeMode
  setMode: (mode: ThemeMode) => void
  toggleMode: () => void
}

export const useThemeStore = create<ThemeState>((set, get) => ({
  mode: typeof window === 'undefined' ? 'light' : readMode(),
  setMode: (mode) => {
    window.localStorage.setItem(MODE_KEY, mode)
    applyTheme(mode)
    set({ mode })
  },
  toggleMode: () => {
    get().setMode(get().mode === 'light' ? 'dark' : 'light')
  },
}))

export function bootstrapTheme() {
  const mode = readMode()
  applyTheme(mode)
  useThemeStore.setState({ mode })
}
