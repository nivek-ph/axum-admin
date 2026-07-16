use std::path::{Path, PathBuf};

use file_storage::files::{FileError, FileService, MAX_UPLOAD_BYTES};
use uuid::Uuid;

fn upload_dir() -> PathBuf {
    std::env::temp_dir().join(format!("ava-file-upload-test-{}", Uuid::new_v4()))
}

#[sqlx::test(migrations = "../../migrations")]
async fn file_can_be_uploaded_in_multiple_chunks(pool: sqlx::PgPool) {
    let upload_dir = upload_dir();
    let service = FileService::new(pool.clone(), upload_dir.to_string_lossy());

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
async fn oversized_file_is_rejected_while_streaming_and_cleaned_up(pool: sqlx::PgPool) {
    let upload_dir = upload_dir();
    let service = FileService::new(pool.clone(), upload_dir.to_string_lossy());
    let mut upload = service
        .begin_upload("large.bin", "", "")
        .await
        .expect("upload should start");

    upload
        .write_chunk(&vec![0; MAX_UPLOAD_BYTES])
        .await
        .expect("bytes at the limit should be accepted");
    let error = upload
        .write_chunk(&[1])
        .await
        .expect_err("the first byte above the limit should be rejected");
    assert!(matches!(error, FileError::TooLarge));
    upload
        .abort()
        .await
        .expect("rejected upload should clean up");

    let stored_count: i64 = sqlx::query_scalar("select count(*) from uploaded_files")
        .fetch_one(&pool)
        .await
        .expect("stored file count should be readable");
    assert_eq!(stored_count, 0);

    let mut entries = tokio::fs::read_dir(&upload_dir)
        .await
        .expect("upload directory should exist");
    assert!(
        entries
            .next_entry()
            .await
            .expect("upload directory should be readable")
            .is_none(),
        "rejected upload should not leave a temporary file"
    );

    tokio::fs::remove_dir_all(upload_dir)
        .await
        .expect("test upload directory should be removed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn file_at_the_limit_is_fully_persisted(pool: sqlx::PgPool) {
    let upload_dir = upload_dir();
    let service = FileService::new(pool.clone(), upload_dir.to_string_lossy());
    let mut upload = service
        .begin_upload("limit.bin", "", "")
        .await
        .expect("upload should start");

    upload
        .write_chunk(&vec![0; MAX_UPLOAD_BYTES])
        .await
        .expect("bytes at the limit should be accepted");
    let stored = upload.finish().await.expect("upload should finish");

    let stored_count: i64 = sqlx::query_scalar("select count(*) from uploaded_files")
        .fetch_one(&pool)
        .await
        .expect("stored file count should be readable");
    assert_eq!(stored_count, 1);
    let stored_name = Path::new(&stored.url)
        .file_name()
        .expect("stored URL should contain a file name");
    let metadata = tokio::fs::metadata(upload_dir.join(stored_name))
        .await
        .expect("stored file should exist");
    assert_eq!(metadata.len(), MAX_UPLOAD_BYTES as u64);

    tokio::fs::remove_dir_all(upload_dir)
        .await
        .expect("test upload directory should be removed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn finalization_failure_removes_the_temporary_file(pool: sqlx::PgPool) {
    let upload_dir = upload_dir();
    let service = FileService::new(pool, upload_dir.to_string_lossy());
    let mut upload = service
        .begin_upload("report.pdf", "finance", "report")
        .await
        .expect("upload should start");
    upload
        .write_chunk(b"report contents")
        .await
        .expect("upload content should write");

    let mut entries = tokio::fs::read_dir(&upload_dir)
        .await
        .expect("upload directory should exist");
    let temp_path = entries
        .next_entry()
        .await
        .expect("upload directory should be readable")
        .expect("temporary file should exist")
        .path();
    tokio::fs::remove_file(temp_path)
        .await
        .expect("test should make finalization fail");

    let error = upload
        .finish()
        .await
        .expect_err("missing temporary file should fail finalization");
    assert!(matches!(error, FileError::Io(_)));
    assert!(
        entries
            .next_entry()
            .await
            .expect("upload directory should be readable")
            .is_none(),
        "failed finalization should not leave a file"
    );

    tokio::fs::remove_dir_all(upload_dir)
        .await
        .expect("test upload directory should be removed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn metadata_failure_removes_the_uploaded_file(pool: sqlx::PgPool) {
    let upload_dir = upload_dir();
    let service = FileService::new(pool.clone(), upload_dir.to_string_lossy());
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
