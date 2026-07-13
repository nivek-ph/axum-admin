<template>
  <header class="header">
    <div>
      <div class="header-eyebrow">{{ $t('Admin Console') }}</div>
      <div class="header-title">{{ currentTitle }}</div>
    </div>

    <div class="header-actions">
      <LanguageSwitch />
      <div class="status-chip">
        <span class="status-dot" />
        <span>{{ $t('Local API') }}</span>
      </div>
      <div class="user-chip">
        <div class="user-avatar">
          {{ userInitial }}
        </div>
        <div>
          <div class="user-name">{{ authStore.userInfo?.nickName || $t('Not signed in') }}</div>
          <div class="user-subtitle">
            {{ authStore.roleLabel || $t('Guest') }}
          </div>
        </div>
      </div>
      <button
        v-if="authStore.isAuthenticated"
        class="logout-button"
        type="button"
        data-test="logout-button"
        :aria-label="$t('Logout')"
        :title="$t('Logout')"
        @click="logout"
      >
        <LogOut :size="17" />
      </button>
    </div>
  </header>
</template>

<script setup lang="ts">
import { LogOut } from '@lucide/vue'
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import LanguageSwitch from '@/components/LanguageSwitch.vue'
import { t } from '@/i18n'
import { useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'

const authStore = useAuthStore()
const menuStore = useMenuStore()
const route = useRoute()
const router = useRouter()

const currentTitle = computed(() => {
  const matched = menuStore.items.find((item) => item.path === route.path)
  return t(matched?.label || 'Admin Console')
})

const userInitial = computed(() => {
  const source = authStore.userInfo?.nickName || authStore.userInfo?.userName || 'A'
  return source.slice(0, 1).toUpperCase()
})

function logout() {
  authStore.clearToken()
  menuStore.resetAccess()
  router.replace('/login')
}
</script>

<style scoped>
.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 18px;
  padding: 22px 24px 16px;
  border-bottom: 1px solid var(--panel-border);
}

.header-eyebrow {
  color: var(--text-muted);
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.18em;
}

.header-title {
  margin-top: 8px;
  font-size: 28px;
  line-height: 1;
  letter-spacing: -0.03em;
  color: #18231d;
  font-weight: 700;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 14px;
}

.status-chip,
.user-chip {
  display: flex;
  align-items: center;
  gap: 10px;
  height: 48px;
  padding: 0 14px;
  border-radius: 12px;
  border: 1px solid var(--panel-border);
  background: #ffffff;
}

.status-chip {
  color: #3f3f46;
  font-size: 12px;
  font-weight: 700;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 999px;
  background: #22c55e;
}

.user-avatar {
  width: 32px;
  height: 32px;
  border-radius: 10px;
  display: grid;
  place-items: center;
  background: #18181b;
  color: #fafafa;
  font-weight: 700;
}

.user-name {
  color: #1c2923;
  font-size: 14px;
  font-weight: 700;
}

.user-subtitle {
  margin-top: 2px;
  color: var(--text-muted);
  font-size: 12px;
}

.logout-button {
  width: 48px;
  height: 48px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 12px;
  border: 1px solid var(--panel-border);
  background: #ffffff;
  color: #52525b;
  cursor: pointer;
  transition:
    border-color 0.16s ease,
    background 0.16s ease,
    color 0.16s ease;
}

.logout-button:hover {
  border-color: #d4d4d8;
  background: #f7f7f8;
  color: #18181b;
}
</style>
