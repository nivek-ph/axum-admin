CREATE TABLE uploaded_file_sessions (
    id TEXT PRIMARY KEY,
    storage_id BIGINT NOT NULL REFERENCES sys_storages(id) ON DELETE RESTRICT,
    name TEXT NOT NULL,
    object_name TEXT NOT NULL,
    ext TEXT NOT NULL DEFAULT '',
    tag TEXT NOT NULL DEFAULT '',
    category TEXT NOT NULL DEFAULT '',
    total_size BIGINT NOT NULL,
    uploaded_size BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_uploaded_file_sessions_created_at
ON uploaded_file_sessions (created_at);

DELETE FROM sys_menu_apis
WHERE menu_id = 3302 AND method = 'POST' AND path_pattern = '/api/files/upload';

INSERT INTO sys_menu_apis (menu_id, method, path_pattern)
VALUES
    (3302, 'POST', '/api/files/uploads'),
    (3302, 'GET', '/api/files/uploads/{id}'),
    (3302, 'PATCH', '/api/files/uploads/{id}'),
    (3302, 'POST', '/api/files/uploads/{id}/complete');
