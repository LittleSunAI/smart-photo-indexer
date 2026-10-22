use crate::*;
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind { Copy, Move, Trash }
impl Kind {
    pub fn journal(self) -> &'static str {
        match self { Self::Copy => "explorerCopy", Self::Move => "explorerMove", Self::Trash => "explorerTrash" }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase", deny_unknown_fields)]
pub enum Source {
    Explicit { paths: Vec<String> },
    Query { query_id: String, #[serde(default)] excluded_paths: Vec<String> },
    Clipboard { selection_id: String, revision: u64 },
}
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "camelCase", rename_all_fields = "camelCase", deny_unknown_fields)]
pub enum SelectionInput {
    Capture { kind: Kind, source: Source }, Clear, Inspect,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Choice { Replace, KeepBoth, Skip }
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "camelCase", rename_all_fields = "camelCase", deny_unknown_fields)]
pub enum OperationInput {
    Start { request_id: String, kind: Kind, source: Source, destination_folder: Option<String> },
    ResolveConflict { operation_id: String, conflict_id: String, choice: Choice, #[serde(default)] apply_to_all: bool },
    CancelRemaining { operation_id: String },
    Inspect { operation_id: Option<String>, #[serde(default)] offset: usize, limit: Option<usize> },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Fingerprint {
    pub identity: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub modified_ns: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapturedFile {
    pub path: String,
    pub identity: String,
    pub size_bytes: u64,
    pub modified_ns: String,
}
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionInfo {
    pub selection_id: Option<String>, pub revision: u64, pub kind: Option<Kind>,
    pub count: usize, pub ready: bool,
}
#[derive(Clone)]
pub struct Clipboard { pub info: SelectionInfo, pub files: std::sync::Arc<Vec<CapturedFile>> }

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    pub name: String, pub path: String, pub size_bytes: u64, pub modified_at: Option<u64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conflict {
    pub conflict_id: String, pub ordinal: usize, pub source: FileInfo, pub target: FileInfo,
    pub target_fingerprint: Fingerprint, pub requires_fresh_decision: bool,
    pub replace_allowed: bool,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Counts {
    pub copied: usize, pub moved: usize, pub trashed: usize, pub skipped: usize,
    pub failed: usize, pub partial: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub ordinal: usize, pub old_path: String, pub new_path: Option<String>, pub image_id: Option<String>,
    pub status: String, pub message: Option<String>, pub filesystem_applied: bool,
    pub index_applied: bool, pub editor_applied: bool,
}
impl Outcome {
    pub fn refused(ordinal: usize, path: &str, status: &str, message: String) -> Self {
        Self { ordinal, old_path: path.into(), new_path: None, image_id: None, status: status.into(),
            message: Some(message), filesystem_applied: false, index_applied: false, editor_applied: false }
    }
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationView {
    pub operation_id: String, pub revision: u64, pub state: String, pub counts: Counts,
    pub total: usize, pub processed: usize, pub conflict: Option<Conflict>,
    pub changed_paths: Vec<Outcome>, pub warnings: Vec<String>, pub has_more: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectBinding {
    pub project_id: String, pub expected_source_path: String, pub expected_source_sha: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
    pub version: u8, pub batch_id: String, pub ordinal: usize, pub kind: Kind, pub phase: String,
    pub source_path: String, pub target_path: Option<String>, pub source_fingerprint: Fingerprint,
    pub target_before: Option<Fingerprint>, pub stage_path: Option<String>, pub stage_identity: Option<String>,
    pub backup_path: Option<String>, pub published_fingerprint: Option<Fingerprint>,
    pub source_removed: bool, pub editor_database: Option<String>, pub project_bindings: Vec<ProjectBinding>,
    pub paused_task_ids: Vec<String>, pub trash_receipt: Option<Value>, pub image_id: Option<String>,
    pub filesystem_applied: bool, pub index_applied: bool, pub editor_applied: bool,
    pub outcome: Option<Outcome>,
}
impl Payload {
    pub fn id(&self) -> String { format!("{}-{}", self.batch_id, self.ordinal) }
}
