<template>
  <div class="admin-page">
    <section class="page-hero">
      <div class="page-hero-main">
        <span class="page-hero-kicker">Audit Trail</span>
        <h2 class="page-hero-title">{{ $t('Audit events') }}</h2>
        <p class="page-hero-subtitle">{{ $t('Review business and security decisions in one structured trail.') }}</p>

        <div class="page-metrics">
          <div class="page-metric">
            <div class="page-metric-label">{{ $t('Rows on page') }}</div>
            <div class="page-metric-value">{{ events.length }}</div>
          </div>
          <div class="page-metric">
            <div class="page-metric-label">{{ $t('Total events') }}</div>
            <div class="page-metric-value">{{ total }}</div>
          </div>
          <div class="page-metric">
            <div class="page-metric-label">{{ $t('Denied or failed') }}</div>
            <div class="page-metric-value">{{ unsuccessfulCount }}</div>
          </div>
        </div>
      </div>
    </section>

    <section class="page-panel">
      <div class="page-panel-header">
        <div>
          <h3 class="page-panel-title">{{ $t('Structured audit trail') }}</h3>
          <p class="page-panel-subtitle">{{ $t('Filter by actor, action, resource, or outcome.') }}</p>
        </div>
        <div class="page-panel-actions">
          <UiButton @click="loadEvents" :loading="loading">{{ $t('Refresh') }}</UiButton>
        </div>
      </div>

      <div class="page-panel-toolbar inline-filter audit-event-filter">
        <UiInput v-model="filters.actor" :placeholder="$t('Filter by actor')" clearable />
        <UiSelect v-model="filters.action" clearable :placeholder="$t('Action')">
          <UiOption v-for="action in actionOptions" :key="action" :label="action" :value="action" />
        </UiSelect>
        <UiSelect v-model="filters.resourceType" clearable :placeholder="$t('Resource')">
          <UiOption v-for="resource in resourceOptions" :key="resource" :label="resource" :value="resource" />
        </UiSelect>
        <UiInput v-model="filters.resourceId" :placeholder="$t('Resource ID')" clearable />
        <UiSelect v-model="filters.result" clearable :placeholder="$t('Result')">
          <UiOption label="Succeeded" value="succeeded" />
          <UiOption label="Denied" value="denied" />
          <UiOption label="Failed" value="failed" />
        </UiSelect>
        <UiDateTimePicker v-model="filters.startedAt" class="audit-time-filter" label="Start time (UTC)" />
        <UiDateTimePicker v-model="filters.endedAt" class="audit-time-filter" label="End time (UTC)" />
        <UiButton type="primary" @click="handleSearch">{{ $t('Search') }}</UiButton>
      </div>

      <div class="surface-card">
        <UiTable :data="events" :loading="loading" style="width: 100%">
          <UiTableColumn prop="id" label="ID" width="80" />
          <UiTableColumn prop="actorLabel" :label="$t('Actor')" min-width="130" />
          <UiTableColumn prop="action" :label="$t('Action')" min-width="180" />
          <UiTableColumn :label="$t('Resource')" min-width="150">
            <template #default="{ row }">
              {{ row.resourceType }}<span v-if="row.resourceId">:{{ row.resourceId }}</span>
            </template>
          </UiTableColumn>
          <UiTableColumn :label="$t('Result')" width="110">
            <template #default="{ row }">
              <UiTag :type="resultTagType(row.result)">{{ row.result }}</UiTag>
            </template>
          </UiTableColumn>
          <UiTableColumn prop="reasonCode" :label="$t('Reason')" min-width="170" />
          <UiTableColumn prop="createdAt" :label="$t('Time')" min-width="180" />
          <UiTableColumn :label="$t('Actions')" width="100">
            <template #default="{ row }">
              <UiButton link type="primary" @click="openDetail(row.id)">{{ $t('View') }}</UiButton>
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

    <UiDialog v-model="detailVisible" :title="$t('Audit event detail')" width="680px">
      <dl v-if="selectedEvent" class="event-detail">
        <dt>{{ $t('Actor') }}</dt>
        <dd>{{ selectedEvent.actorLabel || '-' }}</dd>
        <dt>{{ $t('Action') }}</dt>
        <dd>{{ selectedEvent.action }}</dd>
        <dt>{{ $t('Resource') }}</dt>
        <dd>{{ selectedEvent.resourceType }}<span v-if="selectedEvent.resourceId">:{{ selectedEvent.resourceId }}</span></dd>
        <dt>{{ $t('Result') }}</dt>
        <dd>{{ selectedEvent.result }}</dd>
        <dt>{{ $t('Reason') }}</dt>
        <dd>{{ selectedEvent.reasonCode || '-' }}</dd>
        <dt>{{ $t('Source') }}</dt>
        <dd>{{ selectedEvent.sourceIp || '-' }}</dd>
        <dt>{{ $t('User agent') }}</dt>
        <dd>{{ selectedEvent.userAgent || '-' }}</dd>
        <dt>{{ $t('Changes') }}</dt>
        <dd><pre>{{ formatChanges(selectedEvent.changes) }}</pre></dd>
      </dl>
    </UiDialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { ElMessage } from '@/ui/feedback'

import { fetchAuditEvent, fetchAuditEvents, type AuditEventRecord } from '@/api/logs'
import { t } from '@/i18n'

const actionOptions = ['auth.login', 'auth.access_denied', 'user.assign_roles']
const resourceOptions = ['account', 'route', 'user']
const events = ref<AuditEventRecord[]>([])
const loading = ref(false)
const total = ref(0)
const detailVisible = ref(false)
const selectedEvent = ref<AuditEventRecord | null>(null)
const page = ref(1)
const pageSize = ref(10)
const filters = reactive({
  actor: '',
  action: '',
  resourceType: '',
  resourceId: '',
  result: '',
  startedAt: '',
  endedAt: ''
})
const unsuccessfulCount = computed(() => events.value.filter((event) => event.result !== 'succeeded').length)

function resultTagType(result: AuditEventRecord['result']) {
  if (result === 'succeeded') return 'success'
  if (result === 'denied') return 'warning'
  return 'danger'
}

function formatChanges(changes: unknown[]) {
  return changes.length ? JSON.stringify(changes, null, 2) : '-'
}

async function openDetail(id: number) {
  try {
    selectedEvent.value = await fetchAuditEvent(id)
    detailVisible.value = selectedEvent.value !== null
  } catch {
    ElMessage.error(t('Failed to load audit event detail'))
  }
}

async function loadEvents() {
  loading.value = true
  try {
    const result = await fetchAuditEvents({
      page: page.value,
      pageSize: pageSize.value,
      ...filters
    })
    events.value = result.list
    total.value = result.total
  } catch {
    ElMessage.error(t('Failed to load audit events'))
  } finally {
    loading.value = false
  }
}

function handleSearch() {
  page.value = 1
  loadEvents()
}

function handlePageChange(nextPage: number) {
  page.value = nextPage
  loadEvents()
}

onMounted(loadEvents)
</script>

<style scoped>
.audit-event-filter {
  grid-template-columns: repeat(4, minmax(150px, 1fr));
}

.audit-event-filter > .audit-time-filter {
  flex: 0 1 220px;
  max-width: 220px;
}

@media (max-width: 640px) {
  .audit-event-filter > .audit-time-filter {
    flex-basis: 100%;
    width: 100%;
    max-width: none;
  }
}

.event-detail {
  display: grid;
  grid-template-columns: 110px minmax(0, 1fr);
  gap: 8px 16px;
  margin: 0;
  padding: 12px 24px;
}

.event-detail dt {
  color: var(--ui-text-muted);
}

.event-detail dd {
  margin: 0;
  min-width: 0;
  overflow-wrap: anywhere;
}

.event-detail pre {
  margin: 0;
  white-space: pre-wrap;
}

.pagination {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}
</style>
