INSERT INTO sys_depts (id, parent_id, name, code, sort, status)
VALUES (1, NULL, 'Head Office', 'head_office', 0, 'enabled');

INSERT INTO sys_roles (id, code, name, status, sort)
VALUES (1, 'super_admin', 'Super Admin', 'enabled', 0);

SELECT setval(pg_get_serial_sequence('sys_depts', 'id'), 1);
SELECT setval(pg_get_serial_sequence('sys_roles', 'id'), 1);

INSERT INTO sys_menus (
    id, parent_id, path, name, hidden, component, sort, title, icon, menu_type, status, permission
)
VALUES
    (1, NULL, '/dashboard', 'dashboard', false, '', 10, 'Dashboard', 'odometer', 'page', 'enabled', 'system:dashboard:view'),

    (10, NULL, '/organization', 'organization', false, '', 20, 'Organization', 'users', 'directory', 'enabled', NULL),
    (11, 10, '/users', 'users', false, '', 10, 'Users', 'user', 'page', 'enabled', 'system:user:list'),
    (1101, 11, '', 'users:create', true, '', 10, 'Create user', '', 'action', 'enabled', 'system:user:create'),
    (1102, 11, '', 'users:update', true, '', 20, 'Update user', '', 'action', 'enabled', 'system:user:update'),
    (1103, 11, '', 'users:delete', true, '', 30, 'Delete user', '', 'action', 'enabled', 'system:user:delete'),
    (1104, 11, '', 'users:reset-password', true, '', 40, 'Reset password', '', 'action', 'enabled', 'system:user:reset-password'),
    (1105, 11, '', 'users:assign-roles', true, '', 50, 'Assign roles', '', 'action', 'enabled', 'system:user:assign-roles'),
    (1106, 11, '', 'users:permissions-read', true, '', 60, 'Read user permissions', '', 'action', 'enabled', 'system:user:permissions-read'),
    (1107, 11, '', 'users:permissions-update', true, '', 70, 'Update user permissions', '', 'action', 'enabled', 'system:user:permissions-update'),
    (12, 10, '/roles', 'roles', false, '', 20, 'Roles', 'shield', 'page', 'enabled', 'system:role:list'),
    (1201, 12, '', 'roles:create', true, '', 10, 'Create role', '', 'action', 'enabled', 'system:role:create'),
    (1202, 12, '', 'roles:update', true, '', 20, 'Update role', '', 'action', 'enabled', 'system:role:update'),
    (1203, 12, '', 'roles:delete', true, '', 30, 'Delete role', '', 'action', 'enabled', 'system:role:delete'),
    (1204, 12, '', 'roles:menus-read', true, '', 40, 'Read role menus', '', 'action', 'enabled', 'system:role:menus-read'),
    (1205, 12, '', 'roles:menus-update', true, '', 50, 'Update role menus', '', 'action', 'enabled', 'system:role:update-permission'),
    (1210, 12, '', 'roles:permissions-read', true, '', 60, 'Read role permissions', '', 'action', 'enabled', 'system:role:permissions-read'),
    (1211, 12, '', 'roles:permissions-update', true, '', 70, 'Update role permissions', '', 'action', 'enabled', 'system:role:permissions-update'),
    (13, 10, '/departments', 'departments', false, '', 30, 'Departments', 'building', 'page', 'enabled', 'system:dept:list'),
    (1301, 13, '', 'departments:create', true, '', 10, 'Create department', '', 'action', 'enabled', 'system:dept:create'),
    (1302, 13, '', 'departments:get', true, '', 20, 'Get department', '', 'action', 'enabled', 'system:dept:get'),
    (1303, 13, '', 'departments:update', true, '', 30, 'Update department', '', 'action', 'enabled', 'system:dept:update'),
    (1304, 13, '', 'departments:delete', true, '', 40, 'Delete department', '', 'action', 'enabled', 'system:dept:delete'),

    (20, NULL, '/access', 'access-control', false, '', 30, 'Access Control', 'lock', 'directory', 'enabled', NULL),
    (21, 20, '/menus', 'menus', false, '', 10, 'Access Catalog', 'menu', 'page', 'enabled', 'system:menu:list'),

    (30, NULL, '/system', 'system', false, '', 40, 'System', 'settings', 'directory', 'enabled', NULL),
    (31, 30, '/params', 'params', false, '', 10, 'Params', 'sliders', 'page', 'enabled', 'system:param:list'),
    (3101, 31, '', 'params:create', true, '', 10, 'Create param', '', 'action', 'enabled', 'system:param:create'),
    (3102, 31, '', 'params:get', true, '', 20, 'Get param', '', 'action', 'enabled', 'system:param:get'),
    (3103, 31, '', 'params:update', true, '', 30, 'Update param', '', 'action', 'enabled', 'system:param:update'),
    (3104, 31, '', 'params:delete', true, '', 40, 'Delete param', '', 'action', 'enabled', 'system:param:delete'),
    (3105, 31, '', 'params:batch-delete', true, '', 50, 'Batch delete params', '', 'action', 'enabled', 'system:param:batch-delete'),
    (32, 30, '/dictionaries', 'dictionaries', false, '', 20, 'Dictionaries', 'book', 'page', 'enabled', 'system:dictionary:list'),
    (3201, 32, '', 'dictionaries:create', true, '', 10, 'Create dictionary', '', 'action', 'enabled', 'system:dictionary:create'),
    (3202, 32, '', 'dictionaries:update', true, '', 20, 'Update dictionary', '', 'action', 'enabled', 'system:dictionary:update'),
    (3203, 32, '', 'dictionaries:delete', true, '', 30, 'Delete dictionary', '', 'action', 'enabled', 'system:dictionary:delete'),
    (3204, 32, '', 'dictionaries:import', true, '', 40, 'Import dictionaries', '', 'action', 'enabled', 'system:dictionary:import'),
    (3205, 32, '', 'dictionaries:export', true, '', 50, 'Export dictionary', '', 'action', 'enabled', 'system:dictionary:export'),
    (3211, 32, '', 'dictionary-details:create', true, '', 60, 'Create dictionary detail', '', 'action', 'enabled', 'system:dictionary-detail:create'),
    (3212, 32, '', 'dictionary-details:update', true, '', 70, 'Update dictionary detail', '', 'action', 'enabled', 'system:dictionary-detail:update'),
    (3213, 32, '', 'dictionary-details:delete', true, '', 80, 'Delete dictionary detail', '', 'action', 'enabled', 'system:dictionary-detail:delete'),
    (33, 30, '/files', 'files', false, '', 30, 'Files', 'folder', 'page', 'enabled', 'system:file:list'),
    (3301, 33, '', 'files:import-url', true, '', 10, 'Import file URL', '', 'action', 'enabled', 'system:file:import-url'),
    (3302, 33, '', 'files:upload', true, '', 20, 'Upload file', '', 'action', 'enabled', 'system:file:upload'),
    (3303, 33, '', 'files:delete', true, '', 30, 'Delete file', '', 'action', 'enabled', 'system:file:delete'),
    (3304, 33, '', 'files:rename', true, '', 40, 'Rename file', '', 'action', 'enabled', 'system:file:rename'),

    (40, NULL, '/audit', 'audit', false, '', 50, 'Audit', 'history', 'directory', 'enabled', NULL),
    (41, 40, '/audit-events', 'audit-events', false, '', 10, 'Audit Events', 'history', 'page', 'enabled', 'system:audit-event:list');

INSERT INTO sys_menu_apis (menu_id, method, path_pattern)
VALUES
    (11, 'GET', '/api/users'),
    (1101, 'POST', '/api/users'),
    (1102, 'PUT', '/api/users/{id}'),
    (1103, 'DELETE', '/api/users/{id}'),
    (1104, 'POST', '/api/users/{id}/password/reset'),
    (1105, 'PUT', '/api/users/{id}/roles'),
    (1106, 'GET', '/api/users/{id}/permissions'),
    (1107, 'PUT', '/api/users/{id}/permissions'),

    (12, 'GET', '/api/roles'),
    (1201, 'POST', '/api/roles'),
    (1202, 'PUT', '/api/roles/{id}'),
    (1203, 'DELETE', '/api/roles/{id}'),
    (1204, 'GET', '/api/roles/{id}/menus'),
    (1205, 'PUT', '/api/roles/{id}/menus'),
    (1210, 'GET', '/api/roles/{id}/permissions'),
    (1211, 'PUT', '/api/roles/{id}/permissions'),

    (13, 'GET', '/api/depts'),
    (1301, 'POST', '/api/depts'),
    (1302, 'GET', '/api/depts/{id}'),
    (1303, 'PUT', '/api/depts/{id}'),
    (1304, 'DELETE', '/api/depts/{id}'),
    (21, 'GET', '/api/menus/tree'),

    (31, 'GET', '/api/params'),
    (31, 'GET', '/api/params/by-key'),
    (3101, 'POST', '/api/params'),
    (3102, 'GET', '/api/params/{id}'),
    (3103, 'PUT', '/api/params/{id}'),
    (3104, 'DELETE', '/api/params/{id}'),
    (3105, 'DELETE', '/api/params/batch'),

    (32, 'GET', '/api/dictionaries'),
    (32, 'GET', '/api/dictionaries/{id}'),
    (32, 'GET', '/api/dictionaries/{id}/tree'),
    (32, 'GET', '/api/dictionaries/{id}/tree/{node_id}'),
    (32, 'GET', '/api/dictionaries/by-type/{dictionary_type}/tree'),
    (32, 'GET', '/api/dictionaries/{id}/tree/{node_id}/children'),
    (32, 'GET', '/api/dictionaries/{id}/tree/{node_id}/path'),
    (3201, 'POST', '/api/dictionaries'),
    (3202, 'PUT', '/api/dictionaries/{id}'),
    (3203, 'DELETE', '/api/dictionaries/{id}'),
    (3204, 'POST', '/api/dictionaries/import'),
    (3205, 'GET', '/api/dictionaries/{id}/export'),
    (3211, 'POST', '/api/dictionaries/{id}/tree'),
    (3212, 'PUT', '/api/dictionaries/{id}/tree/{node_id}'),
    (3213, 'DELETE', '/api/dictionaries/{id}/tree/{node_id}'),

    (33, 'GET', '/api/files'),
    (3301, 'POST', '/api/files/import-url'),
    (3302, 'POST', '/api/files/upload'),
    (3303, 'DELETE', '/api/files/{id}'),
    (3304, 'PATCH', '/api/files/{id}/name'),

    (41, 'GET', '/api/audit/events'),
    (41, 'GET', '/api/audit/events/{id}'),
    (41, 'POST', '/api/audit/events/analyze'),
    (41, 'GET', '/api/audit/events/stats');

INSERT INTO sys_role_menus (role_id, menu_id)
SELECT 1, id
FROM sys_menus
WHERE menu_type IN ('directory', 'page');

INSERT INTO casbin_rule (ptype, v0, v1, v2, v3, v4, v5)
SELECT 'p', 'role:1', menu.permission, '', '', '', ''
FROM sys_role_menus access
JOIN sys_menus menu ON menu.id = access.menu_id
WHERE access.role_id = 1
  AND menu.menu_type = 'page'
UNION ALL
SELECT 'p', 'role:1', permission, '', '', '', ''
FROM sys_menus
WHERE menu_type = 'action';
