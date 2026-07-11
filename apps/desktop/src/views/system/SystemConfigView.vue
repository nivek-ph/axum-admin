<template>
  <div class="admin-page">
    <section class="page-hero">
      <div class="page-hero-main">
        <span class="page-hero-kicker">{{ $t('System settings') }}</span>
        <h2 class="page-hero-title">{{ $t('System config') }}</h2>
        <p class="page-hero-subtitle">{{ $t('Show key runtime settings for the current environment.') }}</p>

        <div class="page-metrics">
          <div class="page-metric">
            <div class="page-metric-label">{{ $t('Runtime environment') }}</div>
            <div class="page-metric-value">{{ config.system.env || '-' }}</div>
          </div>
          <div class="page-metric">
            <div class="page-metric-label">{{ $t('Service address') }}</div>
            <div class="page-metric-value">{{ addrLabel }}</div>
          </div>
          <div class="page-metric">
            <div class="page-metric-label">{{ $t('Captcha') }}</div>
            <div class="page-metric-value">{{ $t(config.captcha.openCaptcha ? 'On' : 'Close') }}</div>
          </div>
        </div>
      </div>

    </section>

    <section class="dashboard-grid">
      <article class="page-panel">
        <div class="page-panel-header">
          <div>
            <h3 class="page-panel-title">{{ $t('System params') }}</h3>
            <p class="page-panel-subtitle">{{ $t('Runtime settings most relevant to this console.') }}</p>
          </div>
          <div class="page-panel-actions">
            <UiButton @click="loadConfig" :loading="loading">{{ $t('Refresh') }}</UiButton>
          </div>
        </div>

        <div class="surface-card state-list">
          <div class="state-row">
            <span>{{ $t('Environment') }}</span>
            <strong>{{ config.system.env || '-' }}</strong>
          </div>
          <div class="state-row">
            <span>{{ $t('Service address') }}</span>
            <strong>{{ config.system.addr || '-' }}</strong>
          </div>
          <div class="state-row">
            <span>{{ $t('Database') }}</span>
            <strong>{{ config.system['db-type'] || '-' }}</strong>
          </div>
          <div class="state-row">
            <span>{{ $t('Multi-login') }}</span>
            <strong>{{ $t(config.system['use-multipoint'] ? 'On' : 'Close') }}</strong>
          </div>
        </div>
      </article>

      <article class="page-panel">
        <div class="page-panel-header">
          <div>
            <h3 class="page-panel-title">{{ $t('Local config') }}</h3>
            <p class="page-panel-subtitle">{{ $t('Local runtime settings.') }}</p>
          </div>
        </div>

        <div class="surface-card state-list">
          <div class="state-row">
            <span>{{ $t('Storage path') }}</span>
            <strong>{{ config.local.storePath || '-' }}</strong>
          </div>
          <div class="state-row">
            <span>{{ $t('Captcha switch') }}</span>
            <strong>{{ $t(config.captcha.openCaptcha ? 'On' : 'Close') }}</strong>
          </div>
          <div class="state-row">
            <span>{{ $t('Captcha timeout') }}</span>
            <strong>{{ config.captcha.openCaptchaTimeOut || 0 }}</strong>
          </div>
          <div class="state-row">
            <span>{{ $t('Strict auth') }}</span>
            <strong>{{ $t(config.system['use-strict-auth'] ? 'On' : 'Close') }}</strong>
          </div>
        </div>
      </article>
    </section>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { ElMessage } from '@/ui/feedback'

import { fetchSystemConfig, type SystemConfig } from '@/api/system'
import { t } from '@/i18n'

const loading = ref(false)
const config = reactive<SystemConfig>({
  system: { env: '', addr: '', 'db-type': '' },
  captcha: { openCaptcha: 1, openCaptchaTimeOut: 300 },
  local: { storePath: '' }
})

const addrLabel = computed(() => {
  if (!config.system.addr) return '-'
  return config.system.addr.replace('127.0.0.1:', ':')
})

async function loadConfig() {
  loading.value = true
  try {
    const nextConfig = await fetchSystemConfig()
    Object.assign(config, nextConfig)
  } catch {
    ElMessage.error(t('Failed to load system config'))
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  loadConfig()
})
</script>

<style scoped>
.dashboard-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 18px;
}

.state-list {
  padding: 6px 18px;
}

.state-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 18px 0;
}

.state-row + .state-row {
  border-top: 1px solid var(--panel-border);
}

.state-row span {
  color: var(--text-secondary);
}

.state-row strong {
  color: var(--text-primary);
}

@media (max-width: 960px) {
  .dashboard-grid {
    grid-template-columns: 1fr;
  }
}
</style>
