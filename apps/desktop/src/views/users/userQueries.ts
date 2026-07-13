import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'

import { listRoles } from '@/api/system/roles'
import { assignUserRoles, createUser, deleteUser, fetchUsers, resetUserPassword, type CreateUserForm } from '@/api/users'

export const userQueryKeys = {
  all: ['users'] as const,
  list: (page: number, pageSize: number) => ['users', 'list', { page, pageSize }] as const,
  roles: ['users', 'roles'] as const
}

export function useUsersQuery(page = 1, pageSize = 10) {
  return useQuery({
    queryKey: userQueryKeys.list(page, pageSize),
    queryFn: () => fetchUsers(page, pageSize)
  })
}

export function useUserRolesQuery() {
  return useQuery({
    queryKey: userQueryKeys.roles,
    queryFn: listRoles
  })
}

export function useDeleteUserMutation() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: deleteUser,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: userQueryKeys.all })
  })
}

export function useCreateUserMutation() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (form: CreateUserForm) => createUser(form),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: userQueryKeys.all })
  })
}

export function useUpdateUserAuthoritiesMutation() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({ id, roleIds }: { id: number; roleIds: number[] }) => assignUserRoles(id, { roleIds }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: userQueryKeys.all })
  })
}

export function useResetUserPasswordMutation() {
  return useMutation({
    mutationFn: ({ id, password }: { id: number; password: string }) => resetUserPassword(id, password)
  })
}
