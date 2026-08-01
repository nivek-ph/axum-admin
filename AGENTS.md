# axum-admin

This file gives repo-specific guidance for agents working in this project.

## Project Shape

- Backend process entry points live in `apps/ava`; the Axum HTTP capability crates live under `crates/`.
- The React Admin Console lives in `apps/desktop` and runs as a browser SPA.
- The previous Vue application and its Tauri wrapper are preserved only in the `v1.1.0` tag.
- Database migrations live in `migrations/`.
- Uploaded local files are served from `uploads/`; do not commit generated upload data.

## Backend

- Use REST-style routes under `/api`.
- Public routes are registered in `crates/api/src/routes/public`.
- Authenticated routes are registered in `crates/api/src/routes/protected` and use the `Authorization: Bearer <token>` header.
- Keep response bodies in the shared envelope shape:

```json
{
  "code": "OK",
  "message": "ok",
  "data": {}
}
```

- Use `api::AppError` and `crates/api/src/mappings.rs` for stable error codes and messages.
- Keep business logic in the owning capability crate (`crates/iam`, `crates/audit`, `crates/metadata`, `crates/file-storage`, etc.) rather than pushing it into route handlers.
- When adding SQL schema changes, create a new migration in `migrations/`; do not edit an already-applied migration unless the user explicitly confirms the database can be reset.
- Keep `sqlx::migrate!("../../migrations")` working from `crates/db`.
- Prefer explicit domain errors over generic string errors.

### IAM

- Read [`docs/architecture/iam.md`](docs/architecture/iam.md) before changing IAM, protected API
  routes, the access catalog, authentication middleware, or Admin Console access workflows.
- Keep Request Access, Accounts, Roles, Menus, and private Authorization responsibilities distinct.
  Axum middleware performs token/session work and calls `AccessService::evaluate` exactly once for
  IAM request admission; HTTP handlers call Accounts or Roles for access administration and never
  call private Authorization directly.
- PostgreSQL is authoritative. `sys_role_menus` stores Page Access; `casbin_rule` stores concrete
  Permission policy (`p`) and user-role membership (`g`). Redis only propagates reload notifications
  and must not become an authorization source of truth.
- Page Access controls navigation, not backend authorization. Selecting a page for a Role must
  atomically include that page's entry Permission; operation/action Permissions remain independent.
  Direct Permissions remain valid without Page Access and must not silently create navigation.
- Effective Permissions are the additive union of Direct Permissions and Permissions inherited from
  enabled Roles. Do not add deny rules, wildcard grants, `is_system` authorization flags, frontend
  role-code bypasses, or configurable Data Scope without an explicit architecture change.
- Preserve the concrete protected `super_admin` model and the final-active-member invariant. Catalog
  additions and their concrete `super_admin` Page Access and Permission grants belong in the same
  migration change set.
- Authorization mutations must preserve PostgreSQL transaction boundaries, deterministic locks,
  post-commit reload semantics, periodic repair, and fail-closed request error mappings. Do not claim
  strict fail-closed policy freshness: the current runtime retains the last successfully loaded
  Enforcer when a reload fails; see the documented consistency boundary.

### Error Design

- Route and middleware handlers should return `api::AppResult<T>`.
- `crates/api` owns the public HTTP boundary types `AppError`, `AppResult<T>`, and `ApiResponse<T>`.
- Repeated fixed HTTP contracts may use crate-private `ErrorSpec` constants. Consume them with ordinary `ok_or` and `?`; do not add per-error constructor helpers or extension traits.
- Keep stable error specs in the owning layer:
  - domain errors: the owning capability crate's local `error.rs` or `errors.rs`
  - API boundary errors: `crates/api/src/mappings.rs`, with route-local errors only for multi-capability workflows such as login
- Keep stable, context-independent conversions from private implementation errors into a domain error in the owning module's `error.rs`; service code should propagate them with `?`.
- Add `impl From<...> for AppError` only when the source error has one stable API meaning in every context.
- When the same error type has context-specific semantics, map it explicitly at the call site with `.map_err(...)`.
- Keep user-management and authentication errors distinct:
  - CRUD/user management returns `AccountError` from `crates/iam/src/accounts`.
  - Login uses the route-local `LoginError`; unknown users and incorrect passwords both become `INVALID_CREDENTIALS`.
  - Auth middleware calls `AccessService::evaluate`; `AccessEvaluationError` maps a missing/deleted token user to `SESSION_INVALID` and a disabled user to `USER_DISABLED`.

## Frontend

- The Admin Console is React + Vite + React Router + Zustand + TanStack Query + TanStack Table + Axios + shadcn/ui on Base UI, with Tabler Icons.
- API wrappers live in `apps/desktop/src/api`; keep endpoint paths aligned with `crates/api/src/routes`.
- Keep the default API base URL as `http://127.0.0.1:3000/api` unless changing the runtime contract intentionally.
- Use the shared HTTP client in `apps/desktop/src/api/http.ts` so backend envelope errors surface through the same path.
- Keep UI changes consistent with the existing admin layout: dense, practical, and workflow-oriented.
- Add or update Vitest coverage when changing API wrappers, stores, router behavior, or view workflows.

## Rust Style

- Use the workspace dependencies declared in the root `Cargo.toml`.
- Keep local workspace crates listed before third-party dependencies.
- Prefer small modules with clear ownership over broad shared helpers.
- Avoid helper functions that are only used once unless they clarify a complex block.
- When using `format!`, inline variables in `{}` when possible.
- Prefer exhaustive `match` arms over wildcard arms when the enum is local and meaningful.
- Run formatting after Rust edits:

```bash
cargo fmt --all
```

## Verification

Use the narrowest meaningful check first, then broaden when shared behavior changed.

Backend:

```bash
cargo test --workspace
```

Frontend:

```bash
cd apps/desktop
pnpm test
pnpm build
```

For frontend/backend integration changes, run both servers and verify the real UI path:

```bash
cargo run -p ava serve
cd apps/desktop && pnpm dev
```

Bootstrap login:

```text
ADMIN_USERNAME / ADMIN_PASSWORD from the environment
```

Before claiming a change is complete, report the exact verification commands that were run and whether they passed.

## Agent skills

### Issue tracker

When the local `.notes/` tracker is present, use `.notes/agents/issue-tracker.md` for planning specs
and implementation issues. `.notes/` is local temporary working material and is not a source for
committed project documentation. When a design or decision needs to be retained or committed, move
the reviewed content into the appropriate location under `docs/` and update tracked references to
point there; do not make committed documentation depend on `.notes/`.

### Triage labels

When present, the local tracker uses the five-role vocabulary in `.notes/agents/triage-labels.md`.

### Domain docs

Tracked, current architecture is under `docs/architecture/`; IAM's canonical implementation document
is [`docs/architecture/iam.md`](docs/architecture/iam.md). Local `.notes/` files hold temporary
planning context and task history and may describe superseded targets; use them for provenance, not
as durable architecture or evidence that behavior is implemented. See `.notes/agents/domain.md` when
the local tracker is available.
