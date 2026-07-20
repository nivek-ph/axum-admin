import { cleanup, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { MemoryRouter } from 'react-router-dom'

import i18n from '@/i18n'
import { useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'

import { ProfilePage } from './ProfilePage'

const changeOwnPassword = vi.hoisted(() => vi.fn())
const updateOwnProfile = vi.hoisted(() => vi.fn())

vi.mock('@/api/users', () => ({
  changeOwnPassword,
  updateOwnProfile,
}))

describe('ProfilePage', () => {
  beforeEach(async () => {
    changeOwnPassword.mockReset()
    updateOwnProfile.mockReset()
    useAuthStore.getState().setSession({
      accessToken: 'token',
      refreshToken: 'refresh',
      userInfo: {
        id: 1,
        userName: 'admin',
        nickName: 'Admin',
        phone: '',
        email: 'admin@example.com',
        homeRoute: 'dashboard',
        roles: [{ id: 1, code: 'super_admin', name: 'Super Admin' }],
      },
    })
    useMenuStore.getState().resetAccess()
    await i18n.changeLanguage('en-US')
  })

  afterEach(() => {
    cleanup()
    useAuthStore.getState().clearSession()
  })

  it('changes password then clears the session so the user must sign in again', async () => {
    const user = userEvent.setup()
    changeOwnPassword.mockResolvedValue({ code: 'OK', message: 'updated', data: null })

    render(
      <MemoryRouter>
        <ProfilePage />
      </MemoryRouter>,
    )

    await user.click(screen.getByRole('button', { name: 'Modify' }))
    await user.type(screen.getByLabelText('Current password'), 'old-password')
    await user.type(screen.getByLabelText('New password'), 'new-password')
    await user.type(screen.getByLabelText('Confirm new password'), 'new-password')
    await user.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() => {
      expect(changeOwnPassword).toHaveBeenCalledWith({ password: 'old-password', newPassword: 'new-password' })
    })
    expect(useAuthStore.getState().accessToken).toBe('')
    expect(useAuthStore.getState().userInfo).toBeNull()
  })

  it('rejects mismatched confirmation without calling the API', async () => {
    const user = userEvent.setup()
    render(
      <MemoryRouter>
        <ProfilePage />
      </MemoryRouter>,
    )

    await user.click(screen.getByRole('button', { name: 'Modify' }))
    await user.type(screen.getByLabelText('Current password'), 'old-password')
    await user.type(screen.getByLabelText('New password'), 'new-password')
    await user.type(screen.getByLabelText('Confirm new password'), 'other-password')
    await user.click(screen.getByRole('button', { name: 'Save' }))

    expect(changeOwnPassword).not.toHaveBeenCalled()
  })

  it('saves profile fields and updates the session user info', async () => {
    const user = userEvent.setup()
    updateOwnProfile.mockResolvedValue({ code: 'OK', message: 'updated', data: null })

    render(
      <MemoryRouter>
        <ProfilePage />
      </MemoryRouter>,
    )

    expect(screen.getByText('admin@example.com')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Edit profile' }))
    await user.clear(screen.getByLabelText('Nickname'))
    await user.type(screen.getByLabelText('Nickname'), 'Ada')
    await user.clear(screen.getByLabelText('Phone'))
    await user.type(screen.getByLabelText('Phone'), '13800000000')
    await user.clear(screen.getByLabelText('Email'))
    await user.type(screen.getByLabelText('Email'), 'ada@example.com')
    await user.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() => {
      expect(updateOwnProfile).toHaveBeenCalledWith({
        nickName: 'Ada',
        phone: '13800000000',
        email: 'ada@example.com',
      })
    })
    expect(useAuthStore.getState().userInfo).toMatchObject({
      nickName: 'Ada',
      phone: '13800000000',
      email: 'ada@example.com',
    })
    expect(screen.getAllByText('Ada').length).toBeGreaterThan(0)
    expect(screen.getByText('13800000000')).toBeInTheDocument()
    expect(screen.getByText('ada@example.com')).toBeInTheDocument()
  })
})
