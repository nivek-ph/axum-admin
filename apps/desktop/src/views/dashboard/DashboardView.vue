<template>
  <div class="admin-page">
    <section class="page-hero">
      <div class="page-hero-main">
        <span class="page-hero-kicker">{{ $t('Overview') }}</span>
        <h2 class="page-hero-title">{{ $t('Dashboard') }}</h2>
        <p class="page-hero-subtitle">
          {{ $t('Central access to the main admin modules: users, roles, menus, and API permissions.') }}
        </p>

        <div class="page-metrics">
          <div class="page-metric">
            <div class="page-metric-label">{{ $t('Entries') }}</div>
            <div class="page-metric-value">{{ menuStore.items.length }}</div>
          </div>
          <div class="page-metric">
            <div class="page-metric-label">{{ $t('Current identity') }}</div>
            <div class="page-metric-value">{{ currentRole }}</div>
          </div>
        </div>
      </div>

      <aside class="page-hero-side">
        <div>
          <div class="page-note-label">{{ $t('System overview') }}</div>
          <div class="page-note-value">{{ $t('Users, roles, menus, and API management are available.') }}</div>
        </div>
        <div class="page-hero-actions">
          <UiButton
            v-for="action in heroActions"
            :key="action.name"
            :type="action.type"
            @click="router.push(action.path)"
          >
            {{ $t(action.title) }}
          </UiButton>
        </div>
      </aside>
    </section>

    <section class="dashboard-grid">
      <article class="page-panel">
        <div class="page-panel-header">
          <div>
            <h3 class="page-panel-title">{{ $t('Shortcuts') }}</h3>
            <p class="page-panel-subtitle">{{ $t('Open the core pages that are already wired.') }}</p>
          </div>
        </div>

        <div class="shortcut-grid">
          <button
            v-for="item in quickLinks"
            :key="item.path"
            class="shortcut-card"
            type="button"
            @click="router.push(item.path)"
          >
            <div class="shortcut-title">{{ $t(item.title) }}</div>
            <div class="shortcut-subtitle">{{ $t(item.description) }}</div>
          </button>
        </div>
      </article>

    </section>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'

import { t } from '@/i18n'
import { useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'

const router = useRouter()
const authStore = useAuthStore()
const menuStore = useMenuStore()

const currentRole = computed(() => authStore.roleLabel || t('Guest'))

const allQuickLinks = [
  { name: 'users', path: '/users', title: 'Users', description: 'Review user status, reset passwords, and remove accounts.' },
  { name: 'roles', path: '/roles', title: 'Roles', description: 'Manage roles, permissions, and members.' },
  { name: 'menus', path: '/menus', title: 'Menus', description: 'Manage menu structure and role visibility.' },
  { name: 'apis', path: '/apis', title: 'API directory', description: 'Review registered backend endpoints.' }
]
const quickLinks = computed(() => allQuickLinks.filter((item) => menuStore.canAccessRouteName(item.name)))
const heroActions = computed(() =>
  [
    { name: 'users', path: '/users', title: 'Users', type: 'default' },
    { name: 'roles', path: '/roles', title: 'Roles', type: 'primary' }
  ].filter((item) => menuStore.canAccessRouteName(item.name))
)
</script>

<style scoped>
.dashboard-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 18px;
}

.shortcut-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 14px;
}

.shortcut-card {
  padding: 18px;
  border: 1px solid rgba(92, 102, 89, 0.08);
  border-radius: 22px;
  background: rgba(255, 255, 255, 0.62);
  text-align: left;
  cursor: pointer;
  transition: transform 160ms ease, box-shadow 160ms ease, border-color 160ms ease;
}

.shortcut-card:hover {
  transform: translateY(-2px);
  border-color: rgba(55, 95, 75, 0.18);
  box-shadow: 0 18px 32px rgba(67, 75, 68, 0.08);
}

.shortcut-title {
  color: #1c2923;
  font-size: 16px;
  font-weight: 700;
}

.shortcut-subtitle {
  margin-top: 8px;
  color: var(--text-secondary);
  font-size: 13px;
}

@media (max-width: 960px) {
  .dashboard-grid,
  .shortcut-grid {
    grid-template-columns: 1fr;
  }
}
</style>
