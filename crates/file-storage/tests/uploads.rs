use std::path::{Path, PathBuf};

use file_storage::files::{
    FileError, FileListQuery, FileService, ImportFileUrl, MAX_UPLOAD_BYTES, StartUpload,
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

#[sqlx::test(migrations = "../../migrations")]
async fn file_can_be_uploaded_in_multiple_chunks(pool: sqlx::PgPool) {
    let upload_dir = upload_dir();
    let service = managed_service(&pool, &upload_dir).await;

    let mut upload = service
        .begin_upload("../../Quarterly Report.PDF", "finance", "report")
        .await
        .expect("upload should start");
    upload
        .write_chunk(b"quarterly ")
        .await
        .expect("first chunk should write");
    upload
        .write_chunk(b"results")
        .await
        .expect("second chunk should write");
    let stored = upload.finish().await.expect("upload should finish");

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
async fn aborted_upload_removes_the_partial_object(pool: sqlx::PgPool) {
    let upload_dir = upload_dir();
    let service = managed_service(&pool, &upload_dir).await;
    let mut upload = service
        .begin_upload("report.pdf", "finance", "report")
        .await
        .expect("upload should start");
    upload
        .write_chunk(b"report contents")
        .await
        .expect("upload content should write");

    upload.abort().await.expect("upload should abort cleanly");
    let mut entries = tokio::fs::read_dir(&upload_dir)
        .await
        .expect("upload directory should exist");
    assert!(
        entries
            .next_entry()
            .await
            .expect("upload directory should be readable")
            .is_none(),
        "aborted upload should not leave a partial object"
    );

    tokio::fs::remove_dir_all(upload_dir)
        .await
        .expect("test upload directory should be removed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn metadata_failure_removes_the_uploaded_file(pool: sqlx::PgPool) {
    let upload_dir = upload_dir();
    let service = managed_service(&pool, &upload_dir).await;
    let mut upload = service
        .begin_upload("report.pdf", "finance", "report")
        .await
        .expect("upload should start");
    upload
        .write_chunk(b"report contents")
        .await
        .expect("upload content should write");

    sqlx::query("drop table uploaded_files")
        .execute(&pool)
        .await
        .expect("test should make metadata persistence fail");
    let error = upload
        .finish()
        .await
        .expect_err("metadata failure should fail the upload");
    assert!(matches!(error, FileError::Database(_)));

    let mut entries = tokio::fs::read_dir(&upload_dir)
        .await
        .expect("upload directory should exist");
    assert!(
        entries
            .next_entry()
            .await
            .expect("upload directory should be readable")
            .is_none(),
        "failed upload should not leave a stored file"
    );

    tokio::fs::remove_dir_all(upload_dir)
        .await
        .expect("test upload directory should be removed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn metadata_delete_failure_keeps_the_managed_object(pool: sqlx::PgPool) {
    let upload_dir = upload_dir();
    let service = managed_service(&pool, &upload_dir).await;
    let mut upload = service
        .begin_upload("report.pdf", "finance", "report")
        .await
        .expect("upload should start");
    upload
        .write_chunk(b"report contents")
        .await
        .expect("upload content should write");
    let stored = upload.finish().await.expect("upload should finish");
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
    assert_eq!(
        tokio::fs::read(upload_dir.join(stored_name))
            .await
            .expect("managed object should remain available"),
        b"report contents"
    );

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
