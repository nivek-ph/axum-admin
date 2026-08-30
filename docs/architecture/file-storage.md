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

The base schema and seed data create an enabled local default rooted at `./uploads` and link each
newly uploaded file to the backend that accepted it. PostgreSQL is the only runtime source for
storage settings; API startup does not read file-storage environment variables.
Valid local roots are not created while storage records are loaded or managed. The filesystem
backend prepares its root only when that storage is used for an object operation, so an unused local
record does not require a writable deployment filesystem.

Supported drivers are:

- `local`, backed by an OpenDAL filesystem operator and served through `/uploads/{object}`.
- `s3`, backed by an OpenDAL S3 operator and addressed through its configured public base URL.

An enabled storage can become the default. The current default cannot be disabled or deleted,
and a storage referenced by uploaded files or active upload sessions cannot be deleted. Storage
code, driver, and object-location fields are immutable after creation because persisted files and
upload sessions address that backend directly. For local storage this includes `root`; for S3 it
includes `root`, `bucket`, `region`, `endpoint`, `public_base_url`, and virtual-host style.
Credentials may still be rotated in place.

Managed uploads use chunks of at most 4 MiB and persist their current offset in
`uploaded_file_sessions`. Each chunk is claimed with a short conditional database update, written
outside a database transaction under an operation-specific object key, then recorded in
`uploaded_file_parts` by a second short transaction. The Admin Console resumes the same file from
the persisted offset after an interrupted request and retains that resume ID across transient status
errors. A rate-limited chunk waits for the server's `Retry-After` duration and resumes automatically.
Files are limited to 1 GiB. Every upload resolves the database default when its session
starts. Default selection and session insertion hold the same transaction-scoped lifecycle lock as
default changes and storage deletion. Reads and deletes require the file's persisted
`(storage_id, object_name)` identity rather than reverse-parsing its public URL. Local responses
stream from OpenDAL instead of buffering the whole object and honor single HTTP byte ranges with
`206 Partial Content`; malformed or unsatisfiable ranges return `416 Range Not Satisfiable`.
Storage resolution reads PostgreSQL directly, so another process's default or credential change
does not remain stale.

Imported external URLs have no storage association, including imported `/uploads/...` URLs. They
are not served through the local upload route, and deleting them removes metadata only without
touching an object store.
Disabled storage is excluded from new default selection but remains readable for associated files.
Completion uses the upload session ID as an idempotency key. A short database update claims the
session as `completing`; object assembly runs outside a transaction into an operation-specific final
object; and a final short transaction inserts the file row and removes the session. Explicit failure
paths abort and delete the in-progress final object before releasing the claim. Operation tokens fence
concurrent workers. A retry after a committed response loss returns the already-created file.
Completed chunk prefixes are removed on the successful path and retried when completion is called
again or the service restarts. Upload sessions and in-progress operations expire one hour after their
last successful activity. An expired session cannot be resumed; the client starts a new upload.
Service startup claims each stale session before deleting its current final object, temporary object
prefix, and session row. A per-session PostgreSQL advisory lock covers object I/O, so startup skips
a stale-looking session while an old chunk write or completion still owns its object writer.

Managed deletion first persists `deletion_pending`, which hides the file from listings and local
serving, then deletes the object without buffering it in application memory. A retry can finish a
pending deletion after interruption or an uncertain object-delete or metadata-delete result;
service startup also resumes persisted pending deletions.

## Credentials

S3 access keys and secret keys are stored as plain text in PostgreSQL. API responses expose only
`hasAccessKey` and `hasSecretKey`; they never return credential values. Sending an empty credential
while updating a storage preserves the stored value. Both credentials are required for S3 storage;
missing either credential is rejected when the configuration is created, updated, or loaded. The S3
adapter disables OpenDAL's environment, shared-profile, and EC2 metadata credential sources, so it
cannot silently fall back to ambient credentials. Temporary session tokens are not supported.

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
