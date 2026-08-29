CREATE TABLE sys_storages (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    code TEXT NOT NULL UNIQUE,
    driver TEXT NOT NULL,
    root TEXT,
    bucket TEXT,
    region TEXT,
    endpoint TEXT,
    public_base_url TEXT,
    access_key TEXT,
    secret_key TEXT,
    virtual_host_style BOOLEAN NOT NULL DEFAULT false,
    enabled BOOLEAN NOT NULL DEFAULT true,
    is_default BOOLEAN NOT NULL DEFAULT false,
    sort INTEGER NOT NULL DEFAULT 999,
    description TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_sys_storages_default
ON sys_storages (is_default)
WHERE is_default;

CREATE INDEX idx_sys_storages_list
ON sys_storages (driver, enabled, sort, id);

INSERT INTO sys_storages (
    name, code, driver, root, enabled, is_default, sort, description
)
VALUES (
    'Local storage', 'local', 'local', './uploads', true, true, 0,
    'Local file storage'
);

ALTER TABLE uploaded_files
ADD COLUMN storage_id BIGINT REFERENCES sys_storages(id) ON DELETE RESTRICT;

UPDATE uploaded_files
SET storage_id = (SELECT id FROM sys_storages WHERE code = 'local')
WHERE storage_id IS NULL AND url LIKE '/uploads/%';

CREATE INDEX idx_uploaded_files_storage
ON uploaded_files (storage_id);

INSERT INTO sys_menus (
    id, parent_id, path, name, hidden, component, sort, title, icon, menu_type, status, permission
)
VALUES
    (34, 30, '/sys-storage', 'sys-storage', false, '', 40, 'Storages', 'database', 'page', 'enabled', 'system:storage:list'),
    (3401, 34, '', 'sys-storage:create', true, '', 10, 'Create storage', '', 'action', 'enabled', 'system:storage:create'),
    (3402, 34, '', 'sys-storage:update', true, '', 20, 'Update storage', '', 'action', 'enabled', 'system:storage:update'),
    (3403, 34, '', 'sys-storage:delete', true, '', 30, 'Delete storage', '', 'action', 'enabled', 'system:storage:delete'),
    (3404, 34, '', 'sys-storage:update-status', true, '', 40, 'Enable or disable storage', '', 'action', 'enabled', 'system:storage:update-status'),
    (3405, 34, '', 'sys-storage:set-default', true, '', 50, 'Set default storage', '', 'action', 'enabled', 'system:storage:set-default');

INSERT INTO sys_menu_apis (menu_id, method, path_pattern)
VALUES
    (34, 'GET', '/api/storages'),
    (34, 'GET', '/api/storages/{id}'),
    (3401, 'POST', '/api/storages'),
    (3402, 'PUT', '/api/storages/{id}'),
    (3403, 'DELETE', '/api/storages/{id}'),
    (3404, 'PATCH', '/api/storages/{id}/status'),
    (3405, 'PUT', '/api/storages/{id}/default');

INSERT INTO casbin_rule (ptype, v0, v1, v2, v3, v4, v5)
SELECT 'p', 'role:' || role.id::text, permission.code, '', '', '', ''
FROM sys_roles role
CROSS JOIN (
    VALUES
        ('system:storage:list'),
        ('system:storage:create'),
        ('system:storage:update'),
        ('system:storage:delete'),
        ('system:storage:update-status'),
        ('system:storage:set-default')
) AS permission(code)
WHERE role.code = 'super_admin'
ON CONFLICT ON CONSTRAINT unique_key_sqlx_adapter DO NOTHING;
