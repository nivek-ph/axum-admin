# axum-vue-admin

This file gives repo-specific guidance for agents working in this project.

## Project Shape

- Backend code lives in `apps/api` and workspace crates under `crates/`.
- Frontend code lives in `apps/desktop`.
- Database migrations live in `migrations/`.
- Uploaded local files are served from `uploads/`; do not commit generated upload data.

## Backend

- Use REST-style routes under `/api`.
- Public routes are registered in `apps/api/src/routes/public`.
- Authenticated routes are registered in `apps/api/src/routes/protected` and use the `Authorization: Bearer <token>` header.
- Keep response bodies in the shared envelope shape:

```json
{
  "code": "OK",
  "message": "ok",
  "data": {}
}
```

- Use `admin_httpz::AppError` and the local `errors.rs` modules for stable error codes and messages.
- Keep business logic in the owning domain crate (`crates/system`, `crates/file-storage`, etc.) rather than pushing it into route handlers.
- When adding SQL schema changes, create a new migration in `migrations/`; do not edit an already-applied migration unless the user explicitly confirms the database can be reset.
- Keep `sqlx::migrate!("../../migrations")` working from `apps/api`.
- Prefer explicit domain errors over generic string errors.

### Error Design

- Route and middleware handlers should return `admin_httpz::AppResult<T>`.
- `crates/httpz` owns the shared HTTP boundary types: `AppError`, `AppResult<T>`, `ErrorSpec`, `ErrorSpecExt`, and `OptionAppExt`.
- Keep stable error specs in the owning layer:
  - domain errors: `crates/system/src/errors.rs` and `crates/file-storage/src/errors.rs`
  - API boundary errors: `apps/api/src/errors.rs`
- Add `impl From<...> for AppError` only when the source error has one stable API meaning in every context.
- When the same error type has context-specific semantics, map it explicitly at the call site with `.map_err(...)`.
- Do not collapse `LoginError` into one API mapping:
  - CRUD/user management keeps `crates/system/src/users.rs` `From<LoginError> for AppError`.
  - Login maps `InvalidCredentials` and `UserNotFound` to `INVALID_CREDENTIALS` to avoid account enumeration.
  - Auth middleware maps a missing/deleted token user to `SESSION_INVALID`.
- `AuthSessionError` has one auth-session semantic, so `From<AuthSessionError> for AppError` is acceptable.

## Frontend

- The desktop app is Vue 3 + Vite + Pinia + Vue Router + Axios + Nuxt UI.
- API wrappers live in `apps/desktop/src/api`; keep endpoint paths aligned with `apps/api/src/routes`.
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
npm test
npm run build
```

For frontend/backend integration changes, run both servers and verify the real UI path:

```bash
cargo run -p api
cd apps/desktop && npm run dev
```

Default login:

```text
admin / 123456
```

Before claiming a change is complete, report the exact verification commands that were run and whether they passed.

## Git

- Do not commit or push unless the user explicitly asks for it.
- Do not revert user changes. If unrelated dirty files exist, leave them alone.
- Keep generated build artifacts, local uploads, and temporary browser screenshots out of commits.
