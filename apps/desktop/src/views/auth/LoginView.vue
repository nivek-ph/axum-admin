<template>
  <div class="auth-page">
    <section class="auth-shell">
      <div class="auth-brand">
        <div class="auth-brand-kicker">axum-vue-admin</div>
        <h1 class="auth-title">{{ $t('Admin Console') }}</h1>
        <p class="auth-subtitle">
          {{ $t('Rust + Vue admin for users, roles, menus, and API permissions.') }}
        </p>
      </div>

      <UiCard class="login-card" shadow="never">
        <div class="login-card-top">
          <div class="login-card-kicker">{{ $t('Account sign-in') }}</div>
          <LanguageSwitch />
        </div>
        <h2 class="login-card-title">{{ $t('Sign in') }}</h2>
        <UiForm class="form" @submit.prevent="handleLogin">
          <UiFormItem>
            <UiInput v-model="form.username" placeholder="Username" />
          </UiFormItem>
          <UiFormItem>
            <UiInput v-model="form.password" type="password" placeholder="Password" showPassword />
          </UiFormItem>
          <UiFormItem>
            <div class="captcha-row">
              <UiInput
                v-model="form.captcha"
                class="captcha-input"
                placeholder="Captcha"
                autocomplete="off"
              />
              <button
                class="captcha-image-button"
                type="button"
                :title="$t('Refresh captcha')"
                :aria-label="$t('Refresh captcha')"
                @click="loadCaptcha"
              >
                <img v-if="captchaImage" :src="captchaImage" :alt="$t('Captcha')" class="captcha-image" />
                <span v-else>{{ $t('Refresh captcha') }}</span>
                <span v-if="captchaImage" class="captcha-refresh-mark" aria-hidden="true">↻</span>
              </button>
            </div>
          </UiFormItem>
          <UiFormItem>
            <UiButton type="primary" class="w-full" @click="handleLogin" :loading="loading">
              {{ $t('Sign in') }}
            </UiButton>
          </UiFormItem>
        </UiForm>
      </UiCard>
    </section>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from '@/ui/feedback'

import LanguageSwitch from '@/components/LanguageSwitch.vue'
import { fetchCaptcha, getMenu, getUserInfo, login } from '@/api/auth'
import { getApiErrorMessage } from '@/api/http'
import { t } from '@/i18n'
import { useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'

const router = useRouter()
const authStore = useAuthStore()
const menuStore = useMenuStore()
const loading = ref(false)
const captchaImage = ref('')
const form = reactive({
  username: '',
  password: '',
  captcha: '',
  captchaId: ''
})

async function loadCaptcha() {
  form.captcha = ''
  form.captchaId = ''
  captchaImage.value = ''
  try {
    const res = await fetchCaptcha()
    if (res.code === 'OK' && res.data) {
      form.captchaId = res.data.captchaId
      captchaImage.value = res.data.picPath
      return
    }
    ElMessage.error(res.message || t('Failed to load captcha'))
  } catch (err) {
    ElMessage.error(getApiErrorMessage(err, t('Failed to load captcha')))
  }
}

onMounted(loadCaptcha)

async function handleLogin() {
  if (loading.value) return
  loading.value = true
  try {
    const res = await login(form)
    if (res.code !== 'OK') {
      ElMessage.error(res.message || t('Sign in failed'))
      return
    }
    const loginToken = res.data?.token
    const loginUser = res.data?.user
    if (!loginToken || !loginUser) {
      authStore.clearToken()
      menuStore.resetAccess()
      ElMessage.error(res.message || t('Sign in failed'))
      return
    }

    authStore.setSession(loginToken, loginUser)
    const [userInfoRes, menuRes] = await Promise.all([
      getUserInfo(loginToken),
      getMenu(loginToken)
    ])
    if (menuRes.code !== 'OK') {
      authStore.clearToken()
      menuStore.resetAccess()
      ElMessage.error(menuRes.message || t('Failed to load menus'))
      return
    }

    const currentUser = userInfoRes.code === 'OK' ? userInfoRes.data?.userInfo || loginUser : loginUser
    authStore.setSession(loginToken, currentUser)
    menuStore.setAuthorizedMenus(menuRes.data?.menus || [], authStore.isSuperAdmin)
    await router.push(menuStore.firstAuthorizedPath())
  } catch (err) {
    ElMessage.error(getApiErrorMessage(err, t('Sign in failed')))
    await loadCaptcha()
  } finally {
    loading.value = false
  }
}
</script>

<style scoped>
.auth-page {
  min-height: 100vh;
  display: grid;
  place-items: center;
  padding: 24px;
}

.auth-shell {
  display: grid;
  grid-template-columns: minmax(0, 1.2fr) minmax(360px, 430px);
  gap: 24px;
  width: min(1180px, 100%);
  padding: 16px;
  border-radius: 24px;
  border: 1px solid var(--shell-border);
  background: #fafaf9;
  box-shadow: var(--shell-shadow);
}

.auth-brand {
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 12px;
  min-height: 360px;
  padding: 26px;
  border-radius: 20px;
  border: 1px solid #27272a;
  background: #18181b;
  color: #fafafa;
}

.auth-brand-kicker,
.login-card-kicker {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.18em;
}

.auth-brand-kicker {
  color: rgba(244, 244, 245, 0.54);
}

.auth-title {
  margin: 0;
  font-size: 38px;
  line-height: 1.08;
  letter-spacing: -0.04em;
}

.auth-subtitle {
  margin: 0;
  max-width: 36ch;
  color: rgba(244, 244, 245, 0.72);
  font-size: 14px;
  line-height: 1.55;
}

.login-card {
  display: flex;
  flex-direction: column;
  justify-content: center;
  padding: 28px 24px !important;
  border-radius: 20px;
  border: 1px solid var(--panel-border);
  background: #ffffff !important;
}

.login-card-kicker {
  color: var(--text-muted);
}

.login-card-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.login-card-title {
  margin: 14px 0 0;
  font-size: 32px;
  line-height: 1;
  letter-spacing: -0.04em;
  color: #18181b;
}

.login-card-subtitle {
  margin: 12px 0 0;
  color: var(--text-secondary);
  font-size: 14px;
}

.form {
  margin-top: 24px;
}

.login-note {
  margin-top: 10px;
  color: var(--text-muted);
  font-size: 12px;
}

.w-full {
  width: 100%;
}

.captcha-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 170px;
  gap: 12px;
  width: 100%;
}

.captcha-input {
  min-height: 48px;
}

.captcha-input:focus-visible {
  border-color: #a8a29e;
  box-shadow: 0 0 0 3px rgba(24, 24, 27, 0.07);
}

.captcha-image-button {
  position: relative;
  height: 48px;
  overflow: hidden;
  padding: 2px 4px;
  border: 1px solid #d6d3d1;
  border-radius: 8px;
  background: #fff;
  cursor: pointer;
  transition: border-color 0.15s ease, box-shadow 0.15s ease;
}

.captcha-image-button:hover {
  border-color: #a8a29e;
  box-shadow: 0 0 0 3px rgba(24, 24, 27, 0.05);
}

.captcha-image {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: contain;
}

.captcha-refresh-mark {
  position: absolute;
  top: 2px;
  right: 4px;
  display: grid;
  place-items: center;
  width: 16px;
  height: 16px;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.9);
  color: #71717a;
  font-size: 12px;
  line-height: 1;
}

@media (max-width: 980px) {
  .auth-shell {
    grid-template-columns: 1fr;
  }

  .auth-brand {
    min-height: auto;
  }

}
</style>
