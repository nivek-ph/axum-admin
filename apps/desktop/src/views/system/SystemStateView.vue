<template>
  <div class="admin-page">
    <section class="page-hero">
      <div class="page-hero-main">
        <span class="page-hero-kicker">{{ $t('Runtime status') }}</span>
        <h2 class="page-hero-title">{{ $t('System status') }}</h2>
        <p class="page-hero-subtitle">{{ $t('Review service address, environment, and resource status.') }}</p>

        <div class="page-metrics">
          <div class="page-metric">
            <div class="page-metric-label">{{ $t('Runtime environment') }}</div>
            <div class="page-metric-value">{{ config.system.env || '-' }}</div>
          </div>
          <div class="page-metric">
            <div class="page-metric-label">{{ $t('CPU cores') }}</div>
            <div class="page-metric-value">{{ server.os.numCpu || 0 }}</div>
          </div>
          <div class="page-metric">
            <div class="page-metric-label">{{ $t('Memory') }}</div>
            <div class="page-metric-value">{{ server.ram.usedPercent || 0 }}%</div>
          </div>
        </div>
      </div>

    </section>

    <section class="dashboard-grid">
      <article class="page-panel">
        <div class="page-panel-header">
          <div>
            <h3 class="page-panel-title">{{ $t('Runtime config') }}</h3>
            <p class="page-panel-subtitle">{{ $t('Summary of system config from the backend.') }}</p>
          </div>
          <div class="page-panel-actions">
            <UiButton @click="loadState" :loading="loading">{{ $t('Refresh') }}</UiButton>
          </div>
        </div>

        <div class="surface-card state-list">
          <div class="state-row">
            <span>{{ $t('Service address') }}</span>
            <strong>{{ config.system.addr || '-' }}</strong>
          </div>
          <div class="state-row">
            <span>{{ $t('Database type') }}</span>
            <strong>{{ config.system['db-type'] || '-' }}</strong>
          </div>
          <div class="state-row">
            <span>{{ $t('File path') }}</span>
            <strong>{{ config.local.storePath || '-' }}</strong>
          </div>
          <div class="state-row">
            <span>{{ $t('Captcha') }}</span>
            <strong>{{ $t(config.captcha.openCaptcha ? 'On' : 'Close') }}</strong>
          </div>
        </div>
      </article>

      <article class="page-panel">
        <div class="page-panel-header">
          <div>
            <h3 class="page-panel-title">{{ $t('Resources') }}</h3>
            <p class="page-panel-subtitle">{{ $t('Basic health view for local operation.') }}</p>
          </div>
        </div>

        <div class="surface-card state-list">
          <div class="state-row">
            <span>{{ $t('OS') }}</span>
            <strong>{{ server.os.goos || '-' }}</strong>
          </div>
          <div class="state-row">
            <span>{{ $t('Compiler') }}</span>
            <strong>{{ server.os.compiler || '-' }}</strong>
          </div>
          <div class="state-row">
            <span>{{ $t('CPU samples') }}</span>
            <strong>{{ cpuPreview }}</strong>
          </div>
          <div class="state-row">
            <span>{{ $t('Disk usage') }}</span>
            <strong>{{ diskPreview }}</strong>
          </div>
        </div>
      </article>
    </section>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { ElMessage } from '@/ui/feedback'

import {
  fetchServerInfo,
  fetchSystemConfig,
  type ServerInfo,
  type SystemConfig
} from '@/api/system'
import { t } from '@/i18n'

const loading = ref(false)
const config = reactive<SystemConfig>({
  system: { env: '', addr: '', 'db-type': '' },
  captcha: { openCaptcha: 1, openCaptchaTimeOut: 300 },
  local: { storePath: '' }
})
const server = reactive<ServerInfo>({
  os: { goos: '', numCpu: 0, compiler: '', goVersion: '', numGoroutine: 0 },
  cpu: { cores: 0, cpus: [] },
  ram: { totalMb: 0, usedMb: 0, usedPercent: 0 },
  disk: []
})

const cpuPreview = computed(() => server.cpu.cpus.slice(0, 4).join(' / ') || '-')
const diskPreview = computed(() => {
  const disk = server.disk[0]
  if (!disk) return '-'
  return `${disk.mountPoint} · ${disk.usedGb}/${disk.totalGb} GB`
})

async function loadState() {
  loading.value = true
  try {
    const [nextConfig, nextServer] = await Promise.all([fetchSystemConfig(), fetchServerInfo()])
    Object.assign(config, nextConfig)
    Object.assign(server, nextServer)
  } catch {
    ElMessage.error(t('Failed to load system status'))
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  loadState()
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
