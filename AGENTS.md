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

### Error Design

- Route and middleware handlers should return `api::AppResult<T>`.
- `crates/api` owns the public HTTP boundary types `AppError`, `AppResult<T>`, and `ApiResponse<T>`.
- Repeated fixed HTTP contracts may use crate-private `ErrorSpec` constants. Consume them with ordinary `ok_or` and `?`; do not add per-error constructor helpers or extension traits.
- Keep stable error specs in the owning layer:
  - domain errors: the owning capability crate's local `error.rs` or `errors.rs`
  - API boundary errors: `crates/api/src/mappings.rs`, with route-local errors only for multi-capability workflows such as login
- Add `impl From<...> for AppError` only when the source error has one stable API meaning in every context.
- When the same error type has context-specific semantics, map it explicitly at the call site with `.map_err(...)`.
- Keep user-management and authentication errors distinct:
  - CRUD/user management returns `UserError` from `crates/iam/src/users`.
  - Login returns `AuthenticateError`; unknown users and incorrect passwords both become `INVALID_CREDENTIALS` to avoid account enumeration.
  - Auth middleware loads an Access Snapshot; `AccessEvaluationError` maps a missing/deleted token user to `SESSION_INVALID` and a disabled user to `USER_DISABLED`.

## Frontend

- The Admin Console is React + Vite + React Router + Zustand + TanStack Query + Axios + Radix UI.
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
cargo run -p ava -- serve
cd apps/desktop && pnpm dev
```

Bootstrap login:

```text
ADMIN_USERNAME / ADMIN_PASSWORD from the environment
```

Before claiming a change is complete, report the exact verification commands that were run and whether they passed.
