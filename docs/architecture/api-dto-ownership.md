# API DTO ownership

`crates/api` owns HTTP response DTOs and request DTOs whose wire shape differs from the owning capability's behavior input. Known response bodies use concrete `ToSchema` types; `serde_json::Value` is reserved for genuinely open JSON fields.

Capability crates keep plain behavior inputs without HTTP Query, camelCase serde renames, `IntoParams`, or OpenAPI schema derives. Routes own the wire DTOs and convert with `From` / `Into` into capability inputs.

Current intentional exception:

- `audit`: the audit list query still carries HTTP/OpenAPI derives and is reused by `routes/audit/events/dto.rs` as a type alias. Migrate it the same way when next touching that surface.

IAM request and response DTOs are API-owned. Routes convert them into Accounts and Roles behavior inputs.

Do not add a capability-side OpenAPI derive unless this document is updated with an explicit exception. Capability models and response views must not derive OpenAPI schemas merely for route documentation.
