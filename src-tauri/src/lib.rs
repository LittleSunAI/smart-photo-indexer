mod ui_locale_generated;
mod ui_error;
mod provider_availability;
use base64::{engine::general_purpose, Engine as _};
use image::codecs::jpeg::JpegEncoder;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufReader, Read};
use std::path::{Component, Path, PathBuf};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Manager;
#[cfg(test)]
mod test_support;
mod file_safety;
mod file_operations;
mod file_actions;
mod batch_rename;
mod viewer_sessions;
mod window_appearance;
mod photo_editor;
mod source_sort;
mod processing;
mod processing_task_image;
mod processing_sources;
mod selection_statistics;
mod xmp_import;
mod analysis_copy;
mod credentials;
mod local_install;
mod provider_transport;
mod provider_readiness;
mod tag_layers;
mod batch_tags;
mod action_journal;
mod guarded_file;
mod profile_preferences;
mod project_preferences;
mod xmp_export;
mod index_backups;
mod collections;
mod image_metadata;
mod description_format;
mod index_observations;
mod thumbnail_cache;
use provider_transport::{get_ai_provider_status, download_local_model};
mod search;
mod lexical_search;
mod scanning;
mod source_paths;
mod library_roots;
mod duplicates;
#[cfg(target_os = "windows")]
mod windows_print;
#[cfg(target_os = "windows")]
mod windows_share;
#[cfg(target_os = "windows")]
mod windows_memory;
use scanning::{scan_directory,next_directory_scan_page,close_directory_scan};
use search::search_indexed_images;
use credentials::{save_provider_credential,get_provider_credentials};
use processing::{prepare_processing_job, processing_job_command, set_processing_concurrency, reconcile_processing_attempts, get_processing_jobs, get_processing_tasks, get_processing_snapshot};
use viewer_sessions::{open_viewer_window, get_viewer_session, set_viewer_active};

const AI_IMAGE_MAX_SIDE: u32 = 1600;
const AI_IMAGE_JPEG_QUALITY: u8 = 88;
const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "webp", "tif", "tiff", "heic", "heif", "avif",
];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PathBreadcrumbDto {
    label: String,
    path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DirectoryScanEntryDto {
    source_revision: Option<String>,
    file_metadata: Option<serde_json::Value>,
    id: String,
    kind: String,
    name: String,
    path: String,
    extension: Option<String>,
    size_bytes: Option<u64>,
    modified_at_unix: Option<u64>,
    child_count: Option<usize>,
    is_image: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DirectoryScanResultDto {
    current_path: String,
    parent_path: Option<String>,
    path_segments: Vec<String>,
    breadcrumbs: Vec<PathBreadcrumbDto>,
    items: Vec<DirectoryScanEntryDto>,
    scanned_directories: usize,
    skipped_entries: usize,
    truncated: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DroppedPathsInspectionResultDto {
    paths: Vec<String>,
    items: Vec<DirectoryScanEntryDto>,
    image_count: usize,
    folder_count: usize,
    file_count: usize,
    unsupported_count: usize,
    inspected_at_unix: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ThumbnailResultDto {
    source_path: String,
    thumbnail_path: String,
    width: u32,
    height: u32,
    was_created: bool,
    created_at_unix: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AiImageDataUrlResultDto {
    source_path: String,
    data_url: String,
    mime_type: String,
    original_size_bytes: u64,
    encoded_size_bytes: usize,
    width: Option<u32>,
    height: Option<u32>,
    was_resized: bool,
    encoded_at_unix: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AiImageFileFingerprintDto {
    source_path: String,
    size_bytes: u64,
    modified_at_unix: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FolderDialogResultDto {
    selected_path: Option<String>,
    opened_at_unix: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct IndexDatabaseInfoDto {
    database_path: String,
    schema_version: i64,
    migrations_applied: usize,
    tables: Vec<String>,
    image_count: i64,
    path_count: i64,
    tag_count: i64,
    created_at_unix: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IndexImageInputDto {
    path: String,
    file_name: String,
    extension: Option<String>,
    size_bytes: Option<u64>,
    modified_at_unix: Option<u64>,
    width: Option<u32>,
    height: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct IndexImageStateDto {
    ai_processed_at_unix: Option<u64>,
    description_format: Option<serde_json::Value>,
    source_revision: Option<String>,
    file_metadata: Option<serde_json::Value>,
    image_id: String,
    path: String,
    file_name: String,
    status: String,
    description: Option<String>,
    tags: Vec<String>,
    description_version: i64,
    tags_version: i64,
    is_favorite: bool,
    collections: Vec<String>,
    index_state: String,
    previous_path: Option<String>,
    first_seen_at_unix: Option<u64>,
    last_seen_at_unix: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct IndexSyncResultDto {
    database_path: String,
    received_images: usize,
    skipped_images: Vec<serde_json::Value>,
    inserted_or_updated_images: usize,
    current_paths_updated: usize,
    tags_linked: usize,
    new_images: usize,
    known_images: usize,
    relocated_images: usize,
    indexed_images: Vec<IndexImageStateDto>,
    schema_version: i64,
    image_count: i64,
    path_count: i64,
    synced_at_unix: u64,
}


#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IndexRescanPathsInputDto {
    root_path: Option<String>,
    limit: Option<u32>,
    after_path: Option<String>,
    excluded_paths: Option<Vec<String>>,
    only_paths: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct IndexRescanPathsResultDto {
    database_path: String,
    root_path: Option<String>,
    checked_paths: usize,
    existing_paths: usize,
    missing_paths: usize,
    revived_paths: usize,
    action_log_entries: usize,
    image_count: i64,
    path_count: i64,
    rescanned_at_unix: u64,
    next_after_path: Option<String>,
    skipped_paths: usize,
}




#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IndexMaintenanceInputDto {
    status: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct IndexMaintenanceImageDto {
    image_id: String,
    file_name: String,
    path: String,
    status: String,
    description: Option<String>,
    tags: Vec<String>,
    size_bytes: Option<u64>,
    modified_at_unix: Option<u64>,
    missing_at_unix: Option<u64>,
    last_seen_at_unix: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct IndexMaintenanceResultDto {
    database_path: String,
    status: String,
    total_matching_images: usize,
    returned_images: usize,
    missing_image_count: usize,
    orphan_tag_count: usize,
    non_current_path_count: usize,
    old_search_history_count: usize,
    images: Vec<IndexMaintenanceImageDto>,
    loaded_at_unix: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoveIndexImagesInputDto {
    image_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RemoveIndexImagesResultDto {
    database_path: String,
    requested_images: usize,
    removed_images: usize,
    missing_requested_images: usize,
    action_log_entries: usize,
    image_count: i64,
    path_count: i64,
    tag_count: i64,
    removed_at_unix: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CleanupIndexResultDto {
    database_path: String,
    orphan_tags_removed: usize,
    old_search_history_removed: usize,
    stale_finished_tasks_removed: usize,
    image_count: i64,
    path_count: i64,
    tag_count: i64,
    cleaned_at_unix: u64,
}


#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CollectionDto {
    id: String,
    name: String,
    description: Option<String>,
    color: Option<String>,
    is_favorite: bool,
    image_count: usize,
    created_at_unix: u64,
    updated_at_unix: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CollectionsResultDto {
    collections: Vec<CollectionDto>,
    total_count: usize,
    loaded_at_unix: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateCollectionInputDto {
    name: String,
    description: Option<String>,
    color: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddImagesToCollectionInputDto {
    collection_id: String,
    image_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CollectionMutationResultDto {
    collection: Option<CollectionDto>,
    requested_images: usize,
    changed_images: usize,
    skipped_images: usize,
    action_log_entries: usize,
    updated_at_unix: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetImageFavoriteInputDto {
    image_id: Option<String>,
    path: Option<String>,
    is_favorite: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SetImageFavoriteResultDto {
    image_id: String,
    path: String,
    file_name: String,
    is_favorite: bool,
    updated_at_unix: u64,
    action_log_id: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IndexMetadataUpdateInputDto {
    description_format: Option<serde_json::Value>,
    image_id: Option<String>,
    path: String,
    description: String,
    tags: Vec<String>,
    fields: Option<Vec<String>>,
    source: Option<String>,
    model: Option<String>,
    expected_description_version: Option<i64>,
    expected_tags_version: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct IndexMetadataUpdateResultDto {
    ai_processed_at_unix: Option<u64>,
    description_format: Option<serde_json::Value>,
    image_id: String,
    path: String,
    file_name: String,
    status: String,
    description: Option<String>,
    tags: Vec<String>,
    description_version: i64,
    tags_version: i64,
    updated_at_unix: u64,
    action_log_id: i64,
    image_count: i64,
    path_count: i64,
    tag_count: i64,
}


#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IndexSearchInputDto {
    fields:Option<Vec<String>>,
    untagged_only: Option<bool>,
    expand_terms: Option<bool>,
    query_id: Option<String>,
    collection_id: Option<String>,
    root_path: Option<String>,
    sort_order: Option<String>,
    date_field: Option<String>,
    date_from_local: Option<String>,
    date_to_local: Option<String>,
    date_to_exclusive: Option<bool>,
    query: String,
    statuses: Vec<String>,
    tags: Option<Vec<String>>,
    tag_mode: Option<String>,
    date_from_unix: Option<u64>,
    date_to_unix: Option<u64>,
    favorite_only: Option<bool>,
    limit: Option<u32>,
    offset: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct IndexSearchResultItemDto {
    ai_processed_at_unix: Option<u64>,
    description_format: Option<serde_json::Value>,
    file_metadata: Option<serde_json::Value>,
    image_id: String,
    path: String,
    file_name: String,
    extension: Option<String>,
    size_bytes: Option<u64>,
    modified_at_unix: Option<u64>,
    status: String,
    description: Option<String>,
    tags: Vec<String>,
    description_version: i64,
    tags_version: i64,
    is_favorite: bool,
    collections: Vec<String>,
    score: i64,
    matched_fields: Vec<String>,
    first_seen_at_unix: Option<u64>,
    last_seen_at_unix: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct IndexSearchResultDto {
    status_counts:serde_json::Value,
    query_id: String,
    has_more: bool,
    next_cursor: Option<u32>,
    query: String,
    total_count: usize,
    returned_count: usize,
    items: Vec<IndexSearchResultItemDto>,
    recent_searches: Vec<String>,
    applied_statuses: Vec<String>,
    applied_tags: Vec<String>,
    applied_tag_mode: String,
    applied_date_from_unix: Option<u64>,
    applied_date_to_unix: Option<u64>,
    applied_favorite_only: bool,
    searched_at_unix: u64,
}


#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActionJournalInputDto {
    image_id: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
    include_search_history: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ActionJournalEntryDto {
    id: String,
    source: String,
    action_type: String,
    image_id: Option<String>,
    image_file_name: Option<String>,
    image_path: Option<String>,
    old_value: Option<String>,
    new_value: Option<String>,
    message: String,
    created_at_unix: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    related_renames: Option<Vec<action_journal::JournalRename>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ActionJournalResultDto {
    total_count: usize,
    returned_count: usize,
    entries: Vec<ActionJournalEntryDto>,
    loaded_at_unix: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateActionLogEntryInputDto {
    action_type: String,
    image_id: Option<String>,
    old_value: Option<String>,
    new_value: Option<String>,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateActionLogEntryResultDto {
    action_log_id: i64,
    created_at_unix: u64,
}


#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SmartRenameInputDto {
    image_id: Option<String>,
    current_path: String,
    file_name: String,
    modified_at_unix: Option<u64>,
    pattern: String,
    plan_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SmartRenameResultDto {
    image_id: Option<String>,
    old_path: String,
    new_path: String,
    old_file_name: String,
    new_file_name: String,
    was_renamed: bool,
    size_bytes: Option<u64>,
    modified_at_unix: Option<u64>,
    action_log_id: Option<i64>,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SmartRenamePreviewItemDto {
    plan_id: Option<String>,
    image_id: Option<String>,
    old_path: String,
    new_path: String,
    old_file_name: String,
    new_file_name: String,
    status: String,
    message: String,
    size_bytes: Option<u64>,
    modified_at_unix: Option<u64>,
    candidate_collisions: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SmartRenamePreviewResultDto {
    total: usize,
    ready: usize,
    unchanged: usize,
    conflicts: usize,
    errors: usize,
    items: Vec<SmartRenamePreviewItemDto>,
    created_at_unix: u64,
}

#[derive(Debug)]
struct ExistingImageRecord {
    ai_processed_at_unix: Option<u64>,
    image_id: String,
    status: String,
    description: Option<String>,
    current_path: Option<String>,
}

#[derive(Debug)]
struct Migration {
    version: i64,
    name: &'static str,
    sql: &'static str,
}

const INDEX_SCHEMA_VERSION: i64 = INDEX_MIGRATIONS[INDEX_MIGRATIONS.len()-1].version;
static MIGRATION_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
static INDEX_SYNC_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

const INDEX_MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "core_index_schema",
    sql: r#"
        CREATE TABLE IF NOT EXISTS images (
            id TEXT PRIMARY KEY,
            identity_key TEXT NOT NULL,
            file_name TEXT NOT NULL,
            extension TEXT,
            size_bytes INTEGER,
            modified_at_unix INTEGER,
            width INTEGER,
            height INTEGER,
            sha256 TEXT,
            perceptual_hash TEXT,
            status TEXT NOT NULL DEFAULT 'notProcessed',
            description TEXT,
            ai_model TEXT,
            ai_processed_at_unix INTEGER,
            created_at_unix INTEGER NOT NULL,
            updated_at_unix INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS image_paths (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            image_id TEXT NOT NULL,
            path TEXT NOT NULL UNIQUE,
            is_current INTEGER NOT NULL DEFAULT 1,
            first_seen_at_unix INTEGER NOT NULL,
            last_seen_at_unix INTEGER NOT NULL,
            missing_at_unix INTEGER,
            FOREIGN KEY (image_id) REFERENCES images(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            kind TEXT NOT NULL DEFAULT 'ai',
            created_at_unix INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS image_tags (
            image_id TEXT NOT NULL,
            tag_id INTEGER NOT NULL,
            source TEXT NOT NULL DEFAULT 'ai',
            confidence REAL,
            created_at_unix INTEGER NOT NULL,
            PRIMARY KEY (image_id, tag_id),
            FOREIGN KEY (image_id) REFERENCES images(id) ON DELETE CASCADE,
            FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY,
            kind TEXT NOT NULL,
            status TEXT NOT NULL,
            image_id TEXT,
            title TEXT NOT NULL,
            message TEXT,
            progress INTEGER NOT NULL DEFAULT 0,
            created_at_unix INTEGER NOT NULL,
            updated_at_unix INTEGER NOT NULL,
            finished_at_unix INTEGER,
            FOREIGN KEY (image_id) REFERENCES images(id) ON DELETE SET NULL
        );

        CREATE TABLE IF NOT EXISTS action_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            action_type TEXT NOT NULL,
            image_id TEXT,
            old_value TEXT,
            new_value TEXT,
            message TEXT NOT NULL,
            created_at_unix INTEGER NOT NULL,
            FOREIGN KEY (image_id) REFERENCES images(id) ON DELETE SET NULL
        );

        CREATE TABLE IF NOT EXISTS search_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            query TEXT NOT NULL,
            result_count INTEGER NOT NULL DEFAULT 0,
            created_at_unix INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS settings_kv (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at_unix INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_images_status ON images(status);
        CREATE INDEX IF NOT EXISTS idx_images_identity_key ON images(identity_key);
        CREATE INDEX IF NOT EXISTS idx_images_file_name ON images(file_name);
        CREATE INDEX IF NOT EXISTS idx_image_paths_image_id ON image_paths(image_id);
        CREATE INDEX IF NOT EXISTS idx_image_paths_current ON image_paths(is_current);
        CREATE INDEX IF NOT EXISTS idx_image_paths_path ON image_paths(path);
        CREATE INDEX IF NOT EXISTS idx_tags_name ON tags(name);
        CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
        CREATE INDEX IF NOT EXISTS idx_action_logs_image_id ON action_logs(image_id);
        CREATE INDEX IF NOT EXISTS idx_search_history_created ON search_history(created_at_unix);
    "#,
}, Migration {
    version: 2,
    name: "favorites_and_collections",
    sql: r#"
        ALTER TABLE images ADD COLUMN is_favorite INTEGER NOT NULL DEFAULT 0;

        CREATE TABLE IF NOT EXISTS collections (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            description TEXT,
            color TEXT,
            is_favorite INTEGER NOT NULL DEFAULT 0,
            created_at_unix INTEGER NOT NULL,
            updated_at_unix INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS collection_images (
            collection_id TEXT NOT NULL,
            image_id TEXT NOT NULL,
            added_at_unix INTEGER NOT NULL,
            PRIMARY KEY (collection_id, image_id),
            FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE,
            FOREIGN KEY (image_id) REFERENCES images(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_images_favorite ON images(is_favorite);
        CREATE INDEX IF NOT EXISTS idx_collection_images_image_id ON collection_images(image_id);
        CREATE INDEX IF NOT EXISTS idx_collection_images_collection_id ON collection_images(collection_id);
    "#,
}, Migration {
    version: 3,
    name: "sha256_identity_index",
    sql: r#"
        CREATE INDEX IF NOT EXISTS idx_images_sha256 ON images(sha256);
    "#,
}, Migration {
    version: 4,
    name: "metadata_provenance_and_versions",
    sql: r#"
        ALTER TABLE images ADD COLUMN description_version INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE images ADD COLUMN tags_version INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE images ADD COLUMN manual_description_set INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE images ADD COLUMN manual_tags_set INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE images ADD COLUMN description_origin TEXT NOT NULL DEFAULT 'legacy';
        ALTER TABLE images ADD COLUMN ai_description TEXT;
        ALTER TABLE images ADD COLUMN legacy_description TEXT;
        ALTER TABLE images ADD COLUMN status_before_missing TEXT;
        UPDATE images SET legacy_description = description;
        CREATE TABLE image_metadata_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            image_id TEXT NOT NULL REFERENCES images(id) ON DELETE CASCADE,
            source TEXT NOT NULL,
            model TEXT,
            payload TEXT NOT NULL,
            created_at_unix INTEGER NOT NULL
        );
        CREATE INDEX idx_metadata_history_image ON image_metadata_history(image_id, id);
    "#,
}, Migration {
    version: 5,
    name: "durable_file_operation_intentions",
    sql: r#"
        CREATE TABLE file_operations (
            id TEXT PRIMARY KEY,
            kind TEXT NOT NULL,
            image_id TEXT REFERENCES images(id) ON DELETE SET NULL,
            source_path TEXT NOT NULL,
            target_path TEXT,
            sha256 TEXT NOT NULL,
            size_bytes INTEGER NOT NULL,
            modified_at_unix INTEGER,
            status TEXT NOT NULL,
            error TEXT,
            created_at_unix INTEGER NOT NULL,
            updated_at_unix INTEGER NOT NULL
        );
        CREATE INDEX idx_file_operations_status ON file_operations(status);
    "#,
}, Migration {
    version: 6,
    name: "native_processing_checkpoint_and_ledger",
    sql: processing::SCHEMA,
}, Migration {
    version: 7,
    name: "full_effective_text_fts5",
    sql: include_str!("search-schema.sql"),
}, Migration {
    version: 8,
    name: "os_protected_provider_credentials",
    sql: credentials::SCHEMA,
}, Migration {
    version: 9,
    name: "fts_updates_by_native_row_identity",
    sql: include_str!("search-schema-rowids.sql"),
}, Migration {
    version: 10,
    name: "compact_processing_observer_and_shared_service_cooldowns",
    sql: include_str!("processing-observer.sql"),
}, Migration {
    version: 11,
    name: "per_tag_provenance_and_user_suppression",
    sql: include_str!("tag-layers.sql"),
}, Migration {
    version: 12,
    name: "typed_capture_dates_and_physical_file_metadata",
    sql: image_metadata::SCHEMA,
}, Migration {
    version: 13,
    name: "bounded_logical_image_path_lookup",
    sql: "CREATE INDEX idx_image_paths_logical_current ON image_paths(image_id,is_current);",
}, Migration {
    version: 14,
    name: "fts_path_identity_updates_without_observation_rewrites",
    sql: include_str!("search-path-updates.sql"),
}, Migration {
    version: 15,
    name: "fts_skips_unchanged_image_text_observations",
    sql: include_str!("search-image-updates.sql"),
}, Migration {
    version: 16,
    name: "typed_manual_description_format",
    sql: "ALTER TABLE images ADD COLUMN description_format TEXT;",
}, Migration {
    version: 17,
    name: "frozen_analysis_output_modes_require_compatible_writer",
    // A compatibility marker protects new tag-only jobs from older writers
    // that would otherwise commit their short response into description too.
    sql: "SELECT 1;",
}, Migration {
    version:18,
    name:"validated_persistent_file_observations",
    sql:"CREATE TABLE file_observations (path TEXT PRIMARY KEY REFERENCES image_paths(path) ON DELETE CASCADE, image_id TEXT NOT NULL REFERENCES images(id) ON DELETE CASCADE, format_version INTEGER NOT NULL, stamp TEXT NOT NULL, sha256 TEXT NOT NULL, metadata TEXT NOT NULL);",
}, Migration {
    version:19,
    name:"verified_duplicate_group_plans",
    sql:"CREATE TABLE duplicate_plans(id TEXT PRIMARY KEY,image_id TEXT NOT NULL REFERENCES images(id) ON DELETE CASCADE,sha256 TEXT NOT NULL,paths TEXT NOT NULL,state TEXT NOT NULL DEFAULT 'ready',created_at_unix INTEGER NOT NULL,result TEXT);",
}, Migration {
    version:20,
    name:"provider_readiness_for_immutable_credentials",
    sql:"CREATE TABLE provider_readiness(provider TEXT NOT NULL,credential_ref TEXT NOT NULL,model TEXT NOT NULL,result TEXT NOT NULL,checked_at_unix INTEGER NOT NULL,PRIMARY KEY(provider,credential_ref,model));",
}, Migration {
    version:21,
    name:"bounded_processing_source_materialization",
    sql:"CREATE TABLE processing_sources(job_id TEXT NOT NULL REFERENCES processing_jobs(id) ON DELETE CASCADE,ordinal INTEGER NOT NULL,path TEXT NOT NULL COLLATE NOCASE,PRIMARY KEY(job_id,ordinal),UNIQUE(job_id,path));",
}, Migration {
    version:22,
    name:"identity_bound_analysis_copy_recovery",
    sql:"ALTER TABLE file_operations ADD COLUMN stage_identity TEXT;",
}, Migration {
    version:23,
    name:"processing_task_context_commands_and_run_ownership",
    sql:processing::queue::SCHEMA,
}, Migration {
    version:24,
    name:"content_bound_analysis_field_readiness",
    sql:processing::fields::SCHEMA,
}, Migration {
    version:25,
    name:"materialized_sources_per_processing_command",
    sql:include_str!("processing-command-sources.sql"),
}, Migration {
    version:26,
    name:"working_queue_visibility_keeps_manual_history",
    sql:"ALTER TABLE processing_jobs ADD COLUMN work_visible INTEGER NOT NULL DEFAULT 1 CHECK(work_visible IN (0,1));",
}, Migration {
    version:27,
    name:"explorer_file_action_journal",
    sql:"ALTER TABLE file_operations ADD COLUMN payload TEXT;",
}, Migration {
    version:28,
    name:"accepted_exact_duplicate_compositions",
    sql:"CREATE TABLE duplicate_decisions(fingerprint TEXT PRIMARY KEY,sha256 TEXT NOT NULL,paths TEXT NOT NULL,created_at_unix INTEGER NOT NULL);",
}];

#[tauri::command]
async fn choose_folder_dialog(initial_path: Option<String>, ui_locale:Option<String>) -> Result<FolderDialogResultDto, String> {
    let mut dialog = rfd::AsyncFileDialog::new().set_title(crate::ui_locale_generated::caption(ui_locale.as_deref(),"dialog.choose-folder"));
    if let Some(path) = initial_path.as_ref().map(|value| value.trim()).filter(|value| !value.is_empty()) {
        let candidate = PathBuf::from(path);
        let folder = if candidate.is_dir() {
            candidate
        } else {
            candidate.parent().map(Path::to_path_buf).unwrap_or(candidate)
        };
        if folder.exists() && folder.is_dir() {
            dialog = dialog.set_directory(folder);
        }
    }

    Ok(FolderDialogResultDto {
        selected_path: dialog.pick_folder().await.map(|file| path_to_string(file.path())),
        opened_at_unix: unix_now(),
    })
}

#[tauri::command]
async fn inspect_dropped_paths(paths: Vec<String>) -> Result<DroppedPathsInspectionResultDto, String> {
    if paths.len()>512{return Err("За один раз можно открыть до 512 путей. Для большого архива выберите папку.".into());}
    tauri::async_runtime::spawn_blocking(move||inspect_dropped_paths_blocking(paths)).await.map_err(|e|e.to_string())?
}
fn inspect_dropped_paths_blocking(paths: Vec<String>) -> Result<DroppedPathsInspectionResultDto, String> {
    let mut clean_paths = Vec::new();
    let mut items = Vec::new();
    let mut image_count = 0usize;
    let mut folder_count = 0usize;
    let mut file_count = 0usize;
    let mut unsupported_count = 0usize;
    let mut skipped_entries = 0usize;

    for raw_path in paths.into_iter().take(512) {
        let trimmed = raw_path.trim();
        if trimmed.is_empty() || clean_paths.iter().any(|existing| existing == trimmed) {
            continue;
        }

        let path = PathBuf::from(trimmed);
        clean_paths.push(trimmed.to_string());

        let metadata = match fs::metadata(&path) {
            Ok(value) => value,
            Err(_) => {
                unsupported_count += 1;
                continue;
            }
        };

        if metadata.is_dir() {
            folder_count += 1;
            items.push(folder_entry(&path, &mut skipped_entries));
            continue;
        }

        if metadata.is_file() {
            if is_supported_image(&path) {
                image_count += 1;
                items.push(file_entry(&path, &metadata, true));
            } else {
                file_count += 1;
                items.push(file_entry(&path, &metadata, false));
            }
            continue;
        }

        unsupported_count += 1;
    }

    items.sort_by(|left, right| match (left.kind.as_str(), right.kind.as_str()) {
        ("folder", "folder") => left.name.to_lowercase().cmp(&right.name.to_lowercase()),
        ("folder", _) => std::cmp::Ordering::Less,
        (_, "folder") => std::cmp::Ordering::Greater,
        ("image", "file") => std::cmp::Ordering::Less,
        ("file", "image") => std::cmp::Ordering::Greater,
        _ => left.name.to_lowercase().cmp(&right.name.to_lowercase()),
    });

    Ok(DroppedPathsInspectionResultDto {
        paths: clean_paths,
        items,
        image_count,
        folder_count,
        file_count,
        unsupported_count,
        inspected_at_unix: unix_now(),
    })
}

#[tauri::command]
async fn open_external_path(app_handle: tauri::AppHandle, path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri_plugin_opener::OpenerExt;
        let target = PathBuf::from(path);
        if !target.is_absolute() { return Err("Для открытия нужен абсолютный путь к файлу или папке.".into()); }
        let metadata = fs::metadata(&target).map_err(|error| format!("Путь недоступен: {error}"))?;
        if !metadata.is_file() && !metadata.is_dir() { return Err("Путь не является файлом или папкой.".into()); }
        app_handle.opener().open_path(path_to_string(&target), None::<&str>).map_err(|error| error.to_string())
    }).await.map_err(|error| error.to_string())?
}

fn maintain_selected_previews(folder:&Path,current:&Path)->Result<(),String>{
 let mut entries=vec![];let mut total=0u64;let base=folder.canonicalize().map_err(|e|e.to_string())?;
 for entry in fs::read_dir(&base).map_err(|e|e.to_string())?{let entry=entry.map_err(|e|e.to_string())?;let name=entry.file_name().to_string_lossy().to_string();let Some(id)=name.strip_prefix("selected-").and_then(|s|s.strip_suffix(".png"))else{continue;};if id.len()!=16||!id.bytes().all(|b|b.is_ascii_hexdigit()){continue;}let path=entry.path();let meta=fs::symlink_metadata(&path).map_err(|e|e.to_string())?;if !meta.is_file()||meta.file_type().is_symlink(){continue;}
 #[cfg(windows)]{use std::os::windows::fs::MetadataExt;if meta.file_attributes()&0x400!=0{continue;}}
 let canonical=path.canonicalize().map_err(|e|e.to_string())?;if canonical.parent()!=Some(base.as_path()){continue;}total+=meta.len();entries.push((meta.modified().unwrap_or(UNIX_EPOCH),canonical,meta.len()));}
 entries.sort_by_key(|e|e.0);let mut count=entries.len();let keep=current.canonicalize().map_err(|e|e.to_string())?;for(_,path,bytes)in entries{if count<=64&&total<=128*1024*1024{break;}if path==keep{continue;}fs::remove_file(&path).map_err(|e|e.to_string())?;count-=1;total=total.saturating_sub(bytes);}Ok(())
}

#[tauri::command]
async fn get_selected_photo_preview(app_handle:tauri::AppHandle,image_path:String)->Result<serde_json::Value,String>{
 let permit=file_safety::image_worker().acquire_owned().await.map_err(|e|e.to_string())?;
 tauri::async_runtime::spawn_blocking(move||{let _permit=permit;let source=validate_existing_image_file(&image_path)?.canonicalize().map_err(|e|e.to_string())?;let revision=index_observations::content_revision(&source)?;let cache_root=thumbnail_cache::directory(&app_handle)?;let folder=cache_root.join("selected");fs::create_dir_all(&folder).map_err(|e|e.to_string())?;if folder.canonicalize().map_err(|e|e.to_string())?.parent()!=Some(cache_root.as_path()){return Err("Каталог подробных превью перенаправлен за пределы кэша".into());}let hash=stable_text_id("selected",&format!("{}|{revision}|2048-alpha-v2",path_to_string(&source)));let output=folder.join(format!("{}.png",hash.replace(':',"-")));
 let fresh=!output.is_file();if fresh{let decoded=file_safety::decode_bounded(&source)?;let preview=if decoded.width()>2048||decoded.height()>2048{decoded.resize(2048,2048,image::imageops::FilterType::Triangle)}else{decoded};file_safety::atomic_save(&output,|out,_|preview.write_to(out,image::ImageFormat::Png).map_err(|e|e.to_string()))?;}
 thumbnail_cache::touch(&output);if fresh{maintain_selected_previews(&folder,&output)?;}
 if index_observations::content_revision(&source)?!=revision{return Err("Фотография изменилась во время подготовки превью".into());}app_handle.asset_protocol_scope().allow_file(&output).map_err(|e|e.to_string())?;let (width,height)=image::image_dimensions(&output).map_err(|e|e.to_string())?;Ok(serde_json::json!({"path":path_to_string(&output),"width":width,"height":height}))
 }).await.map_err(|e|e.to_string())?
}

#[tauri::command]
async fn get_cached_thumbnail(app_handle:tauri::AppHandle,image_path:String,width:u32,height:u32)->Result<Option<ThumbnailResultDto>,String>{
    // Cache validation never waits for the decoder pool and never rebuilds a miss.
    tauri::async_runtime::spawn_blocking(move||get_or_create_thumbnail_internal(app_handle,image_path,width,height,false,true)).await.map_err(|e|e.to_string())?
}

#[tauri::command]
async fn get_or_create_thumbnail(
    app_handle: tauri::AppHandle,
    image_path: String,
    width: u32,
    height: u32,
    force_rebuild: Option<bool>,
) -> Result<ThumbnailResultDto, String> {
    static POOL:std::sync::OnceLock<std::sync::Arc<tokio::sync::Semaphore>>=std::sync::OnceLock::new();
    let pool=POOL.get_or_init(||std::sync::Arc::new(tokio::sync::Semaphore::new(2))).clone();
    let permit=pool.acquire_owned().await.map_err(|e|e.to_string())?;
    tauri::async_runtime::spawn_blocking(move||{
        let _permit=permit;
        get_or_create_thumbnail_internal(app_handle,image_path,width,height,force_rebuild.unwrap_or(false),false)?.ok_or_else(||"Thumbnail was not created.".to_string())
    }).await.map_err(|e|e.to_string())?
}

fn get_or_create_thumbnail_internal(app_handle:tauri::AppHandle,image_path:String,width:u32,height:u32,force_rebuild:bool,cache_only:bool)->Result<Option<ThumbnailResultDto>,String>{
    thumbnail_cache::start(&app_handle);
    let _cache_guard=thumbnail_cache::GATE.read().map_err(|_|"Thumbnail cache lock failed.")?;
    let source_path = PathBuf::from(image_path.trim());
    if image_path.trim().is_empty() {
        return Err("Image path is empty.".to_string());
    }

    if !source_path.exists() {
        return Err(format!("Image file does not exist: {}", source_path.display()));
    }

    if !source_path.is_file() {
        return Err(format!("Path is not a file: {}", source_path.display()));
    }

    let metadata = fs::metadata(&source_path)
        .map_err(|error| format!("Cannot read image metadata {}: {error}", source_path.display()))?;

    let safe_width = width.clamp(64, 1024);
    let safe_height = height.clamp(64, 1024);
    let modified = modified_at_unix(&metadata).unwrap_or(0);
    let revision=index_observations::content_revision(&source_path)?;
    let base_key = thumbnail_cache_key(&source_path, metadata.len(), modified, safe_width, safe_height);
    let cache_key=format!("srgb-v3-{:x}",Sha256::digest(format!("{base_key}-{revision}").as_bytes()));
    let _same_key_guard=thumbnail_cache::lock_key(cache_key.clone())?;
    let cache_dir = thumbnail_cache::directory(&app_handle)?;
    fs::create_dir_all(&cache_dir)
        .map_err(|error| format!("Cannot create thumbnail cache folder {}: {error}", cache_dir.display()))?;

    // JPEG sources cannot carry alpha. Other supported formats may, so their
    // thumbnails use PNG; dropping alpha in JPEG had turned transparent pixels black.
    let source_format=image::ImageReader::open(&source_path)
        .and_then(|reader|reader.with_guessed_format())
        .map_err(|error|format!("Cannot inspect thumbnail source format: {error}"))?
        .format();
    let preserves_alpha=source_format!=Some(image::ImageFormat::Jpeg);
    let thumbnail_path = cache_dir.join(if preserves_alpha {
        format!("{}.png",cache_key.replacen("srgb-v3-","srgb-alpha-v1-",1))
    } else {format!("{cache_key}.jpg")});
    thumbnail_cache::validate_entry(&thumbnail_path)?;
    let now = unix_now();

    if !force_rebuild && thumbnail_path.exists() && image::image_dimensions(&thumbnail_path).is_ok() {
        if revision!=index_observations::content_revision(&source_path)?{return Err("Source changed while preparing preview.".into());}
        thumbnail_cache::touch(&thumbnail_path);app_handle.asset_protocol_scope().allow_file(&thumbnail_path).map_err(|e|e.to_string())?;
        return Ok(Some(ThumbnailResultDto {
            source_path: path_to_string(&source_path),
            thumbnail_path: path_to_string(&thumbnail_path),
            width: safe_width,
            height: safe_height,
            was_created: false,
            created_at_unix: now,
        }));
    }

    if cache_only{return Ok(None);}
    if thumbnail_path.exists(){fs::remove_file(&thumbnail_path).map_err(|e|format!("Cannot discard incomplete cached preview: {e}"))?;}
    let image = file_safety::decode_bounded(&source_path)
        .map_err(|error| format!("Cannot decode image for thumbnail {}: {error}", source_path.display()))?;
    let thumbnail = image.thumbnail(safe_width, safe_height);

    let saved=file_safety::atomic_save(&thumbnail_path,|output,_|{
        if preserves_alpha {
            thumbnail.write_to(output,image::ImageFormat::Png).map_err(|error|format!("Cannot encode alpha thumbnail: {error}"))?;
        } else {
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(output,82);
            encoder.encode_image(&thumbnail).map_err(|error|format!("Cannot encode thumbnail: {error}"))?;
        }
        if revision!=index_observations::content_revision(&source_path)?{return Err("Source changed while preparing preview.".into());}Ok(())
    });
    if revision!=index_observations::content_revision(&source_path)?{return Err("Source changed while preparing preview.".into());}
    if let Err(error)=saved {if !thumbnail_path.exists() || image::image_dimensions(&thumbnail_path).is_err(){return Err(error);}}

    app_handle.asset_protocol_scope().allow_file(&thumbnail_path).map_err(|e|e.to_string())?;
    Ok(Some(ThumbnailResultDto {
        source_path: path_to_string(&source_path),
        thumbnail_path: path_to_string(&thumbnail_path),
        width: safe_width,
        height: safe_height,
        was_created: true,
        created_at_unix: now,
    }))
}

#[tauri::command]
fn get_ai_image_file_fingerprint(image_path: String) -> Result<AiImageFileFingerprintDto, String> {
    let trimmed = image_path.trim();
    if trimmed.is_empty() {
        return Err("Image path is empty.".to_string());
    }

    let source_path = PathBuf::from(trimmed);

    if !source_path.exists() {
        return Err(format!("Image file does not exist: {}", source_path.display()));
    }

    if !source_path.is_file() {
        return Err(format!("Path is not a file: {}", source_path.display()));
    }

    let metadata = fs::metadata(&source_path)
        .map_err(|error| format!("Cannot read image metadata {}: {error}", source_path.display()))?;

    Ok(AiImageFileFingerprintDto {
        source_path: path_to_string(&source_path),
        size_bytes: metadata.len(),
        modified_at_unix: system_time_to_unix_optional(metadata.modified()),
    })
}

#[tauri::command]
async fn encode_image_for_ai(image_path: String) -> Result<AiImageDataUrlResultDto, String> {
    tauri::async_runtime::spawn_blocking(move||encode_image_for_ai_internal(image_path)).await.map_err(|e|e.to_string())?
}
fn encode_image_for_ai_internal(image_path: String) -> Result<AiImageDataUrlResultDto, String> {
    let source_path = PathBuf::from(image_path.trim());
    if image_path.trim().is_empty() {
        return Err("Image path is empty.".to_string());
    }

    if !source_path.exists() {
        return Err(format!("Image file does not exist: {}", source_path.display()));
    }

    if !source_path.is_file() {
        return Err(format!("Path is not a file: {}", source_path.display()));
    }

    let metadata = fs::metadata(&source_path)
        .map_err(|error| format!("Cannot read image metadata {}: {error}", source_path.display()))?;
    let original_size_bytes = metadata.len();

    match file_safety::decode_bounded(&source_path) {
        Ok(image) => {
            let original_width = image.width();
            let original_height = image.height();
            let should_resize = original_width > AI_IMAGE_MAX_SIDE || original_height > AI_IMAGE_MAX_SIDE;
            let encoded_image = if should_resize {
                image.thumbnail(AI_IMAGE_MAX_SIDE, AI_IMAGE_MAX_SIDE)
            } else {
                image
            };

            let mut encoded_bytes = Vec::new();
            {
                let mut encoder = JpegEncoder::new_with_quality(&mut encoded_bytes, AI_IMAGE_JPEG_QUALITY);
                encoder
                    .encode_image(&encoded_image)
                    .map_err(|error| format!("Cannot encode image for AI {}: {error}", source_path.display()))?;
            }

            let encoded_base64 = general_purpose::STANDARD.encode(&encoded_bytes);
            Ok(AiImageDataUrlResultDto {
                source_path: path_to_string(&source_path),
                data_url: format!("data:image/jpeg;base64,{encoded_base64}"),
                mime_type: "image/jpeg".to_string(),
                original_size_bytes,
                encoded_size_bytes: encoded_bytes.len(),
                width: Some(encoded_image.width()),
                height: Some(encoded_image.height()),
                was_resized: should_resize,
                encoded_at_unix: unix_now(),
            })
        }
        Err(decode_error) => Err(decode_error),
    }
}

#[tauri::command]
async fn initialize_index_database(app_handle: tauri::AppHandle) -> Result<IndexDatabaseInfoDto, String> {
    tauri::async_runtime::spawn_blocking(move || initialize_index_database_internal(&app_handle))
        .await.map_err(|error| format!("Database initialization worker failed: {error}"))?
}

#[tauri::command]
async fn get_index_database_info(app_handle: tauri::AppHandle) -> Result<IndexDatabaseInfoDto, String> {
    tauri::async_runtime::spawn_blocking(move || get_index_database_info_internal(app_handle)).await.map_err(|error| error.to_string())?
}

fn get_index_database_info_internal(app_handle: tauri::AppHandle) -> Result<IndexDatabaseInfoDto, String> {
    let mut connection = open_index_connection(&app_handle)?;
    let database_path = index_database_path(&app_handle)?;
    run_index_migrations(&mut connection)?;
    build_index_database_info(&connection, database_path, 0)
}

#[tauri::command]
async fn sync_scanned_images_to_index(
    app_handle: tauri::AppHandle,
    images: Vec<IndexImageInputDto>,
    reuse_observed_identity: Option<bool>,
) -> Result<IndexSyncResultDto, String> {
    tauri::async_runtime::spawn_blocking(move || sync_scanned_images_to_index_observed(app_handle, images, reuse_observed_identity.unwrap_or(false)))
        .await.map_err(|error| format!("Index worker failed: {error}"))?
}

fn sync_scanned_images_to_index_internal(
    app_handle: tauri::AppHandle,
    images: Vec<IndexImageInputDto>,
) -> Result<IndexSyncResultDto, String> {
    sync_scanned_images_to_index_observed(app_handle, images, false)
}

fn sync_scanned_images_to_index_observed(
    app_handle: tauri::AppHandle, images: Vec<IndexImageInputDto>, reuse_observed_identity: bool,
) -> Result<IndexSyncResultDto, String> {
    let _sync_guard = INDEX_SYNC_LOCK.lock().map_err(|_| "Index worker lock failed.".to_string())?;
    let mut connection = open_index_connection(&app_handle)?;
    let database_path = index_database_path(&app_handle)?;
    run_index_migrations(&mut connection)?;
    // File reads precede the write transaction; one disappearing path cannot abort the folder.
    let mut observations=Vec::new();let mut skipped_images=Vec::new();
    for image in &images {
        if source_paths::excluded(Path::new(&image.path)){skipped_images.push(serde_json::json!({"path":image.path,"reason":"Служебный каталог Windows исключён из фотоисточников."}));continue;}
        match index_observations::observe(&connection,Path::new(image.path.trim()),reuse_observed_identity){
        Ok((sha,metadata,stamp))=>observations.push((image,sha,metadata,stamp)),
        Err(error) if reuse_observed_identity=>skipped_images.push(serde_json::json!({"path":image.path,"reason":error})),
        Err(error)=>return Err(error),
    }}
    let now = unix_now();
    let transaction = connection
        // Reserve the writer before reading the snapshot we will update. In
        // WAL, promoting a deferred snapshot after another writer commits
        // fails with BUSY_SNAPSHOT immediately, regardless of busy_timeout.
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|error| format!("Cannot start index sync transaction: {error}"))?;

    let mut inserted_or_updated_images = 0usize;
    let mut current_paths_updated = 0usize;
    let tags_linked = 0usize;
    let mut new_images = 0usize;
    let mut known_images = 0usize;
    let mut relocated_images = 0usize;
    let mut indexed_images = Vec::new();

    for (image,fingerprint,file_metadata,observed_stamp) in &observations {
        let source_revision=Some(index_observations::observed_revision(observed_stamp,fingerprint));
        if image.path.trim().is_empty() || image.file_name.trim().is_empty() {
            continue;
        }

        let incoming_path = image.path.trim();
        let sha256 = Some(fingerprint.clone());
        let identity_key = image_identity_key(image, sha256.as_deref());
        let mut existing_by_path = find_existing_image_by_path(&transaction, incoming_path)?;
        if let Some(record) = existing_by_path.as_ref() {
            let old_hash = read_indexed_sha256(&transaction, &record.image_id)?;
            if old_hash.as_ref().is_some_and(|hash| Some(hash) != sha256.as_ref()) {
                existing_by_path = None;
            }
        }
        let existing_by_sha256 = if existing_by_path.is_none() {
            match sha256.as_deref() {
                Some(value) => find_existing_image_by_sha256(&transaction, value)?,
                None => None,
            }
        } else {
            None
        };
        let mut existing_by_identity = if existing_by_path.is_none() && existing_by_sha256.is_none() {
            find_existing_image_by_identity(&transaction, &legacy_image_identity_key(image))?
        } else {
            None
        };
        if let Some(record) = existing_by_identity.as_ref() {
            if read_indexed_sha256(&transaction, &record.image_id)?.is_some() {
                existing_by_identity = None;
            }
        }

        let existing_record = existing_by_path
            .as_ref()
            .or(existing_by_sha256.as_ref())
            .or(existing_by_identity.as_ref());
        let image_id = existing_record
            .map(|record| record.image_id.clone())
            .unwrap_or_else(|| stable_text_id("image", &identity_key));
        let previous_path = existing_by_sha256
            .as_ref()
            .or(existing_by_identity.as_ref())
            .and_then(|record| record.current_path.clone())
            .filter(|path| path != incoming_path && fs::metadata(path).is_err_and(|e|e.kind()==std::io::ErrorKind::NotFound) && missing_path_is_observable(Path::new(path)));
        let index_state = if existing_by_path.is_some() {
            known_images += 1;
            "known"
        } else if previous_path.is_some() {
            relocated_images += 1;
            "pathUpdated"
        } else if existing_record.is_some() {
            known_images += 1;
            "known"
        } else {
            new_images += 1;
            "new"
        };

        let incoming_status = "notProcessed";
        let status = existing_record
            .map(|record| record.status.as_str())
            .filter(|value| *value == "processed" || *value == "error" || *value == "queued")
            .unwrap_or(incoming_status);
        let size_bytes = image.size_bytes.map(u64_to_i64_saturating);
        let modified_at_unix = image.modified_at_unix.map(u64_to_i64_saturating);
        let width = image.width.map(i64::from);
        let height = image.height.map(i64::from);
        let description: Option<&str> = None;

        transaction
            .execute(
                r#"
                INSERT INTO images (
                    id, identity_key, file_name, extension, size_bytes, modified_at_unix,
                    width, height, sha256, status, description, created_at_unix, updated_at_unix
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)
                ON CONFLICT(id) DO UPDATE SET
                    identity_key = excluded.identity_key,
                    file_name = CASE WHEN ?13 THEN excluded.file_name ELSE images.file_name END,
                    extension = excluded.extension,
                    size_bytes = excluded.size_bytes,
                    modified_at_unix = excluded.modified_at_unix,
                    width = COALESCE(excluded.width, images.width),
                    height = COALESCE(excluded.height, images.height),
                    sha256 = COALESCE(excluded.sha256, images.sha256),
                    status = CASE
                        WHEN images.status = 'processed' THEN images.status
                        WHEN images.status = 'error' THEN images.status
                        WHEN images.status = 'queued' THEN images.status
                        ELSE excluded.status
                    END,
                    description = COALESCE(images.description, excluded.description),
                    updated_at_unix = excluded.updated_at_unix
                "#,
                params![
                    &image_id,
                    &identity_key,
                    image.file_name.trim(),
                    image.extension.as_deref(),
                    size_bytes,
                    modified_at_unix,
                    width,
                    height,
                    sha256.as_deref(),
                    status,
                    description,
                    u64_to_i64_saturating(now),
                    previous_path.is_some(),
                ],
            )
            .map_err(|error| format!("Cannot upsert indexed image {}: {error}", image.file_name))?;
        inserted_or_updated_images += 1;

        if let Some(old_path) = previous_path.as_ref() {
            transaction.execute(
                "UPDATE image_paths SET is_current = 0, missing_at_unix = ?3 WHERE image_id = ?1 AND path = ?2",
                params![&image_id, old_path, u64_to_i64_saturating(now)],
            ).map_err(|error| format!("Cannot update relocated path: {error}"))?;
        }

        transaction
            .execute(
                r#"
                INSERT INTO image_paths (image_id, path, is_current, first_seen_at_unix, last_seen_at_unix, missing_at_unix)
                VALUES (?1, ?2, 1, ?3, ?3, NULL)
                ON CONFLICT(path) DO UPDATE SET
                    image_id = excluded.image_id,
                    is_current = 1,
                    last_seen_at_unix = excluded.last_seen_at_unix,
                    missing_at_unix = NULL
                "#,
                params![&image_id, incoming_path, u64_to_i64_saturating(now)],
            )
            .map_err(|error| format!("Cannot upsert current path for {}: {error}", image.file_name))?;
        current_paths_updated += 1;
        image_metadata::save(&transaction,&image_id,incoming_path,file_metadata)?;
        index_observations::persist(&transaction,Path::new(incoming_path),&image_id,observed_stamp,fingerprint,file_metadata)?;

        if let Some(previous_path_value) = previous_path.as_ref() {
            transaction
                .execute(
                    r#"
                    INSERT INTO action_logs (action_type, image_id, old_value, new_value, message, created_at_unix)
                    VALUES ('path_updated', ?1, ?2, ?3, ?4, ?5)
                    "#,
                    params![
                        &image_id,
                        previous_path_value,
                        incoming_path,
                        format!("Image path refreshed by filesystem scan: {}", image.file_name.trim()),
                        u64_to_i64_saturating(now),
                    ],
                )
                .map_err(|error| format!("Cannot write path update action log for {}: {error}", image.file_name))?;
        }


        let saved_record = find_existing_image_by_path(&transaction, incoming_path)?;
        let tags = read_tags_for_image(&transaction, &image_id)?;
        let (saved_status, saved_description) = saved_record
            .as_ref()
            .map(|record| (record.status.clone(), record.description.clone()))
            .unwrap_or_else(|| (status.to_string(), description.map(ToOwned::to_owned)));
        let saved_is_favorite = read_image_favorite_from_transaction(&transaction, &image_id)?;
        let saved_collections = read_collection_names_for_image_from_transaction(&transaction, &image_id)?;
        let (first_seen_at_unix, last_seen_at_unix) = read_path_seen_times(&transaction, incoming_path)?;

        let (description_version, tags_version) = read_metadata_versions(&transaction, &image_id)?;
        indexed_images.push(IndexImageStateDto {
            ai_processed_at_unix: saved_record.as_ref().and_then(|r|r.ai_processed_at_unix),
            description_format:description_format::read(&transaction,&image_id)?,
            source_revision:source_revision.clone(),
            file_metadata:Some(serde_json::to_value(file_metadata).map_err(|e|e.to_string())?),
            image_id,
            path: incoming_path.to_string(),
            file_name: image.file_name.trim().to_string(),
            status: saved_status,
            description: saved_description,
            tags,
            description_version,
            tags_version,
            is_favorite: saved_is_favorite,
            collections: saved_collections,
            index_state: index_state.to_string(),
            previous_path,
            first_seen_at_unix,
            last_seen_at_unix,
        });
    }

    transaction
        .commit()
        .map_err(|error| format!("Cannot commit index sync transaction: {error}"))?;

    let image_count = count_rows(&connection, "images")?;
    let path_count = count_rows(&connection, "image_paths")?;

    Ok(IndexSyncResultDto {
        database_path: path_to_string(&database_path),
        received_images: images.len(),
        skipped_images,
        inserted_or_updated_images,
        current_paths_updated,
        tags_linked,
        new_images,
        known_images,
        relocated_images,
        indexed_images,
        schema_version: INDEX_SCHEMA_VERSION,
        image_count,
        path_count,
        synced_at_unix: now,
    })
}


#[derive(Debug)]
struct CurrentIndexedPathRow {
    image_id: String,
    file_name: String,
    path: String,
    status: String,
    missing_at_unix: Option<i64>,
}

fn path_is_under_rescan_root(path: &str, root: &Option<PathBuf>) -> bool {
    let Some(root_path) = root else {
        return true;
    };

    let candidate = PathBuf::from(path);
    if candidate.starts_with(root_path) {
        return true;
    }

    let root_text = path_to_string(root_path).replace('\\', "/").trim_end_matches('/').to_lowercase();
    let candidate_text = path_to_string(Path::new(path)).replace('\\', "/").to_lowercase();
    candidate_text == root_text || candidate_text.starts_with(&(root_text + "/"))
}

fn missing_path_is_observable(path:&Path)->bool {
    // Distinguish a missing file/directory from an offline volume or a denied parent.
    for parent in path.ancestors().skip(1){
        match fs::read_dir(parent){Ok(_)=>return true,Err(e) if e.kind()==std::io::ErrorKind::NotFound=>continue,Err(_)=>return false}
    }false
}

#[tauri::command]
async fn rescan_index_paths(app_handle: tauri::AppHandle, input: IndexRescanPathsInputDto) -> Result<IndexRescanPathsResultDto, String> {
    tauri::async_runtime::spawn_blocking(move || rescan_index_paths_internal(app_handle, input)).await.map_err(|error| error.to_string())?
}

fn rescan_index_paths_internal(
    app_handle: tauri::AppHandle,
    input: IndexRescanPathsInputDto,
) -> Result<IndexRescanPathsResultDto, String> {
    let mut connection = open_index_connection(&app_handle)?;
    let database_path = index_database_path(&app_handle)?;
    run_index_migrations(&mut connection)?;

    let now = unix_now();
    let limit = input.limit.unwrap_or(256).clamp(1, 1024) as usize;
    let root_filter = input
        .root_path
        .as_ref()
        .map(|raw| raw.trim())
        .filter(|raw| !raw.is_empty())
        .map(|raw| {
            let path = PathBuf::from(raw);
            path.canonicalize().unwrap_or(path)
        });
    // An unavailable source is not evidence that every photograph has disappeared.
    if let Some(root) = &root_filter { fs::read_dir(root).map_err(|e|format!("Источник недоступен; индекс не изменён: {e}"))?; }
    let root_text=root_filter.as_ref().map(|root|path_to_string(root).replace('\\',"/").trim_end_matches('/').to_lowercase());
    let exclusions=input.excluded_paths.unwrap_or_default();
    if exclusions.len()>128{return Err("Too many scan exclusions.".into());}
    let selected=input.only_paths.map(|paths|if paths.len()>256{Err("Too many selected rescan paths.".to_string())}else{serde_json::to_string(&paths).map_err(|e|e.to_string())}).transpose()?;

    let mut statement = connection
        .prepare(
            r#"
            SELECT
                p.image_id,
                i.file_name,
                p.path,
                i.status,
                p.missing_at_unix
            FROM image_paths p
            INNER JOIN images i ON i.id = p.image_id
            WHERE p.is_current = 1 AND p.path > ?2
              AND (?4 IS NULL OR p.path IN (SELECT value FROM json_each(?4)))
              AND (?3 IS NULL OR unicode_lower(replace(p.path, char(92), '/')) = ?3
                   OR instr(unicode_lower(replace(p.path, char(92), '/')), ?3 || '/') = 1)
            ORDER BY p.path ASC
            LIMIT ?1
            "#,
        )
        .map_err(|error| format!("Cannot prepare index path rescan: {error}"))?;

    let rows = statement
        .query_map(params![(limit+1) as i64,input.after_path.unwrap_or_default(),root_text,selected], |row| {
            Ok(CurrentIndexedPathRow {
                image_id: row.get(0)?,
                file_name: row.get(1)?,
                path: row.get(2)?,
                status: row.get(3)?,
                missing_at_unix: row.get(4)?,
            })
        })
        .map_err(|error| format!("Cannot query indexed paths for rescan: {error}"))?;

    let mut indexed_paths = Vec::new();
    for row in rows {
        let row = row.map_err(|error| format!("Cannot read indexed path row: {error}"))?;
        indexed_paths.push(row);
    }
    drop(statement);

    let more=indexed_paths.len()>limit;if more{indexed_paths.truncate(limit);}
    let next_after_path=if more{indexed_paths.last().map(|row|row.path.clone())}else{None};
    let mut skipped_paths=0;
    // All filesystem calls precede the short SQLite writer transaction.
    let observed=indexed_paths.into_iter().filter_map(|row|{
        if source_paths::excluded(Path::new(&row.path)) || exclusions.iter().any(|path|path_is_under_rescan_root(&row.path,&Some(PathBuf::from(path)))) {return None;}
        match fs::metadata(&row.path){
            Ok(meta) if meta.is_file()=>Some((row,true)),
            Ok(_)=>{skipped_paths+=1;None},
            Err(e) if e.kind()==std::io::ErrorKind::NotFound && missing_path_is_observable(Path::new(&row.path))=>Some((row,false)),
            Err(_)=>{skipped_paths+=1;None},
        }
    }).collect::<Vec<_>>();
    if let Some(root) = &root_filter { fs::read_dir(root).map_err(|e|format!("Источник отключён во время проверки; эта страница не записана: {e}"))?; }

    let transaction = connection
        .transaction()
        .map_err(|error| format!("Cannot start index rescan transaction: {error}"))?;

    let mut checked_paths = 0usize;
    let mut existing_paths = 0usize;
    let mut missing_paths = 0usize;
    let mut revived_paths = 0usize;
    let mut action_log_entries = 0usize;

    for (row,exists_now) in observed {
        // A rename or content replacement between observation and commit invalidates the observation.
        let still_current:bool=transaction.query_row("SELECT EXISTS(SELECT 1 FROM image_paths WHERE image_id=?1 AND path=?2 AND is_current=1)",params![row.image_id,row.path],|r|r.get(0)).map_err(|e|e.to_string())?;
        if !still_current {skipped_paths+=1;continue;}
        checked_paths += 1;

        if exists_now {
            existing_paths += 1;
            transaction
                .execute(
                    "UPDATE image_paths SET last_seen_at_unix = ?2, missing_at_unix = NULL WHERE image_id = ?1 AND path = ?3",
                    params![&row.image_id, u64_to_i64_saturating(now), &row.path],
                )
                .map_err(|error| format!("Cannot refresh existing indexed path {}: {error}", row.path))?;

            if row.missing_at_unix.is_some() || row.status == "missing" {
                revived_paths += 1;
                transaction
                    .execute(
                        r#"
                        UPDATE images
                        SET status = CASE WHEN status = 'missing' THEN COALESCE(status_before_missing,
                                CASE WHEN description IS NOT NULL THEN 'processed' ELSE 'notProcessed' END) ELSE status END,
                            status_before_missing = NULL,
                            updated_at_unix = ?2
                        WHERE id = ?1
                        "#,
                        params![&row.image_id, u64_to_i64_saturating(now)],
                    )
                    .map_err(|error| format!("Cannot revive indexed image {}: {error}", row.file_name))?;

                transaction
                    .execute(
                        r#"
                        INSERT INTO action_logs (action_type, image_id, old_value, new_value, message, created_at_unix)
                        VALUES ('path_restored', ?1, ?2, ?2, ?3, ?4)
                        "#,
                        params![
                            &row.image_id,
                            &row.path,
                            format!("Previously missing image exists again: {}", row.file_name),
                            u64_to_i64_saturating(now),
                        ],
                    )
                    .map_err(|error| format!("Cannot write restore action log for {}: {error}", row.file_name))?;
                action_log_entries += 1;
            }
        } else {
            missing_paths += 1;
            if row.missing_at_unix.is_none() {
                transaction
                    .execute(
                        "UPDATE image_paths SET last_seen_at_unix = ?2, missing_at_unix = ?2 WHERE image_id = ?1 AND path = ?3",
                        params![&row.image_id, u64_to_i64_saturating(now), &row.path],
                    )
                    .map_err(|error| format!("Cannot mark indexed path missing {}: {error}", row.path))?;

                transaction
                    .execute(
                        "UPDATE images SET status_before_missing = CASE WHEN status <> 'missing' THEN status ELSE status_before_missing END,
                         status = 'missing', updated_at_unix = ?2 WHERE id = ?1
                         AND NOT EXISTS (SELECT 1 FROM image_paths WHERE image_id=?1 AND is_current=1 AND missing_at_unix IS NULL)",
                        params![&row.image_id, u64_to_i64_saturating(now)],
                    )
                    .map_err(|error| format!("Cannot mark indexed image missing {}: {error}", row.file_name))?;

                transaction
                    .execute(
                        r#"
                        INSERT INTO action_logs (action_type, image_id, old_value, new_value, message, created_at_unix)
                        VALUES ('path_missing', ?1, ?2, NULL, ?3, ?4)
                        "#,
                        params![
                            &row.image_id,
                            &row.path,
                            format!("Indexed image path is missing on disk: {}", row.file_name),
                            u64_to_i64_saturating(now),
                        ],
                    )
                    .map_err(|error| format!("Cannot write missing action log for {}: {error}", row.file_name))?;
                action_log_entries += 1;
            }
        }
    }

    transaction
        .commit()
        .map_err(|error| format!("Cannot commit index path rescan: {error}"))?;

    Ok(IndexRescanPathsResultDto {
        database_path: path_to_string(&database_path),
        root_path: root_filter.as_ref().map(|path| path_to_string(path)),
        checked_paths,
        existing_paths,
        missing_paths,
        revived_paths,
        action_log_entries,
        image_count: count_rows(&connection, "images")?,
        path_count: count_rows(&connection, "image_paths")?,
        rescanned_at_unix: now,
        next_after_path,
        skipped_paths,
    })
}



fn optional_i64_to_u64(value: Option<i64>) -> Option<u64> {
    value.and_then(|raw| if raw >= 0 { Some(raw as u64) } else { None })
}

fn read_maintenance_counts(connection: &Connection) -> Result<(usize, usize, usize, usize), String> {
    let missing_image_count = connection
        .query_row("SELECT COUNT(*) FROM images WHERE status = 'missing'", [], |row| row.get::<_, i64>(0))
        .map_err(|error| format!("Cannot count missing images: {error}"))? as usize;
    let orphan_tag_count = connection
        .query_row(
            "SELECT COUNT(*) FROM tags WHERE id NOT IN (SELECT tag_id FROM image_tags UNION SELECT tag_id FROM image_tag_layers UNION SELECT tag_id FROM image_tag_suppressions)",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| format!("Cannot count orphan tags: {error}"))? as usize;
    let non_current_path_count = connection
        .query_row("SELECT COUNT(*) FROM image_paths WHERE is_current = 0", [], |row| row.get::<_, i64>(0))
        .map_err(|error| format!("Cannot count non-current paths: {error}"))? as usize;
    let old_search_history_count = connection
        .query_row(
            r#"
            SELECT COUNT(*) FROM search_history
            WHERE id NOT IN (
                SELECT id FROM search_history ORDER BY created_at_unix DESC, id DESC LIMIT 100
            )
            "#,
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| format!("Cannot count old search history entries: {error}"))? as usize;

    Ok((missing_image_count, orphan_tag_count, non_current_path_count, old_search_history_count))
}

#[tauri::command]
async fn get_index_maintenance(app_handle: tauri::AppHandle, input: IndexMaintenanceInputDto) -> Result<IndexMaintenanceResultDto, String> {
    tauri::async_runtime::spawn_blocking(move || get_index_maintenance_internal(app_handle, input)).await.map_err(|error| error.to_string())?
}

fn get_index_maintenance_internal(
    app_handle: tauri::AppHandle,
    input: IndexMaintenanceInputDto,
) -> Result<IndexMaintenanceResultDto, String> {
    let mut connection = open_index_connection(&app_handle)?;
    let database_path = index_database_path(&app_handle)?;
    run_index_migrations(&mut connection)?;

    let status = input
        .status
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("missing")
        .to_string();
    let limit = input.limit.unwrap_or(100).clamp(1, 500) as i64;
    let offset = input.offset.unwrap_or(0).min(100_000) as i64;
    let status_filter = if status == "all" { None } else { Some(status.as_str()) };

    let total_matching_images = if let Some(filter) = status_filter {
        connection
            .query_row("SELECT COUNT(*) FROM images WHERE status = ?1", params![filter], |row| row.get::<_, i64>(0))
            .map_err(|error| format!("Cannot count maintenance images: {error}"))? as usize
    } else {
        connection
            .query_row("SELECT COUNT(*) FROM images", [], |row| row.get::<_, i64>(0))
            .map_err(|error| format!("Cannot count indexed images: {error}"))? as usize
    };

    let sql = if status_filter.is_some() {
        r#"
        SELECT i.id, i.file_name, COALESCE(p.path, ''), i.status, i.description, i.size_bytes,
               i.modified_at_unix, p.missing_at_unix, p.last_seen_at_unix
        FROM images i
        LEFT JOIN image_paths p ON p.rowid = (
            SELECT rowid FROM image_paths WHERE image_id = i.id
            ORDER BY is_current DESC, last_seen_at_unix DESC, path COLLATE NOCASE ASC, path ASC LIMIT 1
        )
        WHERE i.status = ?1
        ORDER BY COALESCE(p.missing_at_unix, p.last_seen_at_unix, i.updated_at_unix) DESC, i.file_name ASC, i.id ASC
        LIMIT ?2 OFFSET ?3
        "#
    } else {
        r#"
        SELECT i.id, i.file_name, COALESCE(p.path, ''), i.status, i.description, i.size_bytes,
               i.modified_at_unix, p.missing_at_unix, p.last_seen_at_unix
        FROM images i
        LEFT JOIN image_paths p ON p.rowid = (
            SELECT rowid FROM image_paths WHERE image_id = i.id
            ORDER BY is_current DESC, last_seen_at_unix DESC, path COLLATE NOCASE ASC, path ASC LIMIT 1
        )
        ORDER BY COALESCE(p.missing_at_unix, p.last_seen_at_unix, i.updated_at_unix) DESC, i.file_name ASC, i.id ASC
        LIMIT ?1 OFFSET ?2
        "#
    };

    let mut statement = connection
        .prepare(sql)
        .map_err(|error| format!("Cannot prepare index maintenance query: {error}"))?;

    let rows = if let Some(filter) = status_filter {
        statement
            .query_map(params![filter, limit, offset], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<i64>>(5)?,
                    row.get::<_, Option<i64>>(6)?,
                    row.get::<_, Option<i64>>(7)?,
                    row.get::<_, Option<i64>>(8)?,
                ))
            })
            .map_err(|error| format!("Cannot read maintenance images: {error}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("Cannot collect maintenance image rows: {error}"))?
    } else {
        statement
            .query_map(params![limit, offset], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<i64>>(5)?,
                    row.get::<_, Option<i64>>(6)?,
                    row.get::<_, Option<i64>>(7)?,
                    row.get::<_, Option<i64>>(8)?,
                ))
            })
            .map_err(|error| format!("Cannot read maintenance images: {error}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("Cannot collect maintenance image rows: {error}"))?
    };
    drop(statement);

    let mut images = Vec::with_capacity(rows.len());
    for (image_id, file_name, path, row_status, description, size_bytes, modified_at_unix, missing_at_unix, last_seen_at_unix) in rows {
        let tags = read_tags_for_image_from_connection(&connection, &image_id)?;
        images.push(IndexMaintenanceImageDto {
            image_id,
            file_name,
            path,
            status: row_status,
            description,
            tags,
            size_bytes: optional_i64_to_u64(size_bytes),
            modified_at_unix: optional_i64_to_u64(modified_at_unix),
            missing_at_unix: optional_i64_to_u64(missing_at_unix),
            last_seen_at_unix: optional_i64_to_u64(last_seen_at_unix),
        });
    }

    let (missing_image_count, orphan_tag_count, non_current_path_count, old_search_history_count) = read_maintenance_counts(&connection)?;

    Ok(IndexMaintenanceResultDto {
        database_path: path_to_string(&database_path),
        status,
        total_matching_images,
        returned_images: images.len(),
        missing_image_count,
        orphan_tag_count,
        non_current_path_count,
        old_search_history_count,
        images,
        loaded_at_unix: unix_now(),
    })
}

#[tauri::command]
async fn remove_index_images(app_handle: tauri::AppHandle, input: RemoveIndexImagesInputDto) -> Result<RemoveIndexImagesResultDto, String> {
    tauri::async_runtime::spawn_blocking(move || remove_index_images_internal(app_handle, input)).await.map_err(|error| error.to_string())?
}

fn remove_index_images_internal(
    app_handle: tauri::AppHandle,
    input: RemoveIndexImagesInputDto,
) -> Result<RemoveIndexImagesResultDto, String> {
    let mut connection = open_index_connection(&app_handle)?;
    let database_path = index_database_path(&app_handle)?;
    run_index_migrations(&mut connection)?;

    let now = unix_now();
    let mut requested_ids = Vec::new();
    for raw_id in input.image_ids {
        let id = raw_id.trim().to_string();
        if !id.is_empty() && !requested_ids.contains(&id) {
            requested_ids.push(id);
        }
    }

    let requested_images = requested_ids.len();
    let transaction = connection
        .transaction()
        .map_err(|error| format!("Cannot start remove-from-index transaction: {error}"))?;

    let mut removed_images = 0usize;
    let mut missing_requested_images = 0usize;
    let mut action_log_entries = 0usize;

    for image_id in requested_ids {
        let active_operations: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM file_operations WHERE image_id=?1 AND status IN ('applying','needsRecovery')",
            params![&image_id], |row| row.get(0)
        ).map_err(|error| error.to_string())?;
        if active_operations > 0 { return Err("An active or unresolved file operation protects this indexed image from removal.".to_string()); }
        let image_info = transaction
            .query_row(
                r#"
                SELECT i.file_name, COALESCE(p.path, '')
                FROM images i
                LEFT JOIN image_paths p ON p.image_id = i.id AND p.is_current = 1
                WHERE i.id = ?1
                "#,
                params![&image_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|error| format!("Cannot read indexed image before removal: {error}"))?;

        let Some((file_name, path)) = image_info else {
            missing_requested_images += 1;
            continue;
        };

        transaction
            .execute(
                r#"
                INSERT INTO action_logs (action_type, image_id, old_value, new_value, message, created_at_unix)
                VALUES ('removed_from_index', NULL, ?1, NULL, ?2, ?3)
                "#,
                params![
                    format!(r#"{{"imageId":"{}","fileName":"{}","path":"{}"}}"#, json_escape(&image_id), json_escape(&file_name), json_escape(&path)),
                    format!("Removed indexed record without deleting the original file: {file_name}"),
                    u64_to_i64_saturating(now),
                ],
            )
            .map_err(|error| format!("Cannot write remove-from-index action log for {file_name}: {error}"))?;
        action_log_entries += 1;

        let deleted = transaction
            .execute("DELETE FROM images WHERE id = ?1", params![&image_id])
            .map_err(|error| format!("Cannot remove indexed image {file_name}: {error}"))?;
        if deleted > 0 {
            removed_images += 1;
        }
    }

    transaction
        .commit()
        .map_err(|error| format!("Cannot commit remove-from-index transaction: {error}"))?;

    Ok(RemoveIndexImagesResultDto {
        database_path: path_to_string(&database_path),
        requested_images,
        removed_images,
        missing_requested_images,
        action_log_entries,
        image_count: count_rows(&connection, "images")?,
        path_count: count_rows(&connection, "image_paths")?,
        tag_count: count_rows(&connection, "tags")?,
        removed_at_unix: now,
    })
}

#[tauri::command]
async fn cleanup_index_garbage(app_handle: tauri::AppHandle) -> Result<CleanupIndexResultDto, String> {
    tauri::async_runtime::spawn_blocking(move || cleanup_index_garbage_internal(app_handle)).await.map_err(|error| error.to_string())?
}

fn cleanup_index_garbage_internal(app_handle: tauri::AppHandle) -> Result<CleanupIndexResultDto, String> {
    let mut connection = open_index_connection(&app_handle)?;
    let database_path = index_database_path(&app_handle)?;
    run_index_migrations(&mut connection)?;

    let now = unix_now();
    let transaction = connection
        .transaction()
        .map_err(|error| format!("Cannot start index cleanup transaction: {error}"))?;

    let orphan_tags_removed = transaction
        .execute(
            "DELETE FROM tags WHERE id NOT IN (SELECT tag_id FROM image_tags UNION SELECT tag_id FROM image_tag_layers UNION SELECT tag_id FROM image_tag_suppressions)",
            [],
        )
        .map_err(|error| format!("Cannot remove orphan tags: {error}"))?;

    let old_search_history_removed = transaction
        .execute(
            r#"
            DELETE FROM search_history
            WHERE id NOT IN (
                SELECT id FROM search_history ORDER BY created_at_unix DESC, id DESC LIMIT 100
            )
            "#,
            [],
        )
        .map_err(|error| format!("Cannot remove old search history rows: {error}"))?;

    let stale_finished_tasks_removed = transaction
        .execute(
            r#"
            DELETE FROM tasks
            WHERE status IN ('done', 'failed', 'cancelled', 'completed')
              AND updated_at_unix < ?1
            "#,
            params![u64_to_i64_saturating(now.saturating_sub(30 * 86_400))],
        )
        .map_err(|error| format!("Cannot remove stale finished tasks: {error}"))?;

    transaction
        .execute(
            r#"
            INSERT INTO action_logs (action_type, image_id, old_value, new_value, message, created_at_unix)
            VALUES ('index_cleanup', NULL, NULL, ?1, ?2, ?3)
            "#,
            params![
                format!(r#"{{"orphanTagsRemoved":{},"oldSearchHistoryRemoved":{},"staleFinishedTasksRemoved":{}}}"#, orphan_tags_removed, old_search_history_removed, stale_finished_tasks_removed),
                "Cleaned safe index garbage: orphan tags, old search history and stale finished tasks.",
                u64_to_i64_saturating(now),
            ],
        )
        .map_err(|error| format!("Cannot write index cleanup action log: {error}"))?;

    transaction
        .commit()
        .map_err(|error| format!("Cannot commit index cleanup transaction: {error}"))?;

    Ok(CleanupIndexResultDto {
        database_path: path_to_string(&database_path),
        orphan_tags_removed,
        old_search_history_removed,
        stale_finished_tasks_removed,
        image_count: count_rows(&connection, "images")?,
        path_count: count_rows(&connection, "image_paths")?,
        tag_count: count_rows(&connection, "tags")?,
        cleaned_at_unix: now,
    })
}

#[tauri::command]
async fn update_image_metadata(
    app_handle: tauri::AppHandle,
    input: IndexMetadataUpdateInputDto,
) -> Result<IndexMetadataUpdateResultDto, String> {
    tauri::async_runtime::spawn_blocking(move || update_image_metadata_internal(app_handle, input))
        .await.map_err(|error| format!("Metadata worker failed: {error}"))?
}

fn update_image_metadata_internal(
    app_handle: tauri::AppHandle,
    input: IndexMetadataUpdateInputDto,
) -> Result<IndexMetadataUpdateResultDto, String> {
    let result=update_image_metadata_guarded(app_handle.clone(), input, None)?;
    let _=viewer_sessions::update_metadata(&app_handle,&result);
    Ok(result)
}

fn update_image_metadata_guarded(
    app_handle: tauri::AppHandle,
    input: IndexMetadataUpdateInputDto,
    checkpoint: Option<&processing::CommitCheckpoint>,
) -> Result<IndexMetadataUpdateResultDto, String> {
    let trimmed_path = input.path.trim();
    if trimmed_path.is_empty() {
        return Err("Image path is empty.".to_string());
    }
    let current_sha = if Path::new(trimmed_path).is_file() {
        Some(compute_file_sha256(Path::new(trimmed_path))?)
    } else { None };

    let mut connection = open_index_connection(&app_handle)?;
    run_index_migrations(&mut connection)?;

    let now = unix_now();
    let normalized_description = input.description.trim().to_string();
    let normalized_tags = normalize_metadata_tags(input.tags);
    let fields = input.fields.unwrap_or_else(|| vec!["description".into(), "tags".into()]);
    if fields.is_empty() || fields.iter().any(|field| field != "description" && field != "tags") {
        return Err("Metadata patch must name description and/or tags.".to_string());
    }
    let source = input.source.as_deref().unwrap_or("manual");
    if source != "manual" && source != "ai" { return Err("Unknown metadata source.".to_string()); }
    if normalized_description.len() > 256 * 1024 { return Err("Description exceeds the supported size.".to_string()); }
    let mut patch_description = fields.iter().any(|field| field == "description");
    let mut patch_tags = fields.iter().any(|field| field == "tags");
    if source == "ai" && patch_description && normalized_description.is_empty() { return Err("An empty AI description cannot be saved as processed.".to_string()); }
    let transaction = connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|error| format!("Cannot start metadata edit transaction: {error}"))?;
    if let Some(checkpoint) = checkpoint { processing::validate_commit(&transaction, checkpoint)?; }

    let image_id = if let Some(raw_image_id) = input.image_id.as_ref().map(|value| value.trim()).filter(|value| !value.is_empty()) {
        let exists: i64 = transaction
            .query_row("SELECT COUNT(*) FROM images WHERE id = ?1", params![raw_image_id], |row| row.get(0))
            .map_err(|error| format!("Cannot check indexed image id: {error}"))?;
        if exists == 0 {
            return Err(format!("Indexed image id was not found: {raw_image_id}"));
        }
        raw_image_id.to_string()
    } else {
        find_existing_image_by_path(&transaction, trimmed_path)?
            .map(|record| record.image_id)
            .ok_or_else(|| format!("Image is not indexed yet: {trimmed_path}"))?
    };

    let path_image_id: Option<String> = transaction.query_row(
        "SELECT image_id FROM image_paths WHERE path = ?1", params![trimmed_path], |row| row.get(0)
    ).optional().map_err(|error| format!("Cannot validate metadata path: {error}"))?;
    if path_image_id.as_deref() != Some(image_id.as_str()) {
        return Err("The file identity changed; reload before saving metadata.".to_string());
    }
    let indexed_sha = read_indexed_sha256(&transaction, &image_id)?;
    if current_sha.is_some() && current_sha != indexed_sha {
        return Err("Image contents changed since indexing. Reload before saving metadata.".to_string());
    }
    if source == "ai" && current_sha.is_none() {
        return Err("The AI source file is no longer available.".to_string());
    }
    if let Some(checkpoint)=checkpoint{
        let allowed=processing::fields::commit_fields(&transaction,checkpoint,&image_id,&fields)?;
        patch_description=allowed.iter().any(|f|f=="description");patch_tags=allowed.iter().any(|f|f=="tags");
    }
    let (description_version, tags_version, manual_description, _manual_tags): (i64, i64, bool, bool) =
        transaction.query_row(
            "SELECT description_version, tags_version, manual_description_set, manual_tags_set FROM images WHERE id = ?1",
            params![&image_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        ).map_err(|error| format!("Cannot read metadata versions: {error}"))?;
    if (patch_description && input.expected_description_version.is_some_and(|version| version != description_version))
        || (patch_tags && input.expected_tags_version.is_some_and(|version| version != tags_version)) {
        return Err("Metadata changed while editing. Your draft was kept; reload or reconcile before saving.".to_string());
    }

    let (old_description, old_status, old_processed_at): (Option<String>, String, Option<u64>) = transaction
        .query_row(
            "SELECT description, status, ai_processed_at_unix FROM images WHERE id = ?1",
            params![&image_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|error| format!("Cannot read current image metadata: {error}"))?;
    let old_tags = read_tags_for_image(&transaction, &image_id)?;

    let apply_description = patch_description && (source == "manual" || !manual_description);
    let old_format=description_format::read(&transaction,&image_id)?;
    let next_format=if apply_description {
        if source=="manual"{description_format::validate(input.description_format,&normalized_description)?}else{None}
    }else{old_format.as_ref().map(ToString::to_string)};
    let apply_tags = patch_tags;
    let saved_description = if !apply_description {
        old_description.clone()
    } else if normalized_description.is_empty() {
        None
    } else {
        Some(normalized_description.clone())
    };
    let description_for_db = saved_description.as_deref();
    if apply_tags{tag_layers::apply(&transaction,&image_id,&source,&normalized_tags,now)?;}
    let next_tags = if apply_tags { read_tags_for_image(&transaction,&image_id)? } else { old_tags.clone() };
    let next_status = if description_for_db.is_some() || !next_tags.is_empty() || source=="ai"&&apply_tags {
        "processed"
    } else if old_status == "error" {
        "error"
    } else {
        "notProcessed"
    };

    transaction
        .execute(
            r#"
            UPDATE images
            SET description = ?2,
                description_format = ?11,
                status = ?3,
                ai_model = CASE WHEN ?5 = 'ai' AND ?10 = 1 THEN COALESCE(?8, ai_model) ELSE ai_model END,
                ai_processed_at_unix = CASE WHEN ?5 = 'ai' AND (?6 = 1 OR ?7 = 1) THEN ?4 ELSE ai_processed_at_unix END,
                description_version = description_version + ?6,
                tags_version = tags_version + ?7,
                manual_description_set = CASE WHEN ?5 = 'manual' AND ?6 = 1 THEN 1 ELSE manual_description_set END,
                manual_tags_set = CASE WHEN ?5 = 'manual' AND ?7 = 1 THEN 1 ELSE manual_tags_set END,
                description_origin = CASE WHEN ?6 = 1 THEN ?5 ELSE description_origin END,
                ai_description = CASE WHEN ?5 = 'ai' AND ?10 = 1 THEN ?9 ELSE ai_description END,
                updated_at_unix = ?4
            WHERE id = ?1
            "#,
            params![&image_id, description_for_db, next_status, u64_to_i64_saturating(now),
                source, i64::from(apply_description), i64::from(apply_tags), input.model.as_deref(), &normalized_description, i64::from(patch_description), next_format.as_deref()],
        )
        .map_err(|error| format!("Cannot update image metadata: {error}"))?;

    // Field completion belongs to these exact bytes; an empty successful AI
    // tags result counts, while explicitly clearing tags manually does not.
    transaction.execute("UPDATE images SET description_content_sha=CASE WHEN ?2 THEN ?4 ELSE description_content_sha END,tags_content_sha=CASE WHEN ?3 THEN ?4 ELSE tags_content_sha END,tags_analysis_completed=CASE WHEN ?3 THEN ?5 ELSE tags_analysis_completed END WHERE id=?1",params![image_id,apply_description,apply_tags,current_sha,source=="ai"&&apply_tags]).map_err(|e|e.to_string())?;


    transaction.execute(
        "INSERT INTO image_metadata_history (image_id, source, model, payload, created_at_unix) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![&image_id, source, input.model.as_deref(),
            serde_json::json!({ "description": normalized_description, "descriptionFormat": next_format.as_ref().and_then(|v|serde_json::from_str::<serde_json::Value>(v).ok()), "tags": normalized_tags, "fields": fields, "jobId": checkpoint.map(|value|&value.job_id), "taskId": checkpoint.map(|value|&value.task_id) }).to_string(),
            u64_to_i64_saturating(now)],
    ).map_err(|error| format!("Cannot preserve metadata provenance: {error}"))?;

    let file_name: String = transaction
        .query_row("SELECT file_name FROM images WHERE id = ?1", params![&image_id], |row| row.get(0))
        .map_err(|error| format!("Cannot read updated image filename: {error}"))?;
    let old_value = serde_json::json!({
        "fileName": file_name,
        "path": trimmed_path,
        "description": old_description,
        "descriptionFormat":old_format,
        "tags": old_tags,
    })
    .to_string();
    let new_value = serde_json::json!({
        "description": description_for_db,
        "descriptionFormat":next_format.as_ref().and_then(|v|serde_json::from_str::<serde_json::Value>(v).ok()),
        "tags": next_tags,
        "source": source,
    })
    .to_string();

    transaction
        .execute(
            r#"
            INSERT INTO action_logs (action_type, image_id, old_value, new_value, message, created_at_unix)
            VALUES ('metadata_updated', ?1, ?2, ?3, ?4, ?5)
            "#,
            params![
                &image_id,
                old_value,
                new_value,
                "Метаданные фотографии сохранены",
                u64_to_i64_saturating(now),
            ],
        )
        .map_err(|error| format!("Cannot write metadata action log: {error}"))?;
    let action_log_id: i64 = transaction
        .query_row("SELECT last_insert_rowid()", [], |row| row.get(0))
        .map_err(|error| format!("Cannot read metadata action log id: {error}"))?;

    let current_path = trimmed_path.to_string();
    let saved_tags = read_tags_for_image(&transaction, &image_id)?;

    if let Some(checkpoint) = checkpoint { processing::complete_commit(&transaction, checkpoint)?; }
    transaction
        .commit()
        .map_err(|error| format!("Cannot commit metadata edit transaction: {error}"))?;

    let image_count = count_rows(&connection, "images")?;
    let path_count = count_rows(&connection, "image_paths")?;
    let tag_count = count_rows(&connection, "tags")?;

    Ok(IndexMetadataUpdateResultDto {
        ai_processed_at_unix: if source=="ai"&&(apply_description||apply_tags){Some(now)}else{old_processed_at},
        description_format:next_format.as_ref().map(|v|serde_json::from_str(v)).transpose().map_err(|e|e.to_string())?,
        image_id,
        path: current_path,
        file_name,
        status: next_status.to_string(),
        description: description_for_db.map(ToOwned::to_owned),
        tags: saved_tags,
        description_version: description_version + i64::from(apply_description),
        tags_version: tags_version + i64::from(apply_tags),
        updated_at_unix: now,
        action_log_id,
        image_count,
        path_count,
        tag_count,
    })
}


fn normalize_search_tags(tags: &[String]) -> Vec<String> {
    let mut normalized = Vec::<String>::new();
    for raw_tag in tags {
        let tag = raw_tag.trim().trim_start_matches('#').to_lowercase();
        if !tag.is_empty() && !normalized.iter().any(|existing| existing == &tag) {
            normalized.push(tag);
        }
    }
    normalized
}

fn read_recent_searches(connection: &Connection) -> Result<Vec<String>, String> {
    let mut statement = connection
        .prepare(
            r#"
            SELECT query
            FROM search_history
            WHERE query <> ''
            GROUP BY lower(query)
            ORDER BY MAX(created_at_unix) DESC
            LIMIT 10
            "#,
        )
        .map_err(|error| format!("Cannot prepare recent search query: {error}"))?;

    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| format!("Cannot query recent searches: {error}"))?;

    let mut searches = Vec::new();
    for row in rows {
        searches.push(row.map_err(|error| format!("Cannot read recent search row: {error}"))?);
    }

    Ok(searches)
}


#[tauri::command]
async fn get_action_journal(app_handle: tauri::AppHandle, input: ActionJournalInputDto) -> Result<ActionJournalResultDto, String> {
    tauri::async_runtime::spawn_blocking(move || get_action_journal_internal(app_handle, input)).await.map_err(|error| error.to_string())?
}

fn get_action_journal_internal(
    app_handle: tauri::AppHandle,
    input: ActionJournalInputDto,
) -> Result<ActionJournalResultDto, String> {
    let mut connection = open_index_connection(&app_handle)?;
    run_index_migrations(&mut connection)?;

    action_journal::read(&connection,input)
}

#[tauri::command]
async fn create_action_log_entry(app_handle: tauri::AppHandle, input: CreateActionLogEntryInputDto) -> Result<CreateActionLogEntryResultDto, String> {
    tauri::async_runtime::spawn_blocking(move || create_action_log_entry_internal(app_handle, input)).await.map_err(|error| error.to_string())?
}

fn create_action_log_entry_internal(
    app_handle: tauri::AppHandle,
    input: CreateActionLogEntryInputDto,
) -> Result<CreateActionLogEntryResultDto, String> {
    let mut connection = open_index_connection(&app_handle)?;
    run_index_migrations(&mut connection)?;

    let action_type = input.action_type.trim();
    if action_type.is_empty() {
        return Err("Action log type is empty.".to_string());
    }

    let message = input.message.trim();
    if message.is_empty() {
        return Err("Action log message is empty.".to_string());
    }

    let now = unix_now();
    let image_id = input.image_id.as_deref().map(str::trim).filter(|value| !value.is_empty());
    let old_value = input.old_value.as_deref();
    let new_value = input.new_value.as_deref();

    connection
        .execute(
            r#"
            INSERT INTO action_logs (action_type, image_id, old_value, new_value, message, created_at_unix)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                action_type,
                image_id,
                old_value,
                new_value,
                message,
                u64_to_i64_saturating(now),
            ],
        )
        .map_err(|error| format!("Cannot write action log entry: {error}"))?;

    let action_log_id = connection.last_insert_rowid();

    Ok(CreateActionLogEntryResultDto {
        action_log_id,
        created_at_unix: now,
    })
}



#[tauri::command]
async fn preview_smart_rename_images(
    app_handle: tauri::AppHandle,
    inputs: Vec<SmartRenameInputDto>,
    analysis_copy: Option<profile_preferences::AnalysisCopy>,
) -> Result<SmartRenamePreviewResultDto, String> {
    tauri::async_runtime::spawn_blocking(move || preview_smart_rename_images_internal(app_handle, inputs, analysis_copy))
        .await.map_err(|error| format!("Rename preview worker failed: {error}"))?
}

fn preview_smart_rename_images_internal(app_handle: tauri::AppHandle, inputs: Vec<SmartRenameInputDto>, copy:Option<profile_preferences::AnalysisCopy>) -> Result<SmartRenamePreviewResultDto, String> {
    let mut connection = open_index_connection(&app_handle)?;
    run_index_migrations(&mut connection)?;
    let mut items = Vec::new();
    for input in &inputs {
        let mut item = preview_smart_rename_image_internal(input);
        if item.status == "ready" || item.status == "unchanged" && copy.as_ref().is_some_and(|copy|copy.enabled) {
            item.plan_id = Some(if let Some(copy)=copy.as_ref().filter(|copy|copy.enabled){analysis_copy::prepare(&connection,&mut item,copy)?}else{file_operations::prepare_rename_plan(&connection, &item)?});
        }
        items.push(item);
    }

    let ready = items.iter().filter(|item| item.status == "ready").count();
    let unchanged = items.iter().filter(|item| item.status == "unchanged").count();
    let conflicts = items.iter().filter(|item| item.status == "conflict").count();
    let errors = items.iter().filter(|item| item.status == "error").count();

    Ok(SmartRenamePreviewResultDto {
        total: items.len(),
        ready,
        unchanged,
        conflicts,
        errors,
        items,
        created_at_unix: unix_now(),
    })
}

fn preview_smart_rename_image_internal(input: &SmartRenameInputDto) -> SmartRenamePreviewItemDto {
    let old_path = PathBuf::from(input.current_path.trim());
    let old_file_name = if input.file_name.trim().is_empty() {
        file_name(&old_path)
    } else {
        input.file_name.trim().to_string()
    };

    let fallback_error = |message: String| SmartRenamePreviewItemDto {
        plan_id: None,
        image_id: input.image_id.clone(),
        old_path: input.current_path.clone(),
        new_path: input.current_path.clone(),
        old_file_name: old_file_name.clone(),
        new_file_name: old_file_name.clone(),
        status: "error".to_string(),
        message,
        size_bytes: None,
        modified_at_unix: input.modified_at_unix,
        candidate_collisions: 0,
    };

    if input.pattern.trim() != "date_unique8" {
        return fallback_error(format!("Unsupported rename pattern: {}", input.pattern));
    }

    if input.current_path.trim().is_empty() {
        return fallback_error("Rename preview source path is empty.".to_string());
    }

    if !old_path.exists() {
        return fallback_error(format!("Source file does not exist: {}", old_path.display()));
    }

    if !old_path.is_file() {
        return fallback_error(format!("Source path is not a file: {}", old_path.display()));
    }

    if !is_supported_image(&old_path) {
        return fallback_error(format!("Smart rename supports image files only: {}", old_path.display()));
    }

    let parent = match old_path.parent().map(Path::to_path_buf) {
        Some(value) => value,
        None => return fallback_error(format!("Cannot find parent folder for {}", old_path.display())),
    };
    let extension = match extension(&old_path) {
        Some(value) => value,
        None => return fallback_error(format!("Cannot read file extension for {}", old_path.display())),
    };
    let metadata = match fs::metadata(&old_path) {
        Ok(value) => value,
        Err(error) => return fallback_error(format!("Cannot read metadata before rename preview {}: {error}", old_path.display())),
    };
    let size_bytes = metadata.len();
    let sha256 = match compute_file_sha256(&old_path) {
        Ok(value) => Some(value),
        Err(_) => None,
    };
    let modified_at = modified_at_unix(&metadata)
        .unwrap_or_else(unix_now);
    let file_metadata=image_metadata::read(&old_path);
    let (date_prefix,date_origin)=match file_metadata.date_taken_local.as_ref(){
        Some(date)=>(date[..10].to_string(),"дата съёмки EXIF"),
        None=>(ymd_label_from_unix(modified_at),"дата изменения файла: EXIF-дата съёмки отсутствует"),
    };
    let mut candidate_collisions = 0usize;

    for salt in 0..10_000u32 {
        let unique8 = smart_rename_unique8(&old_path, size_bytes, modified_at, sha256.as_deref(), salt);
        let candidate_file_name = format!("{date_prefix}_{unique8}.{extension}");
        let candidate_path = parent.join(&candidate_file_name);
        let new_path = path_to_string(&candidate_path);

        if candidate_file_name == old_file_name {
            return SmartRenamePreviewItemDto {
        plan_id: None,
                image_id: input.image_id.clone(),
                old_path: path_to_string(&old_path),
                new_path,
                old_file_name,
                new_file_name: candidate_file_name,
                status: "unchanged".to_string(),
                message: "Filename already matches YYYY-MM-DD_UNIQUE8.ext. Rename will be skipped safely.".to_string(),
                size_bytes: Some(size_bytes),
                modified_at_unix: Some(modified_at),
                candidate_collisions,
            };
        }

        if candidate_path.exists() {
            candidate_collisions += 1;
            continue;
        }

        return SmartRenamePreviewItemDto {
        plan_id: None,
            image_id: input.image_id.clone(),
            old_path: path_to_string(&old_path),
            new_path,
            old_file_name,
            new_file_name: candidate_file_name,
            status: "ready".to_string(),
            message: if candidate_collisions > 0 {
                format!("Ready. {candidate_collisions} occupied candidate name(s) were skipped before finding a free target.")
            } else {
                format!("Готово. Основа имени — {date_origin}, {date_prefix}. Целевой файл свободен.")
            },
            size_bytes: Some(size_bytes),
            modified_at_unix: Some(modified_at),
            candidate_collisions,
        };
    }

    SmartRenamePreviewItemDto {
        plan_id: None,
        image_id: input.image_id.clone(),
        old_path: path_to_string(&old_path),
        new_path: path_to_string(&old_path),
        old_file_name: old_file_name.clone(),
        new_file_name: old_file_name,
        status: "conflict".to_string(),
        message: format!("Cannot find a free smart rename target in {} after checking 10000 candidate names.", parent.display()),
        size_bytes: Some(size_bytes),
        modified_at_unix: Some(modified_at),
        candidate_collisions,
    }
}


#[tauri::command]
async fn smart_rename_image(
    app_handle: tauri::AppHandle,
    input: SmartRenameInputDto,
) -> Result<SmartRenameResultDto, String> {
    tauri::async_runtime::spawn_blocking(move || file_operations::rename(app_handle, input))
        .await.map_err(|error| format!("Rename worker failed: {error}"))?
}

fn action_log_row_to_journal_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<ActionJournalEntryDto> {
    let id: i64 = row.get(0)?;
    let created_at = optional_i64_to_u64(Some(row.get::<_, i64>(8)?)).unwrap_or(0);
    Ok(ActionJournalEntryDto {
        id: format!("action:{id}"),
        source: "action_log".to_string(),
        action_type: row.get(1)?,
        image_id: row.get(2)?,
        image_file_name: row.get(3)?,
        image_path: row.get(4)?,
        old_value: row.get(5)?,
        new_value: row.get(6)?,
        message: row.get(7)?,
        created_at_unix: created_at,
        related_renames: None,
    })
}

fn resolve_scan_root(path: Option<String>) -> Result<PathBuf, String> {
    if let Some(raw_path) = path {
        let trimmed = raw_path.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }

    if let Some(pictures_dir) = pictures_directory() {
        if pictures_dir.exists() && pictures_dir.is_dir() {
            return Ok(pictures_dir);
        }
    }

    std::env::current_dir().map_err(|error| format!("Cannot find a default folder: {error}"))
}

fn pictures_directory() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        if let Some(user_profile) = std::env::var_os("USERPROFILE") {
            return Some(PathBuf::from(user_profile).join("Pictures"));
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return Some(PathBuf::from(home).join("Pictures"));
        }
    }

    None
}

fn folder_entry(path: &Path, skipped_entries: &mut usize) -> DirectoryScanEntryDto {
    DirectoryScanEntryDto {
        source_revision: None,
        file_metadata: None,
        id: stable_id("folder", path),
        kind: "folder".to_string(),
        name: file_name(path),
        path: path_to_string(path),
        extension: None,
        size_bytes: None,
        modified_at_unix: None,
        child_count: Some(count_direct_children(path, skipped_entries)),
        is_image: false,
    }
}

fn file_entry(path: &Path, metadata: &fs::Metadata, is_image: bool) -> DirectoryScanEntryDto {
    DirectoryScanEntryDto {
        source_revision: if is_image { index_observations::content_revision(path).ok() } else { None },
        file_metadata: None,
        id: stable_id(if is_image { "image" } else { "file" }, path),
        kind: if is_image { "image" } else { "file" }.to_string(),
        name: file_name(path),
        path: path_to_string(path),
        extension: extension(path),
        size_bytes: Some(metadata.len()),
        modified_at_unix: modified_at_unix(metadata),
        child_count: None,
        is_image,
    }
}

fn count_direct_children(path: &Path, skipped_entries: &mut usize) -> usize {
    match fs::read_dir(path) {
        Ok(entries) => entries.filter_map(Result::ok).count(),
        Err(_) => {
            *skipped_entries += 1;
            0
        }
    }
}

fn is_supported_image(path: &Path) -> bool {
    extension(path)
        .map(|value| IMAGE_EXTENSIONS.contains(&value.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_lowercase())
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path_to_string(path))
}


fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn path_to_string(path: &Path) -> String {
    let text = path.to_string_lossy().to_string();
    #[cfg(target_os = "windows")]
    {
        if let Some(tail) = text.strip_prefix(r"\\?\UNC\") { return format!(r"\\{tail}"); }
        if let Some(tail) = text.strip_prefix(r"\\?\") { return tail.to_string(); }
    }
    text
}

fn stable_id(prefix: &str, path: &Path) -> String {
    format!("{}:{}", prefix, path_to_string(path))
}

fn modified_at_unix(metadata: &fs::Metadata) -> Option<u64> {
    metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
}

fn system_time_to_unix_optional(value: std::io::Result<SystemTime>) -> Option<u64> {
    value
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
}

fn path_breadcrumbs(path: &Path) -> Vec<PathBreadcrumbDto> {
    let mut breadcrumbs = Vec::new();
    let mut current = PathBuf::new();

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => {
                current.push(prefix.as_os_str());
                let label = prefix.as_os_str().to_string_lossy().to_string();
                breadcrumbs.push(PathBreadcrumbDto { label, path: path_to_string(&current) });
            }
            Component::RootDir => {
                current.push(std::path::MAIN_SEPARATOR.to_string());
                if let Some(root) = breadcrumbs.last_mut() {
                    root.path = path_to_string(&current);
                } else {
                    breadcrumbs.push(PathBreadcrumbDto {
                        label: std::path::MAIN_SEPARATOR.to_string(), path: path_to_string(&current),
                    });
                }
            }
            Component::Normal(value) => {
                current.push(value);
                breadcrumbs.push(PathBreadcrumbDto {
                    label: value.to_string_lossy().to_string(),
                    path: path_to_string(&current),
                });
            }
            Component::CurDir => {}
            Component::ParentDir => {
                current.push("..");
                breadcrumbs.push(PathBreadcrumbDto { label: "..".to_string(), path: path_to_string(&current) });
            }
        }
    }

    if breadcrumbs.is_empty() {
        breadcrumbs.push(PathBreadcrumbDto { label: path_to_string(path), path: path_to_string(path) });
    }

    breadcrumbs
}

#[tauri::command]
fn get_filesystem_roots() -> Result<Vec<String>, String> {
    #[cfg(windows)] {
        let mask = unsafe { windows_sys::Win32::Storage::FileSystem::GetLogicalDrives() };
        if mask == 0 { return Err(format!("Не удалось получить список дисков: {}", std::io::Error::last_os_error())); }
        Ok((0..26).filter(|bit| mask & (1 << bit) != 0)
            .map(|bit| format!("{}:\\", char::from(b'A' + bit))).collect())
    }
    #[cfg(not(windows))] { Ok(vec!["/".into()]) }
}

#[cfg(all(test, windows))]
mod breadcrumb_tests {
    use super::*;
    #[test]
    fn drive_crumb_is_absolute_and_has_no_redundant_root() {
        for path in [r"C:\", r"C:\Photos\Nested", r"D:\Photos"] {
            let crumbs = path_breadcrumbs(Path::new(path));
            assert_eq!(crumbs[0].path, format!("{}\\", &path[..2]));
            assert!(Path::new(&crumbs[0].path).is_absolute());
            assert!(!crumbs.iter().any(|c| c.label == "/"));
            assert_eq!(crumbs.last().unwrap().path, path);
        }
    }
    #[test]
    fn unc_root_remains_absolute() {
        let crumbs = path_breadcrumbs(Path::new(r"\\server\share\Photos"));
        assert_eq!(crumbs[0].path, r"\\server\share\");
        assert!(Path::new(&crumbs[0].path).is_absolute());
        assert_eq!(crumbs.last().unwrap().path, r"\\server\share\Photos");
    }
}


fn initialize_index_database_internal(app_handle: &tauri::AppHandle) -> Result<IndexDatabaseInfoDto, String> {
    let mut connection = open_index_connection(app_handle)?;
    let database_path = index_database_path(app_handle)?;
    let migrations_applied = run_index_migrations(&mut connection)?;
    file_operations::recover(&mut connection)?;
    build_index_database_info(&connection, database_path, migrations_applied)
}

fn application_data_directory(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    if let Some(value) = std::env::var_os("SMART_PHOTO_DATA_DIR") {
        let path = PathBuf::from(value);
        if !path.is_absolute() {
            return Err("SMART_PHOTO_DATA_DIR must be an absolute path".to_string());
        }
        return Ok(path);
    }
    app_handle.path().app_data_dir()
        .map_err(|error| format!("Cannot resolve application data folder: {error}"))
}

fn index_database_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    index_backups::database_path(app_handle)
}

fn sqlite_unicode_lower(context: &rusqlite::functions::Context<'_>) -> rusqlite::Result<Option<String>> {
    Ok(context.get::<Option<String>>(0)?.map(|text| text.to_lowercase()))
}

fn open_index_connection(app_handle: &tauri::AppHandle) -> Result<Connection, String> {
    // A cold viewer and startup workers can open the brand-new archive together.
    // SQLite cannot always wait when concurrent connections both switch to WAL.
    // Serialize configuration with migrations; release before returning the connection.
    let _configuration_guard = MIGRATION_LOCK.lock().map_err(|_| "Database configuration lock failed.".to_string())?;
    let database_path = index_database_path(app_handle)?;
    if let Some(parent) = database_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Cannot create index database folder {}: {error}", parent.display()))?;
    }

    let connection = Connection::open(&database_path)
        .map_err(|error| format!("Cannot open index database {}: {error}", database_path.display()))?;
    connection.busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|error| format!("Cannot configure SQLite timeout: {error}"))?;
    connection.create_scalar_function("unicode_lower",1,rusqlite::functions::FunctionFlags::SQLITE_UTF8 | rusqlite::functions::FunctionFlags::SQLITE_DETERMINISTIC,
        sqlite_unicode_lower).map_err(|error|error.to_string())?;
    source_sort::register(&connection)?;
    connection.create_scalar_function("photo_file_name",1,rusqlite::functions::FunctionFlags::SQLITE_UTF8 | rusqlite::functions::FunctionFlags::SQLITE_DETERMINISTIC,
        |context| { let path=context.get::<String>(0)?; Ok(path.rsplit(['\\','/']).next().unwrap_or(&path).to_string()) }).map_err(|error|error.to_string())?;
    connection
        .execute_batch(
            r#"
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            "#,
        )
        .map_err(|error| format!("Cannot configure index database {}: {error}", database_path.display()))?;

    Ok(connection)
}

fn run_index_migrations(connection: &mut Connection) -> Result<usize, String> {
    let _migration_guard = MIGRATION_LOCK.lock().map_err(|_| "Migration lock failed.".to_string())?;
    connection
        .execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at_unix INTEGER NOT NULL
            );
            "#,
        )
        .map_err(|error| format!("Cannot prepare schema_migrations table: {error}"))?;

    let original_version = current_schema_version(connection)?;
    if original_version > INDEX_SCHEMA_VERSION {
        return Err(format!("Database schema {original_version} requires a newer application. No migration was attempted."));
    }
    if original_version > 0 && original_version < INDEX_SCHEMA_VERSION {
        backup_before_migration(connection, original_version)?;
    }

    let mut applied = 0usize;
    for migration in INDEX_MIGRATIONS {
        let already_applied: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = ?1",
                params![migration.version],
                |row| row.get(0),
            )
            .map_err(|error| format!("Cannot read migration state for version {}: {error}", migration.version))?;

        if already_applied > 0 {
            continue;
        }

        let transaction = connection
            .transaction()
            .map_err(|error| format!("Cannot start migration transaction {}: {error}", migration.version))?;
        transaction
            .execute_batch(migration.sql)
            .map_err(|error| format!("Cannot apply migration {} {}: {error}", migration.version, migration.name))?;
        if migration.version == 4 { normalize_stored_paths(&transaction)?; }
        transaction
            .execute(
                "INSERT INTO schema_migrations (version, name, applied_at_unix) VALUES (?1, ?2, ?3)",
                params![migration.version, migration.name, u64_to_i64_saturating(unix_now())],
            )
            .map_err(|error| format!("Cannot record migration {}: {error}", migration.version))?;
        transaction
            .commit()
            .map_err(|error| format!("Cannot commit migration {}: {error}", migration.version))?;
        applied += 1;
    }

    Ok(applied)
}

fn backup_before_migration(connection: &Connection, version: i64) -> Result<(), String> {
    let Some(source) = connection.path().filter(|path| !path.is_empty()) else { return Ok(()); };
    let directory = Path::new(source).parent().ok_or("Database directory is unavailable.")?.join("backups");
    fs::create_dir_all(&directory).map_err(|error| format!("Cannot create backup directory: {error}"))?;
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|error| error.to_string())?.as_nanos();
    let pending = directory.join(format!("schema-{version}-{nonce}.partial.sqlite"));
    let completed = directory.join(format!("schema-{version}-{nonce}.sqlite"));
    // VACUUM INTO includes committed WAL pages and produces a consistent snapshot.
    connection.execute("VACUUM INTO ?1", params![path_to_string(&pending)])
        .map_err(|error| format!("Database backup failed; migration stopped: {error}"))?;
    let backup = Connection::open_with_flags(&pending, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("Cannot verify migration backup: {error}"))?;
    let integrity: String = backup.query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|error| format!("Cannot verify migration backup integrity: {error}"))?;
    if integrity != "ok" { return Err(format!("Backup integrity failed; migration stopped: {integrity}")); }
    if current_schema_version(&backup)? != version { return Err("Backup schema mismatch; migration stopped.".to_string()); }
    drop(backup);
    fs::rename(&pending, &completed).map_err(|error| format!("Cannot finish database backup: {error}"))?;
    Ok(())
}

fn read_metadata_versions(connection: &Connection, image_id: &str) -> Result<(i64, i64), String> {
    connection.query_row(
        "SELECT description_version, tags_version FROM images WHERE id = ?1",
        params![image_id], |row| Ok((row.get(0)?, row.get(1)?))
    ).map_err(|error| format!("Cannot read metadata versions: {error}"))
}

fn read_indexed_sha256(connection: &Connection, image_id: &str) -> Result<Option<String>, String> {
    connection.query_row("SELECT sha256 FROM images WHERE id = ?1", params![image_id], |row| row.get(0))
        .map_err(|error| format!("Cannot read image identity: {error}"))
}

fn normalize_stored_paths(connection: &Connection) -> Result<(), String> {
    type StoredPath = (i64, String, String, i64, i64, bool, Option<i64>);
    let rows: Vec<StoredPath> = {
        let mut statement = connection.prepare(
            "SELECT id,image_id,path,first_seen_at_unix,last_seen_at_unix,is_current,missing_at_unix FROM image_paths"
        ).map_err(|error| error.to_string())?;
        let records = statement.query_map([], |row| Ok((
            row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?
        ))).map_err(|error| error.to_string())?;
        records.collect::<Result<_, _>>().map_err(|error| error.to_string())?
    };
    let mut groups = std::collections::BTreeMap::<String, Vec<StoredPath>>::new();
    for row in rows {
        let normalized = path_to_string(Path::new(&row.2));
        #[cfg(target_os = "windows")]
        let key = normalized.replace('/', r"\").to_lowercase();
        #[cfg(not(target_os = "windows"))]
        let key = normalized;
        groups.entry(key).or_default().push(row);
    }
    for group in groups.values() {
        if group.iter().any(|row| row.1 != group[0].1) {
            return Err("Two path aliases reference different indexed images. Migration stopped; the backup was preserved for reconciliation.".to_string());
        }
        let normalized = path_to_string(Path::new(&group[0].2));
        let keep = group.iter().find(|row| row.2 == normalized).unwrap_or(&group[0]);
        let first = group.iter().map(|row| row.3).min().unwrap_or(keep.3);
        let last = group.iter().map(|row| row.4).max().unwrap_or(keep.4);
        let current = group.iter().any(|row| row.5);
        let missing = if current { None } else { group.iter().filter_map(|row| row.6).max() };
        for row in group.iter().filter(|row| row.0 != keep.0) {
            connection.execute("DELETE FROM image_paths WHERE id=?1", params![row.0]).map_err(|error| error.to_string())?;
        }
        connection.execute(
            "UPDATE image_paths SET path=?2,first_seen_at_unix=?3,last_seen_at_unix=?4,is_current=?5,missing_at_unix=?6 WHERE id=?1",
            params![keep.0,normalized,first,last,current,missing]
        ).map_err(|error| format!("Cannot normalize stored image path: {error}"))?;
    }
    Ok(())
}

fn build_index_database_info(
    connection: &Connection,
    database_path: PathBuf,
    migrations_applied: usize,
) -> Result<IndexDatabaseInfoDto, String> {
    Ok(IndexDatabaseInfoDto {
        database_path: path_to_string(&database_path),
        schema_version: current_schema_version(connection)?,
        migrations_applied,
        tables: list_database_tables(connection)?,
        image_count: count_rows(connection, "images")?,
        path_count: count_rows(connection, "image_paths")?,
        tag_count: count_rows(connection, "tags")?,
        created_at_unix: unix_now(),
    })
}

fn current_schema_version(connection: &Connection) -> Result<i64, String> {
    connection
        .query_row("SELECT COALESCE(MAX(version), 0) FROM schema_migrations", [], |row| row.get(0))
        .map_err(|error| format!("Cannot read current schema version: {error}"))
}

fn list_database_tables(connection: &Connection) -> Result<Vec<String>, String> {
    let mut statement = connection
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .map_err(|error| format!("Cannot prepare database table list query: {error}"))?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| format!("Cannot query database table list: {error}"))?;

    let mut tables = Vec::new();
    for row in rows {
        tables.push(row.map_err(|error| format!("Cannot read database table name: {error}"))?);
    }

    Ok(tables)
}

fn count_rows(connection: &Connection, table_name: &str) -> Result<i64, String> {
    let sql = match table_name {
        "images" => "SELECT COUNT(*) FROM images",
        "image_paths" => "SELECT COUNT(*) FROM image_paths",
        "tags" => "SELECT COUNT(*) FROM tags",
        _ => return Err(format!("Unsupported count table: {table_name}")),
    };

    connection
        .query_row(sql, [], |row| row.get(0))
        .map_err(|error| format!("Cannot count rows in {table_name}: {error}"))
}


fn find_existing_image_by_path(
    transaction: &rusqlite::Transaction<'_>,
    path: &str,
) -> Result<Option<ExistingImageRecord>, String> {
    transaction
        .query_row(
            r#"
            SELECT images.id, images.status, images.description,
                   (SELECT current_path.path
                    FROM image_paths AS current_path
                    WHERE current_path.image_id = images.id AND current_path.is_current = 1
                    ORDER BY current_path.last_seen_at_unix DESC
                    LIMIT 1) AS current_path, images.ai_processed_at_unix
            FROM image_paths
            JOIN images ON images.id = image_paths.image_id
            WHERE image_paths.path = ?1
            ORDER BY image_paths.is_current DESC, image_paths.last_seen_at_unix DESC
            LIMIT 1
            "#,
            params![path],
            |row| {
                Ok(ExistingImageRecord {
                    image_id: row.get(0)?,
                    status: row.get(1)?,
                    description: row.get(2)?,
                    current_path: row.get(3)?,
                    ai_processed_at_unix: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(|error| format!("Cannot read indexed image by path: {error}"))
}

fn find_existing_image_by_sha256(
    transaction: &rusqlite::Transaction<'_>,
    sha256: &str,
) -> Result<Option<ExistingImageRecord>, String> {
    transaction
        .query_row(
            r#"
            SELECT images.id, images.status, images.description,
                   (SELECT current_path.path
                    FROM image_paths AS current_path
                    WHERE current_path.image_id = images.id AND current_path.is_current = 1
                    ORDER BY current_path.last_seen_at_unix DESC
                    LIMIT 1) AS current_path, images.ai_processed_at_unix
            FROM images
            WHERE images.sha256 = ?1
            ORDER BY images.updated_at_unix DESC
            LIMIT 1
            "#,
            params![sha256],
            |row| {
                Ok(ExistingImageRecord {
                    image_id: row.get(0)?,
                    status: row.get(1)?,
                    description: row.get(2)?,
                    current_path: row.get(3)?,
                    ai_processed_at_unix: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(|error| format!("Cannot read indexed image by SHA-256: {error}"))
}

fn find_existing_image_by_identity(
    transaction: &rusqlite::Transaction<'_>,
    identity_key: &str,
) -> Result<Option<ExistingImageRecord>, String> {
    transaction
        .query_row(
            r#"
            SELECT images.id, images.status, images.description,
                   (SELECT current_path.path
                    FROM image_paths AS current_path
                    WHERE current_path.image_id = images.id AND current_path.is_current = 1
                    ORDER BY current_path.last_seen_at_unix DESC
                    LIMIT 1) AS current_path, images.ai_processed_at_unix
            FROM images
            WHERE images.identity_key = ?1
            ORDER BY images.updated_at_unix DESC
            LIMIT 1
            "#,
            params![identity_key],
            |row| {
                Ok(ExistingImageRecord {
                    image_id: row.get(0)?,
                    status: row.get(1)?,
                    description: row.get(2)?,
                    current_path: row.get(3)?,
                    ai_processed_at_unix: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(|error| format!("Cannot read indexed image by identity key: {error}"))
}


fn read_tags_for_image_from_connection(
    connection: &Connection,
    image_id: &str,
) -> Result<Vec<String>, String> {
    let mut statement = connection
        .prepare(
            r#"
            SELECT tags.name
            FROM image_tags
            JOIN tags ON tags.id = image_tags.tag_id
            WHERE image_tags.image_id = ?1
            ORDER BY tags.name COLLATE NOCASE
            "#,
        )
        .map_err(|error| format!("Cannot prepare tag query: {error}"))?;
    let rows = statement
        .query_map(params![image_id], |row| row.get::<_, String>(0))
        .map_err(|error| format!("Cannot query image tags: {error}"))?;

    let mut tags = Vec::new();
    for row in rows {
        tags.push(row.map_err(|error| format!("Cannot read image tag: {error}"))?);
    }

    Ok(tags)
}

fn read_tags_for_image(
    transaction: &rusqlite::Transaction<'_>,
    image_id: &str,
) -> Result<Vec<String>, String> {
    let mut statement = transaction
        .prepare(
            r#"
            SELECT tags.name
            FROM image_tags
            JOIN tags ON tags.id = image_tags.tag_id
            WHERE image_tags.image_id = ?1
            ORDER BY tags.name COLLATE NOCASE
            "#,
        )
        .map_err(|error| format!("Cannot prepare tag query: {error}"))?;
    let rows = statement
        .query_map(params![image_id], |row| row.get::<_, String>(0))
        .map_err(|error| format!("Cannot query image tags: {error}"))?;

    let mut tags = Vec::new();
    for row in rows {
        tags.push(row.map_err(|error| format!("Cannot read image tag: {error}"))?);
    }

    Ok(tags)
}


fn read_collection_names_for_image_from_connection(
    connection: &Connection,
    image_id: &str,
) -> Result<Vec<String>, String> {
    let mut statement = connection
        .prepare(
            r#"
            SELECT collections.name
            FROM collection_images
            JOIN collections ON collections.id = collection_images.collection_id
            WHERE collection_images.image_id = ?1
            ORDER BY collections.name COLLATE NOCASE
            "#,
        )
        .map_err(|error| format!("Cannot prepare collection query: {error}"))?;
    let rows = statement
        .query_map(params![image_id], |row| row.get::<_, String>(0))
        .map_err(|error| format!("Cannot query image collections: {error}"))?;

    let mut collections = Vec::new();
    for row in rows {
        collections.push(row.map_err(|error| format!("Cannot read image collection: {error}"))?);
    }

    Ok(collections)
}

fn read_collection_names_for_image_from_transaction(
    transaction: &rusqlite::Transaction<'_>,
    image_id: &str,
) -> Result<Vec<String>, String> {
    let mut statement = transaction
        .prepare(
            r#"
            SELECT collections.name
            FROM collection_images
            JOIN collections ON collections.id = collection_images.collection_id
            WHERE collection_images.image_id = ?1
            ORDER BY collections.name COLLATE NOCASE
            "#,
        )
        .map_err(|error| format!("Cannot prepare collection query: {error}"))?;
    let rows = statement
        .query_map(params![image_id], |row| row.get::<_, String>(0))
        .map_err(|error| format!("Cannot query image collections: {error}"))?;

    let mut collections = Vec::new();
    for row in rows {
        collections.push(row.map_err(|error| format!("Cannot read image collection: {error}"))?);
    }

    Ok(collections)
}

fn read_image_favorite_from_transaction(
    transaction: &rusqlite::Transaction<'_>,
    image_id: &str,
) -> Result<bool, String> {
    transaction
        .query_row("SELECT is_favorite FROM images WHERE id = ?1", params![image_id], |row| row.get::<_, i64>(0))
        .map(|value| value != 0)
        .map_err(|error| format!("Cannot read favorite state: {error}"))
}

fn read_path_seen_times(
    transaction: &rusqlite::Transaction<'_>,
    path: &str,
) -> Result<(Option<u64>, Option<u64>), String> {
    transaction
        .query_row(
            "SELECT first_seen_at_unix, last_seen_at_unix FROM image_paths WHERE path = ?1 LIMIT 1",
            params![path],
            |row| {
                let first_seen: Option<i64> = row.get(0)?;
                let last_seen: Option<i64> = row.get(1)?;
                Ok((first_seen.and_then(i64_to_u64), last_seen.and_then(i64_to_u64)))
            },
        )
        .optional()
        .map(|value| value.unwrap_or((None, None)))
        .map_err(|error| format!("Cannot read path seen timestamps: {error}"))
}

fn image_identity_key(image: &IndexImageInputDto, sha256: Option<&str>) -> String {
    match sha256.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => format!("sha256:{}", value.to_lowercase()),
        None => legacy_image_identity_key(image),
    }
}

fn legacy_image_identity_key(image: &IndexImageInputDto) -> String {
    format!(
        "{}|{}|{}|{}",
        image.file_name.trim().to_lowercase(),
        image.size_bytes.unwrap_or(0),
        image.modified_at_unix.unwrap_or(0),
        image.extension.as_deref().unwrap_or("").to_lowercase()
    )
}


fn normalize_metadata_tags(tags: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::new();
    for raw_tag in tags {
        let tag = raw_tag.trim().trim_start_matches('#').trim().to_lowercase();
        if tag.is_empty() || normalized.iter().any(|existing| existing == &tag) {
            continue;
        }
        normalized.push(tag);
    }

    normalized
}

fn compute_file_sha256(path: &Path) -> Result<String, String> {
    let file = fs::File::open(path)
        .map_err(|error| format!("Cannot open file for SHA-256 verification {}: {error}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];

    loop {
        let read_count = reader
            .read(&mut buffer)
            .map_err(|error| format!("Cannot read file for SHA-256 verification {}: {error}", path.display()))?;
        if read_count == 0 {
            break;
        }
        hasher.update(&buffer[..read_count]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn stable_text_id(prefix: &str, value: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }

    format!("{prefix}:{hash:016x}")
}

fn u64_to_i64_saturating(value: u64) -> i64 {
    if value > i64::MAX as u64 {
        i64::MAX
    } else {
        value as i64
    }
}

fn i64_to_u64(value: i64) -> Option<u64> {
    if value < 0 {
        None
    } else {
        Some(value as u64)
    }
}

fn image_mime_type(path: &Path) -> String {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_lowercase()
        .as_str()
    {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "webp" => "image/webp",
        "tif" | "tiff" => "image/tiff",
        "heic" => "image/heic",
        "heif" => "image/heif",
        "avif" => "image/avif",
        _ => "application/octet-stream",
    }
    .to_string()
}

fn thumbnail_cache_key(path: &Path, size_bytes: u64, modified_at: u64, width: u32, height: u32) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in path_to_string(path).as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }

    for byte in size_bytes.to_le_bytes()
        .into_iter()
        .chain(modified_at.to_le_bytes())
        .chain(width.to_le_bytes())
        .chain(height.to_le_bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }

    format!("{hash:016x}_{width}x{height}")
}


fn smart_rename_unique8(path: &Path, size_bytes: u64, modified_at: u64, sha256: Option<&str>, salt: u32) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in b"smart-photo-indexer-rename" {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }

    if let Some(content_hash) = sha256.map(str::trim).filter(|value| !value.is_empty()) {
        for byte in content_hash.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
    } else {
        for byte in path_to_string(path).as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        for byte in size_bytes.to_le_bytes().into_iter().chain(modified_at.to_le_bytes()) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }

    for byte in salt.to_le_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }

    format!("{:08X}", (hash & 0xffff_ffff) as u32)
}

fn ymd_label_from_unix(unix_seconds: u64) -> String {
    let days = (unix_seconds / 86_400) as i64;
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

fn civil_from_days(days_since_unix_epoch: i64) -> (i64, i64, i64) {
    let z = days_since_unix_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if month <= 2 { 1 } else { 0 };
    (year, month, day)
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn path_segments(path: &Path) -> Vec<String> {
    let mut segments = Vec::new();

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => segments.push(prefix.as_os_str().to_string_lossy().to_string()),
            Component::RootDir => {
                if segments.is_empty() {
                    segments.push(std::path::MAIN_SEPARATOR.to_string());
                }
            }
            Component::Normal(value) => segments.push(value.to_string_lossy().to_string()),
            Component::CurDir => {}
            Component::ParentDir => segments.push("..".to_string()),
        }
    }

    if segments.is_empty() {
        segments.push(path_to_string(path));
    }

    segments
}



fn read_collection_by_id(connection: &Connection, collection_id: &str) -> Result<Option<CollectionDto>, String> {
    connection
        .query_row(
            r#"
            SELECT c.id, c.name, c.description, c.color, c.is_favorite, c.created_at_unix, c.updated_at_unix,
                   COUNT(ci.image_id) AS image_count
            FROM collections c
            LEFT JOIN collection_images ci ON ci.collection_id = c.id
            WHERE c.id = ?1
            GROUP BY c.id, c.name, c.description, c.color, c.is_favorite, c.created_at_unix, c.updated_at_unix
            LIMIT 1
            "#,
            params![collection_id],
            |row| {
                Ok(CollectionDto {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    color: row.get(3)?,
                    is_favorite: row.get::<_, i64>(4)? != 0,
                    created_at_unix: optional_i64_to_u64(Some(row.get::<_, i64>(5)?)).unwrap_or(0),
                    updated_at_unix: optional_i64_to_u64(Some(row.get::<_, i64>(6)?)).unwrap_or(0),
                    image_count: row.get::<_, i64>(7)? as usize,
                })
            },
        )
        .optional()
        .map_err(|error| format!("Cannot read collection: {error}"))
}

#[tauri::command]
async fn get_collections(app_handle: tauri::AppHandle) -> Result<CollectionsResultDto, String> {
    tauri::async_runtime::spawn_blocking(move || get_collections_internal(app_handle)).await.map_err(|error| error.to_string())?
}

fn get_collections_internal(app_handle: tauri::AppHandle) -> Result<CollectionsResultDto, String> {
    let mut connection = open_index_connection(&app_handle)?;
    run_index_migrations(&mut connection)?;
    let mut statement = connection
        .prepare(
            r#"
            SELECT c.id, c.name, c.description, c.color, c.is_favorite, c.created_at_unix, c.updated_at_unix,
                   COUNT(ci.image_id) AS image_count
            FROM collections c
            LEFT JOIN collection_images ci ON ci.collection_id = c.id
            GROUP BY c.id, c.name, c.description, c.color, c.is_favorite, c.created_at_unix, c.updated_at_unix
            ORDER BY c.is_favorite DESC, c.updated_at_unix DESC, c.name COLLATE NOCASE
            "#,
        )
        .map_err(|error| format!("Cannot prepare collections query: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(CollectionDto {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                color: row.get(3)?,
                is_favorite: row.get::<_, i64>(4)? != 0,
                created_at_unix: optional_i64_to_u64(Some(row.get::<_, i64>(5)?)).unwrap_or(0),
                updated_at_unix: optional_i64_to_u64(Some(row.get::<_, i64>(6)?)).unwrap_or(0),
                image_count: row.get::<_, i64>(7)? as usize,
            })
        })
        .map_err(|error| format!("Cannot query collections: {error}"))?;
    let mut collections = Vec::new();
    for row in rows {
        collections.push(row.map_err(|error| format!("Cannot read collection row: {error}"))?);
    }
    Ok(CollectionsResultDto { total_count: collections.len(), collections, loaded_at_unix: unix_now() })
}

#[tauri::command]
async fn create_collection(app_handle: tauri::AppHandle, input: CreateCollectionInputDto) -> Result<CollectionMutationResultDto, String> {
    tauri::async_runtime::spawn_blocking(move || create_collection_internal(app_handle, input)).await.map_err(|error| error.to_string())?
}

fn create_collection_internal(app_handle: tauri::AppHandle, input: CreateCollectionInputDto) -> Result<CollectionMutationResultDto, String> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err("Collection name is empty.".to_string());
    }
    let mut connection = open_index_connection(&app_handle)?;
    run_index_migrations(&mut connection)?;
    let now = unix_now();
    if name.chars().count()>200{return Err("Название коллекции слишком длинное.".into());}
    let id = connection.query_row("SELECT id FROM collections WHERE unicode_lower(name)=unicode_lower(?1)",[name],|row|row.get::<_,String>(0)).optional().map_err(|e|e.to_string())?
        .unwrap_or_else(||stable_text_id("collection",&format!("{}|{}",name,SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos())));
    connection
        .execute(
            r#"
            INSERT INTO collections (id, name, description, color, is_favorite, created_at_unix, updated_at_unix)
            VALUES (?1, ?2, ?3, ?4, 0, ?5, ?5)
            ON CONFLICT(id) DO UPDATE SET
                description = COALESCE(excluded.description, collections.description),
                color = COALESCE(excluded.color, collections.color),
                updated_at_unix = excluded.updated_at_unix
            "#,
            params![&id, name, input.description.as_deref().map(str::trim).filter(|value| !value.is_empty()), input.color.as_deref().map(str::trim).filter(|value| !value.is_empty()), u64_to_i64_saturating(now)],
        )
        .map_err(|error| format!("Cannot create collection: {error}"))?;

    let collection = read_collection_by_id(&connection, &id)?;
    Ok(CollectionMutationResultDto { collection, requested_images: 0, changed_images: 0, skipped_images: 0, action_log_entries: 0, updated_at_unix: now })
}

#[tauri::command]
async fn add_images_to_collection(app_handle: tauri::AppHandle, input: AddImagesToCollectionInputDto) -> Result<CollectionMutationResultDto, String> {
    tauri::async_runtime::spawn_blocking(move || add_images_to_collection_internal(app_handle, input)).await.map_err(|error| error.to_string())?
}

fn add_images_to_collection_internal(app_handle: tauri::AppHandle, input: AddImagesToCollectionInputDto) -> Result<CollectionMutationResultDto, String> {
    let collection_id = input.collection_id.trim();
    if collection_id.is_empty() {
        return Err("Collection id is empty.".to_string());
    }
    let mut connection = open_index_connection(&app_handle)?;
    run_index_migrations(&mut connection)?;
    let now = unix_now();
    let transaction = connection.transaction().map_err(|error| format!("Cannot start collection transaction: {error}"))?;
    let collection_exists: i64 = transaction
        .query_row("SELECT COUNT(*) FROM collections WHERE id = ?1", params![collection_id], |row| row.get(0))
        .map_err(|error| format!("Cannot check collection: {error}"))?;
    if collection_exists == 0 {
        return Err(format!("Collection was not found: {collection_id}"));
    }

    let mut changed = 0usize;
    let mut skipped = 0usize;
    let mut logs = 0usize;
    for image_id in &input.image_ids {
        let image_id = image_id.trim();
        if image_id.is_empty() {
            skipped += 1;
            continue;
        }
        let exists: i64 = transaction
            .query_row("SELECT COUNT(*) FROM images WHERE id = ?1", params![image_id], |row| row.get(0))
            .map_err(|error| format!("Cannot check image {image_id}: {error}"))?;
        if exists == 0 {
            skipped += 1;
            continue;
        }
        let affected = transaction
            .execute(
                r#"
                INSERT INTO collection_images (collection_id, image_id, added_at_unix)
                VALUES (?1, ?2, ?3)
                ON CONFLICT(collection_id, image_id) DO NOTHING
                "#,
                params![collection_id, image_id, u64_to_i64_saturating(now)],
            )
            .map_err(|error| format!("Cannot add image to collection: {error}"))?;
        if affected > 0 {
            changed += 1;
            transaction
                .execute(
                    r#"
                    INSERT INTO action_logs (action_type, image_id, old_value, new_value, message, created_at_unix)
                    VALUES ('collection_added', ?1, NULL, ?2, ?3, ?4)
                    "#,
                    params![image_id, collection_id, format!("Image added to collection {collection_id}"), u64_to_i64_saturating(now)],
                )
                .map_err(|error| format!("Cannot write collection action log: {error}"))?;
            logs += 1;
        }
    }
    transaction
        .execute("UPDATE collections SET updated_at_unix = ?2 WHERE id = ?1", params![collection_id, u64_to_i64_saturating(now)])
        .map_err(|error| format!("Cannot update collection timestamp: {error}"))?;
    transaction.commit().map_err(|error| format!("Cannot commit collection changes: {error}"))?;
    let collection = read_collection_by_id(&connection, collection_id)?;
    Ok(CollectionMutationResultDto { collection, requested_images: input.image_ids.len(), changed_images: changed, skipped_images: skipped, action_log_entries: logs, updated_at_unix: now })
}

#[tauri::command]
async fn set_image_favorite(app_handle: tauri::AppHandle, input: SetImageFavoriteInputDto) -> Result<SetImageFavoriteResultDto, String> {
    let worker_handle = app_handle.clone();
    let result = tauri::async_runtime::spawn_blocking(move || set_image_favorite_internal(worker_handle,input))
        .await.map_err(|error| format!("Favorite worker failed: {error}"))??;
    viewer_sessions::update_favorite(&app_handle,&result)?;
    Ok(result)
}

fn set_image_favorite_internal(app_handle: tauri::AppHandle, mut input: SetImageFavoriteInputDto) -> Result<SetImageFavoriteResultDto, String> {
    if input.path.is_some() {
        if let Some(path) = input.path.as_deref() {
            let source = validate_existing_image_file(path)?.canonicalize().map_err(|error| error.to_string())?;
            let metadata = fs::metadata(&source).map_err(|error| error.to_string())?;
            let path = path_to_string(&source);
            let sync = sync_scanned_images_to_index_internal(app_handle.clone(), vec![IndexImageInputDto {
                path: path.clone(), file_name: file_name(&source), extension: extension(&source),
                size_bytes: Some(metadata.len()), modified_at_unix: modified_at_unix(&metadata), width: None, height: None,
            }])?;
            input.image_id = sync.indexed_images.first().map(|image| image.image_id.clone());
            input.path = Some(path);
        }
    }
    let mut connection = open_index_connection(&app_handle)?;
    run_index_migrations(&mut connection)?;
    let now = unix_now();
    let image_id_filter = input.image_id.as_deref().map(str::trim).filter(|value| !value.is_empty());
    let path_filter = input.path.as_deref().map(str::trim).filter(|value| !value.is_empty());
    if image_id_filter.is_none() && path_filter.is_none() {
        return Err("Image id or path is required to set favorite state.".to_string());
    }
    let (image_id, mut file_name, mut path): (String, String, String) = if let Some(image_id) = image_id_filter {
        connection
            .query_row(
                r#"
                SELECT i.id, i.file_name, COALESCE(p.path, '')
                FROM images i
                LEFT JOIN image_paths p ON p.image_id = i.id AND p.is_current = 1
                WHERE i.id = ?1
                ORDER BY p.last_seen_at_unix DESC
                LIMIT 1
                "#,
                params![image_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|error| format!("Cannot find image for favorite update: {error}"))?
    } else {
        let path = path_filter.unwrap();
        connection
            .query_row(
                r#"
                SELECT i.id, i.file_name, p.path
                FROM images i
                INNER JOIN image_paths p ON p.image_id = i.id
                WHERE p.path = ?1
                ORDER BY p.is_current DESC, p.last_seen_at_unix DESC
                LIMIT 1
                "#,
                params![path],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|error| format!("Cannot find image by path for favorite update: {error}"))?
    };
    if let Some(requested_path) = path_filter {
        path = requested_path.to_string();
        file_name = crate::file_name(Path::new(requested_path));
    }
    let favorite_value = if input.is_favorite { 1i64 } else { 0i64 };
    let transaction = connection.transaction().map_err(|error| error.to_string())?;
    transaction
        .execute("UPDATE images SET is_favorite = ?2, updated_at_unix = ?3 WHERE id = ?1", params![&image_id, favorite_value, u64_to_i64_saturating(now)])
        .map_err(|error| format!("Cannot update favorite state: {error}"))?;
    transaction
        .execute(
            r#"
            INSERT INTO action_logs (action_type, image_id, old_value, new_value, message, created_at_unix)
            VALUES ('favorite_changed', ?1, NULL, ?2, ?3, ?4)
            "#,
            params![&image_id, if input.is_favorite { "true" } else { "false" }, format!("Favorite state changed for {file_name}"), u64_to_i64_saturating(now)],
        )
        .map_err(|error| format!("Cannot write favorite action log: {error}"))?;
    let action_log_id: i64 = transaction
        .query_row("SELECT last_insert_rowid()", [], |row| row.get(0))
        .map_err(|error| format!("Cannot read favorite action log id: {error}"))?;
    transaction.commit().map_err(|error| format!("Cannot commit favorite edit: {error}"))?;
    Ok(SetImageFavoriteResultDto { image_id, path, file_name, is_favorite: input.is_favorite, updated_at_unix: now, action_log_id })
}

include!("../../src/modules/imageViewer/tauri-resize-commands.rs");
include!("../../src/modules/imageViewer/tauri-viewer-actions.rs");
include!("../../src/modules/localAi/tauri-local-ai-commands.rs");

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut context = tauri::generate_context!();
    if let Some(profile) = std::env::var_os("SMART_PHOTO_DATA_DIR") {
        context.config_mut().identifier = format!("com.smartphotoindexer.profile.{:x}",
            Sha256::digest(profile.to_string_lossy().as_bytes()));
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
            let app = app.clone();
            tauri::async_runtime::spawn_blocking(move || {
                if let Err(error) = viewer_sessions::launch_arguments(&app,args,cwd) {
                    let _ = viewer_sessions::show_main(&app);
                    eprintln!("Cannot open image request: {error}");
                }
            });
        }))
        .manage(viewer_sessions::ViewerSessions::default())
        .manage(window_appearance::WindowAppearance::default())
        .manage(photo_editor::EditorSessions::default())
        .manage(photo_editor::jobs::EditorJobs::default())
        .manage(processing::ProcessingRuntime::default())
        .manage(file_actions::FileActionsRuntime::default())
        .manage(scanning::ScanSessions::default())
        .manage(search::SearchSessions::default())
        .manage(credentials::CredentialVault::default())
        .manage(local_install::LocalInstaller::default())
        .manage(index_backups::BackupRuntime::default())
        .manage(library_roots::LibraryRuntime::default())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();
            index_backups::apply_pending_before_workers(&handle);
            file_actions::start(handle.clone());
            search::start_cleanup(handle.clone());
            let args = std::env::args().collect();
            let cwd = std::env::current_dir()?.to_string_lossy().to_string();
            if let Err(error) = viewer_sessions::launch_arguments(&handle,args,cwd) {
                viewer_sessions::show_main(&handle).map_err(std::io::Error::other)?;
                eprintln!("Cannot open initial image request: {error}");
            }
            Ok(())
        })
        .on_window_event(move |window,event| {
            if let tauri::WindowEvent::CloseRequested {api,..}=event {photo_editor::close_requested(window,api);}
            #[cfg(target_os = "windows")]
            match event {
                tauri::WindowEvent::Focused(active) => windows_memory::focus(window,*active),
                tauri::WindowEvent::CloseRequested { .. } => windows_memory::focus(window,false),
                _ => {},
            }
            #[cfg(target_os = "windows")]
            if matches!(event,tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed) {
                if let Ok(hwnd) = window.hwnd() { windows_share::release(hwnd.0 as usize); }
            }
            if matches!(event,tauri::WindowEvent::Destroyed) {
                window_appearance::remove(window.app_handle(),window.label());
                viewer_sessions::remove(window.app_handle(),window.label());
                photo_editor::remove(window.app_handle(),window.label());
            }
        })
        .invoke_handler(tauri::generate_handler![
            batch_rename::preview_batch_rename,batch_rename::preview_batch_rename_names,batch_rename::batch_rename_command,batch_rename::get_batch_rename_history,batch_rename::get_batch_rename_selection_paths,
            file_actions::explorer_file_selection,file_actions::explorer_file_operation,
            window_appearance::apply_window_appearance,
            photo_editor::open_photo_editor,photo_editor::open_help_working_window,photo_editor::get_editor_session,photo_editor::save_editor_draft,photo_editor::finish_editor_navigation,photo_editor::apply_editor_local_operation,photo_editor::save_editor_mask,photo_editor::editor_library_command,photo_editor::import_editor_file,photo_editor::export_editor_image,
            photo_editor::save_editor_checkpoint,photo_editor::get_editor_lifecycle,photo_editor::preview_editor_request,photo_editor::probe_editor_model,
            photo_editor::attach_editor_references,photo_editor::editor_reference_capacity,photo_editor::editor_storage_command,photo_editor::show_editor_reconciliation,photo_editor::start_editor_job,photo_editor::get_editor_jobs,photo_editor::get_editor_workbench,photo_editor::preview_project_cleanup,photo_editor::confirm_project_cleanup,photo_editor::editor_job_command,photo_editor::reconcile_editor_request,
            photo_editor::editor_project_package,
            photo_editor::import_editor_url,
            photo_editor::editor_full_backup,photo_editor::get_editor_catalog,photo_editor::get_editor_model_pricing,photo_editor::editor_catalog_command,photo_editor::check_editor_catalog_sources,photo_editor::open_editor_project,photo_editor::publish_editor_copy, photo_editor::preview_editor_replacement, photo_editor::replace_editor_original,photo_editor::get_editor_publications,photo_editor::retry_editor_publication,photo_editor::cancel_editor_publication,
            get_processing_snapshot, processing::get_processing_cost_statistics, download_local_model, get_ai_provider_status,provider_transport::get_cached_provider_status,provider_transport::ensure_ai_provider_ready,
            #[cfg(target_os = "windows")]
            windows_memory::get_webview_memory_statistics,
            next_directory_scan_page, close_directory_scan,
            save_provider_credential, get_provider_credentials, credentials::read_clipboard_text,
            prepare_processing_job, selection_statistics::get_selection_statistics, processing::cancel_processing_preparation, processing::remove_processing_work, processing_job_command, processing::run::create_processing_queue, processing::run::processing_queue_command, processing::run::get_processing_queue_contexts, processing::set_processing_credential, set_processing_concurrency, reconcile_processing_attempts, get_processing_jobs, get_processing_tasks,
            processing::get_unresolved_processing_attempts, processing::resolve_unknown_processing_attempt,
            edit_image_file, set_viewer_active, open_viewer_window, get_viewer_session, viewer_sessions::get_viewer_session_statistics, viewer_sessions::open_search_viewer_window, search::mutate_search_collection, search::get_search_selection_image_ids,
            choose_folder_dialog, get_filesystem_roots, scan_directory, inspect_dropped_paths,
            open_external_path, get_or_create_thumbnail, get_cached_thumbnail,get_selected_photo_preview, thumbnail_cache::get_thumbnail_cache_statistics, thumbnail_cache::clear_thumbnail_cache, get_ai_image_file_fingerprint, encode_image_for_ai,
            initialize_index_database, get_index_database_info, sync_scanned_images_to_index,
            index_observations::get_index_observation_statistics,
            rescan_index_paths, update_image_metadata, xmp_import::import_image_xmp,
            processing_task_image::get_processing_task_image,
            search_indexed_images, search::close_search_snapshot, search::fork_search_snapshot,
            search::export_search_snapshot, image_metadata::inspect_image_metadata,
            get_action_journal, create_action_log_entry, preview_smart_rename_images, smart_rename_image,
            get_index_maintenance, remove_index_images, cleanup_index_garbage,
            get_collections, create_collection, add_images_to_collection,
            collections::rename_collection, collections::delete_collection, collections::remove_images_from_collection,
            batch_tags::preview_batch_tags, batch_tags::commit_batch_tags,
            batch_tags::get_last_batch_tags_action,
            batch_tags::get_batch_tag_summary,
            xmp_export::export_image_xmp,
            library_roots::get_library_roots, library_roots::add_library_root,
            library_roots::update_library_root, library_roots::remove_library_root,
            library_roots::refresh_library_root,
            duplicates::get_duplicate_groups, duplicates::get_duplicate_paths, duplicates::verify_duplicate_paths,duplicates::keep_duplicate_copies,duplicates::accept_duplicate_group,duplicates::reset_duplicate_decisions,project_preferences::get_project_preferences,project_preferences::initialize_project_preferences,project_preferences::patch_project_preferences,profile_preferences::get_profile_preferences,profile_preferences::initialize_profile_preferences,profile_preferences::patch_profile_preferences,
            index_backups::choose_index_backup_path, index_backups::inspect_index_backup,
            index_backups::export_index_backup, index_backups::stage_index_restore,
            index_backups::get_index_restore_status, index_backups::cancel_index_restore,
            set_image_favorite, choose_save_image_path, save_resized_image,
            choose_save_original_image_path, save_original_image_copy,
            trash_image_file, print_image_file, read_image_file_for_share, share_image_file,
            set_desktop_wallpaper, prepare_ollama_runtime,
            local_install::install_local_model,local_install::preview_local_installation, local_install::cancel_local_model_install
        ])
        .run(context)
        .expect("error while running Smart Photo Indexer");
}

#[cfg(test)]
mod index_sql_function_tests {
    use super::*;
    fn connection() -> Connection {
        let db=Connection::open_in_memory().unwrap();
        db.create_scalar_function("unicode_lower",1,rusqlite::functions::FunctionFlags::SQLITE_UTF8 | rusqlite::functions::FunctionFlags::SQLITE_DETERMINISTIC,sqlite_unicode_lower).unwrap();
        db
    }
    #[test]
    fn unicode_lower_preserves_sql_null_and_unicode_text() {
        let db=connection();
        assert_eq!(db.query_row("SELECT unicode_lower(NULL)",[],|r|r.get::<_,Option<String>>(0)).unwrap(),None);
        assert_eq!(db.query_row("SELECT unicode_lower('ПНГ.JPG')",[],|r|r.get::<_,String>(0)).unwrap(),"пнг.jpg");
        assert!(db.query_row("SELECT unicode_lower(123)",[],|r|r.get::<_,String>(0)).is_err());
    }
    #[test]
    fn extension_order_accepts_missing_metadata_and_keeps_unknown_types_last() {
        let db=connection();
        db.execute_batch("CREATE TABLE images(id INTEGER PRIMARY KEY,extension TEXT);INSERT INTO images VALUES(1,'JPG'),(2,'png'),(3,NULL),(4,NULL);").unwrap();
        for (direction,expected) in [("ASC",vec![1,2,3,4]),("DESC",vec![2,1,3,4])] {
            let mut statement=db.prepare(&format!("SELECT id FROM images ORDER BY extension IS NULL,unicode_lower(extension) {direction},id")).unwrap();
            let actual=statement.query_map([],|r|r.get::<_,i64>(0)).unwrap().collect::<Result<Vec<_>,_>>().unwrap();
            assert_eq!(actual,expected);
        }
    }
}

#[cfg(test)]
mod migration_reentry_tests {
    use super::*;
    #[test]
    fn supported_schema_is_the_last_migration_and_can_be_opened_repeatedly() {
        let mut db=Connection::open_in_memory().unwrap();
        run_index_migrations(&mut db).unwrap();
        assert_eq!(current_schema_version(&db).unwrap(),INDEX_SCHEMA_VERSION);
        for (ordinal,migration) in INDEX_MIGRATIONS.iter().enumerate(){assert_eq!(migration.version,ordinal as i64+1);}
        run_index_migrations(&mut db).unwrap();
        assert_eq!(current_schema_version(&db).unwrap(),INDEX_SCHEMA_VERSION);
    }
    #[test]
    fn logical_path_queries_do_not_scan_all_current_archive_paths() {
        let mut db=Connection::open_in_memory().unwrap();run_index_migrations(&mut db).unwrap();
        let plan:String=db.query_row("EXPLAIN QUERY PLAN SELECT group_concat(path,' ') FROM image_paths WHERE image_id='test' AND is_current=1",[],|r|r.get(3)).unwrap();
        assert!(plan.contains("idx_image_paths_logical_current"),"{plan}");
    }
    #[test]
    fn path_observation_avoids_fts_writes_and_reassignment_reindexes_both_images() {
        let mut db=Connection::open_in_memory().unwrap();run_index_migrations(&mut db).unwrap();
        db.execute_batch("INSERT INTO images(id,identity_key,file_name,created_at_unix,updated_at_unix) VALUES('cat','cat','cat.png',1,1),('dog','dog','dog.png',1,1); INSERT INTO image_paths(image_id,path,first_seen_at_unix,last_seen_at_unix) VALUES('cat','oldalias.png',1,1),('cat','keep.png',1,1);").unwrap();
        let before=db.total_changes();db.execute("UPDATE image_paths SET last_seen_at_unix=2 WHERE path='oldalias.png'",[]).unwrap();assert_eq!(db.total_changes()-before,1);
        db.execute("UPDATE image_paths SET image_id='dog' WHERE path='oldalias.png'",[]).unwrap();
        let matched:String=db.query_row("SELECT image_id FROM image_search WHERE image_search MATCH 'oldalias'",[],|r|r.get(0)).unwrap();assert_eq!(matched,"dog");
        let cats:i64=db.query_row("SELECT COUNT(*) FROM image_search WHERE image_search MATCH 'keep' AND image_id='cat'",[],|r|r.get(0)).unwrap();assert_eq!(cats,1);
    }
    #[test]
    fn unchanged_image_observation_keeps_full_text_and_aliases_without_rewriting_fts() {
        let mut db=Connection::open_in_memory().unwrap();run_index_migrations(&mut db).unwrap();
        let text=format!("{} Оранжевый карабин.","Большое полное описание. ".repeat(500));
        db.execute("INSERT INTO images(id,identity_key,file_name,description,created_at_unix,updated_at_unix) VALUES('a','a','photo.png',?1,1,1)",[&text]).unwrap();
        db.execute_batch("INSERT INTO image_paths(image_id,path,first_seen_at_unix,last_seen_at_unix) VALUES('a','C:\\Photos\\alias.png',1,1);").unwrap();
        let before=db.total_changes();db.execute("UPDATE images SET file_name='photo.png',description=?1,updated_at_unix=2 WHERE id='a'",[&text]).unwrap();assert_eq!(db.total_changes()-before,1);
        assert_eq!(db.query_row("SELECT COUNT(*) FROM image_search WHERE image_search MATCH 'карабин OR alias'",[],|r|r.get::<_,i64>(0)).unwrap(),1);
        db.execute_batch("UPDATE images SET description='Обезьяна у дерева',file_name='renamed.png' WHERE id='a';").unwrap();
        assert_eq!(db.query_row("SELECT COUNT(*) FROM image_search WHERE image_search MATCH 'карабин'",[],|r|r.get::<_,i64>(0)).unwrap(),0);
        assert_eq!(db.query_row("SELECT COUNT(*) FROM image_search WHERE image_search MATCH 'обезьяна AND renamed AND alias'",[],|r|r.get::<_,i64>(0)).unwrap(),1);
    }
    #[cfg(windows)]
    #[test]
    fn canonical_windows_prefixes_keep_root_boundaries_and_case_rules() {
        let root=Some(PathBuf::from(r"\\?\D:\Photos\"));
        assert!(path_is_under_rescan_root(r"d:\photos\image.jpg",&root));
        assert!(!path_is_under_rescan_root(r"D:\Photos-old\image.jpg",&root));
        let unc=Some(PathBuf::from(r"\\?\UNC\server\share\Photos"));
        assert!(path_is_under_rescan_root(r"\\server\share\photos\image.jpg",&unc));
        assert!(!path_is_under_rescan_root(r"\\server\share\Photos-old\image.jpg",&unc));
    }
}
