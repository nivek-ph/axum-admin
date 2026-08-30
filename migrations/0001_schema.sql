CREATE TABLE sys_depts (
    id BIGSERIAL PRIMARY KEY,
    parent_id BIGINT REFERENCES sys_depts(id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    code TEXT NOT NULL UNIQUE,
    sort INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'enabled' CHECK (status IN ('enabled', 'disabled')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE sys_roles (
    id BIGSERIAL PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'enabled' CHECK (status IN ('enabled', 'disabled')),
    sort INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE sys_users (
    id BIGSERIAL PRIMARY KEY,
    uuid TEXT NOT NULL UNIQUE,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    nick_name TEXT NOT NULL,
    header_img TEXT NOT NULL,
    home_route TEXT NOT NULL DEFAULT 'dashboard',
    enable BOOLEAN NOT NULL DEFAULT true,
    phone TEXT,
    email TEXT,
    origin_setting JSONB,
    dept_id BIGINT REFERENCES sys_depts(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_sys_users_username ON sys_users(username);

CREATE TABLE IF NOT EXISTS casbin_rule (
    id SERIAL PRIMARY KEY,
    ptype VARCHAR NOT NULL,
    v0 VARCHAR NOT NULL,
    v1 VARCHAR NOT NULL,
    v2 VARCHAR NOT NULL,
    v3 VARCHAR NOT NULL,
    v4 VARCHAR NOT NULL,
    v5 VARCHAR NOT NULL,
    CONSTRAINT unique_key_sqlx_adapter UNIQUE (ptype, v0, v1, v2, v3, v4, v5)
);

CREATE INDEX idx_casbin_rule_subject ON casbin_rule (ptype, v0, v1);

CREATE TABLE IF NOT EXISTS sys_menus (
    id BIGINT PRIMARY KEY,
    parent_id BIGINT REFERENCES sys_menus(id) ON DELETE RESTRICT,
    path TEXT NOT NULL DEFAULT '',
    name TEXT NOT NULL UNIQUE,
    hidden BOOLEAN NOT NULL DEFAULT false,
    component TEXT NOT NULL DEFAULT '',
    sort INTEGER NOT NULL DEFAULT 0,
    active_name TEXT NOT NULL DEFAULT '',
    keep_alive BOOLEAN NOT NULL DEFAULT false,
    default_menu BOOLEAN NOT NULL DEFAULT false,
    title TEXT NOT NULL,
    icon TEXT NOT NULL DEFAULT '',
    close_tab BOOLEAN NOT NULL DEFAULT false,
    transition_type TEXT NOT NULL DEFAULT '',
    parameters JSONB NOT NULL DEFAULT '[]'::JSONB,
    menu_btn JSONB NOT NULL DEFAULT '[]'::JSONB,
    menu_type TEXT NOT NULL CHECK (menu_type IN ('directory', 'page', 'action')),
    status TEXT NOT NULL DEFAULT 'enabled' CHECK (status IN ('enabled', 'disabled')),
    permission TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (
        (menu_type = 'directory' AND permission IS NULL)
        OR (menu_type IN ('page', 'action') AND permission IS NOT NULL)
    )
);

CREATE UNIQUE INDEX idx_sys_menus_permission ON sys_menus(permission)
WHERE permission IS NOT NULL;

CREATE INDEX idx_sys_menus_parent ON sys_menus(parent_id);

CREATE TABLE sys_menu_apis (
    menu_id BIGINT NOT NULL REFERENCES sys_menus(id) ON DELETE CASCADE,
    method TEXT NOT NULL CHECK (method = upper(method)),
    path_pattern TEXT NOT NULL CHECK (path_pattern LIKE '/api%'),
    PRIMARY KEY (method, path_pattern)
);

CREATE INDEX idx_sys_menu_apis_menu ON sys_menu_apis(menu_id);

CREATE TABLE sys_audit_events (
    id BIGSERIAL PRIMARY KEY,
    req_id TEXT NOT NULL,
    actor_id BIGINT,
    actor_label TEXT NOT NULL,
    action TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id TEXT,
    result TEXT NOT NULL,
    reason_code TEXT,
    source_ip TEXT NOT NULL DEFAULT '',
    user_agent TEXT NOT NULL DEFAULT '',
    changes JSONB NOT NULL DEFAULT '[]'::JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_sys_audit_events_req_id ON sys_audit_events(req_id);
CREATE INDEX idx_sys_audit_events_actor ON sys_audit_events(actor_id, created_at DESC);
CREATE INDEX idx_sys_audit_events_action ON sys_audit_events(action, created_at DESC);
CREATE INDEX idx_sys_audit_events_resource ON sys_audit_events(resource_type, resource_id, created_at DESC);
CREATE INDEX idx_sys_audit_events_result ON sys_audit_events(result, created_at DESC);

CREATE TABLE sys_params (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    "key" TEXT NOT NULL UNIQUE,
    value TEXT NOT NULL,
    "desc" TEXT NOT NULL DEFAULT ''
);

CREATE TABLE sys_dictionaries (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    type TEXT NOT NULL UNIQUE,
    status BOOLEAN,
    "desc" TEXT NOT NULL DEFAULT '',
    parent_id BIGINT
);

CREATE TABLE sys_dictionary_details (
    id BIGSERIAL PRIMARY KEY,
    label TEXT NOT NULL,
    value TEXT NOT NULL,
    extend TEXT NOT NULL DEFAULT '',
    status BOOLEAN,
    sort INTEGER NOT NULL DEFAULT 0,
    sys_dictionary_id BIGINT NOT NULL,
    parent_id BIGINT,
    level INTEGER NOT NULL DEFAULT 0,
    path TEXT NOT NULL DEFAULT ''
);

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
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (NOT is_default OR enabled)
);

CREATE UNIQUE INDEX idx_sys_storages_default
ON sys_storages (is_default)
WHERE is_default;

CREATE INDEX idx_sys_storages_list
ON sys_storages (driver, enabled, sort, id);

CREATE TABLE uploaded_files (
    id BIGSERIAL PRIMARY KEY,
    storage_id BIGINT REFERENCES sys_storages(id) ON DELETE RESTRICT,
    upload_id TEXT UNIQUE,
    name TEXT NOT NULL,
    object_name TEXT,
    url TEXT NOT NULL,
    ext TEXT NOT NULL DEFAULT '',
    tag TEXT NOT NULL DEFAULT '',
    category TEXT NOT NULL DEFAULT '',
    size BIGINT NOT NULL DEFAULT 0 CHECK (size BETWEEN 0 AND 1073741824),
    upload_parts_pending BOOLEAN NOT NULL DEFAULT false,
    deletion_pending BOOLEAN NOT NULL DEFAULT false,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_uploaded_files_storage
ON uploaded_files (storage_id);

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
    operation_state TEXT NOT NULL DEFAULT 'uploading',
    operation_token TEXT,
    operation_started_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (
        total_size BETWEEN 0 AND 1073741824
        AND uploaded_size BETWEEN 0 AND total_size
    ),
    CHECK (
        (operation_state = 'uploading' AND operation_token IS NULL AND operation_started_at IS NULL)
        OR
        (operation_state IN ('writing', 'completing', 'cleaning')
            AND operation_token IS NOT NULL
            AND operation_started_at IS NOT NULL)
    )
);

CREATE INDEX idx_uploaded_file_sessions_updated_at
ON uploaded_file_sessions (updated_at);

CREATE TABLE uploaded_file_parts (
    upload_id TEXT NOT NULL REFERENCES uploaded_file_sessions(id) ON DELETE CASCADE,
    part_offset BIGINT NOT NULL CHECK (part_offset >= 0),
    size BIGINT NOT NULL CHECK (size BETWEEN 1 AND 8388608),
    object_name TEXT NOT NULL,
    PRIMARY KEY (upload_id, part_offset),
    UNIQUE (object_name)
);
