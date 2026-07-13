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
    data_scope TEXT NOT NULL DEFAULT 'self' CHECK (
        data_scope IN ('all', 'dept', 'dept_and_children', 'self', 'custom_depts')
    ),
    is_system BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE sys_user_roles (
    user_id BIGINT NOT NULL REFERENCES sys_users(id) ON DELETE CASCADE,
    role_id BIGINT NOT NULL REFERENCES sys_roles(id) ON DELETE RESTRICT,
    PRIMARY KEY (user_id, role_id)
);

CREATE TABLE sys_role_depts (
    role_id BIGINT NOT NULL REFERENCES sys_roles(id) ON DELETE CASCADE,
    dept_id BIGINT NOT NULL REFERENCES sys_depts(id) ON DELETE CASCADE,
    PRIMARY KEY (role_id, dept_id)
);

CREATE TABLE sys_menus (
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

CREATE UNIQUE INDEX idx_sys_menus_permission
    ON sys_menus(permission)
    WHERE permission IS NOT NULL;
CREATE INDEX idx_sys_menus_parent ON sys_menus(parent_id);

CREATE TABLE sys_menu_apis (
    menu_id BIGINT NOT NULL REFERENCES sys_menus(id) ON DELETE CASCADE,
    method TEXT NOT NULL CHECK (method = upper(method)),
    path_pattern TEXT NOT NULL CHECK (path_pattern LIKE '/api%'),
    PRIMARY KEY (method, path_pattern)
);

CREATE INDEX idx_sys_menu_apis_menu ON sys_menu_apis(menu_id);

CREATE TABLE sys_role_menus (
    role_id BIGINT NOT NULL REFERENCES sys_roles(id) ON DELETE CASCADE,
    menu_id BIGINT NOT NULL REFERENCES sys_menus(id) ON DELETE RESTRICT,
    PRIMARY KEY (role_id, menu_id)
);

ALTER TABLE sys_users
    ADD COLUMN dept_id BIGINT REFERENCES sys_depts(id) ON DELETE SET NULL,
    ADD COLUMN is_system BOOLEAN NOT NULL DEFAULT false;

INSERT INTO sys_depts (id, parent_id, name, code, sort, status)
VALUES (1, NULL, 'Head Office', 'head_office', 0, 'enabled');

INSERT INTO sys_roles (id, code, name, status, sort, data_scope, is_system)
VALUES (1, 'super_admin', 'Super Admin', 'enabled', 0, 'all', true);

SELECT setval(pg_get_serial_sequence('sys_depts', 'id'), 1);
SELECT setval(pg_get_serial_sequence('sys_roles', 'id'), 1);
