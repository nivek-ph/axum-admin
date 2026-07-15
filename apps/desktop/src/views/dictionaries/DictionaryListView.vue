<template>
  <div class="admin-page">
    <section class="page-hero">
      <div class="page-hero-main">
        <span class="page-hero-kicker">{{ $t('Dictionaries') }}</span>
        <h2 class="page-hero-title">{{ $t('Dictionary management') }}</h2>
        <p class="page-hero-subtitle">{{ $t('Manage dictionaries and dictionary detail trees.') }}</p>

        <div class="page-metrics">
          <div class="page-metric">
            <div class="page-metric-label">{{ $t('Total dictionaries') }}</div>
            <div class="page-metric-value">{{ dictionaries.length }}</div>
          </div>
          <div class="page-metric">
            <div class="page-metric-label">{{ $t('Enabled dictionaries') }}</div>
            <div class="page-metric-value">{{ enabledCount }}</div>
          </div>
          <div class="page-metric">
            <div class="page-metric-label">{{ $t('Current details') }}</div>
            <div class="page-metric-value">{{ detailCount }}</div>
          </div>
        </div>
      </div>

    </section>

    <section class="dictionary-grid">
      <article class="page-panel">
        <div class="page-panel-header dictionary-list-header">
          <div class="dictionary-list-heading">
            <h3 class="page-panel-title">{{ $t('Dictionary list') }}</h3>
            <p class="page-panel-subtitle">{{ $t('Select a dictionary to view details.') }}</p>
          </div>
          <div class="page-panel-actions dictionary-list-actions">
            <UiButton @click="loadDictionaries" :loading="loading">{{ $t('Refresh') }}</UiButton>
            <UiButton type="primary" @click="openCreateDialog">{{ $t('New') }}</UiButton>
          </div>
        </div>

        <div class="page-panel-toolbar inline-filter dictionary-list-filter">
          <UiInput v-model="filters.name" placeholder="Filter by name" clearable />
          <UiButton type="primary" @click="loadDictionaries">{{ $t('Search') }}</UiButton>
        </div>

        <div class="surface-card">
          <UiTable
            :data="dictionaries"
            :loading="loading"
            highlight-current-row
            style="width: 100%"
            @current-change="handleSelectDictionary"
          >
             <UiTableColumn prop="id" label="ID" width="80" />
          <UiTableColumn prop="name" label="Name" min-width="140" />
          <UiTableColumn prop="type" label="Type" min-width="120" />
          <UiTableColumn label="Status" width="100">
            <template #default="{ row }">
                <UiTag :type="row.status ? 'success' : 'info'">
                  {{ $t(row.status ? 'Enabled' : 'Inactive') }}
                </UiTag>
              </template>
            </UiTableColumn>
             <UiTableColumn label="Actions" width="180">
            <template #default="{ row }">
                <UiButton link type="primary" @click.stop="openEditDialog(row)">{{ $t('Edit') }}</UiButton>
                <UiButton link type="danger" @click.stop="handleDelete(row.id)">{{ $t('Delete') }}</UiButton>
              </template>
            </UiTableColumn>
          </UiTable>
        </div>
      </article>

      <article class="page-panel">
        <div class="page-panel-header">
          <div>
            <h3 class="page-panel-title">{{ $t('Dictionary details') }}</h3>
            <p class="page-panel-subtitle">
              {{ selectedDictionary ? `${selectedDictionary.name} (${selectedDictionary.type})` : $t('Select a dictionary to view details') }}
            </p>
          </div>
        </div>

        <div class="surface-card">
          <UiTable
            :data="details"
            row-key="id"
            default-expand-all
            :loading="detailLoading"
            style="width: 100%"
          >
             <UiTableColumn prop="label" label="Label" min-width="140" />
          <UiTableColumn prop="value" label="Value" min-width="140" />
          <UiTableColumn prop="extend" label="Extra" min-width="140" />
          <UiTableColumn prop="sort" label="Sort" width="80" />
          <UiTableColumn label="Status" width="100">
            <template #default="{ row }">
                <UiTag :type="row.status ? 'success' : 'info'">
                  {{ $t(row.status ? 'Enabled' : 'Inactive') }}
                </UiTag>
              </template>
            </UiTableColumn>
             <UiTableColumn label="Actions" width="240">
            <template #default="{ row }">
                <UiButton link @click="openCreateChildDialog(row)">{{ $t('New child') }}</UiButton>
                <UiButton link type="primary" @click="openDetailDialog(row)">{{ $t('Edit') }}</UiButton>
                <UiButton link type="danger" @click="handleDeleteDetail(row.id)">{{ $t('Delete') }}</UiButton>
              </template>
            </UiTableColumn>
          </UiTable>
        </div>

        <div class="page-panel-toolbar inline-filter detail-actions">
          <UiButton :disabled="!selectedDictionary" @click="openDetailDialog()">{{ $t('New root detail') }}</UiButton>
        </div>
      </article>
    </section>

    <UiDialog v-model="dialogVisible" :title="dialogMode === 'create' ? 'New dictionary' : 'Edit dictionary'" width="560px">
      <UiForm labelWidth="90px" @submit.prevent="submitDictionary">
        <UiFormItem label="Name">
          <UiInput v-model="form.name" placeholder="Example: status dictionary" />
        </UiFormItem>
        <UiFormItem label="Type">
          <UiInput v-model="form.type" placeholder="Example: status" />
        </UiFormItem>
        <UiFormItem label="Status">
          <UiSwitch v-model="statusSwitch" active-text="Enabled" inactive-text="Inactive" />
        </UiFormItem>
        <UiFormItem label="Notes">
          <UiInput v-model="form.desc" type="textarea" :rows="3" placeholder="Dictionary description" />
        </UiFormItem>
      </UiForm>

      <template #footer>
        <UiButton @click="dialogVisible = false">{{ $t('Cancel') }}</UiButton>
        <UiButton type="primary" :loading="submitting" @click="submitDictionary">{{ $t('Save') }}</UiButton>
      </template>
    </UiDialog>

    <UiDialog
      v-model="detailDialogVisible"
      :title="detailDialogMode === 'create' ? 'New detail' : 'Edit detail'"
      width="560px"
    >
      <UiForm labelWidth="100px" @submit.prevent="submitDetail">
        <UiFormItem label="Parent detail">
          <UiSelect v-model="detailForm.parentId" class="w-full" clearable>
            <UiOption :value="null" label="Root detail" />
            <UiOption
              v-for="item in flattenedDetailOptions"
              :key="item.id"
              :label="item.label"
              :value="item.id"
            />
          </UiSelect>
        </UiFormItem>
        <UiFormItem label="Label">
          <UiInput v-model="detailForm.label" placeholder="Example: enabled label" />
        </UiFormItem>
        <UiFormItem label="Value">
          <UiInput v-model="detailForm.value" placeholder="Example: enabled value" />
        </UiFormItem>
        <UiFormItem label="Extra">
          <UiInput v-model="detailForm.extend" placeholder="Extra field" />
        </UiFormItem>
        <UiFormItem label="Sort">
          <UiInputNumber v-model="detailForm.sort" :min="0" :precision="0" class="w-full" />
        </UiFormItem>
        <UiFormItem label="Status">
          <UiSwitch v-model="detailStatusSwitch" active-text="Enabled" inactive-text="Inactive" />
        </UiFormItem>
      </UiForm>

      <template #footer>
        <UiButton @click="detailDialogVisible = false">{{ $t('Cancel') }}</UiButton>
        <UiButton type="primary" :loading="detailSubmitting" @click="submitDetail">{{ $t('Save') }}</UiButton>
      </template>
    </UiDialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { ElMessage, ElMessageBox } from '@/ui/feedback'

import {
  createDictionaryDetail,
  createDictionary,
  deleteDictionaryDetail,
  deleteDictionary,
  fetchDictionaries,
  fetchDictionaryDetails,
  updateDictionaryDetail,
  updateDictionary,
  type DictionaryDetailRecord,
  type DictionaryRecord
} from '@/api/dictionaries'
import { t } from '@/i18n'

type DialogMode = 'create' | 'edit'
type DetailDialogMode = 'create' | 'edit'

const dictionaries = ref<DictionaryRecord[]>([])
const details = ref<DictionaryDetailRecord[]>([])
const selectedDictionary = ref<DictionaryRecord | null>(null)
const loading = ref(false)
const detailLoading = ref(false)
const submitting = ref(false)
const dialogVisible = ref(false)
const dialogMode = ref<DialogMode>('create')
const detailDialogVisible = ref(false)
const detailDialogMode = ref<DetailDialogMode>('create')
const detailSubmitting = ref(false)
const filters = reactive({
  name: ''
})
const form = reactive<DictionaryRecord>({
  id: 0,
  name: '',
  type: '',
  status: true,
  desc: '',
  parentId: null
})
const statusSwitch = ref(true)
const detailStatusSwitch = ref(true)
const detailForm = reactive({
  id: 0,
  label: '',
  value: '',
  extend: '',
  sort: 0,
  sysDictionaryId: 0,
  parentId: null as number | null
})
const enabledCount = computed(() => dictionaries.value.filter((item) => item.status).length)
const detailCount = computed(() => flattenDetails(details.value).length)
const flattenedDetailOptions = computed(() => flattenDetails(details.value))

function flattenDetails(list: DictionaryDetailRecord[]): DictionaryDetailRecord[] {
  return list.flatMap((item) => [item, ...flattenDetails(item.children || [])])
}

function resetForm() {
  form.id = 0
  form.name = ''
  form.type = ''
  form.status = true
  form.desc = ''
  form.parentId = null
  statusSwitch.value = true
}

function resetDetailForm() {
  detailForm.id = 0
  detailForm.label = ''
  detailForm.value = ''
  detailForm.extend = ''
  detailForm.sort = 0
  detailForm.sysDictionaryId = selectedDictionary.value?.id || 0
  detailForm.parentId = null
  detailStatusSwitch.value = true
}

async function loadDictionaries() {
  loading.value = true
  try {
    const list = await fetchDictionaries(filters.name)
    dictionaries.value = list
    if (!selectedDictionary.value && list.length) {
      await handleSelectDictionary(list[0])
    } else if (selectedDictionary.value) {
      const next = list.find((item) => item.id === selectedDictionary.value?.id) || null
      selectedDictionary.value = next
      if (next) {
        await loadDetails(next.id)
      } else {
        details.value = []
      }
    }
  } catch {
    ElMessage.error(t('Failed to load dictionaries'))
  } finally {
    loading.value = false
  }
}

async function loadDetails(sysDictionaryId: number) {
  detailLoading.value = true
  try {
    details.value = await fetchDictionaryDetails(sysDictionaryId)
  } catch {
    ElMessage.error(t('Failed to load dictionary details'))
  } finally {
    detailLoading.value = false
  }
}

async function handleSelectDictionary(item: DictionaryRecord | undefined) {
  if (!item) return
  selectedDictionary.value = item
  await loadDetails(item.id)
}

function openCreateDialog() {
  dialogMode.value = 'create'
  resetForm()
  dialogVisible.value = true
}

function openEditDialog(item: DictionaryRecord) {
  dialogMode.value = 'edit'
  Object.assign(form, item)
  statusSwitch.value = Boolean(item.status)
  dialogVisible.value = true
}

function openDetailDialog(item?: DictionaryDetailRecord) {
  if (!selectedDictionary.value) {
    ElMessage.warning(t('Select a dictionary first'))
    return
  }

  if (item) {
    detailDialogMode.value = 'edit'
    detailForm.id = item.id
    detailForm.label = item.label
    detailForm.value = item.value
    detailForm.extend = item.extend
    detailForm.sort = item.sort
    detailForm.sysDictionaryId = item.sysDictionaryId
    detailForm.parentId = item.parentId ?? null
    detailStatusSwitch.value = Boolean(item.status)
  } else {
    detailDialogMode.value = 'create'
    resetDetailForm()
  }
  detailDialogVisible.value = true
}

function openCreateChildDialog(parent: DictionaryDetailRecord) {
  if (!selectedDictionary.value) {
    ElMessage.warning(t('Select a dictionary first'))
    return
  }

  detailDialogMode.value = 'create'
  resetDetailForm()
  detailForm.parentId = parent.id
  detailDialogVisible.value = true
}

async function submitDictionary() {
  if (!form.name.trim() || !form.type.trim()) {
    ElMessage.warning(t('Please complete dictionary information'))
    return
  }

  submitting.value = true
  try {
    const payload = {
      id: form.id,
      name: form.name.trim(),
      type: form.type.trim(),
      status: statusSwitch.value,
      desc: form.desc.trim(),
      parentId: form.parentId ?? null
    }
    const response =
      dialogMode.value === 'create'
        ? await createDictionary(payload)
        : await updateDictionary(payload)

    if (response.code === 'OK') {
      ElMessage.success(t(dialogMode.value === 'create' ? 'Dictionary created' : 'Dictionary updated'))
      dialogVisible.value = false
      await loadDictionaries()
      return
    }
    ElMessage.error(response.message || t('Failed to save dictionary'))
  } catch {
    ElMessage.error(t('Failed to save dictionary'))
  } finally {
    submitting.value = false
  }
}

async function handleDelete(id: number) {
  try {
    await ElMessageBox.confirm(t('Delete this dictionary?'), t('Notice'), { type: 'warning' })
  } catch {
    return
  }

  try {
    const response = await deleteDictionary(id)
    if (response.code === 'OK') {
      ElMessage.success(t('Dictionary deleted'))
      if (selectedDictionary.value?.id === id) {
        selectedDictionary.value = null
        details.value = []
      }
      await loadDictionaries()
      return
    }
    ElMessage.error(response.message || t('Failed to delete dictionary'))
  } catch {
    ElMessage.error(t('Failed to delete dictionary'))
  }
}

async function submitDetail() {
  if (!selectedDictionary.value) return
  if (!detailForm.label.trim() || !detailForm.value.trim()) {
    ElMessage.warning(t('Please complete detail information'))
    return
  }

  detailSubmitting.value = true
  try {
    const payload = {
      id: detailForm.id,
      label: detailForm.label.trim(),
      value: detailForm.value.trim(),
      extend: detailForm.extend.trim(),
      sort: detailForm.sort,
      sysDictionaryId: selectedDictionary.value.id,
      parentId: detailForm.parentId,
      status: detailStatusSwitch.value
    }

    const response =
      detailDialogMode.value === 'create'
        ? await createDictionaryDetail(payload)
        : await updateDictionaryDetail(payload)

    if (response.code === 'OK') {
      ElMessage.success(t(detailDialogMode.value === 'create' ? 'Dictionary detail created' : 'Dictionary detail updated'))
      detailDialogVisible.value = false
      await loadDetails(selectedDictionary.value.id)
      return
    }
    ElMessage.error(response.message || t('Failed to save dictionary detail'))
  } catch {
    ElMessage.error(t('Failed to save dictionary detail'))
  } finally {
    detailSubmitting.value = false
  }
}

async function handleDeleteDetail(id: number) {
  if (!selectedDictionary.value) return

  try {
    await ElMessageBox.confirm(t('Delete this dictionary detail?'), t('Notice'), { type: 'warning' })
  } catch {
    return
  }

  try {
    const response = await deleteDictionaryDetail(selectedDictionary.value.id, id)
    if (response.code === 'OK') {
      ElMessage.success(t('Dictionary detail deleted'))
      await loadDetails(selectedDictionary.value.id)
      return
    }
    ElMessage.error(response.message || t('Failed to delete dictionary detail'))
  } catch {
    ElMessage.error(t('Failed to delete dictionary detail'))
  }
}

onMounted(() => {
  loadDictionaries()
})
</script>

<style scoped>
.dictionary-grid {
  display: grid;
  grid-template-columns: minmax(0, 1.05fr) minmax(0, 0.95fr);
  gap: 18px;
}

.detail-actions {
  margin-top: 12px;
}

.dictionary-list-header {
  align-items: flex-start;
}

.dictionary-list-heading {
  min-width: 0;
}

.dictionary-list-actions {
  flex: 0 0 auto;
  flex-wrap: nowrap;
}

.w-full {
  width: 100%;
}

@media (max-width: 1100px) {
  .dictionary-grid {
    grid-template-columns: 1fr;
  }

  .dictionary-list-actions {
    flex-wrap: wrap;
  }
}

</style>
