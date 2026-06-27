<template>
  <div class="admin-page">
    <section class="page-hero">
      <div class="page-hero-main">
        <span class="page-hero-kicker">{{ $t('API directory') }}</span>
        <h2 class="page-hero-title">{{ $t('API directory') }}</h2>
        <p class="page-hero-subtitle">{{ $t('Review registered backend endpoints.') }}</p>

        <div class="page-metrics">
          <div class="page-metric">
            <div class="page-metric-label">{{ $t('Rows on page') }}</div>
            <div class="page-metric-value">{{ apis.length }}</div>
          </div>
          <div class="page-metric">
            <div class="page-metric-label">{{ $t('Total APIs') }}</div>
            <div class="page-metric-value">{{ total }}</div>
          </div>
          <div class="page-metric">
            <div class="page-metric-label">{{ $t('API group') }}</div>
            <div class="page-metric-value">{{ apiGroups.length }}</div>
          </div>
        </div>
      </div>

    </section>

    <section class="page-panel">
      <div class="page-panel-header">
        <div>
          <h3 class="page-panel-title">{{ $t('API directory') }}</h3>
          <p class="page-panel-subtitle">{{ $t('Filter by path, description, group, and method.') }}</p>
        </div>
        <div class="page-panel-actions">
          <UiButton @click="loadApis" :loading="loading">{{ $t('Refresh') }}</UiButton>
          <UiButton v-if="canCreateApi" type="primary" @click="openCreateDialog">{{ $t('New') }}</UiButton>
        </div>
      </div>

      <div class="page-panel-toolbar inline-filter">
        <UiInput v-model="filters.path" placeholder="Filter by path" clearable />
        <UiInput v-model="filters.description" placeholder="Filter by description" clearable />
        <UiSelect v-model="filters.apiGroup" clearable placeholder="Group">
          <UiOption v-for="group in apiGroups" :key="group" :label="group" :value="group" />
        </UiSelect>
        <UiSelect v-model="filters.method" clearable placeholder="Method">
          <UiOption v-for="item in methodOptions" :key="item" :label="item" :value="item" />
        </UiSelect>
        <UiButton type="primary" @click="handleSearch">{{ $t('Search') }}</UiButton>
      </div>

      <div class="surface-card">
        <UiTable :data="apis" :loading="loading" style="width: 100%">
          <UiTableColumn prop="ID" label="ID" width="80" />
          <UiTableColumn prop="method" label="Method" width="100">
            <template #default="{ row }">
              <UiTag :type="methodTagType(row.method)">{{ row.method }}</UiTag>
            </template>
          </UiTableColumn>
          <UiTableColumn prop="path" label="Path" min-width="260" />
          <UiTableColumn prop="description" label="Description" min-width="180" />
          <UiTableColumn prop="apiGroup" label="Group" min-width="120" />
          <UiTableColumn v-if="hasApiActions" label="Actions" width="260">
            <template #default="{ row }">
              <UiButton v-if="canUpdateApi" link type="primary" @click="openEditDialog(row)">{{ $t('Edit') }}</UiButton>
              <UiButton v-if="canAssignApiRoles" link @click="openRoleDialog(row)">{{ $t('Assign roles') }}</UiButton>
              <UiButton v-if="canDeleteApi" link type="danger" @click="handleDelete(row)">{{ $t('Delete') }}</UiButton>
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

    <UiDialog
      v-model="dialogVisible"
      :title="dialogMode === 'create' ? 'New API' : 'Edit API'"
      width="560px"
    >
      <UiForm labelWidth="100px" @submit.prevent="submitApi">
        <UiFormItem label="Request method">
          <UiSelect v-model="form.method" class="w-full">
            <UiOption v-for="item in methodOptions" :key="item" :label="item" :value="item" />
          </UiSelect>
        </UiFormItem>
        <UiFormItem label="API path">
          <UiInput v-model="form.path" placeholder="/api/users" />
        </UiFormItem>
        <UiFormItem label="API description">
          <UiInput v-model="form.description" placeholder="Get users" />
        </UiFormItem>
        <UiFormItem label="API group">
          <UiInput v-model="form.apiGroup" placeholder="user" />
        </UiFormItem>
      </UiForm>

      <template #footer>
        <UiButton @click="dialogVisible = false">{{ $t('Cancel') }}</UiButton>
        <UiButton type="primary" :loading="submitting" @click="submitApi">{{ $t('Save') }}</UiButton>
      </template>
    </UiDialog>

    <UiDialog v-model="roleDialogVisible" title="Assign roles" width="520px">
      <div class="dialog-summary">
        {{ selectedApi?.method }} {{ selectedApi?.path }}
      </div>
      <UiSelect
        v-model="selectedAuthorityIds"
        multiple
        filterable
        class="w-full"
        placeholder="Select roles"
      >
        <UiOption
          v-for="authority in authorityOptions"
          :key="authority.authorityId"
          :label="`${authority.authorityName} (${authority.authorityId})`"
          :value="authority.authorityId"
        />
      </UiSelect>

      <template #footer>
        <UiButton @click="roleDialogVisible = false">{{ $t('Cancel') }}</UiButton>
        <UiButton type="primary" :loading="roleSubmitting" @click="submitApiRoles">
          {{ $t('Save roles') }}
        </UiButton>
      </template>
    </UiDialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { ElMessage, ElMessageBox } from '@/ui/feedback'

import { fetchAuthorities, type AuthorityRecord } from '@/api/authorities'
import {
  createApi,
  deleteApi,
  fetchApiGroups,
  fetchApiRoles,
  fetchApis,
  setApiRoles,
  updateApi,
  type ApiRecord
} from '@/api/apis'
import { useAuthStore } from '@/stores/auth'
import { t } from '@/i18n'

type DialogMode = 'create' | 'edit'

const methodOptions = ['GET', 'POST', 'PUT', 'DELETE']
const authStore = useAuthStore()
const authorities = ref<AuthorityRecord[]>([])
const apiGroups = ref<string[]>([])
const apis = ref<ApiRecord[]>([])
const loading = ref(false)
const dialogVisible = ref(false)
const dialogMode = ref<DialogMode>('create')
const submitting = ref(false)
const roleDialogVisible = ref(false)
const roleSubmitting = ref(false)
const selectedApi = ref<ApiRecord | null>(null)
const selectedAuthorityIds = ref<number[]>([])
const total = ref(0)
const page = ref(1)
const pageSize = ref(10)
const filters = reactive({
  path: '',
  description: '',
  apiGroup: '',
  method: ''
})
const form = reactive<ApiRecord>({
  ID: 0,
  path: '',
  description: '',
  apiGroup: '',
  method: 'GET'
})

const authorityOptions = computed(() => flattenAuthorities(authorities.value))
const canCreateApi = computed(() => authStore.can('system:api:create'))
const canUpdateApi = computed(() => authStore.can('system:api:update'))
const canDeleteApi = computed(() => authStore.can('system:api:delete'))
const canAssignApiRoles = computed(
  () => authStore.can('system:api:list-roles') && authStore.can('system:api:assign-roles')
)
const hasApiActions = computed(() => canUpdateApi.value || canAssignApiRoles.value || canDeleteApi.value)

function flattenAuthorities(list: AuthorityRecord[]): AuthorityRecord[] {
  return list.flatMap((item) => [item, ...flattenAuthorities(item.children || [])])
}

function resetForm() {
  form.ID = 0
  form.path = ''
  form.description = ''
  form.apiGroup = ''
  form.method = 'GET'
}

function methodTagType(method: string) {
  if (method === 'GET') return 'success'
  if (method === 'POST') return 'primary'
  if (method === 'PUT') return 'warning'
  return 'danger'
}

async function loadApis() {
  loading.value = true
  try {
    const result = await fetchApis({
      page: page.value,
      pageSize: pageSize.value,
      path: filters.path,
      description: filters.description,
      apiGroup: filters.apiGroup,
      method: filters.method
    })
    apis.value = result.list
    total.value = result.total
  } catch {
    ElMessage.error(t('Failed to load APIs'))
  } finally {
    loading.value = false
  }
}

async function loadBaseData() {
  try {
    const [groups, authorityList] = await Promise.all([fetchApiGroups(), fetchAuthorities()])
    apiGroups.value = groups
    authorities.value = authorityList
  } catch {
    ElMessage.error(t('Failed to load base data'))
  }
}

function handleSearch() {
  page.value = 1
  loadApis()
}

function handlePageChange(nextPage: number) {
  page.value = nextPage
  loadApis()
}

function openCreateDialog() {
  dialogMode.value = 'create'
  resetForm()
  dialogVisible.value = true
}

function openEditDialog(api: ApiRecord) {
  dialogMode.value = 'edit'
  form.ID = api.ID
  form.path = api.path
  form.description = api.description
  form.apiGroup = api.apiGroup
  form.method = api.method
  dialogVisible.value = true
}

async function submitApi() {
  if (!form.path.trim() || !form.description.trim() || !form.apiGroup.trim()) {
    ElMessage.warning(t('Please complete API information'))
    return
  }

  submitting.value = true
  try {
    const payload = {
      ID: form.ID,
      path: form.path.trim(),
      description: form.description.trim(),
      apiGroup: form.apiGroup.trim(),
      method: form.method
    }
    const response =
      dialogMode.value === 'create' ? await createApi(payload) : await updateApi(payload)

    if (response.code === 'OK') {
      ElMessage.success(t(dialogMode.value === 'create' ? 'API created' : 'API updated'))
      dialogVisible.value = false
      await loadBaseData()
      await loadApis()
      return
    }

    ElMessage.error(response.message || t('Failed to save API'))
  } catch {
    ElMessage.error(t('Failed to save API'))
  } finally {
    submitting.value = false
  }
}

async function openRoleDialog(api: ApiRecord) {
  if (!canAssignApiRoles.value) return
  selectedApi.value = api
  roleDialogVisible.value = true
  try {
    const selection = await fetchApiRoles(api.path, api.method)
    selectedAuthorityIds.value = selection.roleIds
  } catch {
    ElMessage.error(t('Failed to load API roles'))
  }
}

async function submitApiRoles() {
  if (!selectedApi.value) return

  roleSubmitting.value = true
  try {
    const response = await setApiRoles(
      selectedApi.value.path,
      selectedApi.value.method,
      selectedAuthorityIds.value
    )
    if (response.code === 'OK') {
      ElMessage.success(t('API roles updated'))
      roleDialogVisible.value = false
      return
    }

    ElMessage.error(response.message || t('Failed to save API roles'))
  } catch {
    ElMessage.error(t('Failed to save API roles'))
  } finally {
    roleSubmitting.value = false
  }
}

async function handleDelete(api: ApiRecord) {
  try {
    await ElMessageBox.confirm(t('Delete API "{name}"?', { name: `${api.method} ${api.path}` }), t('Notice'), {
      type: 'warning'
    })
  } catch {
    return
  }

  try {
    const response = await deleteApi(api.ID)
    if (response.code === 'OK') {
      ElMessage.success(t('API deleted'))
      await loadBaseData()
      await loadApis()
      return
    }

    ElMessage.error(response.message || t('Failed to delete API'))
  } catch {
    ElMessage.error(t('Failed to delete API'))
  }
}

onMounted(async () => {
  await loadBaseData()
  await loadApis()
})
</script>

<style scoped>
.pagination {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}

.dialog-summary {
  margin-bottom: 12px;
  color: #334155;
  font-weight: 600;
}

.w-full {
  width: 100%;
}
</style>
