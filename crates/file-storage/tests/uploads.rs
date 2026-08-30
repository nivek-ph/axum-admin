use std::path::{Path, PathBuf};

use file_storage::files::{
    FileError, FileListQuery, FileService, ImportFileUrl, MAX_UPLOAD_BYTES, StartUpload, StoredFile,
};
use uuid::Uuid;

fn upload_dir() -> PathBuf {
    std::env::temp_dir().join(format!("ava-file-upload-test-{}", Uuid::new_v4()))
}

async fn managed_service(pool: &sqlx::PgPool, root: &Path) -> FileService {
    sqlx::query("update sys_storages set root = $1 where is_default")
        .bind(root.to_string_lossy().as_ref())
        .execute(pool)
        .await
        .expect("default storage root should update");
    FileService::managed(pool.clone())
        .await
        .expect("managed file storage should load")
        .0
}

async fn upload_parts(
    service: &FileService,
    name: &str,
    tag: &str,
    category: &str,
    parts: &[&[u8]],
) -> StoredFile {
    let size = parts.iter().map(|part| part.len() as i64).sum();
    let session = service
        .start_upload(StartUpload {
            name: name.to_string(),
            size,
            tag: tag.to_string(),
            category: category.to_string(),
        })
        .await
        .expect("upload session should start");
    let mut offset = 0;
    for part in parts {
        service
            .write_upload_chunk(&session.id, offset, part)
            .await
            .expect("upload part should write");
        offset += part.len() as i64;
    }
    service
        .complete_upload(&session.id)
        .await
        .expect("upload should complete")
}

#[sqlx::test(migrations = "../../migrations")]
async fn file_can_be_uploaded_in_multiple_chunks(pool: sqlx::PgPool) {
    let upload_dir = upload_dir();
    let service = managed_service(&pool, &upload_dir).await;

    let stored = upload_parts(
        &service,
        "../../Quarterly Report.PDF",
        "finance",
        "report",
        &[b"quarterly ".as_slice(), b"results".as_slice()],
    )
    .await;

    assert_eq!(stored.name, "../../Quarterly Report.PDF");
    assert_eq!(stored.ext, "pdf");
    assert!(!stored.url.contains("Quarterly"));
    assert!(!stored.url.contains(".."));

    let stored_name = Path::new(&stored.url)
        .file_name()
        .expect("stored URL should contain a file name");
    let bytes = tokio::fs::read(upload_dir.join(stored_name))
        .await
        .expect("stored file should be readable");
    assert_eq!(bytes, b"quarterly results");

    tokio::fs::remove_dir_all(upload_dir)
        .await
        .expect("test upload directory should be removed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn oversized_file_is_rejected_before_uploading_chunks(pool: sqlx::PgPool) {
    let upload_dir = upload_dir();
    let service = managed_service(&pool, &upload_dir).await;
    let error = service
        .start_upload(StartUpload {
            name: "large.bin".to_string(),
            size: MAX_UPLOAD_BYTES as i64 + 1,
            tag: String::new(),
            category: String::new(),
        })
        .await
        .expect_err("size above the limit should be rejected");
    assert!(matches!(error, FileError::TooLarge));
    let session_count: i64 = sqlx::query_scalar("select count(*) from uploaded_file_sessions")
        .fetch_one(&pool)
        .await
        .expect("upload session count should be readable");
    assert_eq!(session_count, 0);

    tokio::fs::remove_dir_all(upload_dir)
        .await
        .expect("test upload directory should be removed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn file_at_the_limit_can_start_without_buffering_the_payload(pool: sqlx::PgPool) {
    let upload_dir = upload_dir();
    let service = managed_service(&pool, &upload_dir).await;
    let session = service
        .start_upload(StartUpload {
            name: "limit.bin".to_string(),
            size: MAX_UPLOAD_BYTES as i64,
            tag: String::new(),
            category: String::new(),
        })
        .await
        .expect("file at the limit should start");
    assert_eq!(session.total_size, MAX_UPLOAD_BYTES as i64);

    tokio::fs::remove_dir_all(upload_dir)
        .await
        .expect("test upload directory should be removed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn duplicate_chunk_offset_has_exactly_one_winner(pool: sqlx::PgPool) {
    let upload_dir = upload_dir();
    let service = managed_service(&pool, &upload_dir).await;
    let session = service
        .start_upload(StartUpload {
            name: "concurrent.bin".to_string(),
            size: 4,
            tag: String::new(),
            category: String::new(),
        })
        .await
        .expect("upload session should start");

    let (first, second) = tokio::join!(
        service.write_upload_chunk(&session.id, 0, b"AAAA"),
        service.write_upload_chunk(&session.id, 0, b"BBBB"),
    );
    let expected = match (first, second) {
        (Ok(_), Err(FileError::OffsetMismatch)) => b"AAAA".as_slice(),
        (Err(FileError::OffsetMismatch), Ok(_)) => b"BBBB".as_slice(),
        result => panic!("one request should win the offset lock: {result:?}"),
    };
    let stored = service
        .complete_upload(&session.id)
        .await
        .expect("winning chunk should complete");
    let repeated = service
        .complete_upload(&session.id)
        .await
        .expect("completion retry should return the same file");
    assert_eq!(repeated.id, stored.id);
    let completed_status = service
        .upload_status(&session.id)
        .await
        .expect("completed upload status should survive a lost response");
    assert_eq!(completed_status.uploaded_size, completed_status.total_size);
    let parts_pending: bool =
        sqlx::query_scalar("select upload_parts_pending from uploaded_files where id = $1")
            .bind(stored.id)
            .fetch_one(&pool)
            .await
            .expect("upload part cleanup state should be readable");
    assert!(!parts_pending);
    let object = Path::new(&stored.url)
        .file_name()
        .expect("stored URL should contain an object name");
    assert_eq!(
        tokio::fs::read(upload_dir.join(object))
            .await
            .expect("completed object should exist"),
        expected
    );

    tokio::fs::remove_dir_all(upload_dir)
        .await
        .expect("test upload directory should be removed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn resumable_metadata_failure_keeps_the_session_retryable(pool: sqlx::PgPool) {
    let upload_dir = upload_dir();
    let service = managed_service(&pool, &upload_dir).await;
    let session = service
        .start_upload(StartUpload {
            name: "retryable.txt".to_string(),
            size: 4,
            tag: String::new(),
            category: String::new(),
        })
        .await
        .expect("upload session should start");
    service
        .write_upload_chunk(&session.id, 0, b"data")
        .await
        .expect("upload chunk should write");
    sqlx::query(
        r#"
        create function reject_uploaded_file_insert() returns trigger language plpgsql as $$
        begin
            raise exception 'test insert failure';
        end;
        $$
        "#,
    )
    .execute(&pool)
    .await
    .expect("test insert failure function should be created");
    sqlx::query(
        r#"
        create trigger reject_uploaded_file_insert
        before insert on uploaded_files
        for each row execute function reject_uploaded_file_insert()
        "#,
    )
    .execute(&pool)
    .await
    .expect("test insert failure trigger should be created");

    assert!(matches!(
        service.complete_upload(&session.id).await,
        Err(FileError::Database(_))
    ));
    assert!(!upload_dir.join(&session.object_name).exists());
    assert_eq!(
        service
            .upload_status(&session.id)
            .await
            .expect("failed completion should preserve the session")
            .uploaded_size,
        4
    );

    sqlx::query("drop trigger reject_uploaded_file_insert on uploaded_files")
        .execute(&pool)
        .await
        .expect("insert failure trigger should be removed");
    let stored = service
        .complete_upload(&session.id)
        .await
        .expect("completion retry should succeed");
    assert_eq!(stored.name, "retryable.txt");

    tokio::fs::remove_dir_all(upload_dir)
        .await
        .expect("test upload directory should be removed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn metadata_delete_failure_leaves_a_retryable_pending_delete(pool: sqlx::PgPool) {
    let upload_dir = upload_dir();
    let service = managed_service(&pool, &upload_dir).await;
    let stored = upload_parts(
        &service,
        "report.pdf",
        "finance",
        "report",
        &[b"report contents".as_slice()],
    )
    .await;
    let stored_name = Path::new(&stored.url)
        .file_name()
        .expect("stored URL should contain a file name");

    sqlx::query(
        r#"
        create function reject_uploaded_file_delete() returns trigger language plpgsql as $$
        begin
            raise exception 'test delete failure';
        end;
        $$
        "#,
    )
    .execute(&pool)
    .await
    .expect("test delete failure function should be created");
    sqlx::query(
        r#"
        create trigger reject_uploaded_file_delete
        before delete on uploaded_files
        for each row execute function reject_uploaded_file_delete()
        "#,
    )
    .execute(&pool)
    .await
    .expect("test delete failure trigger should be created");

    let error = service
        .delete(stored.id)
        .await
        .expect_err("metadata delete failure should fail the operation");
    assert!(matches!(error, FileError::Database(_)));
    assert!(!upload_dir.join(stored_name).exists());
    let pending: bool =
        sqlx::query_scalar("select deletion_pending from uploaded_files where id = $1")
            .bind(stored.id)
            .fetch_one(&pool)
            .await
            .expect("failed metadata deletion should remain retryable");
    assert!(pending);

    sqlx::query("drop trigger reject_uploaded_file_delete on uploaded_files")
        .execute(&pool)
        .await
        .expect("delete failure trigger should be removed");
    FileService::managed(pool.clone())
        .await
        .expect("service startup should resume pending deletion");
    let remaining: i64 = sqlx::query_scalar("select count(*) from uploaded_files where id = $1")
        .bind(stored.id)
        .fetch_one(&pool)
        .await
        .expect("file count should be readable");
    assert_eq!(remaining, 0);

    tokio::fs::remove_dir_all(upload_dir)
        .await
        .expect("test upload directory should be removed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn untracked_local_object_is_not_readable(pool: sqlx::PgPool) {
    let upload_dir = upload_dir();
    let service = managed_service(&pool, &upload_dir).await;
    tokio::fs::write(upload_dir.join("untracked.txt"), b"untracked")
        .await
        .expect("untracked object should be written");

    assert!(
        service
            .read_local_object("untracked.txt")
            .await
            .expect("untracked lookup should succeed")
            .is_none()
    );

    tokio::fs::remove_dir_all(upload_dir)
        .await
        .expect("test upload directory should be removed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn deleting_an_imported_local_url_keeps_the_object(pool: sqlx::PgPool) {
    let upload_dir = upload_dir();
    let service = managed_service(&pool, &upload_dir).await;
    let object = upload_dir.join("external.txt");
    tokio::fs::write(&object, b"external")
        .await
        .expect("external object should be written");
    service
        .import_url(ImportFileUrl {
            name: "External file".to_string(),
            url: "/uploads/external.txt".to_string(),
            tag: String::new(),
            category: String::new(),
        })
        .await
        .expect("external URL should be imported");
    let query: FileListQuery = serde_json::from_value(serde_json::json!({
        "page": 1,
        "pageSize": 10,
        "keyword": null,
        "category": null
    }))
    .expect("file list query should deserialize");
    let (files, ..) = service.list(query).await.expect("imported URL should list");

    service
        .delete(files[0].id)
        .await
        .expect("external metadata should delete");

    assert_eq!(
        tokio::fs::read(&object)
            .await
            .expect("external object should remain"),
        b"external"
    );

    tokio::fs::remove_dir_all(upload_dir)
        .await
        .expect("test upload directory should be removed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn expired_upload_session_cannot_be_resumed_before_startup_cleanup(pool: sqlx::PgPool) {
    let upload_dir = upload_dir();
    let service = managed_service(&pool, &upload_dir).await;
    let session = service
        .start_upload(StartUpload {
            name: "expired.txt".to_string(),
            size: 8,
            tag: String::new(),
            category: String::new(),
        })
        .await
        .expect("upload session should start");
    sqlx::query(
        "update uploaded_file_sessions set updated_at = now() - interval '61 minutes' where id = $1",
    )
    .bind(&session.id)
    .execute(&pool)
    .await
    .expect("upload session should expire");

    assert!(matches!(
        service.upload_status(&session.id).await,
        Err(FileError::UploadNotFound)
    ));
    assert!(matches!(
        service.write_upload_chunk(&session.id, 0, b"expired").await,
        Err(FileError::UploadNotFound)
    ));
    assert!(matches!(
        service.complete_upload(&session.id).await,
        Err(FileError::UploadNotFound)
    ));

    let session_count: i64 =
        sqlx::query_scalar("select count(*) from uploaded_file_sessions where id = $1")
            .bind(&session.id)
            .fetch_one(&pool)
            .await
            .expect("upload session count should be readable");
    assert_eq!(session_count, 1);

    tokio::fs::remove_dir_all(upload_dir)
        .await
        .expect("test upload directory should be removed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn startup_reaps_abandoned_upload_sessions_and_parts(pool: sqlx::PgPool) {
    let upload_dir = upload_dir();
    let service = managed_service(&pool, &upload_dir).await;
    let session = service
        .start_upload(StartUpload {
            name: "abandoned.txt".to_string(),
            size: 8,
            tag: String::new(),
            category: String::new(),
        })
        .await
        .expect("upload session should start");
    service
        .write_upload_chunk(&session.id, 0, b"partial")
        .await
        .expect("partial upload should write");
    let prefix = upload_dir.join(format!(".uploads/{}/", session.id));
    assert!(prefix.exists());
    sqlx::query(
        r#"
        update uploaded_file_sessions
        set
            updated_at = now() - interval '61 minutes',
            operation_state = 'writing',
            operation_token = 'abandoned-operation',
            operation_started_at = now() - interval '61 minutes'
        where id = $1
        "#,
    )
    .bind(&session.id)
    .execute(&pool)
    .await
    .expect("upload session should become stale");

    FileService::managed(pool.clone())
        .await
        .expect("service startup should reap stale uploads");

    let session_count: i64 =
        sqlx::query_scalar("select count(*) from uploaded_file_sessions where id = $1")
            .bind(&session.id)
            .fetch_one(&pool)
            .await
            .expect("upload session count should be readable");
    let part_count: i64 =
        sqlx::query_scalar("select count(*) from uploaded_file_parts where upload_id = $1")
            .bind(&session.id)
            .fetch_one(&pool)
            .await
            .expect("upload part count should be readable");
    assert_eq!(session_count, 0);
    assert_eq!(part_count, 0);
    assert!(!prefix.exists());

    tokio::fs::remove_dir_all(upload_dir)
        .await
        .expect("test upload directory should be removed");
}
