# axum-vue-admin

Rust + Vue admin system. The backend exposes REST APIs with Axum, and the
frontend is a Vue 3 + Vite app styled with Nuxt UI.

## Acknowledgements

Product layout and navigation patterns borrow ideas from **[gin-vue-admin](https://github.com/flipped-aurora/gin-vue-admin)** (the common Go + Gin + Vue admin stack; people sometimes shorthand it as "go-vue-admin"). This repo is a separate implementation: the API is Rust (Axum), not Gin.

## Stack

- Backend: Rust 2024, Axum, SQLx, PostgreSQL, Utoipa Swagger UI
- Frontend: Vue 3, Vite, Pinia, Vue Router, Axios, Nuxt UI, Tailwind CSS
- Desktop shell: Tauri 2
- Auth: JWT returned by login and sent with the `Authorization: Bearer <token>` header

## Screenshots

![Dashboard screen](docs/screenshots/dashboard.png)

More screens: [Login](docs/screenshots/login.png) · [User Management](docs/screenshots/users.png) · [API Registry](docs/screenshots/apis.png)

## Workspace layout:

```text
apps/ava           Ava CLI and backend composition root
apps/desktop       Vue/Vite/Tauri desktop frontend
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
npm install
npm run dev
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
VITE_API_BASE_URL=http://127.0.0.1:3000/api npm run dev
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
  - Auth middleware maps missing/deleted users to `SESSION_INVALID`, because an
    already-issued token can no longer resolve to an active session.
- `AuthSessionError` has a single auth-session meaning and can convert through
  `From<AuthSessionError> for AppError`.

## API Overview

Public routes:

| Method | Path                 |
| ------ | -------------------- |
| GET    | `/api/health`        |
| POST   | `/api/auth/login`    |
| POST   | `/api/auth/captcha`  |
| POST   | `/api/init/check-db` |
| POST   | `/api/init/database` |

Protected route groups:

| Area                  | Routes                                                                                                                                                                                                                                            |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Users                 | `GET/POST /api/users`, `PUT/DELETE /api/users/{id}`, `GET/PUT /api/users/me`, `PUT /api/users/me/password`, `PUT /api/users/me/settings`, `PUT /api/users/me/authority`, `POST /api/users/{id}/password/reset`, `PUT /api/users/{id}/authorities` |
| Roles                 | `GET/POST /api/roles`, `PUT/DELETE /api/roles/{authority_id}`, `GET/PUT /api/roles/{authority_id}/users`, `PUT /api/roles/data-authority`                                                                                                         |
| Menus                 | `GET/POST /api/menus`, `GET /api/menus/current`, `GET /api/menus/tree`, `GET/PUT/DELETE /api/menus/{id}`, `GET/PUT /api/menus/{id}/roles`, `GET/POST /api/menus/authority`                                                                        |
| API routes            | `GET/POST /api/routes`, `GET /api/routes/all`, `GET /api/routes/groups`, `GET/PUT/DELETE /api/routes/{id}`, `GET/PUT /api/routes/roles`, `DELETE /api/routes/batch`, `POST /api/routes/casbin/refresh`                                            |
| Params                | `GET/POST /api/params`, `GET /api/params/by-key`, `GET/PUT/DELETE /api/params/{id}`, `DELETE /api/params/batch`                                                                                                                                   |
| Dictionaries          | `GET/POST /api/dictionaries`, `POST /api/dictionaries/import`, `GET/PUT/DELETE /api/dictionaries/{id}`, `GET /api/dictionaries/{id}/export`, `GET/POST /api/dictionaries/{id}/tree`, `GET/PUT/DELETE /api/dictionaries/{id}/tree/{node_id}` |
| Files                 | `GET /api/files`, `POST /api/files/upload`, `POST /api/files/import-url`, `DELETE /api/files/{id}`, `PATCH /api/files/{id}/name`                                                                                                                  |
| Attachment categories | `GET/POST /api/attachment-categories`, `DELETE /api/attachment-categories/{id}`                                                                                                                                                                   |
| Audit events          | `GET /api/audit/events`, `GET /api/audit/events/{id}`                                                                                                                                                                                             |
| System                | `GET/PUT /api/system/config`, `GET /api/system/server-info`, `POST /api/system/reload`                                                                                                                                                            |
| Auth sessions         | `POST /api/auth/logout`                                                                                                                                                                                                                           |

## Features

Main modules:

- Dashboard
- Users
- Roles
- Menus
- API routes
- Params
- Dictionaries and dictionary details
- Files and attachment categories
- Structured audit events
- Profile
- System config
- System state

Main workflows:

- Login and current-menu loading
- User list, delete, reset password
- Role CRUD and role-user assignment
- Menu CRUD and menu-role assignment
- API route CRUD and route-role assignment
- Param CRUD
- Dictionary CRUD
- Dictionary detail CRUD, including child nodes
- File category CRUD
- File URL import
- File multipart upload with preview and progress
- File rename, delete, and preview
- Audit event filtering and detail inspection

## Verification

Backend and workspace tests:

```bash
cargo test --workspace
```

Frontend tests:

```bash
cd apps/desktop
npm test
```

Frontend production build:

```bash
cd apps/desktop
npm run build
```

Recommended manual integration sweep:

1. Start PostgreSQL for the configured `DATABASE_URL`.
2. Start Redis 8 or newer for the configured `REDIS_URL`.
3. Start the backend with `cargo run -p ava -- serve`.
4. Start the frontend with `cd apps/desktop && npm run dev`.
5. Log in with `admin / 123456`.
6. Smoke test user, role, menu, API route, param, dictionary, file, audit, and
   system pages.
