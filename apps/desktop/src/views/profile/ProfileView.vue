<template>
  <div class="admin-page">
    <section class="page-hero">
      <div class="page-hero-main">
        <span class="page-hero-kicker">{{ $t('Account') }}</span>
        <h2 class="page-hero-title">{{ $t('Profile') }}</h2>
        <p class="page-hero-subtitle">{{ $t('View the current account identity and role.') }}</p>

        <div class="page-metrics">
          <div class="page-metric">
            <div class="page-metric-label">{{ $t('Nickname') }}</div>
            <div class="page-metric-value">{{ authStore.userInfo?.nickName || $t('Not signed in') }}</div>
          </div>
          <div class="page-metric">
            <div class="page-metric-label">{{ $t('Username') }}</div>
            <div class="page-metric-value">{{ authStore.userInfo?.userName || '-' }}</div>
          </div>
          <div class="page-metric">
            <div class="page-metric-label">{{ $t('Current role') }}</div>
            <div class="page-metric-value">
              {{ authStore.roleLabel || $t('Guest') }}
            </div>
          </div>
        </div>
      </div>

    </section>

    <section class="page-panel">
      <div class="page-panel-header">
        <div>
          <h3 class="page-panel-title">{{ $t('Identity card') }}</h3>
          <p class="page-panel-subtitle">{{ $t('A compact account identity view.') }}</p>
        </div>
        <div v-if="canOpenDashboard || canOpenUsers" class="page-panel-actions">
          <UiButton v-if="canOpenDashboard" @click="router.push('/dashboard')">{{ $t('Dashboard') }}</UiButton>
          <UiButton v-if="canOpenUsers" type="primary" @click="router.push('/users')">{{ $t('Users') }}</UiButton>
        </div>
      </div>

      <div class="profile-card">
        <div class="profile-avatar">
          {{ (authStore.userInfo?.nickName || authStore.userInfo?.userName || 'A').slice(0, 1) }}
        </div>
        <div class="profile-content">
          <div class="profile-name">{{ authStore.userInfo?.nickName || $t('Not signed in') }}</div>
          <div class="profile-role">{{ authStore.roleLabel || $t('Guest') }}</div>
          <div class="profile-meta">
            {{ $t('User ID') }}：{{ authStore.userInfo?.id || '-' }} / {{ $t('Home route') }}：{{
              authStore.userInfo?.homeRoute || 'dashboard'
            }}
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'

import { useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'

const router = useRouter()
const authStore = useAuthStore()
const menuStore = useMenuStore()
const canOpenDashboard = computed(() => menuStore.canAccessRouteName('dashboard'))
const canOpenUsers = computed(() => menuStore.canAccessRouteName('users'))
</script>

<style scoped>
.profile-card {
  display: flex;
  align-items: center;
  gap: 18px;
  padding: 24px;
  border-radius: 24px;
  background: rgba(255, 255, 255, 0.62);
  border: 1px solid rgba(92, 102, 89, 0.08);
}

.profile-avatar {
  width: 72px;
  height: 72px;
  border-radius: 24px;
  display: grid;
  place-items: center;
  background: linear-gradient(145deg, #355c49 0%, #d2b180 100%);
  color: #fffdf8;
  font-size: 28px;
  font-weight: 700;
}

.profile-name {
  color: #1c2923;
  font-size: 24px;
  font-weight: 700;
}

.profile-role {
  margin-top: 6px;
  color: var(--text-secondary);
  font-size: 14px;
}

.profile-meta {
  margin-top: 10px;
  color: var(--text-muted);
  font-size: 13px;
}
</style>
