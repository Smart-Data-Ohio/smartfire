//! Admin-only, file-based workspace structure transfer. No stored template or migration.
use crate::{
    admin::administrator,
    error::{fail, validation},
    room_management::{broadcast, create_in_transaction},
};
use axum::{
    Router,
    routing::{get, post},
};
use campfire_api_types as api;
use campfire_app::app::{AppCtx, AppState};
use campfire_db::models::{
    announcement,
    audit_log::{AuditLog, Context, NewAuditLog},
    workspace_category::WorkspaceCategory,
};
use campfire_db::{BoardTag, BoardTagPolicy, Connection, Room, RoomType, Tx};
use campfire_kit::{Ctx, Error, Kit, Result, StatusCode, action, unparsed_action};
use campfire_runtime::context::db_error;
use serde::de::DeserializeOwned;
use std::collections::BTreeSet;

pub const FILE_LIMIT: usize = 1024 * 1024;
// JSON escaping plus the confirmed structure can expand a one-MiB file in the request envelope.
const REQUEST_LIMIT: usize = FILE_LIMIT * 8;
const CATEGORY_LIMIT: usize = 100;
const ROOM_LIMIT: usize = 1000;

pub fn routes() -> Router<Kit> {
    Router::new()
        .route("/api/v1/admin/templates/export", get(action(export)))
        .route(
            "/api/v1/admin/templates/preview",
            post(unparsed_action(preview)),
        )
        .route(
            "/api/v1/admin/templates/import",
            post(unparsed_action(import)),
        )
}

endpoint!(export => export_file);
endpoint!(preview => preview_file);
endpoint!(import => import_file);

async fn upload<T: DeserializeOwned>(c: &mut Ctx) -> Result<T> {
    let bytes = c.read_body(REQUEST_LIMIT + 1).await;
    if bytes.len() > REQUEST_LIMIT {
        return Err(fail(c, validation("file", "Template request is too large")));
    }
    serde_json::from_slice(&bytes).map_err(|e| {
        fail(
            c,
            validation("file", &format!("Malformed template upload: {e}")),
        )
    })
}

async fn export_file(c: &mut Ctx) -> Result {
    administrator(c).await?;
    let document = c
        .app()
        .db
        .read_snapshot(|conn| export_structure(conn))
        .await
        .map_err(db_error)?;
    let bytes = serde_json::to_vec_pretty(&document).map_err(Error::internal)?;
    let count = document.rooms.len()
        + document
            .categories
            .iter()
            .map(|c| c.rooms.len())
            .sum::<usize>();
    if document.categories.len() > CATEGORY_LIMIT || count > ROOM_LIMIT || bytes.len() > FILE_LIMIT
    {
        return Err(fail(
            c,
            validation(
                "file",
                "Workspace structure exceeds the template limits (1 MiB, 100 categories, 1000 rooms)",
            ),
        ));
    }
    c.set_header(
        "content-disposition",
        "attachment; filename=\"smartfire-template-v1.json\"",
    );
    c.set_header("x-content-type-options", "nosniff");
    Ok(c.render_as(StatusCode::OK, "application/json", bytes))
}

async fn preview_file(c: &mut Ctx) -> Result {
    administrator(c).await?;
    let input: api::TemplateUpload = upload(c).await?;
    let result = c
        .app()
        .db
        .read_snapshot(move |conn| plan(conn, &input.contents))
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &result)
}

async fn import_file(c: &mut Ctx) -> Result {
    let user = administrator(c).await?;
    let input: api::TemplateImport = upload(c).await?;
    let app = c.app().clone();
    let audit = Context {
        actor: Some((&user).into()),
        ip_address: Some(c.request.remote_ip()?.to_string()),
        user_agent: c.request.user_agent().map(str::to_string),
    };
    let result = c
        .app()
        .db
        .write(move |tx| {
            let preview = plan(tx.conn(), &input.contents)?;
            if !preview.errors.is_empty() {
                return Ok((preview, None));
            }
            if preview.template.as_ref() != Some(&input.expected) {
                return Ok((preview, None));
            }
            let rooms = create_structure(tx, &app, &input.expected, user.id, &audit)?;
            Ok((preview, Some(rooms)))
        })
        .await
        .map_err(db_error)?;
    let (preview, rooms) = result;
    if !preview.errors.is_empty() {
        return Err(fail(c, validation("file", &preview.errors.join("; "))));
    }
    let Some(rooms) = rooms else {
        return Err(fail(
            c,
            api::ApiError::Conflict {
                message: "Workspace structure changed. Preview this file again before importing."
                    .into(),
            },
        ));
    };
    for room in rooms {
        broadcast(c, &room, false).await?;
    }
    c.json(StatusCode::CREATED, &preview)
}

fn kind(kind: api::TemplateRoomKind) -> RoomType {
    match kind {
        api::TemplateRoomKind::Open => RoomType::Open,
        api::TemplateRoomKind::Closed => RoomType::Closed,
        api::TemplateRoomKind::Voice => RoomType::Voice,
        api::TemplateRoomKind::Stage => RoomType::Stage,
        api::TemplateRoomKind::Board => RoomType::Board,
    }
}

fn export_room(conn: &Connection, id: i64) -> campfire_db::Result<api::TemplateRoom> {
    let room = Room::find(conn, id)?;
    let room_kind = match room.room_type {
        RoomType::Open => api::TemplateRoomKind::Open,
        RoomType::Closed => api::TemplateRoomKind::Closed,
        RoomType::Voice => api::TemplateRoomKind::Voice,
        RoomType::Stage => api::TemplateRoomKind::Stage,
        RoomType::Board => api::TemplateRoomKind::Board,
        RoomType::Direct => return Err(campfire_db::Error::RecordNotFound("TemplateRoom")),
    };
    let board = if room.board() {
        let tags = BoardTag::for_room(conn, id)?;
        let policy = BoardTagPolicy::for_room(conn, id)?;
        Some(api::TemplateBoard {
            default_tag: tags
                .iter()
                .position(|tag| Some(tag.id) == policy.default_board_tag_id)
                .map(|i| i as u32),
            tags_required: policy.tags_required,
            tags: tags
                .into_iter()
                .map(|tag| api::TemplateBoardTag {
                    name: tag.name,
                    emoji: tag.emoji,
                })
                .collect(),
        })
    } else {
        None
    };
    Ok(api::TemplateRoom {
        kind: room_kind,
        name: room.name,
        icon_name: room.icon_name,
        topic: room.topic,
        announcement: room.announcement,
        voice_bitrate: room.voice_bitrate,
        voice_user_limit: room.voice_user_limit,
        board,
    })
}

fn export_group(conn: &Connection, id: Option<i64>) -> campfire_db::Result<Vec<api::TemplateRoom>> {
    WorkspaceCategory::rooms(conn, id)?
        .into_iter()
        .map(|row| export_room(conn, row.room_id))
        .collect()
}

fn export_structure(conn: &Connection) -> campfire_db::Result<api::WorkspaceTemplate> {
    Ok(api::WorkspaceTemplate {
        format: api::TemplateFormat::SmartfireTemplate,
        version: 1,
        categories: WorkspaceCategory::ordered(conn)?
            .into_iter()
            .map(|category| {
                Ok(api::TemplateCategory {
                    rooms: export_group(conn, Some(category.id))?,
                    name: category.name,
                })
            })
            .collect::<campfire_db::Result<_>>()?,
        rooms: export_group(conn, None)?,
    })
}

fn invalid_preview(message: String) -> api::TemplatePreview {
    api::TemplatePreview {
        template: None,
        changes: vec![],
        errors: vec![message],
        category_count: 0,
        room_count: 0,
    }
}

fn unique_name(
    name: Option<&str>,
    names: &mut BTreeSet<Option<String>>,
    limit: Option<usize>,
) -> Option<String> {
    let original = name.map(str::to_string);
    if names.insert(original.clone()) {
        return original;
    }
    for number in 2.. {
        let suffix = format!(" ({number})");
        let base = name.unwrap_or("Unnamed room");
        let base: String = base
            .chars()
            .take(limit.map_or(usize::MAX, |n| n.saturating_sub(suffix.chars().count())))
            .collect();
        let candidate = Some(format!("{base}{suffix}"));
        if names.insert(candidate.clone()) {
            return candidate;
        }
    }
    unreachable!()
}

fn plan(conn: &Connection, contents: &str) -> campfire_db::Result<api::TemplatePreview> {
    if contents.len() > FILE_LIMIT {
        return Ok(invalid_preview(
            "Template files must be at most 1 MiB.".into(),
        ));
    }
    let mut document: api::WorkspaceTemplate = match serde_json::from_str(contents) {
        Ok(document) => document,
        Err(e) => return Ok(invalid_preview(format!("Invalid template JSON: {e}"))),
    };
    if document.version != 1 {
        return Ok(invalid_preview(format!(
            "Unsupported template version {}. Expected version 1.",
            document.version
        )));
    }
    let room_count = document.rooms.len()
        + document
            .categories
            .iter()
            .map(|c| c.rooms.len())
            .sum::<usize>();
    if document.categories.len() > CATEGORY_LIMIT || room_count > ROOM_LIMIT {
        return Ok(invalid_preview(
            "Templates allow at most 100 categories and 1000 rooms.".into(),
        ));
    }
    let mut result = api::TemplatePreview {
        template: None,
        changes: vec![],
        errors: vec![],
        category_count: document.categories.len() as u32,
        room_count: room_count as u32,
    };
    let mut category_names = WorkspaceCategory::ordered(conn)?
        .into_iter()
        .map(|c| Some(c.name))
        .collect();
    let mut room_names = std::collections::BTreeSet::new();
    // All alive non-direct rooms, including private rooms this administrator hasn't joined.
    let mut stmt = conn
        .prepare("SELECT name FROM rooms WHERE type != 'Rooms::Direct' AND deleted_at IS NULL")?;
    for name in stmt.query_map([], |row| row.get::<_, Option<String>>(0))? {
        room_names.insert(name?);
    }
    for category in &mut document.categories {
        if category.name.trim().is_empty() || category.name.chars().count() > 50 {
            result.errors.push(format!(
                "Category {:?}: name must contain 1 to 50 characters.",
                category.name
            ));
        }
        let created_name = unique_name(Some(&category.name), &mut category_names, Some(50));
        result.changes.push(api::TemplateChange {
            entity: api::TemplateEntity::Category,
            name: Some(category.name.clone()),
            created_name: created_name.clone(),
        });
        category.name = created_name.unwrap_or_default();
        for room in &mut category.rooms {
            plan_room(conn, room, &mut room_names, &mut result)?;
        }
    }
    for room in &mut document.rooms {
        plan_room(conn, room, &mut room_names, &mut result)?;
    }
    result.template = Some(document);
    Ok(result)
}

fn plan_room(
    conn: &Connection,
    room: &mut api::TemplateRoom,
    names: &mut BTreeSet<Option<String>>,
    result: &mut api::TemplatePreview,
) -> campfire_db::Result<()> {
    let label = room.name.as_deref().unwrap_or("Unnamed room");
    for error in [
        Room::normalize_topic(room.topic.as_deref()).map(|_| ()),
        Room::validate_voice_bitrate(room.voice_bitrate),
        Room::validate_voice_user_limit(room.voice_user_limit),
    ]
    .into_iter()
    .filter_map(std::result::Result::err)
    {
        result.errors.push(format!("Room {label:?}: {error}"));
    }
    room.icon_name = Room::normalize_icon_name(room.icon_name.as_deref());
    if let Some(icon) = &room.icon_name {
        let resolved = campfire_runtime::rich_text::room_icon_resolves(conn, icon)?;
        if !resolved {
            result.errors.push(format!(
                "Room {label:?}: icon {icon:?} is not available in this workspace."
            ));
        }
    }
    if (room.kind == api::TemplateRoomKind::Board) != room.board.is_some() {
        result.errors.push(format!(
            "Room {label:?}: board settings must be present only for board rooms."
        ));
    }
    if let Some(board) = &room.board {
        if board.tags.len() > 20
            || (board.tags_required && board.tags.is_empty())
            || board
                .default_tag
                .is_some_and(|i| i as usize >= board.tags.len())
        {
            result.errors.push(format!(
                "Room {label:?}: invalid board tag count, required tags or default tag index."
            ));
        }
        let mut prior: Vec<&str> = vec![];
        for tag in &board.tags {
            let name = tag.name.trim();
            if name.is_empty()
                || name.chars().count() > 20
                || prior
                    .iter()
                    .any(|other| caseless::default_caseless_match_str(other, name))
            {
                result.errors.push(format!(
                    "Room {label:?}: board tag names must be unique and contain 1 to 20 characters."
                ));
            }
            prior.push(name);
        }
    }
    let created_name = unique_name(room.name.as_deref(), names, None);
    result.changes.push(api::TemplateChange {
        entity: api::TemplateEntity::Room,
        name: room.name.clone(),
        created_name: created_name.clone(),
    });
    room.name = created_name;
    // Match the existing settings and creation paths' canonical values in the preview.
    room.topic =
        Room::normalize_topic(room.topic.as_deref()).unwrap_or_else(|_| room.topic.clone());
    if let Some(board) = &mut room.board {
        for tag in &mut board.tags {
            tag.name = tag.name.trim().into();
        }
    }
    Ok(())
}

fn create_structure(
    tx: &mut Tx<'_>,
    app: &AppState,
    document: &api::WorkspaceTemplate,
    creator: i64,
    audit: &Context,
) -> campfire_db::Result<Vec<Room>> {
    let mut rooms = vec![];
    for category in &document.categories {
        let category = (
            WorkspaceCategory::create(tx, &category.name)?,
            &category.rooms,
        );
        for (position, input) in category.1.iter().enumerate() {
            let room = create_room(tx, app, input, creator, audit)?;
            WorkspaceCategory::move_room(tx, room.id, Some(category.0.id), position as i64)?;
            rooms.push(room);
        }
    }
    let offset = WorkspaceCategory::rooms(tx.conn(), None)?.len();
    for (position, input) in document.rooms.iter().enumerate() {
        let room = create_room(tx, app, input, creator, audit)?;
        WorkspaceCategory::move_room(tx, room.id, None, (offset + position) as i64)?;
        rooms.push(room);
    }
    Ok(rooms)
}

fn create_room(
    tx: &mut Tx<'_>,
    app: &AppState,
    input: &api::TemplateRoom,
    creator: i64,
    audit: &Context,
) -> campfire_db::Result<Room> {
    let mut room = create_in_transaction(
        tx,
        app,
        kind(input.kind),
        input.name.as_deref(),
        input.icon_name.as_deref(),
        creator,
        &[creator],
    )?;
    room.update_topic(tx, input.topic.as_deref())?;
    room.update_voice_bitrate(tx, input.voice_bitrate)?;
    room.update_voice_user_limit(tx, input.voice_user_limit)?;
    room = announcement::set_flag(tx, room.id, input.announcement)?;
    if let Some(board) = &input.board {
        let tags = board
            .tags
            .iter()
            .map(|tag| BoardTag::create(tx, room.id, &tag.name, tag.emoji.as_deref()))
            .collect::<campfire_db::Result<Vec<_>>>()?;
        BoardTagPolicy::update(
            tx,
            room.id,
            board.tags_required,
            board.default_tag.map(|index| tags[index as usize].id),
        )?;
    }
    AuditLog::record(
        tx,
        NewAuditLog {
            action: "room.create".into(),
            target: Some((&room).into()),
            changes: Some(serde_json::json!({"name":room.name})),
            ..Default::default()
        },
        audit,
    )?;
    Ok(room)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn template_conflicts_reserve_names_in_order_and_keep_category_length() {
        let mut names = BTreeSet::from([Some("General".into()), Some("General (2)".into())]);
        assert_eq!(
            unique_name(Some("General"), &mut names, None),
            Some("General (3)".into())
        );
        assert_eq!(
            unique_name(Some("General"), &mut names, None),
            Some("General (4)".into())
        );
        let long = "é".repeat(50);
        let mut names = BTreeSet::from([Some(long.clone())]);
        assert_eq!(
            unique_name(Some(&long), &mut names, Some(50))
                .unwrap()
                .chars()
                .count(),
            50
        );
    }
}
