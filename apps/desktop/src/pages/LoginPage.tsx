import { zodResolver } from '@hookform/resolvers/zod'
import { IconBrandGithub, IconEye, IconEyeOff, IconRefresh } from '@tabler/icons-react'
import { useCallback, useEffect, useRef, useState } from 'react'
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
import { Button, buttonVariants } from '@/components/ui/Button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'

const schema = z.object({
  username: z.string().trim().min(1, 'Username is required'),
  password: z.string().min(1, 'Password is required'),
  captcha: z.string(),
})

type FormValues = z.infer<typeof schema>

const GITHUB_REPOSITORY_URL = 'https://github.com/nivek-ph/axum-admin'

export function LoginPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const [captcha, setCaptcha] = useState<CaptchaData | null>(null)
  const [captchaLoading, setCaptchaLoading] = useState(false)
  const [captchaUnavailable, setCaptchaUnavailable] = useState(false)
  const captchaLoadingRef = useRef(false)
  const [submitting, setSubmitting] = useState(false)
  const [showPassword, setShowPassword] = useState(false)
  const setSession = useAuthStore((state) => state.setSession)
  const setTokenPair = useAuthStore((state) => state.setTokenPair)
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
    if (captchaLoadingRef.current) return
    captchaLoadingRef.current = true
    setCaptchaLoading(true)
    try {
      const response = await fetchCaptcha()
      if (response.code === 'OK') {
        setCaptcha(response.data ?? null)
        setCaptchaUnavailable(false)
      } else {
        setCaptchaUnavailable(true)
        toast.error(response.message)
      }
    } catch (error) {
      setCaptchaUnavailable(true)
      toast.error(t(getApiErrorMessage(error, 'Sign in failed')))
    } finally {
      captchaLoadingRef.current = false
      setCaptchaLoading(false)
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
      setTokenPair(response.data.accessToken, response.data.refreshToken)
      const [userResponse, menuResponse] = await Promise.all([
        getUserInfo(response.data.accessToken),
        getCurrentMenu(response.data.accessToken),
      ])
      const user = userResponse.data?.userInfo ?? response.data.user
      if (userResponse.code !== 'OK' || menuResponse.code !== 'OK') throw new Error(t('Sign in failed'))
      const { accessToken, refreshToken } = useAuthStore.getState()
      setSession({
        accessToken,
        refreshToken,
        userInfo: user,
      })
      setUserAndPermissions(user, menuResponse.data?.permissions ?? [])
      setAuthorizedMenus(menuResponse.data?.menus ?? [])
      const routeName = user.homeRoute?.replace(/^\/+/, '') || 'dashboard'
      const menu = useMenuStore.getState()
      navigate(menu.canAccess(routeName) ? `/${routeName}` : menu.firstAuthorizedPath(), { replace: true })
    } catch (error) {
      useAuthStore.getState().clearSession()
      useMenuStore.getState().resetAccess()
      toast.error(t(getApiErrorMessage(error, 'Sign in failed')))
      await loadCaptcha()
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <main className="grid min-h-svh lg:grid-cols-[minmax(0,1.15fr)_minmax(24rem,0.85fr)]">
      <aside
        aria-hidden="true"
        className="relative hidden overflow-hidden bg-[oklch(0.2_0.03_277)] text-white lg:flex lg:flex-col lg:justify-end lg:p-12 xl:p-16"
      >
        <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(80%_60%_at_18%_12%,oklch(0.46_0.07_277_/_0.5),transparent_60%)]" />
        <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(55%_45%_at_90%_88%,oklch(0.36_0.05_250_/_0.35),transparent_50%)]" />
        <div className="pointer-events-none absolute inset-0 bg-[linear-gradient(180deg,transparent_40%,oklch(0.16_0.03_277_/_0.85)_100%)]" />

        {/* oversized mark watermark */}
        <svg
          className="pointer-events-none absolute -right-[8%] top-[12%] h-[78%] w-auto text-white/[0.07]"
          fill="none"
          viewBox="0 0 32 32"
        >
          <rect fill="currentColor" height="22" opacity="0.45" rx="4" width="10" x="4" y="5" />
          <rect fill="currentColor" height="16" opacity="0.7" rx="4" width="10" x="11" y="8" />
          <rect fill="currentColor" height="22" rx="4" width="10" x="18" y="5" />
        </svg>

        <div
          className="pointer-events-none absolute inset-0 opacity-[0.1] mix-blend-overlay"
          style={{
            backgroundImage:
              "url(\"data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.85' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E\")",
          }}
        />

        <div className="relative z-10 max-w-lg">
          <p className="text-[clamp(4.5rem,9vw,7rem)] font-semibold leading-none tracking-[-0.04em] text-white">ava</p>
          <p className="mt-4 text-lg font-medium text-white/70 xl:mt-5 xl:text-xl">{t('Admin Console')}</p>
        </div>
      </aside>

      <section className="relative flex flex-col bg-background">
        <div className="absolute top-4 right-4 z-10 flex items-center gap-1.5">
          <a
            aria-label="GitHub"
            className={buttonVariants({ size: 'sm', variant: 'ghost' })}
            href={GITHUB_REPOSITORY_URL}
            rel="noreferrer"
            target="_blank"
          >
            <IconBrandGithub />
            GitHub
          </a>
          <ThemeSwitch />
          <LanguageSwitch />
        </div>

        <div className="m-auto w-full max-w-md px-5 py-12 xl:max-w-lg xl:px-8 xl:py-16">
          <div className="mb-6 flex items-center gap-3 lg:hidden">
            <BrandMark />
            <div>
              <p className="text-xs font-medium tracking-[0.22em] uppercase text-muted-foreground">ava</p>
              <strong className="text-lg font-semibold">{t('Admin Console')}</strong>
            </div>
          </div>

          <Card className="border-border/80 shadow-sm">
            <CardHeader className="pb-4">
              <CardTitle className="text-xl xl:text-2xl">{t('Account Login')}</CardTitle>
            </CardHeader>
            <CardContent>
              <form className="space-y-4" noValidate onSubmit={handleSubmit(submit)}>
                <div className="space-y-1.5">
                  <Input
                    aria-describedby={errors.username ? 'username-error' : undefined}
                    aria-invalid={Boolean(errors.username)}
                    aria-label={t('Username')}
                    autoComplete="username"
                    className="h-10 px-3 text-sm md:text-sm"
                    id="username"
                    placeholder={t('Username')}
                    {...register('username')}
                  />
                  {errors.username ? (
                    <span className="text-xs text-destructive" id="username-error" role="alert">
                      {t(errors.username.message ?? 'Username is required')}
                    </span>
                  ) : null}
                </div>

                <div className="space-y-1.5">
                  <div className="relative">
                    <Input
                      aria-describedby={errors.password ? 'password-error' : undefined}
                      aria-invalid={Boolean(errors.password)}
                      aria-label={t('Password')}
                      autoComplete="current-password"
                      className="h-10 px-3 pr-10 text-sm md:text-sm"
                      id="password"
                      placeholder={t('Enter password')}
                      type={showPassword ? 'text' : 'password'}
                      {...register('password')}
                    />
                    <Button
                      aria-label={showPassword ? t('Hide password') : t('Show password')}
                      className="absolute top-1/2 right-1.5 -translate-y-1/2"
                      onClick={() => setShowPassword((value) => !value)}
                      size="icon-sm"
                      type="button"
                      variant="ghost"
                    >
                      {showPassword ? <IconEyeOff /> : <IconEye />}
                    </Button>
                  </div>
                  {errors.password ? (
                    <span className="text-xs text-destructive" id="password-error" role="alert">
                      {t(errors.password.message ?? 'Password is required')}
                    </span>
                  ) : null}
                </div>

                {captcha?.openCaptcha || captchaLoading || captchaUnavailable ? (
                  <div className="space-y-1.5">
                    <div className="flex gap-2.5">
                      <Input
                        aria-describedby={errors.captcha ? 'captcha-error' : undefined}
                        aria-invalid={Boolean(errors.captcha)}
                        aria-label={t('Captcha')}
                        className="h-10 flex-1 px-3 text-sm md:text-sm"
                        disabled={!captcha?.openCaptcha || captchaUnavailable}
                        id="captcha"
                        placeholder={t('Enter captcha')}
                        {...register('captcha')}
                      />
                      <button
                        aria-label="Reload captcha"
                        className="group relative flex h-10 w-[9.5rem] shrink-0 items-center justify-center overflow-hidden rounded-lg border border-border bg-[oklch(0.96_0.02_250)] transition-colors hover:border-ring/50 dark:bg-muted"
                        disabled={captchaLoading}
                        onClick={() => void loadCaptcha()}
                        type="button"
                      >
                        {captcha?.openCaptcha && !captchaUnavailable ? (
                          <>
                            <img
                              alt="Captcha"
                              className="max-h-full max-w-full object-contain select-none"
                              draggable={false}
                              src={captcha.picPath}
                            />
                            <span className="absolute inset-0 flex items-center justify-center bg-background/70 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100">
                              <IconRefresh className="size-3.5" />
                            </span>
                          </>
                        ) : (
                          <IconRefresh className={captchaLoading ? 'size-4 animate-spin' : 'size-4'} />
                        )}
                      </button>
                    </div>
                    {errors.captcha ? (
                      <span className="text-xs text-destructive" id="captcha-error" role="alert">
                        {t(errors.captcha.message ?? 'Captcha is required')}
                      </span>
                    ) : null}
                  </div>
                ) : null}

                <Button className="mt-1 h-10 w-full text-sm" disabled={submitting || captchaUnavailable} type="submit">
                  {submitting ? '…' : t('Sign in')}
                </Button>
              </form>
            </CardContent>
          </Card>
        </div>
      </section>
    </main>
  )
}
