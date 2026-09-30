//! Rooms::FilesController: bounded upload/blob and picker-only Drive reads, authorized first.
use crate::app::AppCtx;
use crate::concerns::{self, Before, before_actions, require_current_user};
use crate::controllers::{
    message_features as features,
    presenters::{accounts, page, storage_error},
};
use campfire_db::{User, room_files};
use campfire_kit::{Ctx, Error, Result, StatusCode, format, halt};
use campfire_storage::{Blob, paths};
fn string(c: &Ctx, key: &str) -> String {
    c.param(key).map(features::param_string).unwrap_or_default()
}
fn message_path(room_id: i64, id: i64, thread: Option<i64>) -> String {
    thread.map_or_else(
        || format!("/rooms/{room_id}/@{id}"),
        |t| format!("/rooms/{room_id}?message_id={id}&thread={t}"),
    )
}
pub async fn index(c: &mut Ctx) -> Result {
    match render(c).await {
        Err(Error::NotFound) => halt(concerns::head(StatusCode::NOT_FOUND)),
        result => result,
    }
}
async fn render(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = features::room(c).await?;
    let viewer = require_current_user(c)?.id;
    c.respond_to(&[&format::HTML])?;
    let file_type = room_files::file_type(&string(c, "type")).to_string();
    let filename = campfire_richtext::ruby::strip(&string(c, "filename")).to_string();
    let upload_page = room_files::page(&string(c, "page"));
    let drive_page = room_files::page(&string(c, "drive_page"));
    let raw_page = c
        .param("page")
        .filter(|v| !v.is_null())
        .map(features::param_string);
    let raw_drive_page = c
        .param("drive_page")
        .filter(|v| !v.is_null())
        .map(features::param_string);
    let storage = c.app().storage.clone();
    let list = c
        .app()
        .db
        .read(move |conn| {
            let mut uploads =
                room_files::uploads(conn, room.id, &file_type, &filename, upload_page * 30)?;
            let mut drives = room_files::drives(conn, room.id, drive_page * 30)?;
            let more_uploads = uploads.len() as i64 > upload_page * 30;
            let more_drive = drives.len() as i64 > drive_page * 30;
            uploads.truncate((upload_page * 30) as usize);
            drives.truncate((drive_page * 30) as usize);
            let blobs =
                Blob::find_many(conn, &uploads.iter().map(|a| a.blob_id).collect::<Vec<_>>())
                    .map_err(storage_error)?;
            let uploads = uploads
                .into_iter()
                .map(|a| {
                    let b = blobs
                        .get(&a.blob_id)
                        .ok_or(campfire_db::Error::RecordNotFound("Blob"))?;
                    Ok(campfire_views::room_files::Upload {
                        filename: b.filename.to_string(),
                        download_path: paths::blob_redirect_path(
                            &*storage.verifier,
                            b,
                            Some("attachment"),
                        ),
                        byte_size: b.byte_size,
                        content_type: b.content_type.clone().unwrap_or_default(),
                        creator_name: a.creator_name,
                        created_at: a.created_at.jiff(),
                        message_path: message_path(room.id, a.message_id, a.thread_id),
                    })
                })
                .collect::<campfire_db::Result<Vec<_>>>()?;
            let drives = drives
                .into_iter()
                .map(|a| campfire_views::room_files::Drive {
                    url: format!("https://drive.google.com/open?id={}", a.file_id),
                    creator_name: a.creator_name,
                    created_at: a.created_at.jiff(),
                    message_path: message_path(room.id, a.message_id, a.thread_id),
                })
                .collect();
            Ok(campfire_views::room_files::Listing {
                room_id: room.id,
                room_name: accounts::room_display_name(conn, &room, &User::find(conn, viewer)?)?,
                file_type,
                filename,
                upload_page,
                drive_page,
                raw_page,
                raw_drive_page,
                uploads,
                drives,
                more_uploads,
                more_drive,
            })
        })
        .await
        .map_err(page::db_error)?;
    page::framed_page!(c, StatusCode::OK, |ctx| campfire_views::room_files::Index {
        ctx,
        list: &list
    })
    .await
}
