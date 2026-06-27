import { describe, expect, it } from 'vitest';

import { buildRoleUsersPayload, normalizeAuthorityListResponse } from './authorities';

describe('authority api adapter', () => {
  it('normalizes backend authority tree payload', () => {
    const result = normalizeAuthorityListResponse({
      data: [
        {
          authorityId: 1,
          authorityName: 'Super Admin',
          children: [{ authorityId: 999, authorityName: 'Operator' }],
        },
      ],
    });

    expect(result).toHaveLength(1);
    expect(result[0].children).toHaveLength(1);
  });

  it('builds mature role user assignment payload', () => {
    expect(buildRoleUsersPayload([4, 2, 4])).toEqual({ userIds: [2, 4] });
  });
});
