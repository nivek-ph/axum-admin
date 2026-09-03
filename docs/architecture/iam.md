# IAM architecture

This document is the canonical contract for identity and access management in axum-admin.

## Capability boundaries

| Capability | Owns | Does not own |
| --- | --- | --- |
| Request Access | Token/session/User validation and one authorization decision for each protected management request | Account or Role administration |
| Accounts | User metadata, password lifecycle, department boundary, assigned Role workflow | Permission policy persistence |
| Roles | Role metadata, lifecycle, and the Role Access workflow | User metadata or direct user grants |
| Menus | Access Catalog loading and navigation projection | Authorization decisions, policy writes, or HTTP topology |
| Authorization | Casbin model, policy validation, Role membership, effective Permission evaluation, reload propagation | Public HTTP DTOs |
| Audit | Append-only administrative events | Authoritative IAM state |

Router-level Authentication validates the token/session and checks that the User still exists and is
enabled. It inserts the authenticated User and crate-private Permission guard dependencies into the
request. A route-local Permission guard performs one Authorization decision for each protected
management method. Self-Service Access is a protected route without a Permission guard; it is not a
special Permission or a separate admission type.

The token/session check uses the session store. User existence and enabled status, User-to-Role
membership, Role enabled status, and concrete Role Permissions are read from one process-local
last-good Authorization snapshot. Authentication and Permission evaluation do not query PostgreSQL
on the request path.

Each management method declares one concrete Permission beside its handler registration with
`.permission(code)` on its `MethodRouter`. HTTP topology belongs only to Axum route
registration. The Access Catalog does not store HTTP methods or paths, Router construction does not
compare routes with PostgreSQL, and Request Access does not resolve Permissions from request paths.
Handlers call Accounts or Roles for administration; they never extract the private guard context or
call private Authorization directly.

## Domain model

### User and Role

A User may have zero, one, or multiple Roles. Membership is a Casbin `g` rule:

```text
g, user:<id>, role:<id>
```

Role codes are metadata, not runtime authorization bypasses. Disabled Roles retain their policy and
memberships but contribute no effective Permission. Existing membership in a disabled Role may be
kept or removed; a disabled Role cannot be newly assigned.

A zero-Role User remains valid and receives only explicit self-service behavior. No implicit Role
or catalog grant is created.

### Permission

A Permission is a concrete, enabled code from the Access Catalog. A Role grant is a Casbin `p` rule:

```text
p, role:<id>, system:user:list
```

The model is additive and allow-only:

```text
effective permissions(user)
  = union(concrete permissions of the user's enabled roles)
```

There are no Direct Permissions, deny rules, wildcard grants, Role inheritance, or configurable
Data Scope. These require an explicit architecture change.

## One Role Access tree

Role Management exposes one tree containing directory, page, and action nodes:

- directories are structural and never become policy rows;
- selecting an action includes the Permission of its owning page;
- selecting a page does not include any action Permission;
- deselecting a page removes its selected actions;
- the backend normalizes and validates the submitted final Permission set.

`GET /api/roles/{id}/access` returns the tree and selected concrete Permissions.
`PUT /api/roles/{id}/access` replaces that Role's concrete final Permission set.

Navigation is a projection, not a second authorization store. Menus selects pages whose page
Permission is effective, then adds their directory ancestors. Action Permissions never create
navigation. The former `sys_role_menus` relation is not part of the current model.

## User Access

User Access exposes only Assigned Roles and Effective Permissions:

- `GET /api/users/{id}/access` returns assigned Roles, including disabled assignments, and the
  effective Permissions contributed by enabled Roles;
- `PUT /api/users/{id}/roles` replaces the final assigned Role set;
- no supported route or response field exposes Direct Permissions.

Only an active member of the protected `super_admin` Role may use access-administration operations.
Ordinary Permission grants do not elevate an account into an access administrator. This domain rule
is enforced even when middleware request admission succeeds.

## Role lifecycle

Normal Roles may be created, edited, enabled, disabled, and deleted. Deleting a Role that still has
members is rejected. An unassigned normal Role is physically deleted with its Casbin policy through
the supported management workflow.

The Role with code `super_admin` is concrete and protected:

- it is enabled;
- it has a concrete grant for every enabled catalog Permission;
- its code, status, access, and deletion cannot be changed through supported APIs;
- its memberships are mutable, including removal of the final active membership.

Removing every active `super_admin` membership intentionally leaves no access administrator. There
is no hidden bootstrap bypass or final-member invariant. Recovery is a deliberate database operation
that inserts a valid `g` membership and then reloads or restarts the service.

For an authorized emergency repair, replace `admin` below with the intended enabled account name,
run the statement against the correct PostgreSQL database, and then reload or restart every service
instance. The empty `v2` through `v5` values are required by the `casbin_rule` schema.

```sql
INSERT INTO casbin_rule (ptype, v0, v1, v2, v3, v4, v5)
SELECT
    'g',
    'user:' || users.id,
    'role:' || roles.id,
    '', '', '', ''
FROM sys_users AS users
CROSS JOIN sys_roles AS roles
WHERE users.username = 'admin'
  AND users.enable = true
  AND roles.code = 'super_admin'
  AND roles.status = 'enabled'
ON CONFLICT (ptype, v0, v1, v2, v3, v4, v5) DO NOTHING;
```

If the statement inserts no row, verify the account name and the protected Role before retrying;
do not create a wildcard policy or a bypass Role as a substitute.

Catalog migrations must add the corresponding concrete `super_admin` grant in the same migration.
Startup validation rejects a missing or disabled protected Role, incomplete concrete grants,
wildcard policy, Direct Permission rows, Role inheritance, unknown subjects, and unknown or disabled
Permissions.

## Persistence and consistency

PostgreSQL is authoritative. Casbin's `SqlxAdapter` is the only application policy persistence
boundary. Application code reads and writes `p` and `g` rules through Casbin Management APIs; it
must not query or modify `casbin_rule` directly.

PostgreSQL retains the Access Catalog directory/page/action tree and its enabled Permission codes,
but no HTTP method/path bindings. `sys_menu_apis` is not part of the current schema or domain model.
The code registration is the sole owner of which Permission guards a management handler.

Casbin Management APIs use `SqlxAdapter` autosave, and each individual Adapter mutation keeps its
own database transaction. A final-set replacement may require one bulk add and one bulk remove; the
design does not add a global policy lock, cross-store compensation, or speculative conflict
protocol. If real concurrent conflicts are observed, solve the measured case rather than expanding
the model pre-emptively.

Policy mutations follow this order:

1. validate the requested domain final set;
2. write policy through Casbin Management APIs and `SqlxAdapter`;
3. publish a Redis reload notification after success;
4. append the audit event on a best-effort basis.

Audit is intentionally non-atomic with policy. If audit storage fails, the committed policy remains
successful and the service emits a high-priority structured error containing the action, resource,
and request identifier.

Every process reloads the complete policy, User status, and Role status into a candidate snapshot and
validates it before swapping the active instance. User and Role status mutations update the local
snapshot and publish a Redis reload notification; Casbin policy saves publish the same class of
notification. Periodic reload repairs missed notifications. A failed reload retains the last
successfully loaded snapshot. Requests fail closed for evaluation errors, but policy freshness is
not strictly fail closed.

Invalid policy at startup is fatal and prevents the application from serving requests.

## Stable errors

The owning capability returns explicit domain errors and `crates/api/src/mappings.rs` maps them to
stable public codes. Important contracts include:

- ordinary access-administration attempts return `PERMISSION_DENIED`;
- assigning an unknown or newly disabled Role returns `INVALID_ROLES`;
- deleting a Role with members returns `ROLE_HAS_MEMBERS`;
- invalid Role Access returns `INVALID_ROLE_ACCESS`;
- protected Role mutations return `ROLE_IMMUTABLE`;
- policy evaluation or load failures become closed authorization errors.

## Required verification

IAM changes require the narrowest relevant checks followed by the shared suites:

```bash
cargo test --workspace
cd apps/desktop && pnpm test
cd apps/desktop && pnpm build
```

For backend/frontend integration, start both applications and exercise the real Admin Console.
Acceptance includes Role Access normalization, one/multiple/zero Roles, disabled Role behavior,
member-protected deletion, protected `super_admin`, ordinary-user denial, navigation from page
Permissions, action authorization, reload propagation, restart persistence, audit success and
injected audit failure, and final-membership removal followed by manual recovery.

Request Access verification also covers Authentication on every protected route, Self-Service for a
zero-Role User, route-local denial and allowance for concrete Permissions, missing guard context
failing closed, `404`/`405`/`HEAD` layer behavior, stable errors, and best-effort denial audit with
the actual request path.
