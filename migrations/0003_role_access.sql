-- Expand Role access into one Casbin-owned page and action Permission set.

INSERT INTO casbin_rule (ptype, v0, v1, v2, v3, v4, v5)
SELECT 'p', 'role:' || access.role_id::text, menu.permission, '', '', '', ''
FROM sys_role_menus access
JOIN sys_menus menu ON menu.id = access.menu_id
WHERE menu.menu_type = 'page'
ON CONFLICT ON CONSTRAINT unique_key_sqlx_adapter DO NOTHING;

-- Legacy operation grants become valid Role Access selections by including the
-- owning page Permission. Existing page and operation grants remain intact.
INSERT INTO casbin_rule (ptype, v0, v1, v2, v3, v4, v5)
SELECT 'p', policy.v0, page.permission, '', '', '', ''
FROM casbin_rule policy
JOIN sys_menus action
  ON action.permission = policy.v1
 AND action.menu_type = 'action'
JOIN sys_menus page
  ON page.id = action.parent_id
 AND page.menu_type = 'page'
WHERE policy.ptype = 'p'
  AND policy.v0 LIKE 'role:%'
ON CONFLICT ON CONSTRAINT unique_key_sqlx_adapter DO NOTHING;

DROP TABLE sys_role_menus;

DELETE FROM sys_menu_apis
WHERE path_pattern IN (
    '/api/roles/{id}/menus',
    '/api/roles/{id}/permissions'
);

DELETE FROM casbin_rule
WHERE ptype = 'p'
  AND v1 IN (
      'system:role:menus-read',
      'system:role:update-permission',
      'system:role:permissions-read',
      'system:role:permissions-update'
  );

UPDATE sys_menus
SET name = 'roles:access-read',
    title = 'Read role access',
    permission = 'system:role:access-read',
    updated_at = now()
WHERE id = 1204;

UPDATE sys_menus
SET name = 'roles:access-update',
    title = 'Update role access',
    permission = 'system:role:access-update',
    updated_at = now()
WHERE id = 1205;

DELETE FROM sys_menus WHERE id IN (1210, 1211);

INSERT INTO sys_menu_apis (menu_id, method, path_pattern)
VALUES
    (1204, 'GET', '/api/roles/{id}/access'),
    (1205, 'PUT', '/api/roles/{id}/access');

INSERT INTO casbin_rule (ptype, v0, v1, v2, v3, v4, v5)
SELECT 'p', 'role:' || role.id::text, permission.code, '', '', '', ''
FROM sys_roles role
CROSS JOIN (
    VALUES
        ('system:role:access-read'),
        ('system:role:access-update')
) AS permission(code)
WHERE role.code = 'super_admin'
ON CONFLICT ON CONSTRAINT unique_key_sqlx_adapter DO NOTHING;
