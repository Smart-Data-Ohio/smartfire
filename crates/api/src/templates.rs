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
    let viewer = administrator(c).await?.id;
    let document = c
        .app()
        .db
        .read_snapshot(move |conn| export_structure(conn, viewer))
        .await
        .map_err(db_error)?;
    if let Some(error) = size_error(&document) {
        return Err(fail(c, validation("file", &error)));
    }
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
    let document = parse_document(&input.contents)
        .map_err(|errors| fail(c, validation("file", &errors.join("; "))))?;
    let checked = document.clone();
    let preview = c
        .app()
        .db
        .read_snapshot(move |conn| preview_document(conn, checked))
        .await
        .map_err(db_error)?;
    if !preview.errors.is_empty() {
        return Err(fail(c, validation("file", &preview.errors.join("; "))));
    }
    if preview.template.as_ref() != Some(&input.expected) {
        return Err(stale_preview(c));
    }
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
            // Only the race-sensitive destination name allocation is repeated under the writer.
            let preview = resolve_names(tx.conn(), document)?;
            if preview.template.as_ref() != Some(&input.expected) {
                return Ok((preview, None));
            }
            let rooms = create_structure(tx, &app, &input.expected, user.id, &audit)?;
            Ok((preview, Some(rooms)))
        })
        .await
        .map_err(db_error)?;
    let (preview, rooms) = result;
    let Some(rooms) = rooms else {
        return Err(stale_preview(c));
    };
    for room in rooms {
        broadcast(c, &room, false).await?;
    }
    c.json(StatusCode::CREATED, &preview)
}

fn stale_preview(c: &mut Ctx) -> Error {
    fail(
        c,
        api::ApiError::Conflict {
            message: "Workspace structure changed. Preview this file again before importing."
                .into(),
        },
    )
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

fn export_group(
    conn: &Connection,
    id: Option<i64>,
    visible: &BTreeSet<i64>,
) -> campfire_db::Result<Vec<api::TemplateRoom>> {
    WorkspaceCategory::rooms(conn, id)?
        .into_iter()
        .filter(|row| visible.contains(&row.room_id))
        .map(|row| export_room(conn, row.room_id))
        .collect()
}

fn export_structure(conn: &Connection, viewer: i64) -> campfire_db::Result<api::WorkspaceTemplate> {
    // The existing membership scope and public-room scope also used by the room chooser.
    let visible = Room::for_user_without_directs(conn, viewer)?
        .into_iter()
        .chain(Room::of_type(conn, RoomType::Open)?)
        .map(|room| room.id)
        .collect();
    Ok(api::WorkspaceTemplate {
        format: api::TemplateFormat::SmartfireTemplate,
        version: 1,
        categories: WorkspaceCategory::ordered(conn)?
            .into_iter()
            .map(|category| {
                Ok(api::TemplateCategory {
                    rooms: export_group(conn, Some(category.id), &visible)?,
                    name: category.name,
                })
            })
            .collect::<campfire_db::Result<_>>()?,
        rooms: export_group(conn, None, &visible)?,
    })
}

fn invalid_preview(errors: Vec<String>) -> api::TemplatePreview {
    api::TemplatePreview {
        template: None,
        changes: vec![],
        errors,
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
    match parse_document(contents) {
        Ok(document) => preview_document(conn, document),
        Err(errors) => Ok(invalid_preview(errors)),
    }
}

fn parse_document(contents: &str) -> std::result::Result<api::WorkspaceTemplate, Vec<String>> {
    if contents.len() > FILE_LIMIT {
        return Err(vec!["Template files must be at most 1 MiB.".into()]);
    }
    let mut document: api::WorkspaceTemplate =
        serde_json::from_str(contents).map_err(|e| vec![format!("Invalid template JSON: {e}")])?;
    if document.version != 1 {
        return Err(vec![format!(
            "Unsupported template version {}. Expected version 1.",
            document.version
        )]);
    }
    if let Some(error) = size_error(&document) {
        return Err(vec![error]);
    }
    let mut errors = vec![];
    for category in &document.categories {
        if category.name.trim().is_empty() {
            errors.push(format!(
                "Category {:?}: name must contain 1 to 50 characters.",
                category.name
            ));
        }
    }
    for room in document
        .categories
        .iter_mut()
        .flat_map(|category| &mut category.rooms)
        .chain(&mut document.rooms)
    {
        validate_room(room, &mut errors);
    }
    if errors.is_empty() {
        Ok(document)
    } else {
        Err(errors)
    }
}

fn size_error(document: &api::WorkspaceTemplate) -> Option<String> {
    if document.categories.len() > CATEGORY_LIMIT {
        return Some("Templates allow at most 100 categories.".into());
    }
    let count = document.rooms.len()
        + document
            .categories
            .iter()
            .map(|category| category.rooms.len())
            .sum::<usize>();
    if count > ROOM_LIMIT {
        return Some("Templates allow at most 1000 rooms.".into());
    }
    for category in &document.categories {
        if category.name.chars().count() > 50 {
            return Some(format!(
                "Category {:?}: name must contain 1 to 50 characters.",
                category.name
            ));
        }
    }
    for room in document
        .categories
        .iter()
        .flat_map(|category| &category.rooms)
        .chain(&document.rooms)
    {
        let label = room.name.as_deref().unwrap_or("Unnamed room");
        if let Some(board) = &room.board {
            if board.tags.len() > campfire_db::models::thread_tag::BOARD_TAG_LIMIT {
                return Some(format!("Room {label:?}: board tags are limited to 20."));
            }
            for tag in &board.tags {
                if tag.name.trim().chars().count()
                    > campfire_db::models::thread_tag::BOARD_TAG_NAME_LIMIT
                {
                    return Some(format!(
                        "Room {label:?}: board tag names must contain 1 to 20 characters."
                    ));
                }
            }
        }
        if let Err(error) = Room::normalize_topic(room.topic.as_deref()) {
            return Some(format!("Room {label:?}: {error}"));
        }
    }
    None
}

fn validate_room(room: &mut api::TemplateRoom, errors: &mut Vec<String>) {
    let label = room.name.as_deref().unwrap_or("Unnamed room");
    for error in [
        Room::validate_voice_bitrate(room.voice_bitrate),
        Room::validate_voice_user_limit(room.voice_user_limit),
    ]
    .into_iter()
    .filter_map(std::result::Result::err)
    {
        errors.push(format!("Room {label:?}: {error}"));
    }
    if (room.kind == api::TemplateRoomKind::Board) != room.board.is_some() {
        errors.push(format!(
            "Room {label:?}: board settings must be present only for board rooms."
        ));
    }
    if let Some(board) = &mut room.board {
        if (board.tags_required && board.tags.is_empty())
            || board
                .default_tag
                .is_some_and(|i| i as usize >= board.tags.len())
        {
            errors.push(format!(
                "Room {label:?}: invalid required tags or default tag index."
            ));
        }
        let mut names = BTreeSet::new();
        for tag in &mut board.tags {
            tag.name = tag.name.trim().into();
            if tag.name.is_empty() || !names.insert(caseless::default_case_fold_str(&tag.name)) {
                errors.push(format!(
                    "Room {label:?}: board tag names must be unique and contain 1 to 20 characters."
                ));
            }
        }
    }
    room.icon_name = Room::normalize_icon_name(room.icon_name.as_deref());
    room.topic =
        Room::normalize_topic(room.topic.as_deref()).unwrap_or_else(|_| room.topic.clone());
}

fn preview_document(
    conn: &Connection,
    document: api::WorkspaceTemplate,
) -> campfire_db::Result<api::TemplatePreview> {
    let mut errors = vec![];
    for room in document
        .categories
        .iter()
        .flat_map(|category| &category.rooms)
        .chain(&document.rooms)
    {
        if let Some(icon) = &room.icon_name
            && !campfire_runtime::rich_text::room_icon_resolves(conn, icon)?
        {
            errors.push(format!(
                "Room {:?}: icon {icon:?} is not available in this workspace.",
                room.name.as_deref().unwrap_or("Unnamed room")
            ));
        }
    }
    let mut preview = resolve_names(conn, document)?;
    preview.errors = errors;
    Ok(preview)
}

fn resolve_names(
    conn: &Connection,
    mut document: api::WorkspaceTemplate,
) -> campfire_db::Result<api::TemplatePreview> {
    let room_count = document.rooms.len()
        + document
            .categories
            .iter()
            .map(|category| category.rooms.len())
            .sum::<usize>();
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
        let created_name = unique_name(Some(&category.name), &mut category_names, Some(50));
        result.changes.push(api::TemplateChange {
            entity: api::TemplateEntity::Category,
            name: Some(category.name.clone()),
            created_name: created_name.clone(),
        });
        category.name = created_name.unwrap_or_default();
        for room in &mut category.rooms {
            plan_room(room, &mut room_names, &mut result);
        }
    }
    for room in &mut document.rooms {
        plan_room(room, &mut room_names, &mut result);
    }
    result.template = Some(document);
    Ok(result)
}

fn plan_room(
    room: &mut api::TemplateRoom,
    names: &mut BTreeSet<Option<String>>,
    result: &mut api::TemplatePreview,
) {
    let created_name = unique_name(room.name.as_deref(), names, None);
    result.changes.push(api::TemplateChange {
        entity: api::TemplateEntity::Room,
        name: room.name.clone(),
        created_name: created_name.clone(),
    });
    room.name = created_name;
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
    let mut room = if input.kind == api::TemplateRoomKind::Open {
        let mut room = Room::create_for_atomically(
            tx,
            RoomType::Open,
            input.name.as_deref(),
            creator,
            &[creator],
        )?;
        room.update_with_icon(
            tx,
            None,
            None,
            Some(input.icon_name.as_deref()),
            campfire_runtime::rich_text::room_icon_resolves,
        )?;
        room
    } else {
        create_in_transaction(
            tx,
            app,
            kind(input.kind),
            input.name.as_deref(),
            input.icon_name.as_deref(),
            creator,
            &[creator],
        )?
    };
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
    #[test]
    fn template_oversize_catalog_is_rejected_before_database_reads() {
        let input = serde_json::json!({"format":"smartfire-template","version":1,"categories":[],"rooms":[{
            "kind":"board","name":"Large board","announcement":false,
            "board":{"tags":(0..20_000).map(|index| serde_json::json!({"name":format!("tag{index}"),"emoji":null})).collect::<Vec<_>>(),"tagsRequired":false,"defaultTag":null}
        }]});
        let contents = input.to_string();
        assert!(contents.len() < FILE_LIMIT);
        let conn = Connection::open_in_memory().unwrap();
        // No schema exists: an invalid file must not query workspace data at all.
        let result = plan(&conn, &contents).expect("Catalog limits must precede database reads");
        assert!(result.template.is_none());
        assert!(result.changes.is_empty());
        assert_eq!(
            result.errors,
            ["Room \"Large board\": board tags are limited to 20."]
        );
    }
}
