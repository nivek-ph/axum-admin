<template>
  <div class="admin-page">
    <section class="role-workspace-header">
      <div>
        <span class="panel-kicker">{{ $t('Role permissions') }}</span>
        <h2 class="workspace-title">{{ $t('Roles') }}</h2>
        <p class="workspace-subtitle">按角色矩阵维护菜单和接口权限。</p>
      </div>
      <div class="workspace-actions">
        <span class="workspace-count">共 {{ total }} 个角色</span>
        <UiButton @click="loadWorkbench" :loading="loading">{{ $t('Refresh list') }}</UiButton>
        <UiButton type="primary" @click="openCreateDialog">{{ $t('New role') }}</UiButton>
      </div>
    </section>

    <section class="role-workbench">
      <aside class="role-sidebar">
        <div class="role-sidebar-header">
          <div>
            <h3 class="role-sidebar-title">角色</h3>
            <p class="role-sidebar-subtitle">{{ authorityOptions.length }} 个角色</p>
          </div>
          <UiButton type="primary" @click="openCreateDialog">+</UiButton>
        </div>

        <UiInput v-model="roleSearch" placeholder="搜索角色名称/ID" />

        <div class="role-list" data-test="role-list">
          <button
            v-for="authority in filteredAuthorityOptions"
            :key="authority.authorityId"
            :class="['role-list-item', selectedAuthorityId === authority.authorityId && 'is-active']"
            type="button"
            @click="selectAuthority(authority)"
          >
            <span class="role-list-main">
              <span class="role-list-name">{{ authority.authorityName }}</span>
              <span class="role-list-meta">ID {{ authority.authorityId }} · {{ authority.defaultRouter || 'dashboard' }}</span>
            </span>
          </button>

          <div v-if="filteredAuthorityOptions.length === 0" class="empty-state">暂无匹配角色</div>
        </div>
      </aside>

      <section class="permission-panel">
        <div class="permission-panel-header">
          <div>
            <p class="panel-kicker">{{ activeTab === 'users' ? '当前角色' : '权限矩阵' }}</p>
            <h3 class="permission-title">
              {{ activeTab === 'users' ? selectedAuthority?.authorityName || '请选择角色' : '全部角色权限' }}
            </h3>
            <p class="permission-subtitle">
              {{
                activeTab === 'users'
                  ? selectedAuthority
                    ? `默认入口：${selectedAuthority.defaultRouter || 'dashboard'}`
                    : '从左侧选择角色后维护成员'
                  : '每一列是一个角色，有权限就勾选。'
              }}
            </p>
          </div>

          <div v-if="selectedAuthority && activeTab === 'users'" class="role-actions">
            <UiButton @click="openEditDialog(selectedAuthority)">{{ $t('Edit') }}</UiButton>
            <UiButton
              type="danger"
              :disabled="selectedAuthority.authorityId === 888"
              @click="handleDelete(selectedAuthority)"
            >
              {{ $t('Delete') }}
            </UiButton>
          </div>
        </div>

        <div v-if="authorityOptions.length > 0" class="permission-tabs">
          <button
            data-test="function-permissions-tab"
            :class="['permission-tab', activeTab === 'menus' && 'is-active']"
            type="button"
            @click="activeTab = 'menus'"
          >
            功能权限
          </button>
          <button
            data-test="api-permissions-tab"
            :class="['permission-tab', activeTab === 'apis' && 'is-active']"
            type="button"
            @click="activeTab = 'apis'"
          >
            接口权限
          </button>
          <button
            data-test="role-users-tab"
            :class="['permission-tab', activeTab === 'users' && 'is-active']"
            type="button"
            @click="activeTab = 'users'"
          >
            角色用户
          </button>
        </div>

        <div v-if="authorityOptions.length === 0" class="empty-state large">暂无角色数据</div>

        <div v-else-if="activeTab === 'menus'" class="permission-content">
          <div class="content-toolbar">
            <div>
              <h4 class="content-title">功能权限</h4>
              <p class="content-subtitle">每一列是一个角色，有权限就勾选。</p>
            </div>
            <UiButton
              v-if="canManageMenuPermissions"
              data-test="save-menu-permissions"
              type="primary"
              :loading="menuSubmitting"
              @click="saveMenuPermissions"
            >
              保存权限
            </UiButton>
          </div>

          <div class="permission-matrix-scroll">
            <table class="permission-matrix">
              <thead>
                <tr>
                  <th class="resource-column">菜单</th>
                  <th class="route-column">路由</th>
                  <th v-for="authority in authorityOptions" :key="authority.authorityId">
                    {{ authority.authorityName }}
                  </th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="menu in flatMenus" :key="menu.ID">
                  <td class="resource-cell" :style="{ '--indent': `${menu.level * 22}px` }">
                    {{ menuTitle(menu) }}
                  </td>
                  <td class="route-cell">{{ menu.path }}</td>
                  <td v-for="authority in authorityOptions" :key="authority.authorityId" class="check-cell">
                    <input
                      :data-test="`menu-permission-${menu.ID}-${authority.authorityId}`"
                      type="checkbox"
                      :disabled="!canManageMenuPermissions"
                      :checked="isMenuRoleChecked(menu.ID, authority.authorityId)"
                      @change="toggleMenuRole(menu.ID, authority.authorityId)"
                    />
                  </td>
                </tr>
              </tbody>
            </table>

            <div v-if="flatMenus.length === 0" class="empty-state">暂无菜单数据</div>
          </div>
        </div>

        <div v-else-if="activeTab === 'apis'" class="permission-content">
          <div class="content-toolbar">
            <div>
              <h4 class="content-title">接口权限</h4>
              <p class="content-subtitle">每一列是一个角色，有权限就勾选。</p>
            </div>
            <UiButton
              v-if="canManageApiPermissions"
              data-test="save-api-permissions"
              type="primary"
              :loading="apiSubmitting"
              @click="saveApiPermissions"
            >
              保存权限
            </UiButton>
          </div>

          <div class="permission-matrix-scroll">
            <table class="permission-matrix">
              <thead>
                <tr>
                  <th class="resource-column">接口</th>
                  <th class="route-column">说明</th>
                  <th v-for="authority in authorityOptions" :key="authority.authorityId">
                    {{ authority.authorityName }}
                  </th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="api in apis" :key="apiPermissionKey(api.path, api.method)">
                  <td class="resource-cell api-resource">
                    <UiTag :type="methodTagType(api.method)">{{ api.method }}</UiTag>
                    <span>{{ api.path }}</span>
                  </td>
                  <td class="route-cell">{{ api.description || api.apiGroup }}</td>
                  <td v-for="authority in authorityOptions" :key="authority.authorityId" class="check-cell">
                    <input
                      :data-test="`api-permission-${api.method}-${api.path}-${authority.authorityId}`"
                      type="checkbox"
                      :disabled="!canManageApiPermissions"
                      :checked="isApiRoleChecked(api, authority.authorityId)"
                      @change="toggleApiRole(api, authority.authorityId)"
                    />
                  </td>
                </tr>
              </tbody>
            </table>

            <div v-if="apis.length === 0" class="empty-state">暂无接口权限</div>
          </div>
        </div>

        <div v-else-if="selectedAuthority" class="permission-content">
          <div class="content-toolbar">
            <div>
              <h4 class="content-title">角色用户</h4>
              <p class="content-subtitle">维护当前角色下的用户成员。</p>
            </div>
            <div class="member-count">
              <span>{{ $t('Selected members') }}</span>
              <strong>{{ selectedUserIds.length }} / {{ userOptions.length }}</strong>
            </div>
          </div>

          <div class="member-tools">
            <UiInput v-model="memberSearch" placeholder="Search users" />
            <UiButton type="primary" :loading="userSubmitting" @click="submitRoleUsers">
              {{ $t('Save members') }}
            </UiButton>
          </div>

          <div class="member-list" data-test="member-list">
            <label
              v-for="user in filteredUserOptions"
              :key="user.ID"
              :class="['member-card', selectedUserIdSet.has(user.ID) && 'is-selected']"
            >
              <input
                class="member-checkbox"
                type="checkbox"
                :checked="selectedUserIdSet.has(user.ID)"
                @change="toggleUserSelection(user.ID)"
              />
              <span class="member-checkmark">
                <span v-if="selectedUserIdSet.has(user.ID)">✓</span>
              </span>
              <span class="member-avatar">{{ userInitial(user) }}</span>
              <span class="member-main">
                <span class="member-name">{{ user.nickName || user.userName }}</span>
                <span class="member-meta">{{ user.userName }}<span v-if="user.email"> · {{ user.email }}</span></span>
              </span>
            </label>

            <div v-if="filteredUserOptions.length === 0" class="empty-state">
              {{ $t(userOptions.length === 0 ? 'No users available' : 'No matching users') }}
            </div>
          </div>
        </div>

        <div v-else class="empty-state large">请先从左侧选择角色</div>
      </section>
    </section>

    <UiDialog
      v-model="dialogVisible"
      :title="dialogMode === 'create' ? 'New role' : 'Edit role'"
      width="520px"
    >
      <UiForm labelWidth="100px" @submit.prevent="submitAuthority">
        <UiFormItem label="Role ID">
          <UiInputNumber
            v-model="form.authorityId"
            :disabled="dialogMode === 'edit'"
            :min="1"
            :precision="0"
            class="w-full"
          />
        </UiFormItem>
        <UiFormItem label="Role name">
          <UiInput v-model="form.authorityName" placeholder="Example: operator admin" />
        </UiFormItem>
        <UiFormItem label="Parent role">
          <UiSelect v-model="form.parentId" class="w-full">
            <UiOption :value="0" label="Top-level role" />
            <UiOption
              v-for="item in authorityOptions"
              :key="item.authorityId"
              :label="`${item.authorityName} (${item.authorityId})`"
              :value="item.authorityId"
            />
          </UiSelect>
        </UiFormItem>
        <UiFormItem label="Default route">
          <UiInput v-model="form.defaultRouter" placeholder="dashboard" />
        </UiFormItem>
      </UiForm>

      <template #footer>
        <UiButton @click="dialogVisible = false">{{ $t('Cancel') }}</UiButton>
        <UiButton type="primary" :loading="submitting" @click="submitAuthority">{{ $t('Save') }}</UiButton>
      </template>
    </UiDialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { ElMessage, ElMessageBox } from '@/ui/feedback'

import {
  createAuthority,
  deleteAuthority,
  fetchAuthorities,
  fetchAuthorityUsers,
  setRoleUsers,
  updateAuthority,
  type AuthorityRecord
} from '@/api/authorities'
import {
  apiPermissionKey,
  fetchApiRoleMatrix,
  fetchApis,
  setApiRoles,
  type ApiRecord
} from '@/api/apis'
import {
  fetchMenuRoleMatrix,
  fetchMenuList,
  setMenuRoles,
  type MenuRecord
} from '@/api/menus'
import { useAuthStore } from '@/stores/auth'
import { usePageChrome } from '@/composables/usePageChrome'
import { fetchUsers, type UserRecord } from '@/api/users'
import { t } from '@/i18n'

type DialogMode = 'create' | 'edit'
type PermissionTab = 'menus' | 'apis' | 'users'
type FlatMenu = MenuRecord & { level: number }

const authorities = ref<AuthorityRecord[]>([])
const menus = ref<MenuRecord[]>([])
const apis = ref<ApiRecord[]>([])
const menuRoleMatrix = ref<Record<number, number[]>>({})
const apiRoleMatrix = ref<Record<string, number[]>>({})
const loading = ref(false)
const accessLoading = ref(false)
const dialogVisible = ref(false)
const dialogMode = ref<DialogMode>('create')
const submitting = ref(false)
const menuSubmitting = ref(false)
const apiSubmitting = ref(false)
const userSubmitting = ref(false)
const selectedAuthorityId = ref<number | null>(null)
const activeTab = ref<PermissionTab>('menus')
const roleSearch = ref('')
const memberSearch = ref('')
const userOptions = ref<UserRecord[]>([])
const selectedUserIds = ref<number[]>([])
const dirtyMenuIds = ref<number[]>([])
const dirtyApiKeys = ref<string[]>([])
const form = reactive({
  authorityId: 0,
  authorityName: '',
  parentId: 0,
  defaultRouter: 'dashboard'
})

const authStore = useAuthStore()
const authorityOptions = computed(() => flattenAuthorities(authorities.value))
const selectedAuthority = computed(
  () => authorityOptions.value.find((item) => item.authorityId === selectedAuthorityId.value) || null
)
const { total } = usePageChrome(authorities, 'roles')
const selectedUserIdSet = computed(() => new Set(selectedUserIds.value))
const flatMenus = computed(() => flattenMenus(menus.value))
const currentAuthorityId = computed(() => authStore.userInfo?.authority?.authorityId || null)
const canManageMenuPermissions = computed(() =>
  canCurrentRoleAccessApi('/api/menus/{id}/roles', 'PUT')
)
const canManageApiPermissions = computed(() =>
  canCurrentRoleAccessApi('/api/routes/roles', 'PUT')
)
const filteredAuthorityOptions = computed(() => {
  const keyword = roleSearch.value.trim().toLowerCase()
  if (!keyword) return authorityOptions.value
  return authorityOptions.value.filter((authority) =>
    [authority.authorityName, authority.authorityId, authority.defaultRouter]
      .filter(Boolean)
      .some((value) => String(value).toLowerCase().includes(keyword))
  )
})
const filteredUserOptions = computed(() => {
  const keyword = memberSearch.value.trim().toLowerCase()
  if (!keyword) return userOptions.value
  return userOptions.value.filter((user) =>
    [user.userName, user.nickName, user.email, user.phone]
      .filter(Boolean)
      .some((value) => String(value).toLowerCase().includes(keyword))
  )
})

function flattenAuthorities(list: AuthorityRecord[]): AuthorityRecord[] {
  return list.flatMap((item) => [item, ...flattenAuthorities(item.children || [])])
}

function flattenMenus(list: MenuRecord[], level = 0): FlatMenu[] {
  return list.flatMap((item) => [
    { ...item, level },
    ...flattenMenus(item.children || [], level + 1)
  ])
}

function resetForm() {
  form.authorityId = 0
  form.authorityName = ''
  form.parentId = 0
  form.defaultRouter = 'dashboard'
}

async function loadWorkbench() {
  loading.value = true
  try {
    const [authorityList, menuTree, menuMatrix, apiResult, apiMatrix] = await Promise.all([
      fetchAuthorities(),
      fetchMenuList(),
      fetchMenuRoleMatrix(),
      fetchApis({ page: 1, pageSize: 500 }),
      fetchApiRoleMatrix()
    ])
    authorities.value = authorityList
    menus.value = menuTree
    menuRoleMatrix.value = menuMatrix
    apis.value = apiResult.list
    apiRoleMatrix.value = apiMatrix
    dirtyMenuIds.value = []
    dirtyApiKeys.value = []

    const stillExists = authorityOptions.value.some((item) => item.authorityId === selectedAuthorityId.value)
    if (!stillExists) {
      selectedAuthorityId.value = authorityOptions.value[0]?.authorityId || null
    }

    if (selectedAuthorityId.value) {
      await loadRoleAccess()
    }
  } catch {
    ElMessage.error(t('Failed to load roles'))
  } finally {
    loading.value = false
  }
}

async function loadRoleAccess() {
  if (!selectedAuthorityId.value) return

  accessLoading.value = true
  try {
    const [users, userIds] = await Promise.all([
      fetchUsers(1, 200),
      fetchAuthorityUsers(selectedAuthorityId.value)
    ])
    userOptions.value = users.list
    selectedUserIds.value = userIds
  } catch {
    ElMessage.error(t('Failed to load role permissions'))
  } finally {
    accessLoading.value = false
  }
}

function selectAuthority(authority: AuthorityRecord) {
  if (selectedAuthorityId.value === authority.authorityId) return
  selectedAuthorityId.value = authority.authorityId
  memberSearch.value = ''
  loadRoleAccess()
}

function openCreateDialog() {
  dialogMode.value = 'create'
  resetForm()
  dialogVisible.value = true
}

function openEditDialog(authority: AuthorityRecord) {
  dialogMode.value = 'edit'
  form.authorityId = authority.authorityId
  form.authorityName = authority.authorityName
  form.parentId = authority.parentId
  form.defaultRouter = authority.defaultRouter || 'dashboard'
  dialogVisible.value = true
}

async function submitAuthority() {
  if (!form.authorityId || !form.authorityName.trim()) {
    ElMessage.warning(t('Please complete role information'))
    return
  }

  submitting.value = true
  try {
    const response =
      dialogMode.value === 'create'
        ? await createAuthority({
            authorityId: form.authorityId,
            authorityName: form.authorityName.trim(),
            parentId: form.parentId
          })
        : await updateAuthority({
            authorityId: form.authorityId,
            authorityName: form.authorityName.trim(),
            parentId: form.parentId,
            defaultRouter: form.defaultRouter.trim() || 'dashboard'
          })

    if (response.code === 'OK') {
      ElMessage.success(t(dialogMode.value === 'create' ? 'Role created' : 'Role updated'))
      dialogVisible.value = false
      selectedAuthorityId.value = form.authorityId
      await loadWorkbench()
      return
    }

    ElMessage.error(response.message || t('Failed to save role'))
  } catch {
    ElMessage.error(t('Failed to save role'))
  } finally {
    submitting.value = false
  }
}

function markDirty<T>(list: T[], value: T) {
  return list.includes(value) ? list : [...list, value]
}

function canCurrentRoleAccessApi(path: string, method: string) {
  const authorityId = currentAuthorityId.value
  if (authorityId === 888) return true
  if (!authorityId) return false
  return (apiRoleMatrix.value[apiPermissionKey(path, method)] || []).includes(authorityId)
}

function isMenuRoleChecked(menuId: number, authorityId: number) {
  return (menuRoleMatrix.value[menuId] || []).includes(authorityId)
}

function toggleMenuRole(menuId: number, authorityId: number) {
  if (!canManageMenuPermissions.value) return

  const current = new Set(menuRoleMatrix.value[menuId] || [])
  if (current.has(authorityId)) {
    current.delete(authorityId)
  } else {
    current.add(authorityId)
  }
  menuRoleMatrix.value = {
    ...menuRoleMatrix.value,
    [menuId]: [...current].sort((a, b) => a - b)
  }
  dirtyMenuIds.value = markDirty(dirtyMenuIds.value, menuId)
}

async function saveMenuPermissions() {
  if (!canManageMenuPermissions.value) return

  if (dirtyMenuIds.value.length === 0) {
    ElMessage.success(t('Role permissions updated'))
    return
  }

  menuSubmitting.value = true
  try {
    await Promise.all(
      dirtyMenuIds.value.map((menuId) => setMenuRoles(menuId, menuRoleMatrix.value[menuId] || []))
    )
    menuRoleMatrix.value = await fetchMenuRoleMatrix()
    dirtyMenuIds.value = []
    ElMessage.success(t('Role permissions updated'))
  } catch {
    ElMessage.error(t('Failed to save permissions'))
  } finally {
    menuSubmitting.value = false
  }
}

function isApiRoleChecked(api: ApiRecord, authorityId: number) {
  return (apiRoleMatrix.value[apiPermissionKey(api.path, api.method)] || []).includes(authorityId)
}

function toggleApiRole(api: ApiRecord, authorityId: number) {
  if (!canManageApiPermissions.value) return

  const key = apiPermissionKey(api.path, api.method)
  const current = new Set(apiRoleMatrix.value[key] || [])
  if (current.has(authorityId)) {
    current.delete(authorityId)
  } else {
    current.add(authorityId)
  }
  apiRoleMatrix.value = {
    ...apiRoleMatrix.value,
    [key]: [...current].sort((a, b) => a - b)
  }
  dirtyApiKeys.value = markDirty(dirtyApiKeys.value, key)
}

async function saveApiPermissions() {
  if (!canManageApiPermissions.value) return

  if (dirtyApiKeys.value.length === 0) {
    ElMessage.success(t('Role permissions updated'))
    return
  }

  apiSubmitting.value = true
  try {
    await Promise.all(
      dirtyApiKeys.value.map((key) => {
        const api = apis.value.find((item) => apiPermissionKey(item.path, item.method) === key)
        if (!api) return Promise.resolve()
        return setApiRoles(api.path, api.method, apiRoleMatrix.value[key] || [])
      })
    )
    apiRoleMatrix.value = await fetchApiRoleMatrix()
    dirtyApiKeys.value = []
    ElMessage.success(t('Role permissions updated'))
  } catch {
    ElMessage.error(t('Failed to save permissions'))
  } finally {
    apiSubmitting.value = false
  }
}

function toggleUserSelection(userId: number) {
  selectedUserIds.value = selectedUserIds.value.includes(userId)
    ? selectedUserIds.value.filter((id) => id !== userId)
    : [...selectedUserIds.value, userId].sort((a, b) => a - b)
}

function userInitial(user: UserRecord) {
  return (user.nickName || user.userName || '?').slice(0, 1).toUpperCase()
}

function menuTitle(menu: MenuRecord) {
  return menu.meta?.title || menu.name || menu.path
}

function methodTagType(method: string) {
  if (method === 'GET') return 'success'
  if (method === 'POST') return 'primary'
  if (method === 'PUT') return 'warning'
  return 'danger'
}

async function submitRoleUsers() {
  if (!selectedAuthorityId.value) return

  userSubmitting.value = true
  try {
    const response = await setRoleUsers(selectedAuthorityId.value, selectedUserIds.value)
    if (response.code === 'OK') {
      ElMessage.success(t('Role members updated'))
      await loadRoleAccess()
      return
    }

    ElMessage.error(response.message || t('Failed to save members'))
  } catch {
    ElMessage.error(t('Failed to save members'))
  } finally {
    userSubmitting.value = false
  }
}

async function handleDelete(authority: AuthorityRecord) {
  try {
    await ElMessageBox.confirm(t('Delete role "{name}"?', { name: authority.authorityName }), t('Notice'), {
      type: 'warning'
    })
  } catch {
    return
  }

  try {
    const response = await deleteAuthority(authority.authorityId)
    if (response.code === 'OK') {
      ElMessage.success(t('Role deleted'))
      if (selectedAuthorityId.value === authority.authorityId) {
        selectedAuthorityId.value = null
      }
      await loadWorkbench()
      return
    }

    ElMessage.error(response.message || t('Failed to delete role'))
  } catch {
    ElMessage.error(t('Failed to delete role'))
  }
}

onMounted(() => {
  loadWorkbench()
})
</script>

<style scoped>
.role-workbench {
  display: grid;
  grid-template-columns: minmax(220px, 260px) minmax(0, 1fr);
  min-height: calc(100vh - 178px);
  overflow: hidden;
  border: 1px solid #e7e5e4;
  border-radius: 18px;
  background: rgba(255, 255, 255, 0.94);
  box-shadow: 0 18px 48px rgba(24, 24, 27, 0.06);
}

.role-workspace-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  border: 1px solid #e7e5e4;
  border-radius: 18px;
  background: rgba(255, 255, 255, 0.94);
  padding: 16px 18px;
  box-shadow: 0 10px 28px rgba(24, 24, 27, 0.04);
}

.workspace-title {
  margin: 4px 0 0;
  color: #18181b;
  font-size: 24px;
  line-height: 1.12;
  font-weight: 780;
}

.workspace-subtitle {
  margin: 6px 0 0;
  color: var(--text-muted);
  font-size: 13px;
}

.workspace-actions {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 10px;
}

.workspace-count {
  color: var(--text-muted);
  font-size: 13px;
  font-weight: 650;
}

.role-sidebar,
.permission-panel {
  background: transparent;
}

.role-sidebar {
  display: grid;
  align-content: start;
  gap: 14px;
  border-right: 1px solid #e7e5e4;
  padding: 18px;
}

.role-sidebar-header,
.permission-panel-header,
.content-toolbar,
.member-tools {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
}

.role-sidebar-title,
.permission-title,
.content-title {
  margin: 0;
  color: #18181b;
  font-weight: 760;
}

.role-sidebar-title {
  font-size: 18px;
}

.role-sidebar-subtitle,
.permission-subtitle,
.content-subtitle,
.panel-kicker {
  margin: 4px 0 0;
  color: var(--text-muted);
  font-size: 13px;
}

.role-list {
  display: grid;
  gap: 6px;
  max-height: 420px;
  overflow-y: auto;
}

.role-list-item {
  display: flex;
  width: 100%;
  border: 1px solid transparent;
  border-radius: 12px;
  background: transparent;
  padding: 12px;
  text-align: left;
  cursor: pointer;
  transition: background 0.16s ease, border-color 0.16s ease;
}

.role-list-item:hover,
.role-list-item.is-active {
  border-color: #d6d3d1;
  background: #f5f5f4;
}

.role-list-item.is-active {
  box-shadow: inset 3px 0 0 #18181b;
}

.role-list-main,
.role-list-name,
.role-list-meta {
  display: block;
  min-width: 0;
}

.role-list-name {
  overflow: hidden;
  color: #18181b;
  font-size: 15px;
  font-weight: 760;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.role-list-meta {
  margin-top: 4px;
  overflow: hidden;
  color: var(--text-muted);
  font-size: 12px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.permission-panel {
  min-width: 0;
  overflow: auto;
}

.permission-panel-header {
  padding: 18px 22px;
}

.permission-title {
  font-size: 24px;
}

.role-actions {
  display: flex;
  gap: 8px;
}

.permission-tabs {
  display: flex;
  gap: 4px;
  border-top: 1px solid #e7e5e4;
  border-bottom: 1px solid #e7e5e4;
  padding: 0 20px;
}

.permission-tab {
  border: 0;
  border-bottom: 2px solid transparent;
  background: transparent;
  color: #52525b;
  padding: 15px 16px 13px;
  font-size: 15px;
  font-weight: 760;
  cursor: pointer;
}

.permission-tab:hover,
.permission-tab.is-active {
  color: #18181b;
}

.permission-tab.is-active {
  border-bottom-color: #18181b;
}

.permission-content {
  display: grid;
  gap: 18px;
  padding: 18px 22px 22px;
}

.content-toolbar {
  align-items: flex-start;
}

.permission-matrix-scroll,
.member-list {
  border: 1px solid #e7e5e4;
  border-radius: 14px;
  overflow: hidden;
  background: #ffffff;
}

.permission-matrix-scroll {
  overflow-x: auto;
}

.permission-matrix {
  width: 100%;
  min-width: 520px;
  border-collapse: separate;
  border-spacing: 0;
}

.permission-matrix th,
.permission-matrix td {
  border-bottom: 1px solid #f1f1f0;
  padding: 13px 14px;
  text-align: left;
  vertical-align: middle;
}

.permission-matrix th {
  position: sticky;
  top: 0;
  z-index: 1;
  background: #f5f5f4;
  color: #52525b;
  font-size: 13px;
  font-weight: 760;
  white-space: nowrap;
}

.permission-matrix th:not(.resource-column):not(.route-column),
.check-cell {
  min-width: 86px;
  text-align: center;
}

.resource-column {
  min-width: 170px;
}

.route-column {
  min-width: 140px;
}

.resource-cell {
  padding-left: calc(14px + var(--indent, 0px));
  color: #18181b;
  font-weight: 700;
  white-space: nowrap;
}

.route-cell {
  accent-color: #18181b;
  overflow: hidden;
  color: var(--text-muted);
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 340px;
}

.check-cell input,
.member-checkbox {
  accent-color: #18181b;
}

.api-resource {
  display: flex;
  align-items: center;
  gap: 10px;
}

.member-tools {
  align-items: stretch;
}

.member-tools :deep(.ui-input) {
  flex: 1;
}

.member-count {
  display: grid;
  justify-items: end;
  gap: 4px;
}

.member-count span {
  color: var(--text-muted);
  font-size: 12px;
}

.member-count strong {
  color: #18181b;
  font-size: 18px;
}

.member-list {
  display: grid;
  gap: 8px;
  max-height: 420px;
  overflow-y: auto;
  border: 0;
  background: transparent;
}

.member-card {
  display: grid;
  grid-template-columns: 22px 34px minmax(0, 1fr);
  align-items: center;
  gap: 12px;
  min-height: 58px;
  border: 1px solid #e7e5e4;
  border-radius: 12px;
  background: #ffffff;
  padding: 10px 12px;
  cursor: pointer;
  transition: border-color 0.16s ease, background 0.16s ease;
}

.member-card:hover,
.member-card.is-selected {
  border-color: #18181b;
  background: #fafaf9;
}

.member-checkbox {
  position: absolute;
  opacity: 0;
  pointer-events: none;
}

.member-checkmark {
  display: grid;
  width: 22px;
  height: 22px;
  place-items: center;
  border: 1px solid #d6d3d1;
  border-radius: 7px;
  color: #ffffff;
  background: #ffffff;
  font-size: 14px;
  font-weight: 800;
}

.member-card.is-selected .member-checkmark {
  border-color: #18181b;
  background: #18181b;
}

.member-avatar {
  display: grid;
  width: 34px;
  height: 34px;
  place-items: center;
  border-radius: 999px;
  background: #f5f5f4;
  color: #27272a;
  font-size: 13px;
  font-weight: 800;
}

.member-main {
  min-width: 0;
}

.member-name,
.member-meta {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.member-name {
  color: #18181b;
  font-size: 14px;
  font-weight: 700;
}

.member-meta {
  margin-top: 2px;
  color: var(--text-muted);
  font-size: 12px;
}

.empty-state {
  border: 1px dashed #d6d3d1;
  border-radius: 12px;
  padding: 20px;
  color: var(--text-muted);
  text-align: center;
}

.empty-state.large {
  margin: 24px;
  padding: 48px;
}

.w-full {
  width: 100%;
}

@media (max-width: 720px) {
  .role-workspace-header {
    align-items: stretch;
    flex-direction: column;
  }

  .workspace-actions {
    justify-content: flex-start;
  }

  .role-workbench {
    grid-template-columns: 1fr;
    min-height: auto;
  }

  .role-sidebar {
    border-right: 0;
    border-bottom: 1px solid #e7e5e4;
  }

  .role-actions,
  .content-toolbar,
  .member-tools {
    align-items: stretch;
    flex-direction: column;
  }
}

@media (min-width: 721px) and (max-width: 1180px) {
  .role-workbench {
    grid-template-columns: minmax(170px, 190px) minmax(0, 1fr);
  }

  .role-sidebar {
    padding: 14px;
  }

  .role-list-item {
    padding: 10px;
  }

  .route-column,
  .route-cell {
    display: none;
  }

  .permission-matrix {
    min-width: 0;
    table-layout: fixed;
  }

  .permission-matrix-scroll {
    overflow-x: hidden;
  }

  .permission-matrix th,
  .permission-matrix td {
    padding: 11px 8px;
  }

  .permission-matrix th:not(.resource-column):not(.route-column),
  .check-cell {
    width: 78px;
    min-width: 78px;
  }

  .permission-matrix th:not(.resource-column):not(.route-column) {
    white-space: normal;
    word-break: break-word;
  }

  .resource-column {
    min-width: 0;
    width: auto;
  }

  .resource-cell {
    white-space: normal;
    word-break: break-word;
  }

  .api-resource {
    align-items: flex-start;
    flex-direction: column;
    gap: 5px;
  }
}
</style>
