# File storage

## Ownership

`crates/file-storage` owns storage definitions, object operations, and file-to-storage
associations. `crates/api` exposes the protected HTTP boundary, while the Admin Console provides the
operator workflow. PostgreSQL is authoritative for managed storage state.

Storages are stored in `sys_storages`. The generic `root` column is the local root
directory for filesystem storage and the object-key prefix within a bucket for S3. Other
driver-facing columns use generic names such as `bucket`, `region`, `endpoint`, and
`public_base_url`; driver-specific validation and mapping belong to the capability service. Uploaded
file metadata references its owning storage through `uploaded_files.storage_id`.

## Storage lifecycle

Migration `0005_storage_management.sql` creates an enabled local default rooted at `./uploads` and
links each newly uploaded file to the backend that accepted it. PostgreSQL is the only runtime
source for storage settings; API startup does not read file-storage environment variables.

Supported drivers are:

- `local`, backed by an OpenDAL filesystem operator and served through `/uploads/{object}`.
- `s3`, backed by an OpenDAL S3 operator and addressed through its configured public base URL.

An enabled storage can become the default. The current default cannot be disabled or deleted,
and a storage referenced by uploaded files cannot be deleted. Storage code and driver
are immutable after creation because they identify the backend contract; connection details remain
editable.

Every managed upload resolves the database default before opening its writer. Reads and deletes
resolve the file's persisted `storage_id`. The in-process registry caches constructed OpenDAL
operators, but database resolution prevents another process's default change from remaining stale.

Imported external URLs have no storage association and remain outside managed object deletion.
During migration, existing `/uploads/...` records are backfilled to the seeded local storage.
Disabled storage is excluded from new default selection but remains readable for associated files.

## Credentials

S3 access keys and secret keys are stored as plain text in PostgreSQL. API responses expose only
`hasAccessKey` and `hasSecretKey`; they never return credential values. Sending an empty credential
while updating a storage preserves the stored value. Temporary session tokens are not supported.

## Authorization

The storage page and actions use concrete IAM permissions:

- `system:storage:list`
- `system:storage:create`
- `system:storage:update`
- `system:storage:delete`
- `system:storage:update-status`
- `system:storage:set-default`

The migration binds each protected route to its owning page or action and grants the concrete
permissions to the protected `super_admin` role.
