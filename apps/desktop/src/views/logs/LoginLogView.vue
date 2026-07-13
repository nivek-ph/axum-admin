<template>
  <div class="admin-page">
    <section class="page-hero">
      <div class="page-hero-main">
        <span class="page-hero-kicker">Audit Trail</span>
        <h2 class="page-hero-title">{{ $t('Login logs') }}</h2>
        <p class="page-hero-subtitle">{{ $t('Review successful and failed sign-ins to diagnose account usage.') }}</p>

        <div class="page-metrics">
          <div class="page-metric">
            <div class="page-metric-label">{{ $t('Rows on page') }}</div>
            <div class="page-metric-value">{{ logs.length }}</div>
          </div>
          <div class="page-metric">
            <div class="page-metric-label">{{ $t('Total logs') }}</div>
            <div class="page-metric-value">{{ total }}</div>
          </div>
          <div class="page-metric">
            <div class="page-metric-label">{{ $t('Failed records') }}</div>
            <div class="page-metric-value">{{ failedCount }}</div>
          </div>
        </div>
      </div>

    </section>

    <section class="page-panel">
      <div class="page-panel-header">
        <div>
          <h3 class="page-panel-title">{{ $t('Login audit') }}</h3>
          <p class="page-panel-subtitle">{{ $t('Keep only core filters to reduce audit noise.') }}</p>
        </div>
        <div class="page-panel-actions">
          <UiButton @click="loadLogs" :loading="loading">{{ $t('Refresh') }}</UiButton>
          <UiButton :disabled="selectedIds.length === 0" @click="handleBatchDelete">
            {{ $t('Delete') }}
          </UiButton>
          <UiButton type="primary" @click="page = 1; loadLogs()">{{ $t('Search') }}</UiButton>
        </div>
      </div>

      <div class="page-panel-toolbar inline-filter">
        <UiInput v-model="filters.username" placeholder="Filter by username" clearable />
        <UiSelect v-model="statusModel" clearable placeholder="Login status">
          <UiOption label="Success" value="success" />
          <UiOption label="Failed" value="failed" />
        </UiSelect>
        <UiButton type="primary" @click="handleSearch">{{ $t('Search') }}</UiButton>
      </div>

      <div class="surface-card">
        <UiTable :data="logs" :loading="loading" style="width: 100%" @selection-change="handleSelectionChange">
          <UiTableColumn type="selection" width="44" />
          <UiTableColumn prop="id" label="ID" width="80" />
          <UiTableColumn prop="username" label="Username" min-width="140" />
          <UiTableColumn prop="ip" label="IP" min-width="140" />
          <UiTableColumn label="Status" width="100">
            <template #default="{ row }">
              <UiTag :type="row.status ? 'success' : 'danger'">
                {{ $t(row.status ? 'Success' : 'Failed') }}
              </UiTag>
            </template>
          </UiTableColumn>
          <UiTableColumn prop="errorMessage" label="Error message" min-width="180" />
          <UiTableColumn prop="createdAt" label="Time" min-width="180" />
          <UiTableColumn label="Actions" width="120">
            <template #default="{ row }">
              <UiButton link type="danger" @click="handleDelete(row.id)">{{ $t('Delete') }}</UiButton>
            </template>
          </UiTableColumn>
        </UiTable>
      </div>

      <div class="pagination">
        <UiPagination
          background
          layout="total, prev, pager, next"
          :total="total"
          :current-page="page"
          :page-size="pageSize"
          @current-change="handlePageChange"
        />
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { ElMessage, ElMessageBox } from '@/ui/feedback'

import { deleteLoginLog, deleteLoginLogs, fetchLoginLogs, type LoginLogRecord } from '@/api/logs'
import { t } from '@/i18n'

const logs = ref<LoginLogRecord[]>([])
const loading = ref(false)
const total = ref(0)
const page = ref(1)
const pageSize = ref(10)
const filters = reactive({
  username: ''
})
const statusModel = ref<'success' | 'failed' | undefined>()
const selectedIds = ref<number[]>([])
const failedCount = computed(() => logs.value.filter((item) => !item.status).length)

async function loadLogs() {
  loading.value = true
  try {
    const result = await fetchLoginLogs({
      page: page.value,
      pageSize: pageSize.value,
      username: filters.username,
      status:
        statusModel.value === undefined
          ? undefined
          : statusModel.value === 'success'
    })
    logs.value = result.list
    total.value = result.total
  } catch {
    ElMessage.error(t('Failed to load login logs'))
  } finally {
    loading.value = false
  }
}

function handleSearch() {
  page.value = 1
  loadLogs()
}

function handlePageChange(nextPage: number) {
  page.value = nextPage
  loadLogs()
}

function handleSelectionChange(rows: LoginLogRecord[]) {
  selectedIds.value = rows.map((row) => row.id)
}

async function handleDelete(id: number) {
  try {
    await ElMessageBox.confirm(t('Delete this login log?'), t('Notice'), { type: 'warning' })
  } catch {
    return
  }

  try {
    const response = await deleteLoginLog(id)
    if (response.code === 'OK') {
      ElMessage.success(t('Login log deleted'))
      await loadLogs()
      return
    }
    ElMessage.error(response.message || t('Delete failed'))
  } catch {
    ElMessage.error(t('Delete failed'))
  }
}

async function handleBatchDelete() {
  if (selectedIds.value.length === 0) return

  try {
    await ElMessageBox.confirm(t('Delete {count} login logs?', { count: selectedIds.value.length }), t('Notice'), {
      type: 'warning'
    })
  } catch {
    return
  }

  try {
    const response = await deleteLoginLogs(selectedIds.value)
    if (response.code === 'OK') {
      ElMessage.success(t('Login logs deleted'))
      selectedIds.value = []
      await loadLogs()
      return
    }
    ElMessage.error(response.message || t('Batch delete failed'))
  } catch {
    ElMessage.error(t('Batch delete failed'))
  }
}

onMounted(() => {
  loadLogs()
})
</script>

<style scoped>
.pagination {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}
</style>
