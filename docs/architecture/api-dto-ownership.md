# API DTO ownership

`crates/api` owns HTTP response DTOs and request DTOs when the wire shape differs from the
owning capability's behavior input. Known response bodies use concrete `ToSchema` types;
`serde_json::Value` is reserved for genuinely open JSON fields.

## When to keep one type

If the HTTP request and capability input are field-for-field identical, reuse the capability
type. Route modules may expose a local type alias so handlers keep importing from their
`dto` module:

```rust
pub type ParameterListRequest = metadata::parameters::ParamListQuery;
pub type RoleRequest = iam::roles::RolePayload;
```

Capability crates may retain `serde` / `utoipa` derives in that case. Do not invent a second
struct plus a pure field-copy `From` impl just to push HTTP derives out of the capability crate.

## When to keep two types

Maintain a separate API DTO only when the boundary does real work, for example:

- plaintext password on the wire vs `password_hash` in the capability
- legacy aliases / compatibility fields that the capability should not see
- validation, normalization, or filtering before the capability call
- different field types or business meaning
- response projection that differs from the internal model

## Current identical-input aliases

- `metadata::dictionaries::DictionaryListQuery`
- `metadata::parameters::{ParamListQuery, ParameterInput}`
- `file-storage::files::FileListQuery`
- `audit::AuditQuery`
- `iam::accounts::{UserListQuery, UpdateCurrentUserInput}`
- `iam::roles::RolePayload`

Do not add a capability-side OpenAPI derive for a response/view type merely for route
documentation. Capability models that are not reused as wire inputs should stay free of
HTTP/OpenAPI derives.
