import { IconKey, IconPencil } from '@tabler/icons-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router-dom'
import { toast } from 'sonner'

import { getApiErrorMessage } from '@/api/http'
import { changeOwnPassword, updateOwnProfile } from '@/api/users'
import { PageHeader } from '@/components/PageHeader'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/Button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'

const emptyPasswordForm = { password: '', newPassword: '', confirmPassword: '' }

function displayValue(value: string | null | undefined, emptyLabel: string) {
  const trimmed = value?.trim()
  return trimmed ? trimmed : emptyLabel
}

export function ProfilePage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const user = useAuthStore((state) => state.userInfo)
  const permissions = useAuthStore((state) => state.permissions)
  const [passwordOpen, setPasswordOpen] = useState(false)
  const [editing, setEditing] = useState(false)
  const [passwordSubmitting, setPasswordSubmitting] = useState(false)
  const [profileSubmitting, setProfileSubmitting] = useState(false)
  const [passwordForm, setPasswordForm] = useState(emptyPasswordForm)
  const [profileForm, setProfileForm] = useState({
    nickName: user?.nickName ?? '',
    phone: user?.phone ?? '',
    email: user?.email ?? '',
  })

  const emptyLabel = t('Not set')
  const roleLabel =
    user?.roles
      ?.map((role) => role.name)
      .filter(Boolean)
      .join(' / ') || emptyLabel

  function updatePassword<K extends keyof typeof emptyPasswordForm>(key: K, value: string) {
    setPasswordForm((current) => ({ ...current, [key]: value }))
  }

  function updateProfile<K extends keyof typeof profileForm>(key: K, value: string) {
    setProfileForm((current) => ({ ...current, [key]: value }))
  }

  function startEditing() {
    setProfileForm({
      nickName: user?.nickName ?? '',
      phone: user?.phone ?? '',
      email: user?.email ?? '',
    })
    setEditing(true)
  }

  function cancelEditing() {
    setEditing(false)
    setProfileForm({
      nickName: user?.nickName ?? '',
      phone: user?.phone ?? '',
      email: user?.email ?? '',
    })
  }

  function closePasswordModal() {
    setPasswordOpen(false)
    setPasswordForm(emptyPasswordForm)
  }

  async function submitPassword() {
    if (!passwordForm.password || !passwordForm.newPassword) {
      toast.error(t('Current password and new password are required'))
      return
    }
    if (passwordForm.newPassword !== passwordForm.confirmPassword) {
      toast.error(t('New passwords do not match'))
      return
    }
    if (passwordForm.password === passwordForm.newPassword) {
      toast.error(t('New password must be different'))
      return
    }

    setPasswordSubmitting(true)
    try {
      const response = await changeOwnPassword({
        password: passwordForm.password,
        newPassword: passwordForm.newPassword,
      })
      if (response.code !== 'OK') throw new Error(response.message || t('Failed to change password'))
      toast.success(t('Password changed. Please sign in again.'))
      useAuthStore.getState().clearSession()
      useMenuStore.getState().resetAccess()
      navigate('/login', { replace: true })
    } catch (error) {
      toast.error(t(getApiErrorMessage(error, 'Failed to change password')))
    } finally {
      setPasswordSubmitting(false)
    }
  }

  async function submitProfile() {
    if (!user) return
    const nickName = profileForm.nickName.trim()
    if (!nickName) {
      toast.error(t('Nickname is required'))
      return
    }

    setProfileSubmitting(true)
    try {
      const payload = {
        nickName,
        phone: profileForm.phone.trim(),
        email: profileForm.email.trim(),
      }
      const response = await updateOwnProfile(payload)
      if (response.code !== 'OK') throw new Error(response.message || t('Failed to update profile'))
      useAuthStore.getState().setUserAndPermissions({ ...user, ...payload }, permissions)
      setEditing(false)
      toast.success(t('Profile updated'))
    } catch (error) {
      toast.error(t(getApiErrorMessage(error, 'Failed to update profile')))
    } finally {
      setProfileSubmitting(false)
    }
  }

  return (
    <div className="space-y-4">
      <PageHeader
        description={
          <h1 className="text-base font-semibold text-foreground">
            {t('View your account identity and security settings.')}
          </h1>
        }
      />

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
        <Card>
          <CardHeader className="flex-row items-center justify-between space-y-0">
            <CardTitle>{t('Basic Info')}</CardTitle>
            {!editing ? (
              <Button onClick={startEditing} size="sm" variant="ghost">
                <IconPencil size={14} />
                {t('Edit profile')}
              </Button>
            ) : null}
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex items-center gap-3">
              <div className="flex size-11 shrink-0 items-center justify-center rounded-full bg-primary text-base font-semibold text-primary-foreground uppercase">
                {(user?.nickName || user?.userName || 'A').slice(0, 1)}
              </div>
              <div className="min-w-0">
                <strong className="block truncate text-sm">{user?.nickName || t('Not signed in')}</strong>
                <p className="truncate text-xs text-muted-foreground">{user?.userName || emptyLabel}</p>
              </div>
            </div>

            {editing ? (
              <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
                <div className="col-span-2 space-y-1.5">
                  <Label htmlFor="profile-nickname">{t('Nickname')}</Label>
                  <Input
                    id="profile-nickname"
                    onChange={(event) => updateProfile('nickName', event.target.value)}
                    value={profileForm.nickName}
                  />
                </div>
                <div className="col-span-2 space-y-1.5">
                  <Label htmlFor="profile-phone">{t('Phone')}</Label>
                  <Input
                    id="profile-phone"
                    onChange={(event) => updateProfile('phone', event.target.value)}
                    value={profileForm.phone}
                  />
                </div>
                <div className="col-span-2 space-y-1.5">
                  <Label htmlFor="profile-email">{t('Email')}</Label>
                  <Input
                    id="profile-email"
                    onChange={(event) => updateProfile('email', event.target.value)}
                    type="email"
                    value={profileForm.email}
                  />
                </div>
                <div className="col-span-2 flex justify-end gap-2">
                  <Button onClick={cancelEditing} variant="outline">
                    {t('Cancel')}
                  </Button>
                  <Button disabled={profileSubmitting} onClick={() => void submitProfile()}>
                    {profileSubmitting ? '…' : t('Save')}
                  </Button>
                </div>
              </div>
            ) : (
              <dl className="grid grid-cols-2 gap-3 text-sm">
                <div>
                  <dt className="text-muted-foreground">{t('Username')}</dt>
                  <dd className="font-medium">{displayValue(user?.userName, emptyLabel)}</dd>
                </div>
                <div>
                  <dt className="text-muted-foreground">{t('Nickname')}</dt>
                  <dd className="font-medium">{displayValue(user?.nickName, emptyLabel)}</dd>
                </div>
                <div>
                  <dt className="text-muted-foreground">{t('Phone')}</dt>
                  <dd className="font-medium">{displayValue(user?.phone, emptyLabel)}</dd>
                </div>
                <div>
                  <dt className="text-muted-foreground">{t('Email')}</dt>
                  <dd className="font-medium">{displayValue(user?.email, emptyLabel)}</dd>
                </div>
                <div>
                  <dt className="text-muted-foreground">{t('Department')}</dt>
                  <dd className="font-medium">{displayValue(user?.deptName, emptyLabel)}</dd>
                </div>
                <div>
                  <dt className="text-muted-foreground">{t('Role')}</dt>
                  <dd className="font-medium">{roleLabel}</dd>
                </div>
                <div>
                  <dt className="text-muted-foreground">{t('Home route')}</dt>
                  <dd className="font-medium">{displayValue(user?.homeRoute || 'dashboard', emptyLabel)}</dd>
                </div>
                <div>
                  <dt className="text-muted-foreground">{t('User ID')}</dt>
                  <dd className="font-medium">{user?.id ?? emptyLabel}</dd>
                </div>
              </dl>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>{t('Security settings')}</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-3 rounded-lg border p-3">
              <div className="flex size-9 shrink-0 items-center justify-center rounded-full bg-muted">
                <IconKey size={18} />
              </div>
              <div className="min-w-0 flex-1 space-y-1">
                <div className="flex items-center gap-2">
                  <strong className="text-sm">{t('Login password')}</strong>
                  <Badge className="border-success/30 bg-success/10 text-success" variant="outline">
                    {t('Configured')}
                  </Badge>
                </div>
                <p className="text-xs text-muted-foreground">
                  {t('Change your password periodically to keep the account secure.')}
                </p>
              </div>
              <Button onClick={() => setPasswordOpen(true)} size="sm" variant="outline">
                {t('Modify')}
              </Button>
            </div>
          </CardContent>
        </Card>
      </div>

      <Dialog
        onOpenChange={(next) => {
          if (!next) closePasswordModal()
          else setPasswordOpen(true)
        }}
        open={passwordOpen}
      >
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>{t('Change password')}</DialogTitle>
          </DialogHeader>
          <div className="space-y-3">
            <div className="space-y-1.5">
              <Label htmlFor="current-password">{t('Current password')}</Label>
              <Input
                autoComplete="current-password"
                id="current-password"
                onChange={(event) => updatePassword('password', event.target.value)}
                type="password"
                value={passwordForm.password}
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="new-password">{t('New password')}</Label>
              <Input
                autoComplete="new-password"
                id="new-password"
                onChange={(event) => updatePassword('newPassword', event.target.value)}
                type="password"
                value={passwordForm.newPassword}
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="confirm-new-password">{t('Confirm new password')}</Label>
              <Input
                autoComplete="new-password"
                id="confirm-new-password"
                onChange={(event) => updatePassword('confirmPassword', event.target.value)}
                type="password"
                value={passwordForm.confirmPassword}
              />
            </div>
          </div>
          <DialogFooter>
            <Button onClick={closePasswordModal} variant="outline">
              {t('Cancel')}
            </Button>
            <Button disabled={passwordSubmitting} onClick={() => void submitPassword()}>
              {passwordSubmitting ? '…' : t('Save')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
