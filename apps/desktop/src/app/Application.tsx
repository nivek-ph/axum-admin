import { QueryClientProvider } from '@tanstack/react-query'
import { useEffect, useMemo, useState } from 'react'
import { BrowserRouter, Navigate, Outlet, Route, Routes, useLocation } from 'react-router-dom'
import { Toaster } from 'sonner'

import { bootstrapAuthSession } from '@/auth/bootstrap'
import { AppLayout } from '@/layouts/AppLayout'
import { createQueryClient } from '@/lib/query'
import { DashboardPage } from '@/pages/DashboardPage'
import { LoginPage } from '@/pages/LoginPage'
import { ProfilePage } from '@/pages/ProfilePage'
import { UsersPage } from '@/pages/users/UsersPage'
import { RolesPage } from '@/pages/roles/RolesPage'
import { MenusPage } from '@/pages/menus/MenusPage'
import { DepartmentsPage } from '@/pages/departments/DepartmentsPage'
import { DictionariesPage } from '@/pages/dictionaries/DictionariesPage'
import { ParamsPage } from '@/pages/params/ParamsPage'
import { FilesPage } from '@/pages/files/FilesPage'
import { AuditPage } from '@/pages/audit/AuditPage'
import { ConfirmProvider } from '@/components/ui/ConfirmProvider'
import { isAuthenticated, useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'

function ProtectedRoutes() {
  const auth = useAuthStore()
  if (!isAuthenticated(auth)) return <Navigate replace to="/login" />
  return <AppLayout />
}

function AuthorizedRoute({ name }: { name: string }) {
  const canAccess = useMenuStore((state) => state.canAccess(name))
  const fallback = useMenuStore((state) => state.firstAuthorizedPath())
  if (!canAccess) return <Navigate replace to={fallback} />
  return <Outlet />
}

function LoginRoute() {
  const location = useLocation()
  const auth = useAuthStore()
  const menu = useMenuStore()
  if (isAuthenticated(auth) && location.pathname === '/login')
    return <Navigate replace to={homePath(auth.userInfo?.homeRoute, menu)} />
  return <LoginPage />
}

function homePath(homeRoute: string | undefined, menu: ReturnType<typeof useMenuStore.getState>) {
  const routeName = homeRoute?.replace(/^\/+/, '')
  return routeName && menu.canAccess(routeName) ? `/${routeName}` : menu.firstAuthorizedPath()
}

function HomeRedirect() {
  const homeRoute = useAuthStore((state) => state.userInfo?.homeRoute)
  const menu = useMenuStore()
  return <Navigate replace to={homePath(homeRoute, menu)} />
}

function AppRoutes() {
  return (
    <Routes>
      <Route path="/login" element={<LoginRoute />} />
      <Route element={<ProtectedRoutes />}>
        <Route element={<AuthorizedRoute name="dashboard" />}>
          <Route path="/dashboard" element={<DashboardPage />} />
        </Route>
        <Route path="/profile" element={<ProfilePage />} />
        <Route element={<AuthorizedRoute name="users" />}>
          <Route path="/users" element={<UsersPage />} />
        </Route>
        <Route element={<AuthorizedRoute name="roles" />}>
          <Route path="/roles" element={<RolesPage />} />
        </Route>
        <Route element={<AuthorizedRoute name="menus" />}>
          <Route path="/menus" element={<MenusPage />} />
        </Route>
        <Route element={<AuthorizedRoute name="departments" />}>
          <Route path="/departments" element={<DepartmentsPage />} />
        </Route>
        <Route element={<AuthorizedRoute name="params" />}>
          <Route path="/params" element={<ParamsPage />} />
        </Route>
        <Route element={<AuthorizedRoute name="dictionaries" />}>
          <Route path="/dictionaries" element={<DictionariesPage />} />
        </Route>
        <Route element={<AuthorizedRoute name="files" />}>
          <Route path="/files" element={<FilesPage />} />
        </Route>
        <Route element={<AuthorizedRoute name="audit-events" />}>
          <Route path="/audit-events" element={<AuditPage />} />
        </Route>
        <Route index element={<HomeRedirect />} />
      </Route>
      <Route path="*" element={<HomeRedirect />} />
    </Routes>
  )
}

export function Application() {
  const queryClient = useMemo(() => createQueryClient(), [])
  const [bootstrapped, setBootstrapped] = useState(() => !isAuthenticated(useAuthStore.getState()))

  useEffect(() => {
    if (!bootstrapped) void bootstrapAuthSession().finally(() => setBootstrapped(true))
  }, [bootstrapped])

  if (!bootstrapped) return <div className="app-loading">Loading Admin Console…</div>

  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <ConfirmProvider>
          <AppRoutes />
          <Toaster richColors position="top-right" visibleToasts={1} />
        </ConfirmProvider>
      </BrowserRouter>
    </QueryClientProvider>
  )
}
