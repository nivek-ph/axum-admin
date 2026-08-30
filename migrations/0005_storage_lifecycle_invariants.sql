ALTER TABLE sys_storages
ADD CONSTRAINT sys_storages_default_enabled CHECK (NOT is_default OR enabled);

ALTER TABLE uploaded_files
ADD COLUMN upload_id TEXT UNIQUE,
ADD COLUMN object_name TEXT,
ADD COLUMN size BIGINT NOT NULL DEFAULT 0 CHECK (size BETWEEN 0 AND 1073741824),
ADD COLUMN upload_parts_pending BOOLEAN NOT NULL DEFAULT false,
ADD COLUMN deletion_pending BOOLEAN NOT NULL DEFAULT false;

ALTER TABLE uploaded_file_sessions
ADD CONSTRAINT uploaded_file_sessions_size CHECK (
    total_size BETWEEN 0 AND 1073741824
    AND uploaded_size BETWEEN 0 AND total_size
);
