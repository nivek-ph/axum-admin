-- Optional demo data for a fictional 50-person company.
--
-- Run after the required bootstrap command:
--   cargo run -p ava init
--   set -a; source .env; set +a
--   psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f docs/sql/init.sql
--
-- This file is intentionally outside migrations/. Schema migrations must not
-- install demo business data when the server starts.

BEGIN;

-- Reuse the required root department created by migrations, but give it a
-- company-facing name when it still has the default seed name.
UPDATE sys_depts
SET name = '明日科技',
    updated_at = now()
WHERE code = 'head_office'
  AND name = 'Head Office';

-- A deliberately shallow organization suitable for about 50 people:
-- management 3, rd 18, sales 10, customer success 5,
-- hr 4, finance 3, marketing 4, operations 3.
INSERT INTO sys_depts (parent_id, name, code, sort, status)
SELECT root.id, seed.name, seed.code, seed.sort, 'enabled'
FROM (
    VALUES
        ('管理层', 'management', 10),
        ('产品与研发部', 'rd', 20),
        ('销售部', 'sales', 30),
        ('客户成功部', 'customer_success', 40),
        ('人事行政部', 'hr', 50),
        ('财务部', 'finance', 60),
        ('市场部', 'marketing', 70),
        ('运营部', 'operations', 80)
) AS seed(name, code, sort)
CROSS JOIN (
    SELECT id
    FROM sys_depts
    WHERE code = 'head_office'
) AS root
ON CONFLICT (code) DO NOTHING;

INSERT INTO sys_roles (code, name, status, sort, data_scope, is_system)
VALUES
    ('executive', '管理层', 'enabled', 100, 'all', false),

    ('rd_manager', '研发负责人', 'enabled', 200, 'dept_and_children', false),
    ('product_manager', '产品经理', 'enabled', 210, 'self', false),
    ('frontend_engineer', '前端工程师', 'enabled', 220, 'self', false),
    ('backend_engineer', '后端工程师', 'enabled', 230, 'self', false),
    ('ui_ux_designer', 'UI/UX 设计师', 'enabled', 240, 'self', false),
    ('qa_engineer', '测试工程师', 'enabled', 250, 'self', false),
    ('devops_engineer', 'DevOps 工程师', 'enabled', 260, 'self', false),

    ('sales_manager', '销售负责人', 'enabled', 300, 'dept_and_children', false),
    ('sales_representative', '销售顾问', 'enabled', 310, 'self', false),

    ('customer_success_manager', '客户成功负责人', 'enabled', 400, 'dept_and_children', false),
    ('customer_success_specialist', '客户成功专员', 'enabled', 410, 'self', false),

    ('people_admin', '人事负责人', 'enabled', 500, 'all', false),
    ('recruiter', '招聘专员', 'enabled', 510, 'self', false),
    ('admin_specialist', '行政专员', 'enabled', 520, 'self', false),

    ('finance_manager', '财务负责人', 'enabled', 600, 'dept_and_children', false),
    ('accountant', '会计', 'enabled', 610, 'self', false),

    ('marketing_manager', '市场负责人', 'enabled', 700, 'dept_and_children', false),
    ('content_marketer', '内容运营', 'enabled', 710, 'self', false),
    ('growth_marketer', '增长运营', 'enabled', 720, 'self', false),

    ('operations_manager', '运营负责人', 'enabled', 800, 'dept_and_children', false),
    ('operations_specialist', '运营专员', 'enabled', 810, 'self', false),

    ('system_operator', '系统管理员', 'enabled', 900, 'all', false),
    ('auditor', '审计员', 'enabled', 910, 'all', false)
ON CONFLICT (code) DO UPDATE
SET name = EXCLUDED.name,
    status = EXCLUDED.status,
    sort = EXCLUDED.sort,
    data_scope = EXCLUDED.data_scope,
    updated_at = now()
WHERE (
    sys_roles.name,
    sys_roles.status,
    sys_roles.sort,
    sys_roles.data_scope
) IS DISTINCT FROM (
    EXCLUDED.name,
    EXCLUDED.status,
    EXCLUDED.sort,
    EXCLUDED.data_scope
);

-- Every job role can access the dashboard. Department heads also receive
-- organization visibility scoped by their role's data_scope. Menu assignments
-- use stable names rather than frontend component paths.
WITH role_menu_names(role_code, menu_name) AS (
    VALUES
        ('executive', 'dashboard'),
        ('rd_manager', 'dashboard'),
        ('product_manager', 'dashboard'),
        ('frontend_engineer', 'dashboard'),
        ('backend_engineer', 'dashboard'),
        ('ui_ux_designer', 'dashboard'),
        ('qa_engineer', 'dashboard'),
        ('devops_engineer', 'dashboard'),
        ('sales_manager', 'dashboard'),
        ('sales_representative', 'dashboard'),
        ('customer_success_manager', 'dashboard'),
        ('customer_success_specialist', 'dashboard'),
        ('people_admin', 'dashboard'),
        ('recruiter', 'dashboard'),
        ('admin_specialist', 'dashboard'),
        ('finance_manager', 'dashboard'),
        ('accountant', 'dashboard'),
        ('marketing_manager', 'dashboard'),
        ('content_marketer', 'dashboard'),
        ('growth_marketer', 'dashboard'),
        ('operations_manager', 'dashboard'),
        ('operations_specialist', 'dashboard'),

        ('executive', 'organization'),
        ('executive', 'users'),
        ('rd_manager', 'organization'),
        ('rd_manager', 'users'),
        ('sales_manager', 'organization'),
        ('sales_manager', 'users'),
        ('customer_success_manager', 'organization'),
        ('customer_success_manager', 'users'),
        ('people_admin', 'organization'),
        ('people_admin', 'users'),
        ('finance_manager', 'organization'),
        ('finance_manager', 'users'),
        ('marketing_manager', 'organization'),
        ('marketing_manager', 'users'),
        ('operations_manager', 'organization'),
        ('operations_manager', 'users'),

        ('people_admin', 'users:create'),
        ('people_admin', 'users:update'),
        ('people_admin', 'users:reset-password'),
        ('people_admin', 'users:assign-roles'),
        ('people_admin', 'roles'),
        ('people_admin', 'roles:users-read'),
        ('people_admin', 'roles:users-update'),
        ('people_admin', 'departments'),
        ('people_admin', 'departments:create'),
        ('people_admin', 'departments:get'),
        ('people_admin', 'departments:update'),

        ('system_operator', 'dashboard'),
        ('system_operator', 'system'),
        ('system_operator', 'params'),
        ('system_operator', 'params:create'),
        ('system_operator', 'params:get'),
        ('system_operator', 'params:update'),
        ('system_operator', 'params:delete'),
        ('system_operator', 'params:batch-delete'),
        ('system_operator', 'dictionaries'),
        ('system_operator', 'dictionaries:create'),
        ('system_operator', 'dictionaries:update'),
        ('system_operator', 'dictionaries:delete'),
        ('system_operator', 'dictionaries:import'),
        ('system_operator', 'dictionaries:export'),
        ('system_operator', 'dictionary-details:create'),
        ('system_operator', 'dictionary-details:update'),
        ('system_operator', 'dictionary-details:delete'),
        ('system_operator', 'files'),
        ('system_operator', 'files:import-url'),
        ('system_operator', 'files:upload'),
        ('system_operator', 'files:delete'),
        ('system_operator', 'files:rename'),

        ('auditor', 'dashboard'),
        ('auditor', 'audit'),
        ('auditor', 'audit-events'),

        ('executive', 'audit'),
        ('executive', 'audit-events')
)
INSERT INTO sys_role_menus (role_id, menu_id)
SELECT role.id, menu.id
FROM role_menu_names assignment
JOIN sys_roles role ON role.code = assignment.role_code
JOIN sys_menus menu ON menu.name = assignment.menu_name
ON CONFLICT DO NOTHING;

-- Demo accounts use a valid Argon2id hash that is not tied to a distributed
-- plaintext password. They can be viewed and managed but cannot be logged into
-- with a documented shared credential.
CREATE TEMP TABLE demo_employee_seed (
    employee_no INTEGER PRIMARY KEY,
    dept_code TEXT NOT NULL,
    job_role_code TEXT NOT NULL
) ON COMMIT DROP;

-- Keep department and job-role membership in one source of truth.
INSERT INTO demo_employee_seed (employee_no, dept_code, job_role_code)
SELECT employee_no, ranges.dept_code, ranges.job_role_code
FROM (
    VALUES
        ('management', 'executive', 1, 3),
        ('rd', 'rd_manager', 4, 4),
        ('rd', 'product_manager', 5, 6),
        ('rd', 'frontend_engineer', 7, 10),
        ('rd', 'backend_engineer', 11, 15),
        ('rd', 'ui_ux_designer', 16, 17),
        ('rd', 'qa_engineer', 18, 20),
        ('rd', 'devops_engineer', 21, 21),
        ('sales', 'sales_manager', 22, 22),
        ('sales', 'sales_representative', 23, 31),
        ('customer_success', 'customer_success_manager', 32, 32),
        ('customer_success', 'customer_success_specialist', 33, 36),
        ('hr', 'people_admin', 37, 38),
        ('hr', 'recruiter', 39, 39),
        ('hr', 'admin_specialist', 40, 40),
        ('finance', 'finance_manager', 41, 41),
        ('finance', 'accountant', 42, 43),
        ('marketing', 'marketing_manager', 44, 44),
        ('marketing', 'content_marketer', 45, 46),
        ('marketing', 'growth_marketer', 47, 47),
        ('operations', 'operations_manager', 48, 48),
        ('operations', 'operations_specialist', 49, 50)
) AS ranges(dept_code, job_role_code, first_employee_no, last_employee_no)
CROSS JOIN LATERAL generate_series(
    ranges.first_employee_no,
    ranges.last_employee_no
) AS employee_no;

INSERT INTO sys_users (
    uuid,
    username,
    password_hash,
    nick_name,
    header_img,
    home_route,
    enable,
    phone,
    email,
    origin_setting,
    dept_id,
    is_system
)
SELECT
    format('00000000-0000-4000-8000-%s', lpad(seed.employee_no::text, 12, '0')),
    format('employee_%s', lpad(seed.employee_no::text, 2, '0')),
    '$argon2id$v=19$m=19456,t=2,p=1$ZGVtby1hY2NvdW50LXNhbHQ$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA',
    format('示例员工 %s', lpad(seed.employee_no::text, 2, '0')),
    '',
    'dashboard',
    true,
    NULL,
    format('employee%s@example.test', lpad(seed.employee_no::text, 2, '0')),
    '{"demo": true}'::jsonb,
    dept.id,
    false
FROM demo_employee_seed seed
JOIN sys_depts dept ON dept.code = seed.dept_code
ON CONFLICT DO NOTHING;

-- Every employee gets one job role. A few people also receive cross-cutting
-- system-operation or audit access.
WITH functional_role_members(role_code, employee_no) AS (
    VALUES
        ('system_operator', 48),
        ('system_operator', 49),
        ('auditor', 41),
        ('auditor', 42)
),
role_members(role_code, employee_no) AS (
    SELECT job_role_code, employee_no
    FROM demo_employee_seed
    UNION ALL
    SELECT role_code, employee_no
    FROM functional_role_members
)
INSERT INTO sys_user_roles (user_id, role_id)
SELECT user_account.id, role.id
FROM role_members assignment
JOIN sys_roles role ON role.code = assignment.role_code
JOIN sys_users user_account
    ON user_account.username = format(
        'employee_%s',
        lpad(assignment.employee_no::text, 2, '0')
    )
ON CONFLICT DO NOTHING;

-- Harmless sample metadata keeps the parameter and dictionary screens useful.
-- These parameters are illustrative and are not consumed by the runtime.
INSERT INTO sys_params (name, "key", value, "desc")
VALUES
    ('公司名称', 'demo.company.name', '明日科技', '演示数据，未接入运行时配置'),
    ('公司人数', 'demo.company.headcount', '50', '演示数据，未接入运行时配置'),
    ('默认时区', 'demo.company.timezone', 'Asia/Shanghai', '演示数据，未接入运行时配置')
ON CONFLICT ("key") DO NOTHING;

INSERT INTO sys_dictionaries (name, type, status, "desc", parent_id)
VALUES
    ('员工状态', 'demo_employee_status', true, '演示员工状态', NULL),
    ('用工类型', 'demo_employment_type', true, '演示用工类型', NULL)
ON CONFLICT (type) DO NOTHING;

WITH dictionary_values(dictionary_type, label, value, sort) AS (
    VALUES
        ('demo_employee_status', '在职', 'active', 10),
        ('demo_employee_status', '休假', 'leave', 20),
        ('demo_employee_status', '离职', 'departed', 30),
        ('demo_employment_type', '正式员工', 'full_time', 10),
        ('demo_employment_type', '实习生', 'intern', 20),
        ('demo_employment_type', '外部顾问', 'contractor', 30)
)
INSERT INTO sys_dictionary_details (
    label,
    value,
    extend,
    status,
    sort,
    sys_dictionary_id,
    parent_id,
    level,
    path
)
SELECT
    seed.label,
    seed.value,
    '',
    true,
    seed.sort,
    dictionary.id,
    NULL,
    0,
    ''
FROM dictionary_values seed
JOIN sys_dictionaries dictionary ON dictionary.type = seed.dictionary_type
WHERE NOT EXISTS (
    SELECT 1
    FROM sys_dictionary_details existing
    WHERE existing.sys_dictionary_id = dictionary.id
      AND existing.value = seed.value
);

COMMIT;
