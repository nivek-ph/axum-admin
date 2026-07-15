<template>
  <div class="admin-page">
    <section class="page-hero">
      <div class="page-hero-main">
        <span class="page-hero-kicker">Operation Trail</span>
        <h2 class="page-hero-title">{{ $t('Operation logs') }}</h2>
        <p class="page-hero-subtitle">{{ $t('Trace operation path, method, status, and user for core governance.') }}</p>

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
            <div class="page-metric-label">{{ $t('Error statuses') }}</div>
            <div class="page-metric-value">{{ errorCount }}</div>
          </div>
        </div>
      </div>

    </section>

    <section class="page-panel">
      <div class="page-panel-header">
        <div>
          <h3 class="page-panel-title">{{ $t('Operation audit') }}</h3>
          <p class="page-panel-subtitle">{{ $t('Focus on method, path, and status without overloading the audit view.') }}</p>
        </div>
        <div class="page-panel-actions">
          <UiButton @click="loadLogs" :loading="loading">{{ $t('Refresh') }}</UiButton>
        </div>
      </div>

      <div class="page-panel-toolbar inline-filter operation-log-filter">
        <UiSelect v-model="filters.method" clearable placeholder="Method">
          <UiOption v-for="item in methodOptions" :key="item" :label="item" :value="item" />
        </UiSelect>
        <UiInput v-model="filters.path" placeholder="Filter by path" clearable />
        <UiSelect v-model="statusModel" clearable placeholder="Status code">
          <UiOption label="200" :value="200" />
          <UiOption label="401" :value="401" />
          <UiOption label="500" :value="500" />
        </UiSelect>
        <UiButton type="primary" @click="handleSearch">{{ $t('Search') }}</UiButton>
      </div>

      <div class="surface-card">
        <UiTable :data="logs" :loading="loading" style="width: 100%">
          <UiTableColumn prop="ID" label="ID" width="80" />
          <UiTableColumn prop="method" label="Method" width="100">
            <template #default="{ row }">
              <UiTag :type="row.status >= 400 ? 'danger' : row.method === 'GET' ? 'success' : 'primary'">
                {{ row.method }}
              </UiTag>
            </template>
          </UiTableColumn>
          <UiTableColumn prop="path" label="Path" min-width="240" />
          <UiTableColumn prop="status" label="Status" width="100" />
          <UiTableColumn label="User" min-width="140">
            <template #default="{ row }">
              {{ row.user?.nickName || row.user?.userName || '-' }}
            </template>
          </UiTableColumn>
          <UiTableColumn prop="createdAt" label="Time" min-width="180" />        </UiTable>
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
import { ElMessage } from '@/ui/feedback'

import { fetchOperationLogs, type OperationLogRecord } from '@/api/logs'
import { t } from '@/i18n'

const methodOptions = ['GET', 'POST', 'PUT', 'DELETE']
const logs = ref<OperationLogRecord[]>([])
const loading = ref(false)
const total = ref(0)
const page = ref(1)
const pageSize = ref(10)
const filters = reactive({
  method: '',
  path: ''
})
const statusModel = ref<number | undefined>()
const errorCount = computed(() => logs.value.filter((item) => item.status >= 400).length)

async function loadLogs() {
  loading.value = true
  try {
    const result = await fetchOperationLogs({
      page: page.value,
      pageSize: pageSize.value,
      method: filters.method,
      path: filters.path,
      status: statusModel.value
    })
    logs.value = result.list
    total.value = result.total
  } catch {
    ElMessage.error(t('Failed to load operation logs'))
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
