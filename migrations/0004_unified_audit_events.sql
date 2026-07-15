CREATE TABLE sys_audit_events (
    id BIGSERIAL PRIMARY KEY,
    actor_id BIGINT,
    actor_label TEXT NOT NULL,
    action TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id TEXT,
    result TEXT NOT NULL CHECK (result IN ('succeeded', 'denied', 'failed')),
    reason_code TEXT,
    source_ip TEXT NOT NULL DEFAULT '',
    user_agent TEXT NOT NULL DEFAULT '',
    changes JSONB NOT NULL DEFAULT '[]'::JSONB CHECK (jsonb_typeof(changes) = 'array'),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_sys_audit_events_actor ON sys_audit_events(actor_id, created_at DESC);
CREATE INDEX idx_sys_audit_events_action ON sys_audit_events(action, created_at DESC);
CREATE INDEX idx_sys_audit_events_resource
    ON sys_audit_events(resource_type, resource_id, created_at DESC);
CREATE INDEX idx_sys_audit_events_result ON sys_audit_events(result, created_at DESC);

INSERT INTO sys_audit_events (
    actor_id,
    actor_label,
    action,
    resource_type,
    resource_id,
    result,
    reason_code,
    source_ip,
    user_agent,
    created_at
)
SELECT
    user_id,
    username,
    'auth.login',
    'account',
    username,
    CASE WHEN status THEN 'succeeded' ELSE 'denied' END,
    CASE
        WHEN status THEN NULL
        WHEN error_message = 'captcha is required' THEN 'captcha_required'
        WHEN error_message = 'captcha is invalid or expired' THEN 'captcha_invalid'
        WHEN error_message = 'invalid username or password' THEN 'invalid_credentials'
        WHEN error_message = 'user is disabled' THEN 'user_disabled'
        ELSE 'login_failed'
    END,
    ip,
    agent,
    created_at
FROM sys_login_logs;

INSERT INTO sys_audit_events (
    actor_id,
    actor_label,
    action,
    resource_type,
    resource_id,
    result,
    reason_code,
    source_ip,
    user_agent,
    created_at
)
SELECT
    r.user_id,
    COALESCE(u.username, r.user_id::TEXT),
    'legacy.http_request',
    'route',
    r.path,
    CASE WHEN r.status < 400 THEN 'succeeded' ELSE 'failed' END,
    CASE WHEN r.status < 400 THEN NULL ELSE 'http_status_' || r.status::TEXT END,
    r.ip,
    r.agent,
    r.created_at
FROM sys_operation_records r
LEFT JOIN sys_users u ON u.id = r.user_id;

DROP TABLE sys_login_logs;
DROP TABLE sys_operation_records;

DELETE FROM sys_role_menus WHERE menu_id = 42;
DELETE FROM sys_menu_apis WHERE menu_id IN (41, 42);

UPDATE sys_menus
SET path = '/audit-events',
    name = 'audit-events',
    component = 'view/logs/audit.vue',
    sort = 10,
    title = 'Audit Events',
    icon = 'history',
    permission = 'system:audit-event:list',
    updated_at = now()
WHERE id = 41;

DELETE FROM sys_menus WHERE id = 42;

INSERT INTO sys_menu_apis (menu_id, method, path_pattern)
VALUES
    (41, 'GET', '/api/audit/events'),
    (41, 'GET', '/api/audit/events/{id}');
