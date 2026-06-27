<template>
  <div class="admin-page">
    <section class="page-hero">
      <div class="page-hero-main">
        <span class="page-hero-kicker">{{ $t('Menu structure') }}</span>
        <h2 class="page-hero-title">{{ $t('Menus') }}</h2>
        <p class="page-hero-subtitle">{{ $t('Manage menu hierarchy, visibility, and role bindings.') }}</p>

        <div class="page-metrics">
          <div class="page-metric">
            <div class="page-metric-label">{{ $t('Total menus') }}</div>
            <div class="page-metric-value">{{ total }}</div>
          </div>
          <div class="page-metric">
            <div class="page-metric-label">{{ $t('Top-level menus') }}</div>
            <div class="page-metric-value">{{ rootMenuCount }}</div>
          </div>
          <div class="page-metric">
            <div class="page-metric-label">{{ $t('Hidden menus') }}</div>
            <div class="page-metric-value">{{ hiddenMenuCount }}</div>
          </div>
        </div>
      </div>

    </section>

    <section class="page-panel">
      <div class="page-panel-header">
        <div>
          <h3 class="page-panel-title">{{ $t('Menu structure') }}</h3>
          <p class="page-panel-subtitle">{{ $t('Edit common menu fields.') }}</p>
        </div>
        <div class="page-panel-actions">
          <UiButton @click="loadData" :loading="loading">{{ $t('Refresh') }}</UiButton>
          <UiButton type="primary" @click="openCreateDialog">{{ $t('New') }}</UiButton>
        </div>
      </div>

      <div class="surface-card">
        <UiTable
          :data="navigationMenus"
          row-key="ID"
          default-expand-all
          :loading="loading"
          style="width: 100%"
        >
          <UiTableColumn prop="ID" label="ID" width="80" />
          <UiTableColumn label="Title" min-width="180">
            <template #default="{ row }">
              {{ row.meta?.title || row.name }}
            </template>
          </UiTableColumn>
          <UiTableColumn prop="path" label="Path" min-width="140" />
          <UiTableColumn prop="name" label="Name" min-width="140" />
          <UiTableColumn prop="component" label="Component" min-width="220" />
          <UiTableColumn prop="sort" label="Sort" width="90" />
          <UiTableColumn label="Hidden" width="90">
            <template #default="{ row }">
              <UiTag :type="row.hidden ? 'info' : 'success'">
                {{ $t(row.hidden ? 'Hidden' : 'Visible') }}
              </UiTag>
            </template>
          </UiTableColumn>
          <UiTableColumn label="Actions" width="280">
            <template #default="{ row }">
              <UiButton link type="primary" @click="openEditDialog(row)">{{ $t('Edit') }}</UiButton>
              <UiButton link @click="openRoleDialog(row)">{{ $t('Assign roles') }}</UiButton>
              <UiButton link type="danger" @click="handleDelete(row)">{{ $t('Delete') }}</UiButton>
            </template>
          </UiTableColumn>
        </UiTable>
      </div>
    </section>

    <UiDialog
      v-model="dialogVisible"
      :title="dialogMode === 'create' ? 'New menu' : 'Edit menu'"
      width="620px"
    >
      <UiForm labelWidth="110px" @submit.prevent="submitMenu">
        <UiFormItem label="Parent menu item">
          <UiSelect v-model="form.parentId" class="w-full">
            <UiOption :value="0" label="Top-level menus" />
            <UiOption
              v-for="item in menuOptions"
              :key="item.ID"
              :label="`${item.meta?.title || item.name} (${item.name})`"
              :value="item.ID"
            />
          </UiSelect>
        </UiFormItem>
        <UiFormItem label="Path">
          <UiInput v-model="form.path" placeholder="users" />
        </UiFormItem>
        <UiFormItem label="Name">
          <UiInput v-model="form.name" placeholder="users" />
        </UiFormItem>
        <UiFormItem label="Title">
          <UiInput v-model="form.meta.title" placeholder="Users" />
        </UiFormItem>
        <UiFormItem label="Component">
          <UiInput v-model="form.component" placeholder="view/users/index.vue" />
        </UiFormItem>
        <UiFormItem label="Sort">
          <UiInputNumber v-model="form.sort" :min="0" :precision="0" class="w-full" />
        </UiFormItem>
        <UiFormItem label="Icon">
          <UiInput v-model="form.meta.icon" placeholder="setting" />
        </UiFormItem>
        <UiFormItem label="Active route">
          <UiInput v-model="form.meta.activeName" placeholder="users" />
        </UiFormItem>
        <UiFormItem label="Transition">
          <UiInput v-model="form.meta.transitionType" placeholder="fade" />
        </UiFormItem>
        <UiFormItem label="Permission ID">
          <UiInputNumber v-model="form.permissionId" :min="0" :precision="0" class="w-full" />
        </UiFormItem>
        <UiFormItem label="Display settings">
          <div class="switch-group">
            <UiSwitch v-model="form.hidden" active-text="Hidden" inactive-text="Visible" />
            <UiSwitch
              v-model="form.meta.keepAlive"
              active-text="Cache"
              inactive-text="No cache"
            />
            <UiSwitch
              v-model="form.meta.defaultMenu"
              active-text="Default menu"
              inactive-text="Normal menu"
            />
          </div>
        </UiFormItem>
      </UiForm>

      <template #footer>
        <UiButton @click="dialogVisible = false">{{ $t('Cancel') }}</UiButton>
        <UiButton type="primary" :loading="submitting" @click="submitMenu">{{ $t('Save') }}</UiButton>
      </template>
    </UiDialog>

    <UiDialog v-model="roleDialogVisible" title="Assign roles" width="520px">
      <UiSelect
        v-model="selectedAuthorityIds"
        multiple
        filterable
        class="w-full"
        placeholder="Select visible roles"
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
        <UiButton type="primary" :loading="roleSubmitting" @click="submitMenuRoles">
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
import { usePageChrome } from '@/composables/usePageChrome'
import {
  createMenu,
  deleteMenu,
  fetchMenuList,
  fetchMenuRoles,
  setMenuRoles,
  updateMenu,
  type MenuRecord
} from '@/api/menus'
import { t } from '@/i18n'

type DialogMode = 'create' | 'edit'

const menus = ref<MenuRecord[]>([])
const authorities = ref<AuthorityRecord[]>([])
const loading = ref(false)
const dialogVisible = ref(false)
const dialogMode = ref<DialogMode>('create')
const submitting = ref(false)
const roleDialogVisible = ref(false)
const roleSubmitting = ref(false)
const selectedMenu = ref<MenuRecord | null>(null)
const selectedAuthorityIds = ref<number[]>([])
const form = reactive(createEmptyMenu())

const navigationMenus = computed(() => filterNavigationMenus(menus.value))
const menuOptions = computed(() => flattenMenus(navigationMenus.value))
const authorityOptions = computed(() => flattenAuthorities(authorities.value))
const { total } = usePageChrome(menus, 'menus')
const rootMenuCount = computed(() => menuOptions.value.filter((item) => item.parentId === 0).length)
const hiddenMenuCount = computed(() => menuOptions.value.filter((item) => item.hidden).length)

function createEmptyMenu(): MenuRecord {
  return {
    ID: 0,
    parentId: 0,
    path: '',
    name: '',
    hidden: false,
    component: '',
    sort: 0,
    meta: {
      activeName: '',
      keepAlive: false,
      defaultMenu: false,
      title: '',
      icon: '',
      closeTab: false,
      transitionType: ''
    },
    parameters: [],
    menuBtn: [],
    permissionId: null,
    children: []
  }
}

function cloneMenu(menu: MenuRecord): MenuRecord {
  return {
    ID: menu.ID,
    parentId: menu.parentId,
    path: menu.path,
    name: menu.name,
    hidden: menu.hidden,
    component: menu.component,
    sort: menu.sort,
    meta: { ...menu.meta },
    parameters: menu.parameters.map((item) => ({ ...item })),
    menuBtn: menu.menuBtn.map((item) => ({ ...item })),
    permission: menu.permission,
    permissionId: menu.permissionId,
    method: menu.method,
    apiPath: menu.apiPath,
    menuType: menu.menuType,
    children: []
  }
}

function assignForm(next: MenuRecord) {
  Object.assign(form, next)
  form.meta = { ...next.meta }
  form.parameters = next.parameters.map((item) => ({ ...item }))
  form.menuBtn = next.menuBtn.map((item) => ({ ...item }))
  form.children = []
}

function filterNavigationMenus(list: MenuRecord[]): MenuRecord[] {
  return list
    .filter((item) => item.menuType !== 'action')
    .map((item) => ({
      ...item,
      children: filterNavigationMenus(item.children || [])
    }))
}

function flattenMenus(list: MenuRecord[]): MenuRecord[] {
  return list.flatMap((item) => [item, ...flattenMenus(item.children || [])])
}

function flattenAuthorities(list: AuthorityRecord[]): AuthorityRecord[] {
  return list.flatMap((item) => [item, ...flattenAuthorities(item.children || [])])
}

async function loadData() {
  loading.value = true
  try {
    const [menuList, authorityList] = await Promise.all([fetchMenuList(), fetchAuthorities()])
    menus.value = menuList
    authorities.value = authorityList
  } catch {
    ElMessage.error(t('Failed to load menus'))
  } finally {
    loading.value = false
  }
}

function openCreateDialog() {
  dialogMode.value = 'create'
  assignForm(createEmptyMenu())
  dialogVisible.value = true
}

function openEditDialog(menu: MenuRecord) {
  dialogMode.value = 'edit'
  assignForm(cloneMenu(menu))
  dialogVisible.value = true
}

async function submitMenu() {
  if (!form.name.trim() || !form.path.trim() || !form.meta.title.trim() || !form.component.trim()) {
    ElMessage.warning(t('Please complete menu information'))
    return
  }

  submitting.value = true
  try {
    const payload = cloneMenu(form)
    const response =
      dialogMode.value === 'create' ? await createMenu(payload) : await updateMenu(payload)

    if (response.code === 'OK') {
      ElMessage.success(t(dialogMode.value === 'create' ? 'Menu created' : 'Menu updated'))
      dialogVisible.value = false
      await loadData()
      return
    }

    ElMessage.error(response.message || t('Failed to save menu'))
  } catch {
    ElMessage.error(t('Failed to save menu'))
  } finally {
    submitting.value = false
  }
}

async function openRoleDialog(menu: MenuRecord) {
  selectedMenu.value = menu
  roleDialogVisible.value = true
  try {
    const selection = await fetchMenuRoles(menu.ID)
    selectedAuthorityIds.value = selection.roleIds
  } catch {
    ElMessage.error(t('Failed to load menu roles'))
  }
}

async function submitMenuRoles() {
  if (!selectedMenu.value) return

  roleSubmitting.value = true
  try {
    const response = await setMenuRoles(selectedMenu.value.ID, selectedAuthorityIds.value)
    if (response.code === 'OK') {
      ElMessage.success(t('Menu roles updated'))
      roleDialogVisible.value = false
      return
    }

    ElMessage.error(response.message || t('Failed to save menu roles'))
  } catch {
    ElMessage.error(t('Failed to save menu roles'))
  } finally {
    roleSubmitting.value = false
  }
}

async function handleDelete(menu: MenuRecord) {
  try {
    await ElMessageBox.confirm(t('Delete menu "{name}"?', { name: menu.meta?.title || menu.name }), t('Notice'), {
      type: 'warning'
    })
  } catch {
    return
  }

  try {
    const response = await deleteMenu(menu.ID)
    if (response.code === 'OK') {
      ElMessage.success(t('Menu deleted'))
      await loadData()
      return
    }

    ElMessage.error(response.message || t('Failed to delete menu'))
  } catch {
    ElMessage.error(t('Failed to delete menu'))
  }
}

onMounted(() => {
  loadData()
})
</script>

<style scoped>
.switch-group {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
}

.dialog-alert {
  margin-bottom: 12px;
}

.w-full {
  width: 100%;
}
</style>
