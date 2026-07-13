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

CREATE TABLE sys_login_logs (
    id BIGSERIAL PRIMARY KEY,
    username TEXT NOT NULL,
    ip TEXT NOT NULL DEFAULT '',
    status BOOLEAN NOT NULL,
    error_message TEXT NOT NULL DEFAULT '',
    agent TEXT NOT NULL DEFAULT '',
    user_id BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE sys_operation_records (
    id BIGSERIAL PRIMARY KEY,
    ip TEXT NOT NULL DEFAULT '',
    method TEXT NOT NULL,
    path TEXT NOT NULL,
    status INTEGER NOT NULL DEFAULT 200,
    agent TEXT NOT NULL DEFAULT '',
    error_message TEXT NOT NULL DEFAULT '',
    body TEXT NOT NULL DEFAULT '',
    resp TEXT NOT NULL DEFAULT '',
    user_id BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

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

CREATE TABLE attachment_categories (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    pid BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE uploaded_files (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    url TEXT NOT NULL,
    tag TEXT NOT NULL DEFAULT '',
    class_id BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
