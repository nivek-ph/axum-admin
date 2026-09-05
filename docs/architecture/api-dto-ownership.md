# API DTO ownership

`crates/api` owns HTTP response DTOs and request DTOs whose wire shape differs from the owning capability's behavior input. Known response bodies use concrete `ToSchema` types; `serde_json::Value` is reserved for genuinely open JSON fields.

Capability types may retain `utoipa` derives only when `crates/api` deliberately re-exports an identical thin input with a local type alias. This avoids two field-for-field DTOs while keeping route handlers dependent on their local `dto` module.

Current retained derives are:

- `metadata::dictionaries`: the list query is identical to the input used by `routes/dictionaries/dto.rs`.
- `metadata::parameters`: list and mutation inputs are identical to the inputs used by `routes/parameters/dto.rs`.
- `file-storage::files`: the list query is identical to the input used by `routes/files/dto.rs`.
- `audit`: the audit list query is an intentional read-only exception reused by `routes/audit/events/dto.rs`.

IAM request and response DTOs are API-owned. Routes convert them into Accounts and Roles behavior
inputs instead of retaining OpenAPI derives in `crates/iam`.

Do not add a capability-side OpenAPI derive unless a local API type alias consumes it and this list is updated. Capability models and response views must not derive OpenAPI schemas merely for route documentation.