<template>
  <div class="admin-page">
    <section class="role-workspace-header">
      <div>
        <span class="panel-kicker">{{ $t('Role permissions') }}</span>
        <h2 class="workspace-title">{{ $t('Roles') }}</h2>
        <p class="workspace-subtitle">{{ $t('Manage unified permissions by page and button, shared by frontend and backend.') }}</p>
      </div>
      <div class="workspace-actions">
        <UiButton @click="loadWorkbench" :loading="loading">{{ $t('Refresh') }}</UiButton>
        <UiButton v-if="canCreateRole" data-test="open-create-role" type="primary" @click="openCreateDialog">{{ $t('New') }}</UiButton>
      </div>
    </section>

    <section class="role-workbench">
      <aside class="role-sidebar">
        <div class="role-sidebar-header">
          <div>
            <h3 class="role-sidebar-title">{{ $t('Role') }}</h3>
            <p class="role-sidebar-subtitle">{{ $t('Total {count} roles', { count: roleOptions.length }) }}</p>
          </div>
          <UiButton v-if="canCreateRole" type="primary" @click="openCreateDialog">+</UiButton>
        </div>

        <UiInput v-model="roleSearch" placeholder="Search role name/ID" />

        <div class="role-list" data-test="role-list">
          <button
            v-for="role in filteredRoleOptions"
            :key="role.id"
            :class="['role-list-item', selectedRoleId === role.id && 'is-active']"
            type="button"
            @click="selectRole(role)"
          >
            <span class="role-list-main">
              <span class="role-list-name">{{ role.name }}</span>
              <span class="role-list-meta">ID {{ role.id }} · {{ role.code }}</span>
            </span>
          </button>

          <div v-if="filteredRoleOptions.length === 0" class="empty-state">{{ $t('No matching roles') }}</div>
        </div>
      </aside>

      <section class="permission-panel">
        <div class="permission-panel-header">
          <div>
            <p class="panel-kicker">{{ $t(activeTab === 'users' ? 'Current role' : 'Function permissions') }}</p>
            <h3 class="permission-title">
              {{ selectedRole?.name || $t('Select a role') }}
            </h3>
            <p class="permission-subtitle">
              {{
                activeTab === 'users'
                  ? selectedRole
                    ? selectedRole.code
                    : $t('Select a role from the left to manage members')
                  : selectedRole
                    ? $t('Select page access and operation buttons. Hover to inspect the mapped API.')
                    : $t('Select a role from the left to manage function permissions')
              }}
            </p>
          </div>

          <div v-if="selectedRole && activeTab === 'users'" class="role-actions">
            <UiButton v-if="canUpdateRole" @click="openEditDialog(selectedRole)">{{ $t('Edit') }}</UiButton>
            <UiButton
              v-if="canDeleteRole"
              type="danger"
              :disabled="isSystemRole(selectedRole)"
              @click="handleDelete(selectedRole)"
            >
              {{ $t('Delete') }}
            </UiButton>
          </div>
        </div>

        <div v-if="roleOptions.length > 0" class="permission-tabs">
          <button
            data-test="basic-info-tab"
            :class="['permission-tab', activeTab === 'basic' && 'is-active']"
            type="button"
            @click="switchTab('basic')"
          >
            {{ $t('Basic Info') }}
          </button>
          <button
            data-test="menu-authorization-tab"
            :class="['permission-tab', activeTab === 'menus' && 'is-active']"
            type="button"
            @click="switchTab('menus')"
          >
            {{ $t('Menu Authorization') }}
          </button>
          <button
            data-test="data-scope-tab"
            :class="['permission-tab', activeTab === 'scope' && 'is-active']"
            type="button"
            @click="switchTab('scope')"
          >
            {{ $t('Data Scope') }}
          </button>
          <button
            v-if="canViewRoleUsers"
            data-test="role-users-tab"
            :class="['permission-tab', activeTab === 'users' && 'is-active']"
            type="button"
            @click="switchTab('users')"
          >
            {{ $t('Assigned Users') }}
          </button>
        </div>

        <div v-if="roleOptions.length === 0" class="empty-state large">{{ $t('No role data') }}</div>

        <div v-else-if="activeTab === 'basic' && selectedRole" class="permission-content">
          <div class="role-detail-grid">
            <div class="role-detail-item">
              <span>{{ $t('Role code') }}</span>
              <strong>{{ selectedRole.code }}</strong>
            </div>
            <div class="role-detail-item">
              <span>{{ $t('Status') }}</span>
              <strong>{{ selectedRole.status }}</strong>
            </div>
            <div class="role-detail-item">
              <span>{{ $t('Data Scope') }}</span>
              <strong>{{ dataScopeLabel(selectedRole.data_scope) }}</strong>
            </div>
            <div class="role-detail-item">
              <span>{{ $t('Sort') }}</span>
              <strong>{{ selectedRole.sort }}</strong>
            </div>
          </div>
        </div>

        <div v-else-if="activeTab === 'menus' && selectedRole" class="permission-content">
          <div class="content-toolbar">
            <div>
              <h4 class="content-title">{{ $t('Menu Authorization') }}</h4>
              <p class="content-subtitle">{{ $t('Select page access to include button permissions under that page and avoid visible pages with 403 APIs.') }}</p>
            </div>
            <UiButton
              v-if="canManageFunctionPermissions && !isSuperAdminRole"
              data-test="save-function-permissions"
              type="primary"
              :loading="functionSubmitting"
              @click="saveFunctionPermissions"
            >
              {{ $t('Save permissions') }}
            </UiButton>
          </div>

          <div class="permission-catalog-scroll">
            <table class="permission-catalog">
              <thead>
                <tr>
                  <th class="resource-column">{{ $t('Menu') }}</th>
                  <th class="actions-column">{{ $t('Permission') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="menu in flatMenus" :key="menu.id">
                  <td class="resource-cell" :style="{ '--indent': `${menu.level * 22}px` }">
                    <div class="menu-resource">
                      <span class="menu-resource-title">{{ menuTitle(menu) }}</span>
                      <span class="menu-resource-path">{{ menu.path }}</span>
                    </div>
                  </td>
                  <td class="actions-cell">
                    <div class="permission-actions">
                      <label
                        class="permission-chip"
                        :class="pageAccessChecked(menu) && 'is-checked'"
                        data-test="page-access-chip"
                      >
                        <input
                          :data-test="`menu-permission-${menu.id}-${selectedRole.id}`"
                          type="checkbox"
                          :disabled="isPermissionEditingDisabled"
                          :checked="pageAccessChecked(menu)"
                          @change="onPageAccessChange(menu, $event)"
                        />
                        <span class="permission-chip-label">{{ $t('Page access') }}</span>
                        <span class="permission-chip-tip">{{ $t('Page visible') }} · {{ menu.path }}</span>
                      </label>

                      <label
                        v-for="action in actionsForMenu(menu)"
                        :key="action.id"
                        class="permission-chip"
                        :class="actionAccessChecked(action) && 'is-checked'"
                      >
                        <input
                          :data-test="`action-permission-${action.permission || action.id}-${selectedRole.id}`"
                          type="checkbox"
                          :disabled="isPermissionEditingDisabled"
                          :checked="actionAccessChecked(action)"
                          @change="onActionAccessChange(menu, action, $event)"
                        />
                        <span class="permission-chip-label">{{ actionTitle(action) }}</span>
                        <span class="permission-chip-tip">{{ actionTip(action) }}</span>
                      </label>

                      <span v-if="actionsForMenu(menu).length === 0" class="permission-empty">{{ $t('No button permissions') }}</span>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>

            <div v-if="flatMenus.length === 0" class="empty-state">{{ $t('No menu data') }}</div>
          </div>
        </div>

        <div v-else-if="activeTab === 'scope' && selectedRole" class="permission-content">
          <div class="content-toolbar">
            <div>
              <h4 class="content-title">{{ $t('Data Scope') }}</h4>
              <p class="content-subtitle">{{ $t('Limit what records this role can read or operate on.') }}</p>
            </div>
            <UiButton
              data-test="save-data-scope"
              type="primary"
              :loading="dataScopeSubmitting"
              @click="saveDataScope"
            >
              {{ $t('Save data scope') }}
            </UiButton>
          </div>

          <div class="data-scope-layout">
            <div class="data-scope-options">
              <label
                v-for="option in dataScopeOptions"
                :key="option.value"
                class="data-scope-option"
                :class="selectedDataScope === option.value && 'is-selected'"
              >
                <input
                  :data-test="`data-scope-${option.value}`"
                  type="radio"
                  name="data-scope"
                  :value="option.value"
                  :checked="selectedDataScope === option.value"
                  @change="selectedDataScope = option.value"
                />
                <span>
                  <strong>{{ $t(option.label) }}</strong>
                  <small>{{ $t(option.description) }}</small>
                </span>
              </label>
            </div>

            <div class="dept-scope-panel">
              <h5>{{ $t('Custom departments') }}</h5>
              <div class="dept-scope-list">
                <label
                  v-for="dept in flatDeptOptions"
                  :key="dept.id"
                  class="dept-scope-row"
                  :style="{ '--indent': `${dept.level * 18}px` }"
                >
                  <input
                    :data-test="`role-dept-${dept.id}`"
                    type="checkbox"
                    :disabled="selectedDataScope !== 'custom_depts'"
                    :checked="selectedDeptIdSet.has(dept.id)"
                    @change="toggleDeptScope(dept.id)"
                  />
                  <span>{{ dept.name }}</span>
                  <small>{{ dept.code }}</small>
                </label>

                <div v-if="flatDeptOptions.length === 0" class="empty-state">{{ $t('No department data') }}</div>
              </div>
            </div>
          </div>
        </div>

        <div v-else-if="activeTab === 'users' && selectedRole" class="permission-content">
          <div class="content-toolbar">
            <div>
              <h4 class="content-title">{{ $t('Role users') }}</h4>
              <p class="content-subtitle">{{ $t('Maintain users assigned to the current role.') }}</p>
            </div>
            <div class="member-count">
              <span>{{ $t('Selected members') }}</span>
              <strong>{{ selectedUserIds.length }} / {{ userOptions.length }}</strong>
            </div>
          </div>

          <div class="member-tools">
            <UiInput v-model="memberSearch" placeholder="Search users" />
            <UiButton
              v-if="canAssignRoleUsers"
              type="primary"
              :loading="userSubmitting"
              @click="submitRoleUsers"
            >
              {{ $t('Save members') }}
            </UiButton>
          </div>

          <div class="member-list" data-test="member-list">
            <label
              v-for="user in filteredUserOptions"
              :key="user.id"
              :class="['member-card', selectedUserIdSet.has(user.id) && 'is-selected']"
            >
              <input
                class="member-checkbox"
                type="checkbox"
                :checked="selectedUserIdSet.has(user.id)"
                @change="toggleUserSelection(user.id)"
              />
              <span class="member-checkmark">
                <span v-if="selectedUserIdSet.has(user.id)">✓</span>
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

        <div v-else class="empty-state large">{{ $t('Select a role from the left first') }}</div>
      </section>
    </section>

    <UiDialog
      v-model="dialogVisible"
      :title="dialogMode === 'create' ? 'New role' : 'Edit role'"
      width="520px"
    >
      <UiForm labelWidth="100px" @submit.prevent="submitRole">
        <UiFormItem v-if="dialogMode === 'edit'" label="Role ID">
          <UiInputNumber
            v-model="form.id"
            disabled
            :min="1"
            :precision="0"
            class="w-full"
          />
        </UiFormItem>
        <UiFormItem label="Role name">
          <UiInput v-model="form.name" data-test="role-name-input" placeholder="Example: operator admin" />
        </UiFormItem>
        <UiFormItem label="Role code">
          <UiInput v-model="form.code" data-test="role-code-input" placeholder="operator_admin" />
        </UiFormItem>
      </UiForm>

      <template #footer>
        <UiButton @click="dialogVisible = false">{{ $t('Cancel') }}</UiButton>
        <UiButton data-test="role-dialog-save" type="primary" :loading="submitting" @click="submitRole">{{ $t('Save') }}</UiButton>
      </template>
    </UiDialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { ElMessage, ElMessageBox } from '@/ui/feedback'

import {
  fetchMenuList,
  type MenuRecord
} from '@/api/menus'
import { useAuthStore } from '@/stores/auth'
import { fetchUsers, type UserRecord } from '@/api/users'
import {
  createRole,
  deleteRole,
  getRoleDeptIds,
  getRolePermissionIds,
  getRoleUserIds,
  listRoles,
  setRoleDeptIds,
  setRolePermissionIds,
  setRoleUserIds,
  type RoleResource,
  updateRole
} from '@/api/system/roles'
import { listDepts, type DeptRecord } from '@/api/system/depts'
import { t } from '@/i18n'

type DialogMode = 'create' | 'edit'
type PermissionTab = 'basic' | 'menus' | 'scope' | 'users'
type FlatMenu = MenuRecord & { level: number }
type FlatDept = DeptRecord & { level: number }
type RoleDataScope = 'all' | 'dept' | 'dept_and_children' | 'self' | 'custom_depts'

const roles = ref<RoleResource[]>([])
const menus = ref<MenuRecord[]>([])
const selectedMenuIds = ref<number[]>([])
const loading = ref(false)
const accessLoading = ref(false)
const dialogVisible = ref(false)
const dialogMode = ref<DialogMode>('create')
const submitting = ref(false)
const menuSubmitting = ref(false)
const functionSubmitting = computed(() => menuSubmitting.value)
const userSubmitting = ref(false)
const selectedRoleId = ref<number | null>(null)
const activeTab = ref<PermissionTab>('menus')
const roleSearch = ref('')
const memberSearch = ref('')
const userOptions = ref<UserRecord[]>([])
const selectedUserIds = ref<number[]>([])
const selectedPermissionIds = ref<number[]>([])
const deptTree = ref<DeptRecord[]>([])
const selectedDeptIds = ref<number[]>([])
const selectedDataScope = ref<RoleDataScope>('all')
const permissionsDirty = ref(false)
const dataScopeSubmitting = ref(false)
const form = reactive({
  id: 0,
  name: '',
  code: '',
  status: 'enabled',
  sort: 0,
  dataScope: 'all'
})

const authStore = useAuthStore()
const roleOptions = computed(() => roles.value)
const selectedRole = computed(
  () => roleOptions.value.find((item) => item.id === selectedRoleId.value) || null
)
const selectedUserIdSet = computed(() => new Set(selectedUserIds.value))
const selectedDeptIdSet = computed(() => new Set(selectedDeptIds.value))
const selectedMenuIdSet = computed(() => new Set(selectedMenuIds.value))
const flatMenus = computed(() => flattenMenus(menus.value))
const flatDeptOptions = computed(() => flattenDepts(deptTree.value))
const canCreateRole = computed(() => authStore.can('system:role:create'))
const canUpdateRole = computed(() => authStore.can('system:role:update'))
const canDeleteRole = computed(() => authStore.can('system:role:delete'))
const canAssignRoleUsers = computed(() => authStore.can('system:role:assign-users'))
const canViewRoleUsers = computed(() => authStore.can('system:role:list-users') && authStore.can('system:user:list'))
const canManageFunctionPermissions = computed(() => authStore.can('system:role:update-permission'))
const isSuperAdminRole = computed(
  () => selectedRole.value ? isSystemRole(selectedRole.value) : false
)
const isPermissionEditingDisabled = computed(
  () => !canManageFunctionPermissions.value || isSuperAdminRole.value
)
const dataScopeOptions: Array<{ value: RoleDataScope; label: string; description: string }> = [
  { value: 'all', label: 'All data', description: 'Can access all records.' },
  { value: 'dept', label: 'Own department', description: 'Can access records in the user department.' },
  { value: 'dept_and_children', label: 'Department and children', description: 'Can access records in the user department tree.' },
  { value: 'self', label: 'Self only', description: 'Can access records owned by the current user.' },
  { value: 'custom_depts', label: 'Custom departments', description: 'Can access records in selected departments.' }
]
const filteredRoleOptions = computed(() => {
  const keyword = roleSearch.value.trim().toLowerCase()
  if (!keyword) return roleOptions.value
  return roleOptions.value.filter((role) =>
    [role.name, role.id, role.code]
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

function isSystemRole(role: RoleResource) {
  return role.is_system || role.code === 'super_admin'
}

function flattenMenus(list: MenuRecord[], level = 0): FlatMenu[] {
  return list
    .filter((item) => item.menuType !== 'action')
    .flatMap((item) => [
      { ...item, level },
      ...flattenMenus((item.children || []).filter((child) => child.menuType !== 'action'), level + 1)
    ])
}

function flattenAllMenus(list: MenuRecord[]): MenuRecord[] {
  return list.flatMap((item) => [item, ...flattenAllMenus(item.children || [])])
}

function menuPermissionIdsFor(menuIds: number[]) {
  return [...menuIds]
}

function menuIdsForPermissionIds(permissionIds: number[]) {
  return [...permissionIds].sort((left, right) => left - right)
}

function flattenDepts(list: DeptRecord[], level = 0): FlatDept[] {
  return list.flatMap((item) => [
    { ...item, level },
    ...flattenDepts(item.children || [], level + 1)
  ])
}

function resetForm() {
  form.id = 0
  form.name = ''
  form.code = ''
  form.status = 'enabled'
  form.sort = 0
  form.dataScope = 'all'
}

async function loadWorkbench() {
  loading.value = true
  try {
    const [roleList, menuTree] = await Promise.all([
      listRoles(),
      fetchMenuList()
    ])
    roles.value = roleList
    menus.value = menuTree
    permissionsDirty.value = false

    const stillExists = roleOptions.value.some((item) => item.id === selectedRoleId.value)
    if (!stillExists) {
      selectedRoleId.value = roleOptions.value[0]?.id || null
    } else if (!selectedRoleId.value) {
      selectedRoleId.value = roleOptions.value[0]?.id || null
    }

    await loadActiveRoleTab()
  } catch {
    ElMessage.error(t('Failed to load roles'))
  } finally {
    loading.value = false
  }
}

async function loadRoleAccess() {
  if (!selectedRoleId.value) return

  accessLoading.value = true
  try {
    const [users, userIds] = await Promise.all([
      fetchUsers(1, 200),
      getRoleUserIds(selectedRoleId.value)
    ])
    userOptions.value = users.list
    selectedUserIds.value = userIds
  } catch {
    ElMessage.error(t('Failed to load role permissions'))
  } finally {
    accessLoading.value = false
  }
}

async function loadRoleMenuPermissions() {
  if (!selectedRoleId.value) return

  accessLoading.value = true
  try {
    const permissionIds = await getRolePermissionIds(selectedRoleId.value)
    selectedPermissionIds.value = permissionIds
    selectedMenuIds.value = menuIdsForPermissionIds(permissionIds)
    permissionsDirty.value = false
  } catch {
    ElMessage.error(t('Failed to load role permissions'))
  } finally {
    accessLoading.value = false
  }
}

async function loadRoleDataScope() {
  if (!selectedRoleId.value) return

  accessLoading.value = true
  try {
    const [depts, deptIds] = await Promise.all([
      listDepts(),
      getRoleDeptIds(selectedRoleId.value)
    ])
    deptTree.value = depts
    selectedDeptIds.value = deptIds
    selectedDataScope.value = normalizeDataScope(selectedRole.value?.data_scope)
  } catch {
    ElMessage.error(t('Failed to load data scope'))
  } finally {
    accessLoading.value = false
  }
}

async function loadActiveRoleTab() {
  if (!selectedRoleId.value) return

  if (activeTab.value === 'users' && canViewRoleUsers.value) {
    await loadRoleAccess()
  } else if (activeTab.value === 'menus') {
    await loadRoleMenuPermissions()
  } else if (activeTab.value === 'scope') {
    await loadRoleDataScope()
  }
}

function selectRole(role: RoleResource) {
  if (selectedRoleId.value === role.id) return
  selectedRoleId.value = role.id
  memberSearch.value = ''
  loadActiveRoleTab()
}

function switchTab(tab: PermissionTab) {
  activeTab.value = tab
  loadActiveRoleTab()
}

function openCreateDialog() {
  dialogMode.value = 'create'
  resetForm()
  dialogVisible.value = true
}

function openEditDialog(role: RoleResource) {
  dialogMode.value = 'edit'
  form.id = role.id
  form.name = role.name
  form.code = role.code
  form.status = role.status
  form.sort = role.sort
  form.dataScope = role.data_scope
  dialogVisible.value = true
}

async function submitRole() {
  if (!form.name.trim() || !form.code.trim()) {
    ElMessage.warning(t('Please complete role information'))
    return
  }

  submitting.value = true
  try {
    const response =
      dialogMode.value === 'create'
        ? await createRole({
            code: form.code.trim(),
            name: form.name.trim(),
            status: 'enabled',
            sort: 0,
            data_scope: 'all'
          })
        : await updateRole(form.id, {
            code: form.code.trim(),
            name: form.name.trim(),
            status: form.status,
            sort: form.sort,
            data_scope: form.dataScope
          })

    if (response.code === 'OK') {
      const savedRoleId = dialogMode.value === 'create'
        ? Number(response.data?.role?.id)
        : form.id
      ElMessage.success(t(dialogMode.value === 'create' ? 'Role created' : 'Role updated'))
      dialogVisible.value = false
      selectedRoleId.value = Number.isFinite(savedRoleId) ? savedRoleId : null
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

function actionsForMenu(menu: MenuRecord) {
  return (menu.children || [])
    .filter((child) => child.menuType === 'action')
    .sort((left, right) => left.sort - right.sort || left.id - right.id)
}

function pageAccessChecked(menu: FlatMenu) {
  return selectedMenuIdSet.value.has(menu.id)
}

function actionAccessChecked(action: MenuRecord) {
  return selectedMenuIdSet.value.has(action.id)
}

function setMenuAccess(menuId: number, enabled: boolean, includeDescendants = true) {
  const current = new Set(selectedMenuIds.value)
  const all = flattenAllMenus(menus.value)
  const byId = new Map(all.map((menu) => [menu.id, menu]))
  if (enabled) {
    let node = byId.get(menuId)
    while (node) {
      current.add(node.id)
      node = node.parentId ? byId.get(node.parentId) : undefined
    }
    const addChildren = (id: number) => {
      all.filter((item) => item.parentId === id).forEach((item) => {
        current.add(item.id)
        addChildren(item.id)
      })
    }
    if (includeDescendants) {
      addChildren(menuId)
    }
  } else {
    current.delete(menuId)
    const removeChildren = (id: number) => {
      all.filter((item) => item.parentId === id).forEach((item) => {
        current.delete(item.id)
        removeChildren(item.id)
      })
    }
    removeChildren(menuId)
  }
  selectedMenuIds.value = [...current].sort((left, right) => left - right)
  permissionsDirty.value = true
}

function onPageAccessChange(menu: FlatMenu, event: Event) {
  if (!selectedRoleId.value || isPermissionEditingDisabled.value) return

  const enabled = (event.target as HTMLInputElement).checked
  setMenuAccess(menu.id, enabled)
  for (const action of actionsForMenu(menu)) {
    setMenuAccess(action.id, enabled)
  }
}

function onActionAccessChange(menu: FlatMenu, action: MenuRecord, event: Event) {
  if (!selectedRoleId.value || isPermissionEditingDisabled.value) return

  const enabled = (event.target as HTMLInputElement).checked
  setMenuAccess(action.id, enabled, false)
}

async function saveFunctionPermissions() {
  await saveMenuPermissions()
}

async function saveMenuPermissions(): Promise<boolean> {
  if (!selectedRoleId.value || isPermissionEditingDisabled.value) return false

  if (!permissionsDirty.value) {
    ElMessage.success(t('Role permissions updated'))
    return true
  }

  menuSubmitting.value = true
  try {
    const nextPermissionIds = menuPermissionIdsFor(selectedMenuIds.value).sort((left, right) => left - right)

    await setRolePermissionIds(selectedRoleId.value, nextPermissionIds)
    selectedPermissionIds.value = nextPermissionIds
    selectedMenuIds.value = menuIdsForPermissionIds(nextPermissionIds)
    permissionsDirty.value = false
    ElMessage.success(t('Role permissions updated'))
    return true
  } catch {
    ElMessage.error(t('Failed to save permissions'))
    return false
  } finally {
    menuSubmitting.value = false
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
  return t(menu.meta?.title || menu.name || menu.path)
}

function actionTitle(action: MenuRecord) {
  return t(action.meta?.title || action.permission || action.name)
}

function actionTip(action: MenuRecord) {
  const route = [action.method, action.apiPath].filter(Boolean).join(' ')
  return action.permission ? `${action.permission}${route ? ` · ${route}` : ''}` : route
}

function normalizeDataScope(value?: string): RoleDataScope {
  if (
    value === 'all' ||
    value === 'dept' ||
    value === 'dept_and_children' ||
    value === 'self' ||
    value === 'custom_depts'
  ) {
    return value
  }
  return 'all'
}

function dataScopeLabel(value: string) {
  return t(dataScopeOptions.find((option) => option.value === value)?.label || 'All data')
}

function toggleDeptScope(deptId: number) {
  if (selectedDataScope.value !== 'custom_depts') return

  selectedDeptIds.value = selectedDeptIds.value.includes(deptId)
    ? selectedDeptIds.value.filter((id) => id !== deptId)
    : [...selectedDeptIds.value, deptId].sort((a, b) => a - b)
}

async function saveDataScope() {
  if (!selectedRole.value) return

  dataScopeSubmitting.value = true
  try {
    await updateRole(selectedRole.value.id, {
      code: selectedRole.value.code,
      name: selectedRole.value.name,
      status: selectedRole.value.status,
      sort: selectedRole.value.sort,
      data_scope: selectedDataScope.value
    })
    await setRoleDeptIds(
      selectedRole.value.id,
      selectedDataScope.value === 'custom_depts' ? [...selectedDeptIds.value].sort((a, b) => a - b) : []
    )
    ElMessage.success(t('Data scope updated'))
    await loadWorkbench()
  } catch {
    ElMessage.error(t('Failed to save data scope'))
  } finally {
    dataScopeSubmitting.value = false
  }
}

async function submitRoleUsers() {
  if (!selectedRoleId.value) return

  userSubmitting.value = true
  try {
    const response = await setRoleUserIds(selectedRoleId.value, selectedUserIds.value)
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

async function handleDelete(role: RoleResource) {
  try {
    await ElMessageBox.confirm(t('Delete role "{name}"?', { name: role.name }), t('Notice'), {
      type: 'warning'
    })
  } catch {
    return
  }

  try {
    const response = await deleteRole(role.id)
    if (response.code === 'OK') {
      ElMessage.success(t('Role deleted'))
      if (selectedRoleId.value === role.id) {
        selectedRoleId.value = null
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
  flex-wrap: nowrap;
  justify-content: flex-end;
  gap: 10px;
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
.permission-catalog-scroll,
.member-list {
  border: 1px solid #e7e5e4;
  border-radius: 14px;
  overflow: hidden;
  background: #ffffff;
}

.permission-matrix-scroll,
.permission-catalog-scroll {
  overflow-x: auto;
}

.permission-catalog {
  width: 100%;
  min-width: 720px;
  border-collapse: separate;
  border-spacing: 0;
}

.permission-catalog th,
.permission-catalog td {
  border-bottom: 1px solid #f1f1f0;
  padding: 14px 16px;
  text-align: left;
  vertical-align: top;
}

.permission-catalog th {
  position: sticky;
  top: 0;
  z-index: 1;
  background: #f5f5f4;
  color: #52525b;
  font-size: 13px;
  font-weight: 760;
}

.actions-column {
  min-width: 420px;
}

.menu-resource {
  display: grid;
  gap: 4px;
}

.menu-resource-title {
  color: #18181b;
  font-weight: 700;
}

.menu-resource-path {
  color: var(--text-muted);
  font-size: 12px;
}

.permission-actions {
  display: grid;
  grid-template-columns: repeat(auto-fill, 136px);
  align-items: center;
  gap: 8px 10px;
}

.permission-chip {
  position: relative;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  width: max-content;
  max-width: 136px;
  min-height: 34px;
  border: 1px solid #e7e5e4;
  border-radius: 999px;
  background: #ffffff;
  padding: 0 12px 0 10px;
  cursor: pointer;
  transition: border-color 0.16s ease, background 0.16s ease, box-shadow 0.16s ease;
}

.permission-chip:hover,
.permission-chip.is-checked {
  border-color: #18181b;
  background: #fafaf9;
}

.permission-chip:hover {
  box-shadow: 0 10px 24px rgba(24, 24, 27, 0.08);
}

.permission-chip input {
  accent-color: #18181b;
}

.permission-chip-label {
  color: #18181b;
  font-size: 13px;
  font-weight: 650;
  white-space: nowrap;
}

.permission-chip-tip {
  position: absolute;
  left: 50%;
  bottom: calc(100% + 10px);
  z-index: 3;
  transform: translateX(-50%) translateY(4px);
  min-width: max-content;
  max-width: 360px;
  border: 1px solid #e7e5e4;
  border-radius: 10px;
  background: #18181b;
  color: #fafafa;
  padding: 8px 10px;
  font-size: 12px;
  line-height: 1.45;
  white-space: nowrap;
  opacity: 0;
  pointer-events: none;
  transition: opacity 0.16s ease, transform 0.16s ease;
}

.permission-chip-tip::after {
  content: '';
  position: absolute;
  top: 100%;
  left: 50%;
  transform: translateX(-50%);
  border: 6px solid transparent;
  border-top-color: #18181b;
}

.permission-chip:hover .permission-chip-tip {
  opacity: 1;
  transform: translateX(-50%) translateY(0);
}

.permission-empty {
  display: inline-flex;
  align-items: center;
  min-height: 34px;
  color: var(--text-muted);
  font-size: 13px;
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

.role-detail-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 12px;
}

.role-detail-item {
  display: grid;
  gap: 6px;
  border: 1px solid #e7e5e4;
  border-radius: 12px;
  background: #ffffff;
  padding: 14px;
}

.role-detail-item span {
  color: var(--text-muted);
  font-size: 12px;
}

.role-detail-item strong {
  color: #18181b;
  font-size: 14px;
  font-weight: 760;
}

.permission-resource-list,
.data-scope-options,
.dept-scope-list {
  display: grid;
  gap: 8px;
}

.permission-resource-list,
.dept-scope-panel {
  border: 1px solid #e7e5e4;
  border-radius: 14px;
  background: #ffffff;
  padding: 12px;
}

.permission-resource-row,
.data-scope-option,
.dept-scope-row {
  display: flex;
  align-items: center;
  gap: 10px;
  border: 1px solid transparent;
  border-radius: 10px;
  padding: 10px;
}

.permission-resource-row:hover,
.data-scope-option:hover,
.data-scope-option.is-selected,
.dept-scope-row:hover {
  border-color: #e7e5e4;
  background: #fafaf9;
}

.permission-resource-main {
  display: grid;
  gap: 2px;
  min-width: 0;
}

.permission-resource-main strong {
  color: #18181b;
  font-size: 14px;
}

.permission-resource-main span,
.permission-resource-meta,
.data-scope-option small,
.dept-scope-row small {
  color: var(--text-muted);
  font-size: 12px;
}

.permission-resource-meta {
  margin-left: auto;
  white-space: nowrap;
}

.data-scope-layout {
  display: grid;
  grid-template-columns: minmax(220px, 320px) minmax(0, 1fr);
  gap: 14px;
}

.data-scope-option {
  align-items: flex-start;
  background: #ffffff;
}

.data-scope-option span {
  display: grid;
  gap: 3px;
}

.dept-scope-panel h5 {
  margin: 0 0 10px;
  color: #18181b;
  font-size: 14px;
}

.dept-scope-row {
  padding-left: calc(10px + var(--indent, 0px));
}

.dept-scope-row small {
  margin-left: auto;
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
  .member-tools,
  .data-scope-layout {
    align-items: stretch;
    flex-direction: column;
  }

  .data-scope-layout {
    display: grid;
    grid-template-columns: 1fr;
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
