-- Contract User access to role membership plus read-only Effective Access.

DELETE FROM casbin_rule
WHERE ptype = 'p' AND v0 LIKE 'user:%';

DELETE FROM sys_menu_apis
WHERE path_pattern = '/api/users/{id}/permissions';

DELETE FROM casbin_rule
WHERE ptype = 'p'
  AND v1 IN (
      'system:user:permissions-read',
      'system:user:permissions-update'
  );

UPDATE sys_menus
SET name = 'users:access-read',
    title = 'Read user access',
    permission = 'system:user:access-read',
    updated_at = now()
WHERE id = 1106;

DELETE FROM sys_menus WHERE id = 1107;

INSERT INTO sys_menu_apis (menu_id, method, path_pattern)
VALUES (1106, 'GET', '/api/users/{id}/access');

INSERT INTO casbin_rule (ptype, v0, v1, v2, v3, v4, v5)
SELECT 'p', 'role:' || role.id::text, 'system:user:access-read', '', '', '', ''
FROM sys_roles role
WHERE role.code = 'super_admin'
ON CONFLICT ON CONSTRAINT unique_key_sqlx_adapter DO NOTHING;
