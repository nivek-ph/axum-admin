<template>
  <div class="admin-page">
    <section class="page-hero">
      <div class="page-hero-main">
        <span class="page-hero-kicker">{{ $t('User directory') }}</span>
        <h2 class="page-hero-title">{{ $t('Users') }}</h2>
        <p class="page-hero-subtitle">{{ $t('Review accounts, roles, status, and common actions in one place.') }}</p>

        <div class="page-metrics">
          <div class="page-metric">
            <div class="page-metric-label">{{ $t('Total users') }}</div>
            <div class="page-metric-value">{{ total }}</div>
          </div>
          <div class="page-metric">
            <div class="page-metric-label">{{ $t('Enabled users') }}</div>
            <div class="page-metric-value">{{ enabledCount }}</div>
          </div>
          <div class="page-metric">
            <div class="page-metric-label">{{ $t('Role types') }}</div>
            <div class="page-metric-value">{{ roleCount }}</div>
          </div>
        </div>
      </div>
    </section>

    <section class="page-panel">
      <div class="page-panel-header">
        <div>
          <h3 class="page-panel-title">{{ $t('Accounts') }}</h3>
          <p class="page-panel-subtitle">{{ $t('Common actions stay inline for faster operation.') }}</p>
        </div>
        <div class="page-panel-actions">
          <UiButton @click="loadUsers" :loading="loading">{{ $t('Refresh') }}</UiButton>
          <UiButton v-if="canCreateUser" data-test="new-user-button" type="primary" @click="openCreateDialog">
            {{ $t('New') }}
          </UiButton>
        </div>
      </div>

      <div class="surface-card">
        <UiTable :data="users" :loading="loading" style="width: 100%">
          <UiTableColumn prop="id" label="ID" width="80" />
          <UiTableColumn prop="userName" label="Username" min-width="140" />
          <UiTableColumn prop="nickName" label="Nickname" min-width="140" />
          <UiTableColumn prop="phone" label="Phone" min-width="140" />
          <UiTableColumn prop="email" label="Email" min-width="180" />
          <UiTableColumn label="Department" min-width="140">
            <template #default="{ row }">
              {{ row.deptName || '-' }}
            </template>
          </UiTableColumn>
          <UiTableColumn label="Roles" min-width="180">
            <template #default="{ row }">
              <div class="user-role-tags">
                <UiTag v-for="role in userRoles(row)" :key="role.id" type="info">
                  {{ role.name }}
                </UiTag>
                <span v-if="userRoles(row).length === 0">-</span>
              </div>
            </template>
          </UiTableColumn>
          <UiTableColumn label="Status" width="100">
            <template #default="{ row }">
              <UiTag :type="row.enable === 1 ? 'success' : 'danger'">
                {{ $t(row.enable === 1 ? 'Enabled' : 'Disabled') }}
              </UiTag>
            </template>
          </UiTableColumn>
          <UiTableColumn label="Actions" width="280">
            <template #default="{ row }">
              <div class="user-row-actions">
                <UiButton
                  v-if="canAssignUserRoles"
                  link
                  data-test="change-user-role-button"
                  @click="openRoleDialog(row)"
                >
                  {{ $t('Change role') }}
                </UiButton>
                <UiButton
                  v-if="canResetUserPassword"
                  link
                  data-test="reset-user-password-button"
                  @click="handleResetPassword(row.id)"
                >
                  {{ $t('Reset password') }}
                </UiButton>
                <UiButton
                  v-if="canDeleteUser"
                  link
                  type="danger"
                  data-test="delete-user-button"
                  @click="handleDelete(row.id)"
                >
                  {{ $t('Delete') }}
                </UiButton>
                <span v-if="!hasUserRowActions" data-test="user-row-no-actions" class="empty-action">-</span>
              </div>
            </template>
          </UiTableColumn>
        </UiTable>
      </div>
    </section>

    <UiDialog v-model="createDialogVisible" title="New user" width="560px">
      <UiForm class="user-form" @submit.prevent="handleCreateUser">
        <UiFormItem label="Username">
          <UiInput v-model="createForm.userName" placeholder="Username" />
        </UiFormItem>
        <UiFormItem label="Nickname">
          <UiInput v-model="createForm.nickName" placeholder="Nickname" />
        </UiFormItem>
        <UiFormItem label="Password">
          <UiInput v-model="createForm.password" type="password" placeholder="Password" showPassword />
        </UiFormItem>
        <UiFormItem label="Role">
          <UiSelect v-model="createForm.roleIds" multiple data-test="user-role-select" placeholder="Select role">
            <UiOption
              v-for="role in roleOptions"
              :key="role.id"
              :label="role.name"
              :value="role.id"
            />
          </UiSelect>
        </UiFormItem>
        <UiFormItem label="Phone">
          <UiInput v-model="createForm.phone" placeholder="Phone" />
        </UiFormItem>
        <UiFormItem label="Email">
          <UiInput v-model="createForm.email" placeholder="Email" />
        </UiFormItem>
        <UiFormItem label="Status">
          <UiSelect v-model="createForm.enable">
            <UiOption label="Enabled" :value="1" />
            <UiOption label="Disabled" :value="0" />
          </UiSelect>
        </UiFormItem>
      </UiForm>
      <template #footer>
        <UiButton @click="createDialogVisible = false">{{ $t('Cancel') }}</UiButton>
        <UiButton type="primary" :loading="createUserMutation.isPending.value" @click="handleCreateUser">
          {{ $t('Create') }}
        </UiButton>
      </template>
    </UiDialog>

    <UiDialog v-model="roleDialogVisible" title="Change user role" width="480px">
      <UiForm class="user-form" @submit.prevent="handleUpdateUserRole">
        <UiFormItem label="Username">
          <UiInput :model-value="selectedUser?.userName || ''" disabled />
        </UiFormItem>
        <UiFormItem label="Role">
          <UiSelect v-model="roleForm.roleIds" multiple data-test="edit-user-role-select" placeholder="Select role">
            <UiOption
              v-for="role in roleOptions"
              :key="role.id"
              :label="role.name"
              :value="role.id"
            />
          </UiSelect>
        </UiFormItem>
      </UiForm>
      <template #footer>
        <UiButton @click="roleDialogVisible = false">{{ $t('Cancel') }}</UiButton>
        <UiButton type="primary" :loading="updateUserAuthoritiesMutation.isPending.value" @click="handleUpdateUserRole">
          {{ $t('Save') }}
        </UiButton>
      </template>
    </UiDialog>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { ElMessage, ElMessageBox } from '@/ui/feedback'

import type { UserRecord } from '@/api/users'
import { usePageChrome } from '@/composables/usePageChrome'
import { getApiErrorMessage } from '@/api/http'
import { t } from '@/i18n'
import { useAuthStore } from '@/stores/auth'
import { useCreateUserMutation, useDeleteUserMutation, useResetUserPasswordMutation, useUpdateUserAuthoritiesMutation, useUserRolesQuery, useUsersQuery } from './userQueries'

const authStore = useAuthStore()
const usersQuery = useUsersQuery()
const rolesQuery = useUserRolesQuery()
const createUserMutation = useCreateUserMutation()
const deleteUserMutation = useDeleteUserMutation()
const resetUserPasswordMutation = useResetUserPasswordMutation()
const updateUserAuthoritiesMutation = useUpdateUserAuthoritiesMutation()
const users = computed(() => usersQuery.data.value?.list || [])
const loading = computed(() => usersQuery.isFetching.value || deleteUserMutation.isPending.value || updateUserAuthoritiesMutation.isPending.value)
const createDialogVisible = ref(false)
const roleDialogVisible = ref(false)
const selectedUser = ref<UserRecord>()
const createForm = reactive({
  userName: '',
  nickName: '',
  password: '123456',
  phone: '',
  email: '',
  enable: 1,
  roleIds: [] as number[]
})
const roleForm = reactive({
  roleIds: [] as number[]
})
const { total } = usePageChrome(users, 'users')
const enabledCount = computed(() => users.value.filter((item) => item.enable === 1).length)
const roleCount = computed(
  () => new Set(users.value.flatMap((item) => item.roles || []).map((role) => role.id)).size
)
const roleOptions = computed(() => rolesQuery.data.value || [])
const canCreateUser = computed(() => authStore.can('system:user:create'))
const canAssignUserRoles = computed(() => authStore.can('system:user:assign-roles'))
const canResetUserPassword = computed(() => authStore.can('system:user:reset-password'))
const canDeleteUser = computed(() => authStore.can('system:user:delete'))
const hasUserRowActions = computed(
  () => canAssignUserRoles.value || canResetUserPassword.value || canDeleteUser.value
)

watch(roleOptions, (roles) => {
  if (createDialogVisible.value && createForm.roleIds.length === 0) {
    createForm.roleIds = roles[0] ? [roles[0].id] : []
  }
})

async function loadUsers() {
  try {
    await usersQuery.refetch()
  } catch (err) {
    ElMessage.error(getApiErrorMessage(err, t('Failed to load users')))
  }
}

function resetCreateForm() {
  createForm.userName = ''
  createForm.nickName = ''
  createForm.password = '123456'
  createForm.phone = ''
  createForm.email = ''
  createForm.enable = 1
  createForm.roleIds = roleOptions.value[0] ? [roleOptions.value[0].id] : []
}

function openCreateDialog() {
  if (!canCreateUser.value) return
  resetCreateForm()
  createDialogVisible.value = true
}

function openRoleDialog(user: UserRecord) {
  if (!canAssignUserRoles.value) return
  selectedUser.value = user
  roleForm.roleIds = user.roleIds?.length
    ? [...user.roleIds]
    : (user.roles || []).map((role) => role.id)
  roleDialogVisible.value = true
}

async function handleCreateUser() {
  if (!canCreateUser.value) return
  if (!createForm.userName.trim() || !createForm.nickName.trim() || !createForm.password || createForm.roleIds.length === 0) {
    ElMessage.error(t('Username, nickname, password, and role are required'))
    return
  }

  try {
    const res = await createUserMutation.mutateAsync({ ...createForm })
    if (res.code === 'OK') {
      ElMessage.success(t('Created'))
      createDialogVisible.value = false
      resetCreateForm()
    } else {
      ElMessage.error(res.message || t('Create failed'))
    }
  } catch (err) {
    ElMessage.error(getApiErrorMessage(err, t('Create failed')))
  }
}

async function handleUpdateUserRole() {
  if (!canAssignUserRoles.value) return
  if (!selectedUser.value || roleForm.roleIds.length === 0) {
    ElMessage.error(t('Select role'))
    return
  }

  try {
    const res = await updateUserAuthoritiesMutation.mutateAsync({
      id: selectedUser.value.id,
      roleIds: roleForm.roleIds
    })
    if (res.code === 'OK') {
      ElMessage.success(t('User role updated'))
      roleDialogVisible.value = false
    } else {
      ElMessage.error(res.message || t('Failed to update user role'))
    }
  } catch (err) {
    ElMessage.error(getApiErrorMessage(err, t('Failed to update user role')))
  }
}

async function handleDelete(id: number) {
  if (!canDeleteUser.value) return
  try {
    await ElMessageBox.confirm(t('Delete this user?'), t('Notice'), {
      type: 'warning'
    })
  } catch {
    return
  }

  try {
    const res = await deleteUserMutation.mutateAsync(id)
    if (res.code === 'OK') {
      ElMessage.success(t('Deleted'))
    } else {
      ElMessage.error(res.message || t('Delete failed'))
    }
  } catch (err) {
    ElMessage.error(getApiErrorMessage(err, t('Delete failed')))
  }
}

async function handleResetPassword(id: number) {
  if (!canResetUserPassword.value) return
  try {
    const res = await resetUserPasswordMutation.mutateAsync({ id, password: '123456' })
    if (res.code === 'OK') {
      ElMessage.success(t('Password reset to 123456'))
    } else {
      ElMessage.error(res.message || t('Reset failed'))
    }
  } catch (err) {
    ElMessage.error(getApiErrorMessage(err, t('Reset failed')))
  }
}

function userRoles(user: UserRecord) {
  return user.roles || []
}
</script>

<style scoped>
.user-form {
  display: grid;
  gap: 14px;
}

.user-role-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.user-row-actions {
  align-items: center;
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.empty-action {
  color: var(--color-text-muted);
}
</style>
