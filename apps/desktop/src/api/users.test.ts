import { describe, expect, it } from 'vitest';

import {
  buildCreateUserPayload,
  normalizeUserListResponse,
} from './users';

describe('user api adapter', () => {
  it('normalizes backend list payload', () => {
    const result = normalizeUserListResponse({
      data: {
        list: [{ ID: 1, userName: 'admin' }],
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
      userName: 'alice',
      nickName: 'Alice',
      passWord: '123456',
      phone: undefined,
      email: 'alice@example.com',
      enable: 1,
      roleIds: [1],
      deptId: 1,
    });
  });
});
