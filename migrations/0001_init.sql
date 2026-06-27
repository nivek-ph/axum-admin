create table if not exists sys_users (
    id bigserial primary key,
    uuid text not null unique,
    username text not null unique,
    password_hash text not null,
    nick_name text not null,
    header_img text not null,
    authority_id bigint not null default 1,
    authority_name text not null default 'Super Admin',
    default_router text not null default 'dashboard',
    enable boolean not null default true,
    phone text,
    email text,
    origin_setting jsonb,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create index if not exists idx_sys_users_username on sys_users(username);

create table if not exists sys_menus (
    id bigserial primary key,
    parent_id bigint not null default 0,
    path text not null,
    name text not null unique,
    hidden boolean not null default false,
    component text not null,
    sort integer not null default 0,
    active_name text not null default '',
    keep_alive boolean not null default false,
    default_menu boolean not null default false,
    title text not null,
    icon text not null default '',
    close_tab boolean not null default false,
    transition_type text not null default '',
    parameters jsonb not null default '[]'::jsonb,
    menu_btn jsonb not null default '[]'::jsonb,
    menu_type text not null default 'page',
    permission text,
    method text,
    api_path text
);

create unique index if not exists idx_sys_menus_permission
    on sys_menus (permission)
    where permission is not null;

create table if not exists sys_apis (
    id bigserial primary key,
    path text not null,
    description text not null,
    api_group text not null,
    method text not null,
    unique (path, method)
);

create table if not exists sys_login_logs (
    id bigserial primary key,
    username text not null,
    ip text not null default '',
    status boolean not null,
    error_message text not null default '',
    agent text not null default '',
    user_id bigint,
    created_at timestamptz not null default now()
);

create table if not exists sys_operation_records (
    id bigserial primary key,
    ip text not null default '',
    method text not null,
    path text not null,
    status integer not null default 200,
    agent text not null default '',
    error_message text not null default '',
    body text not null default '',
    resp text not null default '',
    user_id bigint not null,
    created_at timestamptz not null default now()
);

create table if not exists sys_params (
    id bigserial primary key,
    name text not null,
    "key" text not null unique,
    value text not null,
    "desc" text not null default ''
);

create table if not exists sys_dictionaries (
    id bigserial primary key,
    name text not null,
    type text not null unique,
    status boolean,
    "desc" text not null default '',
    parent_id bigint
);

create table if not exists sys_dictionary_details (
    id bigserial primary key,
    label text not null,
    value text not null,
    extend text not null default '',
    status boolean,
    sort integer not null default 0,
    sys_dictionary_id bigint not null,
    parent_id bigint,
    level integer not null default 0,
    path text not null default ''
);

create table if not exists attachment_categories (
    id bigserial primary key,
    name text not null,
    pid bigint not null default 0
);

create table if not exists uploaded_files (
    id bigserial primary key,
    name text not null,
    url text not null,
    tag text not null default '',
    class_id bigint not null default 0,
    updated_at timestamptz not null default now()
);

with api_seed(path, description, api_group, method) as (
    values
        ('/api/auth/logout', 'User logout', 'auth', 'POST'),
        ('/api/users/me', 'Get current user', 'user', 'GET'),
        ('/api/users/me', 'Update current user', 'user', 'PUT'),
        ('/api/users/me/password', 'Change current user password', 'user', 'PUT'),
        ('/api/users/me/settings', 'Update current user settings', 'user', 'PUT'),
        ('/api/users', 'List users', 'user', 'GET'),
        ('/api/users', 'Create user', 'user', 'POST'),
        ('/api/users/{id}', 'Update user', 'user', 'PUT'),
        ('/api/users/{id}', 'Delete user', 'user', 'DELETE'),
        ('/api/users/{id}/password/reset', 'Reset user password', 'user', 'POST'),
        ('/api/menus/current', 'Get menus', 'menu', 'GET'),
        ('/api/menus', 'List menus', 'menu', 'GET'),
        ('/api/menus', 'Create menu', 'menu', 'POST'),
        ('/api/menus/tree', 'Get menu tree', 'menu', 'GET'),
        ('/api/menus/{id}', 'Get menu', 'menu', 'GET'),
        ('/api/menus/{id}', 'Update menu', 'menu', 'PUT'),
        ('/api/menus/{id}', 'Delete menu', 'menu', 'DELETE'),
        ('/api/menus/{id}/roles', 'Get menu roles', 'menu', 'GET'),
        ('/api/menus/{id}/roles', 'Update menu roles', 'menu', 'PUT'),
        ('/api/roles', 'List roles', 'role', 'GET'),
        ('/api/roles', 'Create role', 'role', 'POST'),
        ('/api/roles/{authority_id}', 'Update role', 'role', 'PUT'),
        ('/api/roles/{authority_id}', 'Delete role', 'role', 'DELETE'),
        ('/api/roles/{authority_id}/users', 'Get role users', 'role', 'GET'),
        ('/api/roles/{authority_id}/users', 'Update role users', 'role', 'PUT'),
        ('/api/roles/data-authority', 'Update role data authority', 'role', 'PUT'),
        ('/api/routes', 'List APIs', 'api', 'GET'),
        ('/api/routes', 'Create API', 'api', 'POST'),
        ('/api/routes/all', 'List all APIs', 'api', 'GET'),
        ('/api/routes/groups', 'List API groups', 'api', 'GET'),
        ('/api/routes/casbin/refresh', 'Refresh API policies', 'api', 'POST'),
        ('/api/routes/authority', 'Get role APIs', 'api', 'GET'),
        ('/api/routes/role-matrix', 'Get API role matrix', 'api', 'GET'),
        ('/api/routes/roles', 'Get API roles', 'api', 'GET'),
        ('/api/routes/roles', 'Update API roles', 'api', 'PUT'),
        ('/api/routes/{id}', 'Get API', 'api', 'GET'),
        ('/api/routes/{id}', 'Update API', 'api', 'PUT'),
        ('/api/routes/{id}', 'Delete API', 'api', 'DELETE'),
        ('/api/routes/batch', 'Delete APIs', 'api', 'DELETE'),
        ('/api/attachment-categories', 'List attachment categories', 'file', 'GET'),
        ('/api/attachment-categories', 'Create attachment category', 'file', 'POST'),
        ('/api/attachment-categories/{id}', 'Delete attachment category', 'file', 'DELETE'),
        ('/api/files', 'List files', 'file', 'GET'),
        ('/api/files/import-url', 'Import file URL', 'file', 'POST'),
        ('/api/files/upload', 'Upload file', 'file', 'POST'),
        ('/api/files/{id}', 'Delete file', 'file', 'DELETE'),
        ('/api/files/{id}/name', 'Rename file', 'file', 'PATCH'),
        ('/api/params', 'List params', 'params', 'GET'),
        ('/api/params', 'Create param', 'params', 'POST'),
        ('/api/params/by-key', 'Get param by key', 'params', 'GET'),
        ('/api/params/batch', 'Delete params', 'params', 'DELETE'),
        ('/api/params/{id}', 'Get param', 'params', 'GET'),
        ('/api/params/{id}', 'Update param', 'params', 'PUT'),
        ('/api/params/{id}', 'Delete param', 'params', 'DELETE'),
        ('/api/dictionaries', 'List dictionaries', 'dictionary', 'GET'),
        ('/api/dictionaries', 'Create dictionary', 'dictionary', 'POST'),
        ('/api/dictionaries/import', 'Import dictionary', 'dictionary', 'POST'),
        ('/api/dictionaries/{id}', 'Get dictionary', 'dictionary', 'GET'),
        ('/api/dictionaries/{id}', 'Update dictionary', 'dictionary', 'PUT'),
        ('/api/dictionaries/{id}', 'Delete dictionary', 'dictionary', 'DELETE'),
        ('/api/dictionaries/{id}/export', 'Export dictionary', 'dictionary', 'GET'),
        ('/api/dictionaries/{id}/details/tree', 'Get dictionary details tree', 'dictionary', 'GET'),
        ('/api/dictionary-details', 'Create dictionary detail', 'dictionary', 'POST'),
        ('/api/dictionary-details/tree-by-type', 'Get dictionary detail tree by type', 'dictionary', 'GET'),
        ('/api/dictionary-details/by-parent', 'Get dictionary details by parent', 'dictionary', 'GET'),
        ('/api/dictionary-details/{id}', 'Get dictionary detail', 'dictionary', 'GET'),
        ('/api/dictionary-details/{id}', 'Update dictionary detail', 'dictionary', 'PUT'),
        ('/api/dictionary-details/{id}', 'Delete dictionary detail', 'dictionary', 'DELETE'),
        ('/api/dictionary-details/{id}/path', 'Get dictionary detail path', 'dictionary', 'GET'),
        ('/api/login-logs', 'List login logs', 'logs', 'GET'),
        ('/api/login-logs', 'Delete login logs', 'logs', 'DELETE'),
        ('/api/login-logs/{id}', 'Get login log', 'logs', 'GET'),
        ('/api/login-logs/{id}', 'Delete login log', 'logs', 'DELETE'),
        ('/api/operation-logs', 'List operation logs', 'logs', 'GET'),
        ('/api/operation-logs', 'Delete operation logs', 'logs', 'DELETE'),
        ('/api/operation-logs/{id}', 'Delete operation log', 'logs', 'DELETE'),
        ('/api/system/config', 'Get system config', 'system', 'GET'),
        ('/api/system/config', 'Update system config', 'system', 'PUT'),
        ('/api/system/server-info', 'Get server info', 'system', 'GET'),
        ('/api/system/reload', 'Reload system', 'system', 'POST')
)
insert into sys_apis (path, description, api_group, method)
select path, description, api_group, method
from api_seed
on conflict (path, method) do update
set description = excluded.description,
    api_group = excluded.api_group;
