<template>
  <div class="admin-page">
    <section class="page-hero">
      <div class="page-hero-main">
        <span class="page-hero-kicker">{{ $t('File library') }}</span>
        <h2 class="page-hero-title">{{ $t('File management') }}</h2>
        <p class="page-hero-subtitle">{{ $t('Manage uploads and external file URLs with flat metadata.') }}</p>
        <div class="page-metrics">
          <div class="page-metric">
            <div class="page-metric-label">{{ $t('Files on page') }}</div>
            <div class="page-metric-value">{{ files.list.length }}</div>
          </div>
          <div class="page-metric">
            <div class="page-metric-label">{{ $t('Total files') }}</div>
            <div class="page-metric-value">{{ files.total }}</div>
          </div>
        </div>
      </div>
    </section>

    <section class="page-panel">
      <div class="page-panel-header">
        <div>
          <h3 class="page-panel-title">{{ $t('File list') }}</h3>
          <p class="page-panel-subtitle">{{ $t('Extension, tag, and category are independent metadata fields.') }}</p>
        </div>
        <div class="page-panel-actions">
          <UiButton @click="loadData" :loading="loading">{{ $t('Refresh') }}</UiButton>
          <UiButton @click="triggerUpload">{{ $t('Upload') }}</UiButton>
          <UiButton type="primary" @click="openImportDialog">{{ $t('Import URL') }}</UiButton>
        </div>
      </div>

      <div class="page-panel-toolbar inline-filter">
        <UiInput v-model="filters.keyword" placeholder="Filter by name or URL" clearable />
        <UiInput v-model="filters.category" placeholder="Filter by category" clearable />
        <UiButton type="primary" @click="handleSearch">{{ $t('Search') }}</UiButton>
        <UiButton @click="handleReset">{{ $t('Reset') }}</UiButton>
      </div>

      <div class="surface-card">
        <UiTable :data="files.list" :loading="loading" style="width: 100%">
          <UiTableColumn prop="id" label="ID" width="80" />
          <UiTableColumn prop="name" label="Name" min-width="160" />
          <UiTableColumn prop="url" label="URL" min-width="220" />
          <UiTableColumn prop="ext" label="Extension" width="100" />
          <UiTableColumn prop="tag" label="Tag" width="120" />
          <UiTableColumn prop="category" label="Category" width="140" />
          <UiTableColumn prop="updatedAt" label="Updated at" min-width="180" />
          <UiTableColumn label="Actions" width="170">
            <template #default="{ row }">
              <UiButton link type="primary" @click="openRenameDialog(row)">{{ $t('Rename') }}</UiButton>
              <UiButton link type="danger" @click="handleDelete(row.id)">{{ $t('Delete') }}</UiButton>
            </template>
          </UiTableColumn>
        </UiTable>
      </div>

      <div class="pagination">
        <UiPagination
          background
          layout="total, prev, pager, next"
          :total="files.total"
          :current-page="page"
          :page-size="pageSize"
          @current-change="handlePageChange"
        />
      </div>
    </section>

    <UiDialog v-model="importDialogVisible" title="Import URL" width="560px">
      <UiForm labelWidth="90px" @submit.prevent="submitImport">
        <UiFormItem label="Name"><UiInput v-model="importForm.name" /></UiFormItem>
        <UiFormItem label="URL"><UiInput v-model="importForm.url" /></UiFormItem>
        <UiFormItem label="Tag"><UiInput v-model="importForm.tag" /></UiFormItem>
        <UiFormItem label="Category"><UiInput v-model="importForm.category" /></UiFormItem>
      </UiForm>
      <template #footer>
        <UiButton @click="importDialogVisible = false">{{ $t('Cancel') }}</UiButton>
        <UiButton type="primary" :loading="submitting" @click="submitImport">{{ $t('Import') }}</UiButton>
      </template>
    </UiDialog>

    <UiDialog v-model="renameDialogVisible" title="Rename file" width="520px">
      <UiForm labelWidth="90px" @submit.prevent="submitRename">
        <UiFormItem label="Name"><UiInput v-model="renameForm.name" /></UiFormItem>
      </UiForm>
      <template #footer>
        <UiButton @click="renameDialogVisible = false">{{ $t('Cancel') }}</UiButton>
        <UiButton type="primary" :loading="submitting" @click="submitRename">{{ $t('Save') }}</UiButton>
      </template>
    </UiDialog>

    <input ref="fileInput" type="file" class="hidden-input" @change="handleFileSelected" />
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'

import { deleteFile, fetchFiles, importFileUrl, renameFile, uploadFile, type FileListResult, type FileRecord } from '@/api/files'
import { getApiErrorMessage } from '@/api/http'
import { t } from '@/i18n'
import { ElMessage, ElMessageBox } from '@/ui/feedback'

const loading = ref(false)
const submitting = ref(false)
const page = ref(1)
const pageSize = ref(10)
const fileInput = ref<HTMLInputElement | null>(null)
const importDialogVisible = ref(false)
const renameDialogVisible = ref(false)
const files = reactive<FileListResult>({ list: [], total: 0, page: 1, pageSize: 10 })
const filters = reactive({ keyword: '', category: '' })
const importForm = reactive({ name: '', url: '', tag: '', category: '' })
const renameForm = reactive({ id: 0, name: '' })

async function loadData() {
  loading.value = true
  try {
    Object.assign(files, await fetchFiles({
      page: page.value,
      pageSize: pageSize.value,
      keyword: filters.keyword,
      category: filters.category
    }))
  } catch (error) {
    ElMessage.error(getApiErrorMessage(error, t('Failed to load files')))
  } finally {
    loading.value = false
  }
}

function handleSearch() {
  page.value = 1
  loadData()
}

function handleReset() {
  filters.keyword = ''
  filters.category = ''
  handleSearch()
}

function handlePageChange(nextPage: number) {
  page.value = nextPage
  loadData()
}

function openImportDialog() {
  Object.assign(importForm, { name: '', url: '', tag: '', category: filters.category })
  importDialogVisible.value = true
}

async function submitImport() {
  if (!importForm.name.trim() || !importForm.url.trim()) {
    ElMessage.warning(t('Name and URL are required'))
    return
  }
  submitting.value = true
  try {
    await importFileUrl({ ...importForm })
    importDialogVisible.value = false
    ElMessage.success(t('File imported'))
    await loadData()
  } catch (error) {
    ElMessage.error(getApiErrorMessage(error, t('Failed to import file')))
  } finally {
    submitting.value = false
  }
}

function openRenameDialog(file: FileRecord) {
  renameForm.id = file.id
  renameForm.name = file.name
  renameDialogVisible.value = true
}

async function submitRename() {
  if (!renameForm.name.trim()) return
  submitting.value = true
  try {
    await renameFile({ id: renameForm.id, name: renameForm.name.trim() })
    renameDialogVisible.value = false
    ElMessage.success(t('File renamed'))
    await loadData()
  } catch (error) {
    ElMessage.error(getApiErrorMessage(error, t('Failed to rename file')))
  } finally {
    submitting.value = false
  }
}

async function handleDelete(id: number) {
  try {
    await ElMessageBox.confirm(t('Delete this file?'), t('Notice'), { type: 'warning' })
    await deleteFile(id)
    ElMessage.success(t('File deleted'))
    await loadData()
  } catch (error) {
    if (error === 'cancel' || error === 'close') return
    ElMessage.error(getApiErrorMessage(error, t('Failed to delete file')))
  }
}

function triggerUpload() {
  fileInput.value?.click()
}

async function handleFileSelected(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  loading.value = true
  try {
    await uploadFile(file, { category: filters.category })
    ElMessage.success(t('File uploaded'))
    await loadData()
  } catch (error) {
    ElMessage.error(getApiErrorMessage(error, t('Failed to upload file')))
  } finally {
    loading.value = false
    input.value = ''
  }
}

onMounted(loadData)
</script>

<style scoped>
.hidden-input {
  display: none;
}

.pagination {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}
</style>
