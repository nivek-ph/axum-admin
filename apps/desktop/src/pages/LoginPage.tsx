import { zodResolver } from '@hookform/resolvers/zod'
import { Eye, EyeOff } from 'lucide-react'
import { useCallback, useEffect, useState } from 'react'
import { useForm } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router-dom'
import { toast } from 'sonner'
import { z } from 'zod'

import { fetchCaptcha, getCurrentMenu, getUserInfo, login, type CaptchaData } from '@/api/auth'
import { getApiErrorMessage } from '@/api/http'
import { BrandMark } from '@/components/BrandMark'
import { LanguageSwitch } from '@/components/LanguageSwitch'
import { ThemeSwitch } from '@/components/ThemeSwitch'
import { Button } from '@/components/ui/Button'
import { isSuperAdmin, useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'

const schema = z.object({
  username: z.string().trim().min(1, 'Username is required'),
  password: z.string().min(1, 'Password is required'),
  captcha: z.string(),
})

type FormValues = z.infer<typeof schema>

export function LoginPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const [captcha, setCaptcha] = useState<CaptchaData | null>(null)
  const [submitting, setSubmitting] = useState(false)
  const [showPassword, setShowPassword] = useState(false)
  const setSession = useAuthStore((state) => state.setSession)
  const setUserAndPermissions = useAuthStore((state) => state.setUserAndPermissions)
  const setAuthorizedMenus = useMenuStore((state) => state.setAuthorizedMenus)
  const activeSchema = schema.extend({
    captcha: captcha?.openCaptcha ? z.string().trim().min(1, 'Captcha is required') : z.string(),
  })
  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<FormValues>({
    resolver: zodResolver(activeSchema),
    defaultValues: { username: '', password: '', captcha: '' },
  })

  const loadCaptcha = useCallback(async () => {
    try {
      const response = await fetchCaptcha()
      if (response.code === 'OK') setCaptcha(response.data ?? null)
      else toast.error(response.message)
    } catch (error) {
      toast.error(getApiErrorMessage(error, t('Sign in failed')))
    }
  }, [t])

  useEffect(() => {
    void loadCaptcha()
  }, [loadCaptcha])

  async function submit(values: FormValues) {
    setSubmitting(true)
    try {
      const response = await login({ ...values, captchaId: captcha?.captchaId ?? '' })
      if (response.code !== 'OK' || !response.data) throw new Error(response.message || t('Sign in failed'))
      setSession({
        accessToken: response.data.accessToken,
        refreshToken: response.data.refreshToken,
        userInfo: response.data.user,
      })
      const [userResponse, menuResponse] = await Promise.all([
        getUserInfo(response.data.accessToken),
        getCurrentMenu(response.data.accessToken),
      ])
      const user = userResponse.data?.userInfo ?? response.data.user
      if (userResponse.code !== 'OK' || menuResponse.code !== 'OK') throw new Error(t('Sign in failed'))
      setUserAndPermissions(user, menuResponse.data?.permissions ?? [])
      setAuthorizedMenus(menuResponse.data?.menus ?? [], isSuperAdmin(user))
      const routeName = user.homeRoute?.replace(/^\/+/, '') || 'dashboard'
      const menu = useMenuStore.getState()
      navigate(menu.canAccess(routeName) ? `/${routeName}` : menu.firstAuthorizedPath(), { replace: true })
    } catch (error) {
      useAuthStore.getState().clearSession()
      useMenuStore.getState().resetAccess()
      toast.error(getApiErrorMessage(error, t('Sign in failed')))
      await loadCaptcha()
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <main className="login-page">
      <aside className="login-stage" aria-hidden="true">
        <div className="login-stage-glow" />
        <div className="login-stage-grid" />
        <div className="login-stage-copy">
          <BrandMark />
          <p className="eyebrow">axum-admin</p>
          <strong>{t('Admin Console')}</strong>
        </div>
      </aside>
      <section className="login-panel">
        <div className="login-toolbar">
          <ThemeSwitch />
          <LanguageSwitch />
        </div>
        <div className="login-card">
          <h1 className="login-title">{t('Account Login')}</h1>
          <form noValidate onSubmit={handleSubmit(submit)}>
            <div className="login-field">
              <input
                aria-describedby={errors.username ? 'username-error' : undefined}
                aria-invalid={Boolean(errors.username)}
                aria-label={t('Username')}
                autoComplete="username"
                id="username"
                placeholder={t('Username')}
                {...register('username')}
              />
              {errors.username && (
                <span className="field-error" id="username-error" role="alert">
                  {t(errors.username.message ?? 'Username is required')}
                </span>
              )}
            </div>
            <div className="login-field">
              <div className="login-password">
                <input
                  aria-describedby={errors.password ? 'password-error' : undefined}
                  aria-invalid={Boolean(errors.password)}
                  aria-label={t('Password')}
                  autoComplete="current-password"
                  id="password"
                  placeholder={t('Enter password')}
                  type={showPassword ? 'text' : 'password'}
                  {...register('password')}
                />
                <button
                  aria-label={showPassword ? t('Hide password') : t('Show password')}
                  className="login-password-toggle"
                  onClick={() => setShowPassword((value) => !value)}
                  type="button"
                >
                  {showPassword ? <EyeOff size={16} /> : <Eye size={16} />}
                </button>
              </div>
              {errors.password && (
                <span className="field-error" id="password-error" role="alert">
                  {t(errors.password.message ?? 'Password is required')}
                </span>
              )}
            </div>
            {captcha?.openCaptcha && (
              <div className="login-field">
                <div className="captcha-row">
                  <input
                    aria-describedby={errors.captcha ? 'captcha-error' : undefined}
                    aria-invalid={Boolean(errors.captcha)}
                    aria-label={t('Captcha')}
                    id="captcha"
                    placeholder={t('Enter captcha')}
                    {...register('captcha')}
                  />
                  <button
                    aria-label="Reload captcha"
                    className="captcha-image"
                    onClick={() => void loadCaptcha()}
                    type="button"
                  >
                    <img alt="Captcha" src={captcha.picPath} />
                  </button>
                </div>
                {errors.captcha && (
                  <span className="field-error" id="captcha-error" role="alert">
                    {t(errors.captcha.message ?? 'Captcha is required')}
                  </span>
                )}
              </div>
            )}
            <Button className="login-submit" disabled={submitting} type="submit" variant="primary">
              {submitting ? '…' : t('Sign in')}
            </Button>
          </form>
        </div>
      </section>
    </main>
  )
}
