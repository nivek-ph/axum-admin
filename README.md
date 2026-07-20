# axum-admin

Rust + React admin system. The backend exposes REST APIs with Axum, and the
default Admin Console is a React + Vite single-page application.

The previous Vue 3 + Tauri implementation is available in the
[`v1.1.0`](../../tree/v1.1.0) tag.

## Acknowledgements

Product layout and navigation patterns borrow ideas from **[gin-vue-admin](https://github.com/flipped-aurora/gin-vue-admin)** (the common Go + Gin + Vue admin stack; people sometimes shorthand it as "go-vue-admin"). This repo is a separate implementation: the API is Rust (Axum), not Gin.

## Stack

- Backend: Rust 2024, Axum, SQLx, PostgreSQL, Utoipa Swagger UI
- Frontend: React, Vite, React Router, Zustand, TanStack Query, Axios, Radix UI, Tailwind CSS
- Auth: JWT returned by login and sent with the `Authorization: Bearer <token>` header

## Workspace layout:

```text
apps/ava           Ava CLI and backend composition root
apps/desktop       React/Vite Admin Console
crates/api         Axum HTTP adapter
crates/audit       structured business and security audit events
crates/auth        password, token, and captcha helpers
crates/db          database connection and migrations
crates/file-storage
crates/iam         users, roles, departments, menus, and access control
crates/metadata    parameters and dictionaries
migrations         SQLx migrations
```

## Environment

Copy `.env.example` to `.env` or export the variables in your shell.

Required:

- `HTTP_PORT`
- `DATABASE_URL`
- `REDIS_URL` (Redis 8 or newer required)
- `JWT_SECRET`

Optional:

- `PUBLIC_BASE_URL`
- `ADMIN_USERNAME`, default `admin`
- `ADMIN_NICKNAME`, default `Administrator`

Required only by `ava init`:

- `ADMIN_PASSWORD`

Example:

```bash
cp .env.example .env
```

Default local database URL from `.env.example`:

```text
postgres://postgres:postgres@localhost/ava
```

On API startup, the server runs migrations. Default authority, menu, route
registry, and admin user data are initialized by the Ava CLI.

## Run

Start the backend:

```bash
cargo run -p ava -- serve
```

The API listens on:

```text
http://127.0.0.1:3000
```

Swagger UI:

```text
http://127.0.0.1:3000/swagger-ui/
```

Start the frontend:

```bash
cd apps/desktop
pnpm install
pnpm dev
```

The frontend defaults to:

```text
http://127.0.0.1:5173
```

The frontend API base URL defaults to:

```text
http://127.0.0.1:3000/api
```

Override it with:

```bash
VITE_API_BASE_URL=http://127.0.0.1:3000/api pnpm dev
```

Login after running `ava init`:

```text
username: value of ADMIN_USERNAME (default: admin)
password: value of ADMIN_PASSWORD
```

Bootstrap default system data when setting up a database:

```bash
cargo run -p ava -- init
```

## API Contract

Successful responses use a stable envelope:

```json
{
  "code": "OK",
  "message": "ok",
  "data": {}
}
```

Error responses use the same `code` and `message` shape where possible.
Authenticated requests send the JWT in the `Authorization: Bearer <token>` header.

## Error Design

- `crates/api` owns the HTTP response envelope and the shared error boundary:
  public `AppError` and `AppResult<T>` types. Repeated fixed HTTP contracts use
  crate-private `ErrorSpec` constants; callers use ordinary `ok_or` and `?`.
- Stable user-facing error codes and messages live in the owning layer:
  domain errors in their owning capability crate and API boundary errors in
  `crates/api/src/mappings.rs`, with route-local errors reserved for multi-capability workflows.
- Route and middleware handlers should return `AppResult<T>`.
- Use `impl From<DomainError> for AppError` only when the source error has one
  stable HTTP/API meaning everywhere it is used.
- Use explicit `.map_err(...)` at the call site when the same domain error needs
  different HTTP semantics in different contexts.
- User-management and authentication errors remain distinct:
  - CRUD/user management returns `UserError` from `crates/iam/src/users`.
  - Login returns `AuthenticateError`; unknown users and incorrect passwords both become
    `INVALID_CREDENTIALS` so the login API does not reveal whether an account exists.
  - Auth middleware loads an Access Snapshot; access evaluation maps missing/deleted
    users to `SESSION_INVALID` and disabled users to `USER_DISABLED`.

## API Overview

Public routes:

| Method | Path                |
| ------ | ------------------- |
| GET    | `/api/health`       |
| POST   | `/api/auth/login`   |
| POST   | `/api/auth/refresh` |
| POST   | `/api/auth/captcha` |

Authenticated route groups:

| Area         | Prefix or route      |
| ------------ | -------------------- |
| Users        | `/api/users`         |
| Roles        | `/api/roles`         |
| Departments  | `/api/depts`         |
| Menus        | `/api/menus`         |
| Params       | `/api/params`        |
| Dictionaries | `/api/dictionaries`  |
| Files        | `/api/files`         |
| Audit events | `/api/audit/events`  |
| Logout       | `/api/auth/logout`   |

See Swagger UI for the current method-level contract.

## Features

Main Admin Console modules:

- Dashboard
- Users
- Roles
- Menus
- Departments
- Params
- Dictionaries and dictionary details
- Files
- Structured audit events
- Profile

Main workflows:

- Login and current-menu loading
- User list, delete, reset password
- Role CRUD and role-user assignment
- Read-only menu and permission catalog
- Department hierarchy CRUD
- Param CRUD
- Dictionary CRUD
- Dictionary detail CRUD, including child nodes
- File URL import
- File multipart upload with current loading feedback
- File rename and delete
- Audit event filtering and detail inspection
- Profile settings and password change

Percentage upload progress and expanded action-level ACL are future enhancements and do not block
the React parity gate.

## Verification

Backend and workspace tests:

```bash
cargo test --workspace
```

Frontend tests:

```bash
cd apps/desktop
pnpm test
```

Frontend production build:

```bash
cd apps/desktop
pnpm build
```

Recommended manual integration sweep:

1. Start PostgreSQL for the configured `DATABASE_URL`.
2. Start Redis 8 or newer for the configured `REDIS_URL`.
3. Start the backend with `cargo run -p ava -- serve`.
4. Start the frontend with `cd apps/desktop && pnpm dev`.
5. Log in with `ADMIN_USERNAME / ADMIN_PASSWORD` from the environment.
6. Complete the fixed parity path: login, a Users workflow, save a non-system
   role's permissions, upload a file, and open an audit-event detail.

React Router uses browser history. Production static hosting must serve `apps/desktop/dist/index.html`
for unknown non-API paths so direct navigation and reloads such as `/roles` continue to work.
