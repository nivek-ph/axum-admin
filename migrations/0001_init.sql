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
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_sys_users_username ON sys_users(username);

CREATE TABLE sys_audit_events (
    id BIGSERIAL PRIMARY KEY,
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

CREATE INDEX idx_sys_audit_events_actor ON sys_audit_events(actor_id, created_at DESC);
CREATE INDEX idx_sys_audit_events_action ON sys_audit_events(action, created_at DESC);
CREATE INDEX idx_sys_audit_events_resource
    ON sys_audit_events(resource_type, resource_id, created_at DESC);
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

CREATE TABLE uploaded_files (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    url TEXT NOT NULL,
    ext TEXT NOT NULL DEFAULT '',
    tag TEXT NOT NULL DEFAULT '',
    category TEXT NOT NULL DEFAULT '',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
