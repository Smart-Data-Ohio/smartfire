//! `has_one_attached` as `User::Avatar` (`:avatar`) and `Account` (`:logo`) use it, over
//! `campfire_storage` (reference/app/models/user/avatar.rb, account.rb, and Active Storage's
//! `Attached::Changes::CreateOne` / `Attachment`).
//!
//! Assigning an uploaded file and saving the record, in the record's transaction:
//! the old attachment is destroyed first (`has_one ... dependent: :destroy` replacing its target;
//! its blob is purged after commit, `dependent: :purge_later`), then the new blob and attachment
//! rows are inserted, and each attachment change touches the record
//! (`belongs_to :record, touch: true`). A fresh blob's analyzer runs after commit (analysis touches
//! the record again): media analyzers use a durable job, while `NullAnalyzer` runs inline.
//!
//! Unlike Rails, which uploads after commit, the file is uploaded before the transaction
//! ([`Assignment::stage`]), so the writer never waits on copying and checksumming it; a
//! transaction that rolls back deletes it again.

use std::sync::Arc;

use campfire_db::{CachedStatements, Connection, Event, Tx};
use campfire_kit::{Error, Param, Result, UploadedFile};
use campfire_storage::{Blob, Filename, Staged, Variation};

use crate::active_storage::{keep_after_commit, stage_file};
use crate::queue::AnalyzeJob;
use crate::app::App;

/// An uploaded file (`ActionDispatch::Http::UploadedFile`), still in its multipart tempfile.
#[derive(Debug, Clone)]
pub struct Upload {
    pub file: Arc<UploadedFile>,
    pub filename: String,
    pub content_type: Option<String>,
}

impl Upload {
    /// The upload in `param`, if it is one. `""` and nil mean "no change" to the controllers
    /// here (`params.permit(...).compact` / `avatar=` with nil or "" deletes, see [`Assignment`]).
    pub fn from_param(param: Option<&Param>) -> Option<Upload> {
        let file = param.and_then(Param::as_file)?;
        Some(Upload { file: file.clone(), filename: file.original_filename.clone(), content_type: file.content_type.clone() })
    }

    /// Uploads the file to storage for a blob whose row is saved next, off the async threads.
    pub async fn stage(self, app: &App) -> Result<Staged> {
        let path = self.file.path().to_path_buf();
        stage_file(app, path, Filename::new(self.filename), self.content_type).await
    }
}

/// What `record.avatar = value` does with a permitted param value: `Create` holds the [`Upload`],
/// then, once staged, the [`Staged`] blob.
#[derive(Debug, Clone)]
pub enum Assignment<U = Upload> {
    /// The key wasn't given.
    Unchanged,
    /// `nil` or `""`: `Attached::Changes::DeleteOne` (the attachment is destroyed on save).
    Delete,
    /// An uploaded file: `Attached::Changes::CreateOne`.
    Create(U),
    /// A direct upload's signed blob id, resolved before saving.
    Signed(String),
    /// A verified, existing blob.
    Existing(Blob),
    /// Anything else (e.g. a plain string that isn't a signed blob id): Rails raises.
    Invalid,
}

impl Assignment {
    pub fn from_params(params: &campfire_kit::ParamMap, key: &str) -> Result<Assignment> {
        if !params.contains_key(key) {
            return Ok(Assignment::Unchanged);
        }
        match params.get(key) {
            None => Ok(Assignment::Delete),
            Some(param) if param.is_null() || param.as_str() == Some("") => Ok(Assignment::Delete),
            Some(param) if param.as_str().is_some() => Ok(Assignment::Signed(param.as_str().unwrap().to_string())),
            Some(param) => Ok(Upload::from_param(Some(param)).map_or(Assignment::Invalid, Assignment::Create)),
        }
    }

    /// Uploads a new file, so the save only has rows to write.
    pub async fn stage(self, app: &App) -> Result<Assignment<Staged>> {
        Ok(match self {
            Assignment::Unchanged => Assignment::Unchanged,
            Assignment::Delete => Assignment::Delete,
            Assignment::Create(upload) => Assignment::Create(upload.stage(app).await?),
            Assignment::Signed(signed) => {
                let id = campfire_storage::paths::verify_signed_blob_id(&*app.storage.verifier, &signed, app.clock.now())
                    .ok_or_else(|| Error::internal(anyhow::anyhow!("invalid blob signature")))?;
                let blob = app.db.read(move |conn| Blob::find(conn, id).map_err(storage_error)).await.map_err(Error::internal)?
                    .ok_or(Error::NotFound)?;
                let storage = app.storage.clone();
                let blob = tokio::task::spawn_blocking(move || storage.identify_blob(blob)).await.map_err(Error::internal)?.map_err(Error::internal)?;
                Assignment::Existing(blob)
            }
            Assignment::Existing(blob) => Assignment::Existing(blob),
            Assignment::Invalid => Assignment::Invalid,
        })
    }
}

/// `record.<name>.attached?`'s blob: the attachment's blob, if any.
pub fn attached_blob(conn: &Connection, record_type: &str, record_id: i64, name: &str) -> campfire_db::Result<Option<Blob>> {
    Blob::attached(conn, record_type, record_id, name).map_err(storage_error)
}

/// Applies an assignment inside the record's save. Durable analysis jobs are atomic with it.
pub fn assign(tx: &mut Tx<'_>, record: Record, name: &str, assignment: Assignment<Staged>) -> campfire_db::Result<()> {
    match assignment {
        Assignment::Unchanged => Ok(()),
        Assignment::Delete => {
            destroy(tx, record, name)?;
            Ok(())
        }
        Assignment::Create(staged) => attach(tx, record, name, staged),
        Assignment::Existing(blob) => attach_existing(tx, record, name, blob),
        Assignment::Signed(_) => Err(campfire_db::Error::Other("unstaged signed blob assignment".into())),
        Assignment::Invalid => Err(campfire_db::Error::Other("Could not find or build blob: expected attachable".into())),
    }
}

/// The record an attachment belongs to: its polymorphic type and its table.
#[derive(Debug, Clone, Copy)]
pub struct Record {
    pub record_type: &'static str,
    pub table: &'static str,
    pub id: i64,
    branding: Option<rails_compat::blob_branding::Marker>,
}

impl Record {
    pub fn user(id: i64) -> Self {
        Self {
            record_type: "User",
            table: "users",
            id,
            branding: None,
        }
    }

    pub fn workspace_icon(id: i64) -> Self {
        Self {
            record_type: "WorkspaceIcon",
            table: "workspace_icons",
            id,
            branding: None,
        }
    }

    pub fn account(id: i64, secrets: &rails_compat::Secrets) -> Self {
        Self {
            record_type: "Account",
            table: "accounts",
            id,
            branding: Some(rails_compat::blob_branding::Marker::new(secrets)),
        }
    }

    fn branding_marker(self, name: &str) -> Option<rails_compat::blob_branding::Marker> {
        self.branding.filter(|_| matches!(name, "logo" | "banner"))
    }
}

/// `record.<name> = uploaded_file; record.save`: replaces any current attachment.
pub fn attach(tx: &mut Tx<'_>, record: Record, name: &str, staged: Staged) -> campfire_db::Result<()> {
    let blob = staged.insert(tx.conn(), tx.now().jiff()).map_err(storage_error)?;
    keep_after_commit(tx, staged);
    attach_existing(tx, record, name, blob)
}

/// `Attached::Changes::CreateOne`: reuse the attachment when it already has this blob.
pub fn attach_existing(
    tx: &mut Tx<'_>,
    record: Record,
    name: &str,
    blob: Blob,
) -> campfire_db::Result<()> {
    // While attached, a logo or banner is branding through its Account association, so its
    // metadata stays what Rails records; `destroy` marks it before that association goes.
    let blob = save_existing(tx, blob)?;
    if attached_blob(tx.conn(), record.record_type, record.id, name)?
        .is_some_and(|attached| attached.id == blob.id)
    {
        return Ok(());
    }
    destroy(tx, record, name)?;
    let now = tx.now();
    campfire_storage::blob::insert_attachment(
        tx.conn(),
        name,
        record.record_type,
        record.id,
        blob.id,
        now.jiff(),
    )
    .map_err(storage_error)?;
    super::accounts::touch(tx.conn(), record.table, record.id, tx.now())?;
    enqueue_analysis(tx, &blob);
    Ok(())
}

/// Save `identify_without_saving`'s changes with the record's attachment, not in a prior write.
pub fn save_existing(tx: &mut Tx<'_>, identified: Blob) -> campfire_db::Result<Blob> {
    let mut blob = Blob::find(tx.conn(), identified.id)
        .map_err(storage_error)?
        .ok_or(campfire_db::Error::RecordNotFound("ActiveStorage::Blob"))?;
    if !blob.is_identified() && identified.is_identified() {
        blob.metadata
            .set("identified", campfire_storage::Json::Bool(true));
        blob.content_type = identified.content_type;
        tx.conn().execute_cached(
            "UPDATE active_storage_blobs SET content_type = ?1, metadata = ?2 WHERE id = ?3",
            rusqlite::params![blob.content_type, blob.metadata.encode(), blob.id],
        )?;
        crate::active_storage::touch_attachment_records(tx, blob.id)?;
    }
    Ok(blob)
}

/// `Attachment#analyze_blob_later`: media jobs commit with the attachment; Rails' synchronous
/// `NullAnalyzer` updates metadata in its own write after the attachment commits.
pub fn enqueue_analysis(tx: &mut Tx<'_>, blob: &Blob) {
    if blob.is_analyzed() {
        return;
    }
    if campfire_storage::analyze::Analyzer::for_content_type(blob.content_type()).analyze_later() {
        tx.emit_after_commit(Event::job(&AnalyzeJob { blob_id: blob.id }));
    } else {
        let blob_id = blob.id;
        tx.after_commit(move |after| {
            campfire_db::run_write(after.conn(), after.env(), |tx| {
                let Some(mut blob) = Blob::find(tx.conn(), blob_id).map_err(storage_error)? else {
                    return Ok(());
                };
                if !blob.is_analyzed() {
                    blob.metadata.set("analyzed", campfire_storage::Json::Bool(true));
                    blob.update_metadata(tx.conn(), blob.metadata.clone()).map_err(storage_error)?;
                    crate::active_storage::touch_attachment_records(tx, blob.id)?;
                }
                Ok(())
            })
        });
    }
}

/// `record.<name>.destroy` (the attachment): delete it, touch the record, and purge its blob
/// after commit. Nothing happens without an attachment (`delegate_missing_to :attachment, allow_nil: true`).
pub fn destroy(tx: &mut Tx<'_>, record: Record, name: &str) -> campfire_db::Result<bool> {
    let attachment: Option<(i64, i64)> = tx
        .conn()
        .query_row_cached(
            "SELECT id, blob_id FROM active_storage_attachments WHERE record_type = ?1 AND record_id = ?2 AND name = ?3 LIMIT 1",
            rusqlite::params![record.record_type, record.id, name],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map(Some)
        .or_else(|error| if error == rusqlite::Error::QueryReturnedNoRows { Ok(None) } else { Err(error) })?;
    let Some((attachment_id, blob_id)) = attachment else { return Ok(false) };
    if let Some(marker) = record.branding_marker(name) {
        // Keep queued analysis bounded after the Account and variant associations disappear.
        crate::active_storage::mark_branding_tree(tx.conn(), marker, blob_id)?;
    }
    tx.conn().execute_cached("DELETE FROM active_storage_attachments WHERE id = ?1", [attachment_id])?;
    super::accounts::touch(tx.conn(), record.table, record.id, tx.now())?;
    tx.emit_after_commit(Event::PurgeBlob { blob_id });
    Ok(true)
}

/// `record.<name>.variant(name).processed if record.<name>.variable?`: the processed variant's
/// blob, or `None` when there's no attachment or it can't be transformed.
pub async fn processed_variant(app: &App, record: Record, name: &str, transformations: Variation) -> Result<Option<Blob>> {
    let name = name.to_string();
    let blob = app
        .db
        .read(move |conn| attached_blob(conn, record.record_type, record.id, &name))
        .await
        .map_err(Error::internal)?;
    let Some(blob) = blob.filter(Blob::is_variable) else { return Ok(None) };
    crate::active_storage::processed_representation(app, blob, transformations).await.map(Some)
}

pub fn storage_error(error: campfire_storage::Error) -> campfire_db::Error {
    campfire_db::Error::Other(error.to_string())
}
