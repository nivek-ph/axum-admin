import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { useAuthStore } from '@/stores/auth';

const httpApi = vi.hoisted(() => ({
  put: vi.fn(),
}));

vi.mock('./http', () => ({
  http: {
    put: httpApi.put,
  },
}));

import {
  buildCreateUserPayload,
  changeOwnPassword,
  normalizeUserListResponse,
} from './users';

describe('user api adapter', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    httpApi.put.mockReset();
  });

  it('normalizes backend list payload', () => {
    const result = normalizeUserListResponse({
      data: {
        list: [{ id: 1, userName: 'admin' }],
        total: 1,
        page: 1,
        pageSize: 10,
      },
    });

    expect(result.list).toHaveLength(1);
    expect(result.total).toBe(1);
  });

  it('maps create-user form values to the backend register payload', () => {
    expect(
      buildCreateUserPayload({
        userName: 'alice',
        nickName: 'Alice',
        password: '123456',
        phone: '',
        email: 'alice@example.com',
        enable: 1,
        roleIds: [1],
        deptId: 1,
      })
    ).toEqual({
      username: 'alice',
      nickName: 'Alice',
      password: '123456',
      phone: undefined,
      email: 'alice@example.com',
      enable: 1,
      roleIds: [1],
      deptId: 1,
    });
  });

  it('sends the self password change to the protected endpoint', async () => {
    useAuthStore().setSession('access-token', 'refresh-token', {
      id: 1,
      userName: 'admin',
      nickName: 'Admin',
    });
    const payload = {
      password: 'old-password',
      newPassword: 'new-password',
    };

    await changeOwnPassword(payload);

    expect(httpApi.put).toHaveBeenCalledWith('/users/me/password', payload, {
      headers: {
        Authorization: 'Bearer access-token',
      },
    });
  });
});
