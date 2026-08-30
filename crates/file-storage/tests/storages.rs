use std::path::{Path, PathBuf};

use file_storage::{
    files::{FileService, StartUpload, StoredFile},
    storages::{StorageBackendInput, StorageError, StorageInput},
};
use uuid::Uuid;

fn upload_dir(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("ava-storage-{label}-{}", Uuid::new_v4()))
}

fn local_input(code: &str, root: &Path) -> StorageInput {
    StorageInput {
        name: format!("Storage {code}"),
        code: code.to_string(),
        backend: StorageBackendInput::Local {
            root: root.to_string_lossy().into_owned(),
        },
        enabled: true,
        sort: 10,
        description: "test storage".to_string(),
    }
}

async fn set_default_root(pool: &sqlx::PgPool, root: &Path) {
    sqlx::query("update sys_storages set root = $1 where is_default")
        .bind(root.to_string_lossy().as_ref())
        .execute(pool)
        .await
        .expect("default storage root should update");
}

async fn upload_file(service: &FileService, name: &str, bytes: &[u8]) -> StoredFile {
    let session = service
        .start_upload(StartUpload {
            name: name.to_string(),
            size: bytes.len() as i64,
            tag: String::new(),
            category: String::new(),
        })
        .await
        .expect("upload session should start");
    service
        .write_upload_chunk(&session.id, 0, bytes)
        .await
        .expect("upload chunk should write");
    service
        .complete_upload(&session.id)
        .await
        .expect("upload should complete")
}

#[sqlx::test(migrations = "../../migrations")]
async fn migration_seeds_the_local_default(pool: sqlx::PgPool) {
    let (_, storages) = FileService::managed(pool)
        .await
        .expect("managed storage should load");

    let list = storages
        .list(Default::default())
        .await
        .expect("storages should list");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].code, "local");
    assert!(list[0].enabled);
    assert!(list[0].is_default);
    assert_eq!(list[0].root.as_deref(), Some("./uploads"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn concurrent_loads_share_the_seeded_default(pool: sqlx::PgPool) {
    let first = FileService::managed(pool.clone());
    let second = FileService::managed(pool.clone());
    let (first, second) = tokio::join!(first, second);

    first.expect("first managed storage should load");
    second.expect("second managed storage should load");
    let local_count: i64 =
        sqlx::query_scalar("select count(*) from sys_storages where code = 'local'")
            .fetch_one(&pool)
            .await
            .expect("local storage count should be readable");
    let default_count: i64 =
        sqlx::query_scalar("select count(*) from sys_storages where is_default")
            .fetch_one(&pool)
            .await
            .expect("default storage count should be readable");
    assert_eq!(local_count, 1);
    assert_eq!(default_count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn default_switch_is_observed_by_an_already_running_file_service(pool: sqlx::PgPool) {
    let first_root = upload_dir("first");
    let second_root = upload_dir("second");
    set_default_root(&pool, &first_root).await;
    let (files, storages) = FileService::managed(pool.clone())
        .await
        .expect("managed storage should load");
    let second = storages
        .create(local_input("secondary", &second_root))
        .await
        .expect("second storage should be created");
    storages
        .set_default(second.id)
        .await
        .expect("default should switch");

    let stored = upload_file(&files, "report.txt", b"managed storage").await;

    assert_eq!(stored.storage_id, Some(second.id));
    let object = Path::new(&stored.url)
        .file_name()
        .expect("stored URL should contain an object name");
    assert_eq!(
        tokio::fs::read(second_root.join(object))
            .await
            .expect("object should be stored under the new default"),
        b"managed storage"
    );
    assert!(!first_root.join(object).exists());

    tokio::fs::remove_dir_all(first_root)
        .await
        .expect("first test directory should be removed");
    tokio::fs::remove_dir_all(second_root)
        .await
        .expect("second test directory should be removed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn protected_and_referenced_configurations_cannot_be_removed(pool: sqlx::PgPool) {
    let default_root = upload_dir("protected-default");
    let secondary_root = upload_dir("protected-secondary");
    set_default_root(&pool, &default_root).await;
    let (files, storages) = FileService::managed(pool)
        .await
        .expect("managed storage should load");
    let default = storages
        .list(Default::default())
        .await
        .expect("storages should list")
        .remove(0);

    assert!(matches!(
        storages.set_enabled(default.id, false).await,
        Err(StorageError::DefaultProtected)
    ));
    assert!(matches!(
        storages.delete(default.id).await,
        Err(StorageError::DefaultProtected)
    ));

    let secondary = storages
        .create(local_input("secondary", &secondary_root))
        .await
        .expect("second storage should be created");
    storages
        .set_default(secondary.id)
        .await
        .expect("default should switch");
    let stored = upload_file(&files, "used.txt", b"used").await;
    storages
        .set_default(default.id)
        .await
        .expect("default should switch back");
    storages
        .set_enabled(secondary.id, false)
        .await
        .expect("referenced non-default storage may be disabled");
    let object = Path::new(&stored.url)
        .file_name()
        .and_then(|value| value.to_str())
        .expect("stored URL should contain an object name");
    assert_eq!(
        files
            .read_local_object(object)
            .await
            .expect("disabled associated storage should remain readable")
            .expect("associated object should exist"),
        b"used"
    );

    assert!(matches!(
        storages.delete(secondary.id).await,
        Err(StorageError::InUse)
    ));

    tokio::fs::remove_dir_all(default_root)
        .await
        .expect("default test directory should be removed");
    tokio::fs::remove_dir_all(secondary_root)
        .await
        .expect("secondary test directory should be removed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn default_selection_and_disable_preserve_an_enabled_default(pool: sqlx::PgPool) {
    let default_root = upload_dir("concurrent-default");
    let secondary_root = upload_dir("concurrent-secondary");
    set_default_root(&pool, &default_root).await;
    let (_, storages) = FileService::managed(pool.clone())
        .await
        .expect("managed storage should load");
    let secondary = storages
        .create(local_input("secondary", &secondary_root))
        .await
        .expect("secondary storage should be created");

    let (set_default, disable) = tokio::join!(
        storages.set_default(secondary.id),
        storages.set_enabled(secondary.id, false),
    );
    assert!(set_default.is_ok() ^ disable.is_ok());

    let list = storages
        .list(Default::default())
        .await
        .expect("storages should list");
    let default = list
        .iter()
        .find(|storage| storage.is_default)
        .expect("one default should remain");
    assert!(default.enabled);

    tokio::fs::remove_dir_all(default_root)
        .await
        .expect("default test directory should be removed");
    tokio::fs::remove_dir_all(secondary_root)
        .await
        .expect("secondary test directory should be removed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn storage_location_is_immutable_but_metadata_can_change(pool: sqlx::PgPool) {
    let root = upload_dir("immutable-location");
    set_default_root(&pool, &root).await;
    let (_, storages) = FileService::managed(pool.clone())
        .await
        .expect("managed storage should load");
    let storage = storages
        .list(Default::default())
        .await
        .expect("storages should list")
        .remove(0);
    let mut input = local_input(&storage.code, &root);
    input.name = "Renamed storage".to_string();
    let renamed = storages
        .update(storage.id, input.clone())
        .await
        .expect("metadata-only update should succeed");
    assert_eq!(renamed.name, "Renamed storage");

    input.backend = StorageBackendInput::Local {
        root: upload_dir("moved-location").to_string_lossy().into_owned(),
    };
    assert!(matches!(
        storages.update(storage.id, input).await,
        Err(StorageError::ImmutableIdentity)
    ));

    tokio::fs::remove_dir_all(root)
        .await
        .expect("test directory should be removed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn s3_credentials_are_stored_but_never_returned(pool: sqlx::PgPool) {
    let root = upload_dir("credentials");
    set_default_root(&pool, &root).await;
    let (_, storages) = FileService::managed(pool.clone())
        .await
        .expect("managed storage should load");
    let input = StorageInput {
        name: "Object storage".to_string(),
        code: "object_store".to_string(),
        backend: StorageBackendInput::S3 {
            root: Some("uploads".to_string()),
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            endpoint: Some("https://s3.example.com".to_string()),
            public_base_url: "https://cdn.example.com/uploads".to_string(),
            access_key: Some("plain-access-key".to_string()),
            secret_key: Some("plain-secret-key".to_string()),
            virtual_host_style: false,
        },
        enabled: true,
        sort: 20,
        description: String::new(),
    };
    let view = storages
        .create(input)
        .await
        .expect("S3 storage should be created");
    assert_eq!(view.root.as_deref(), Some("uploads"));
    assert!(view.has_access_key);
    assert!(view.has_secret_key);

    let credentials: (String, String) = sqlx::query_as(
        r#"
        select
            access_key,
            secret_key
        from sys_storages
        where id = $1
        "#,
    )
    .bind(view.id)
    .fetch_one(&pool)
    .await
    .expect("credentials should be readable");
    assert_eq!(credentials.0, "plain-access-key");
    assert_eq!(credentials.1, "plain-secret-key");

    tokio::fs::remove_dir_all(root)
        .await
        .expect("test directory should be removed");
}
