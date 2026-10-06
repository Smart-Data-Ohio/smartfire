//! Message forwards and their private source resolver; mutations use WS8a's forwarder/copier.
use campfire_db::{ChannelThread, Message, Room, Timestamp, models::forwarder::{self, Destination}};
use campfire_kit::{Ctx, Error, Param, ParamMap, Permit, Result, StatusCode, format, permit_keys};
use serde_json::{Value, json};
use crate::{app::AppCtx, concerns::{before_actions, Before, cast_integer, require_current_user}, controllers::{messages, presenters::page::db_error}, messaging::ForwarderCopier};

async fn source(c: &mut Ctx) -> Result<Message> {
    let user = require_current_user(c)?.id;
    let id = c.params.get("message_id").or_else(|| c.params.get("id")).and_then(Param::to_s).as_deref().and_then(cast_integer).ok_or(Error::NotFound)?;
    let room = c.params.get("room_id").filter(|v| v.is_present()).map(|v| v.to_s().as_deref().and_then(cast_integer).ok_or(Error::NotFound)).transpose()?;
    let thread = c.params.get("thread_id").filter(|v| v.is_present()).map(|v| v.to_s().as_deref().and_then(cast_integer).ok_or(Error::NotFound)).transpose()?;
    c.app().db.read(move |conn| {
        let message = Message::find_reachable(conn, user, id)?;
        if let Some(room) = room {
            if message.room_id != room || message.thread_id != thread { return Err(campfire_db::Error::RecordNotFound("Message")); }
            if let Some(thread) = thread && ChannelThread::find(conn, thread)?.room_id != room { return Err(campfire_db::Error::RecordNotFound("ChannelThread")); }
        }
        Ok(message)
    }).await.map_err(db_error)
}

fn payload(c: &Ctx) -> Result<ParamMap> {
    let source = c.params.get("forward").filter(|v| v.is_present()).map(Param::as_hash).unwrap_or(Some(&c.params))
        .ok_or_else(|| Error::internal(anyhow::anyhow!("forward parameters do not support indexing")))?;
    Ok(source.permit(&[Permit::Key("note".into()), Permit::Nested("destinations".into(), permit_keys(&["room_id", "thread_id"]))]))
}
fn optional_id(value: Option<&Param>) -> Option<i64> { value.filter(|v| v.is_present()).and_then(Param::to_s).as_deref().map(|id| cast_integer(id).unwrap_or(0)) }
fn destinations_from(data: &ParamMap) -> Result<Vec<Destination>> {
    let Some(value) = data.get("destinations") else {return Ok(Vec::new())};
    let Param::Array(values) = value else {return Err(Error::internal(anyhow::anyhow!("destination does not support to_h")))};
    Ok(values.iter().map(|value| Destination {room_id: optional_id(value.get("room_id")), thread_id: optional_id(value.get("thread_id"))}).collect())
}
fn render_json(c: &mut Ctx, status: StatusCode, value: &Value) -> Result { Ok(c.render(status, &format::JSON, serde_json::to_string(value).map_err(Error::internal)?)) }
fn render_error(c: &mut Ctx, message: &str) -> Result {
    if c.format()? == Some(&format::JSON) {render_json(c, StatusCode::UNPROCESSABLE_ENTITY, &json!({"error": message}))}
    else {Ok(c.head(StatusCode::UNPROCESSABLE_ENTITY))}
}

pub async fn create(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let original = source(c).await?;
    let data = payload(c)?;
    let destinations = destinations_from(&data)?;
    let note = data.get("note").and_then(Param::to_s).filter(|value| !campfire_views::helpers::is_blank(value));
    let copier = ForwarderCopier::new(c.app().storage.clone());
    let creator = require_current_user(c)?.id;
    let snapshot = original.clone();
    // Test-only input boundary for the Rails fixture's Random.uuid provider. Production
    // requests always use the domain's random IDs and never recognize this header.
    #[cfg(any(test, feature = "test-support"))]
    let fixture_ids = c.request.header("x-ws8bm-forward-client-ids").map(|ids| ids.split(',').map(str::to_string).collect::<Vec<_>>());
    let results = c.app().db.write(move |tx| {
        // Source authorization is rechecked alongside all destination writes.
        Message::find_reachable(tx.conn(), creator, snapshot.id)?;
        #[cfg(any(test, feature = "test-support"))]
        if let Some(ids) = fixture_ids {
            let mut ids = ids.into_iter();
            return forwarder::forward_with_client_ids(tx, &snapshot, &destinations, note.as_deref(), creator, &copier,
                &mut || ids.next().expect("forward fixture ID per destination"));
        }
        forwarder::forward(tx, &snapshot, &destinations, note.as_deref(), creator, &copier)
    }).await;
    let results = match results {
        Ok(Ok(results)) => results,
        Ok(Err(refusal)) => return render_error(c, &refusal.to_string()),
        Err(campfire_db::Error::RecordInvalid(errors)) => return render_error(c, &campfire_views::helpers::to_sentence(&errors.full_messages(), " and ")),
        Err(campfire_db::Error::Other(message)) if ["You cannot forward to that room", "That thread is locked"].contains(&message.as_str()) => return render_error(c, &message),
        Err(error) => return Err(db_error(error)),
    };
    let mut rows = Vec::new();
    for result in results {
        let id = result.message.id;
        let record = c.app().db.read(move |conn| Message::find(conn, id)).await.map_err(db_error)?;
        messages::broadcast_create(c, &result.room, &record).await?;
        rows.push((result.room.id, result.thread.map(|thread| thread.id), record));
    }
    c.no_store();
    if *c.respond_to(&[&format::HTML, &format::JSON])? == format::HTML {
        return c.redirect_to(&c.url_for(&campfire_db::message_pin::message_path(&original)));
    }
    let viewer = require_current_user(c)?.clone();
    let base = c.url_for("");
    let result = messages::present(c, move |p| Ok(json!({"forwards": rows.iter().map(|(room, thread, message)| Ok(json!({"destination": {"room_id": room, "thread_id": thread},
        "message": messages::payload::message(p, message, &viewer, &base)?}))).collect::<campfire_db::Result<Vec<_>>>()?}))).await?;
    render_json(c, StatusCode::CREATED, &result)
}

pub async fn destinations(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    source(c).await?;
    let viewer = require_current_user(c)?.clone();
    let now = Timestamp::from_jiff(c.now());
    let result = c.app().db.read(move |conn| {
        let mut rooms = Room::for_user(conn, viewer.id)?;
        rooms.sort_by_cached_key(|room| room.name.as_ref().map(|name| name.to_ascii_lowercase()));
        let ids = rooms.iter().map(|room| room.id).collect::<Vec<_>>();
        let threads = ChannelThread::for_rooms(conn, &ids)?;
        let mut direct_names = std::collections::HashMap::<i64, Vec<String>>::new();
        // One member query for every reachable direct; no per-room/thread/user lookup.
        let mut statement = conn.prepare("SELECT memberships.room_id, users.id, users.name FROM memberships INNER JOIN users ON users.id = memberships.user_id INNER JOIN rooms ON rooms.id = memberships.room_id WHERE rooms.type = 'Rooms::Direct' AND rooms.id IN (SELECT room_id FROM memberships WHERE user_id = ?) ORDER BY memberships.id")?;
        for row in statement.query_map([viewer.id], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?)))? {
            let (room, user, name) = row?;
            if user != viewer.id { direct_names.entry(room).or_default().push(name); }
        }
        let rows = rooms.iter().filter(|room| !room.board()).map(|room| {
            let name = if room.direct() {
                let name = campfire_views::helpers::to_sentence(direct_names.get(&room.id).map_or(&[], Vec::as_slice), " and ");
                if name.is_empty() {viewer.name.clone()} else {name}
            } else {room.name.clone().unwrap_or_default()};
            let threads = if room.direct() {Vec::new()} else {threads.iter().filter(|thread| thread.room_id == room.id && thread.locked_at.is_none())
                .map(|thread| json!({"id": thread.id, "name": thread.name, "status": thread.status_in_room(room, now).name()})).collect()};
            Ok(json!({"room_id": room.id, "name": name, "direct": room.direct(), "threads": threads}))
        }).collect::<campfire_db::Result<Vec<_>>>()?;
        Ok(json!({"destinations": rows}))
    }).await.map_err(db_error)?;
    c.no_store();
    render_json(c, StatusCode::OK, &result)
}

pub async fn forward_source(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let message = source(c).await?;
    let viewer = require_current_user(c)?.id;
    let base = c.url_for("");
    let value = c.app().db.read(move |conn| {
        let source = match message.forwarded_from_message_id {
            None => None,
            Some(id) => match Message::find_reachable(conn, viewer, id) { Ok(source) => Some(source), Err(campfire_db::Error::RecordNotFound(_)) => None, Err(error) => return Err(error) },
        };
        Ok(json!({"source": source.map(|source| json!({"url": format!("{base}{}", campfire_db::message_pin::message_path(&source))}))}))
    }).await.map_err(db_error)?;
    c.no_store();
    render_json(c, StatusCode::OK, &value)
}
