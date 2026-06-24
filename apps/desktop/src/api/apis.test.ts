import { describe, expect, it } from 'vitest';

import {
  normalizeApiListResponse,
  normalizeApiRoleMatrixResponse,
  normalizeApiRoleSelection,
  normalizeAuthorityApiListResponse,
} from './apis';

describe('api registry adapter', () => {
  it('normalizes paginated api payload', () => {
    const result = normalizeApiListResponse({
      data: {
        list: [{ ID: 1, path: '/api/users', method: 'GET', apiGroup: 'user', description: 'Get user list' }],
        total: 1,
        page: 1,
        pageSize: 10,
      },
    });

    expect(result.list).toHaveLength(1);
    expect(result.total).toBe(1);
  });

  it('normalizes assigned api role ids', () => {
    const result = normalizeApiRoleSelection({
      data: {
        authorityIds: [888],
      },
    });

    expect(result.authorityIds).toEqual([888]);
  });

  it('normalizes APIs assigned to one role', () => {
    const result = normalizeAuthorityApiListResponse({
      data: {
        apis: [{ ID: 1, path: '/api/users', method: 'GET', apiGroup: 'user', description: 'List users' }],
      },
    });

    expect(result).toHaveLength(1);
    expect(result[0].path).toBe('/api/users');
  });

  it('normalizes API role matrix payload', () => {
    const result = normalizeApiRoleMatrixResponse({
      data: {
        items: [{ path: '/api/users', method: 'GET', authorityIds: [888] }],
      },
    });

    expect(result).toEqual({
      'GET /api/users': [888],
    });
  });
});
