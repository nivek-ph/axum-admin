import { KeyRound, Pencil } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router-dom'
import { toast } from 'sonner'

import { getApiErrorMessage } from '@/api/http'
import { changeOwnPassword, updateOwnProfile } from '@/api/users'
import { Button } from '@/components/ui/Button'
import { Modal } from '@/components/ui/Modal'
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
      toast.error(getApiErrorMessage(error, t('Failed to change password')))
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
      toast.error(getApiErrorMessage(error, t('Failed to update profile')))
    } finally {
      setProfileSubmitting(false)
    }
  }

  return (
    <div className="page-stack profile-page">
      <header className="page-header">
        <div>
          <p className="eyebrow">{t('Current account')}</p>
          <h1>{t('Profile')}</h1>
          <p>{t('View your account identity and security settings.')}</p>
        </div>
      </header>

      <div className="profile-layout">
        <section className="profile-panel">
          <div className="profile-panel-header">
            <h2>{t('Basic Info')}</h2>
            {!editing ? (
              <Button onClick={startEditing} variant="ghost">
                <Pencil size={14} />
                {t('Edit profile')}
              </Button>
            ) : null}
          </div>

          <div className="profile-identity">
            <div className="avatar">{(user?.nickName || user?.userName || 'A').slice(0, 1)}</div>
            <div>
              <strong>{user?.nickName || t('Not signed in')}</strong>
              <p>{user?.userName || emptyLabel}</p>
            </div>
          </div>

          {editing ? (
            <div className="form-grid profile-edit-form">
              <label className="form-span-2">
                {t('Nickname')}
                <input
                  onChange={(event) => updateProfile('nickName', event.target.value)}
                  value={profileForm.nickName}
                />
              </label>
              <label className="form-span-2">
                {t('Phone')}
                <input onChange={(event) => updateProfile('phone', event.target.value)} value={profileForm.phone} />
              </label>
              <label className="form-span-2">
                {t('Email')}
                <input
                  onChange={(event) => updateProfile('email', event.target.value)}
                  type="email"
                  value={profileForm.email}
                />
              </label>
              <div className="form-span-2 profile-edit-actions">
                <Button onClick={cancelEditing}>{t('Cancel')}</Button>
                <Button disabled={profileSubmitting} onClick={() => void submitProfile()} variant="primary">
                  {profileSubmitting ? '…' : t('Save')}
                </Button>
              </div>
            </div>
          ) : (
            <dl className="profile-fields">
              <div>
                <dt>{t('Username')}</dt>
                <dd>{displayValue(user?.userName, emptyLabel)}</dd>
              </div>
              <div>
                <dt>{t('Nickname')}</dt>
                <dd>{displayValue(user?.nickName, emptyLabel)}</dd>
              </div>
              <div>
                <dt>{t('Phone')}</dt>
                <dd>{displayValue(user?.phone, emptyLabel)}</dd>
              </div>
              <div>
                <dt>{t('Email')}</dt>
                <dd>{displayValue(user?.email, emptyLabel)}</dd>
              </div>
              <div>
                <dt>{t('Department')}</dt>
                <dd>{displayValue(user?.deptName, emptyLabel)}</dd>
              </div>
              <div>
                <dt>{t('Role')}</dt>
                <dd>{roleLabel}</dd>
              </div>
              <div>
                <dt>{t('Home route')}</dt>
                <dd>{displayValue(user?.homeRoute || 'dashboard', emptyLabel)}</dd>
              </div>
              <div>
                <dt>{t('User ID')}</dt>
                <dd>{user?.id ?? emptyLabel}</dd>
              </div>
            </dl>
          )}
        </section>

        <section className="profile-panel">
          <div className="profile-panel-header">
            <h2>{t('Security settings')}</h2>
          </div>
          <div className="security-list">
            <div className="security-row">
              <div className="security-icon">
                <KeyRound size={18} />
              </div>
              <div className="security-copy">
                <div className="security-title">
                  <strong>{t('Login password')}</strong>
                  <span className="status enabled">{t('Configured')}</span>
                </div>
                <p>{t('Change your password periodically to keep the account secure.')}</p>
              </div>
              <Button onClick={() => setPasswordOpen(true)}>{t('Modify')}</Button>
            </div>
          </div>
        </section>
      </div>

      <Modal
        footer={
          <>
            <Button onClick={closePasswordModal}>{t('Cancel')}</Button>
            <Button disabled={passwordSubmitting} onClick={() => void submitPassword()} variant="primary">
              {passwordSubmitting ? '…' : t('Save')}
            </Button>
          </>
        }
        onOpenChange={(next) => {
          if (!next) closePasswordModal()
          else setPasswordOpen(true)
        }}
        open={passwordOpen}
        title={t('Change password')}
      >
        <div className="form-grid">
          <label className="form-span-2">
            {t('Current password')}
            <input
              autoComplete="current-password"
              onChange={(event) => updatePassword('password', event.target.value)}
              type="password"
              value={passwordForm.password}
            />
          </label>
          <label className="form-span-2">
            {t('New password')}
            <input
              autoComplete="new-password"
              onChange={(event) => updatePassword('newPassword', event.target.value)}
              type="password"
              value={passwordForm.newPassword}
            />
          </label>
          <label className="form-span-2">
            {t('Confirm new password')}
            <input
              autoComplete="new-password"
              onChange={(event) => updatePassword('confirmPassword', event.target.value)}
              type="password"
              value={passwordForm.confirmPassword}
            />
          </label>
        </div>
      </Modal>
    </div>
  )
}
