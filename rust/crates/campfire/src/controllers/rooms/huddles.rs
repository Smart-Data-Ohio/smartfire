//! `Rooms::HuddlesController` and `Users::HuddlePresenceController`.
use crate::controllers::presenters::{self, page::db_error};
use crate::{
    app::AppCtx,
    concerns::{self, AuthenticatedBy, Before},
};
use campfire_db::models::{
    huddle_grant::{HuddleGrant, IN_CALL_WINDOW},
    room_delete::HuddleConfig,
};
use campfire_db::{Membership, Room, Timestamp, User};
use campfire_kit::{Ctx, Error, Result, StatusCode, halt};
use rails_compat::{jwt::livekit, unicode};
use serde_json::{Value, json};

type PresentParticipants = Vec<(User, Vec<String>)>;

fn error(c: &mut Ctx, status: StatusCode, message: &str) -> Result<()> {
    halt(c.json(status, &json!({"error":message}))?)
}
fn request_authentication(c: &mut Ctx) -> Result<()> {
    error(c, StatusCode::UNAUTHORIZED, "Authentication required")
}
fn deny_bots(c: &mut Ctx) -> Result<()> {
    if concerns::authenticated_by(c) == AuthenticatedBy::BotKey {
        error(c, StatusCode::FORBIDDEN, "Bots cannot join huddles")?;
    }
    Ok(())
}
async fn before(c: &mut Ctx) -> Result<()> {
    // `prepend_before_action :prevent_caching` also covers authentication failures.
    c.set_header("cache-control", "no-store");
    concerns::before_actions_with_authentication(
        c,
        Before::default(),
        Some(request_authentication),
        Some(deny_bots),
    )
    .await?;
    if !c.app().config.huddle.configured() {
        error(
            c,
            StatusCode::SERVICE_UNAVAILABLE,
            "Huddles are not configured",
        )?;
    }
    if concerns::current_user(c).is_some_and(User::is_bot) {
        error(c, StatusCode::FORBIDDEN, "Bots cannot join huddles")?;
    }
    if !concerns::current_user(c).is_some_and(User::is_active) {
        error(c, StatusCode::FORBIDDEN, "User cannot join huddles")?;
    }
    Ok(())
}
async fn set_room(c: &mut Ctx) -> Result<(Membership, Room)> {
    match concerns::set_room(c).await {
        Ok((membership, room)) if !room.deleted() => Ok((membership, room)),
        Ok(_) | Err(Error::NotFound) => {
            error(c, StatusCode::NOT_FOUND, "Room not found or inaccessible")?;
            unreachable!()
        }
        Err(error) => Err(error),
    }
}
async fn room_json(c: &Ctx, room: &Room) -> Result<Value> {
    let room = room.clone();
    let user = concerns::require_current_user(c)?.clone();
    c.app()
        .db
        .read(move |conn| {
            let name = if room.direct() {
                room.direct_display_name(conn, Some(&user), None)?
            } else {
                room.name.clone()
            };
            Ok(json!({"id":room.id,"name":name}))
        })
        .await
        .map_err(db_error)
}
pub async fn show(c: &mut Ctx) -> Result {
    before(c).await?;
    let (_, room) = set_room(c).await?;
    c.json(StatusCode::OK, &json!({"room":room_json(c, &room).await?}))
}
pub async fn create(c: &mut Ctx) -> Result {
    before(c).await?;
    let (membership, room) = set_room(c).await?;
    let config = c.app().config.huddle.clone();
    let domain = HuddleConfig {
        api_secret: config.api_secret.clone(),
        admin_configured: config.admin_configured(),
    };
    let session_id = concerns::current_session(c)
        .ok_or_else(|| Error::internal(anyhow::anyhow!("no Current.session")))?
        .id;
    let room_id = room.id;
    let grant = match c
        .app()
        .db
        .write(move |tx| HuddleGrant::issue(tx, session_id, membership.id, room_id, &domain))
        .await
    {
        Ok(grant) => grant,
        Err(campfire_db::Error::Other(message)) if message == "HuddleGrant::Ineligible" => {
            error(c, StatusCode::NOT_FOUND, "Room not found or inaccessible")?;
            unreachable!()
        }
        Err(error) => return Err(db_error(error)),
    };
    let token = livekit::participant_token(
        config.api_key.as_deref().unwrap(),
        config.api_secret.as_deref().unwrap(),
        &livekit::Participant {
            name: &concerns::require_current_user(c)?.name,
            identity: &grant.identity,
            room_name: &grant.room_name,
            can_publish: livekit::can_publish(
                grant.server_muted,
                room.room_type == campfire_db::RoomType::Stage,
                grant.stage_role.as_deref(),
            ),
        },
        c.now().as_second(),
    );
    c.json(StatusCode::OK, &json!({"url":config.public_url,"token":token,"room":room_json(c,&room).await?,"identity":grant.identity,"grant_id":grant.id}))
}
fn participant(c: &Ctx, user: &User, identities: Vec<String>) -> Value {
    json!({"id":user.id,"name":user.name,"avatar_url":c.url_for(&presenters::avatar_path(&c.app().secrets,user)),"identities":identities})
}
pub async fn participants(c: &mut Ctx) -> Result {
    before(c).await?;
    let (_, room) = set_room(c).await?;
    let now = Timestamp::from_jiff(c.now());
    let (users, mut identities) = c
        .app()
        .db
        .read(move |conn| {
            Ok((
                HuddleGrant::participants_for(conn, room.id, now)?,
                HuddleGrant::participant_identities_for(conn, room.id, now)?,
            ))
        })
        .await
        .map_err(db_error)?;
    let users: Vec<_> = users
        .iter()
        .map(|user| participant(c, user, identities.remove(&user.id).unwrap_or_default()))
        .collect();
    c.json(StatusCode::OK, &users)
}
pub async fn leave(c: &mut Ctx) -> Result {
    before(c).await?;
    let (_, room) = set_room(c).await?;
    let session_id = concerns::current_session(c)
        .ok_or_else(|| Error::internal(anyhow::anyhow!("no Current.session")))?
        .id;
    c.app().db.write(move |tx| {
        let ids: Vec<i64> = tx.conn().prepare_cached("SELECT id FROM huddle_grants WHERE session_id=? AND room_id=? AND revoked_at IS NULL ORDER BY id")?.query_map(rusqlite::params![session_id,room.id], |row| row.get(0))?.collect::<rusqlite::Result<_>>()?;
        for id in ids {
            if let Some(mut grant) = HuddleGrant::find_by_id(tx.conn(),id)? {
                grant.mark_out_of_call(tx,None)?;
            }
        }
        Ok(())
    }).await.map_err(db_error)?;
    Ok(c.head(StatusCode::NO_CONTENT))
}
pub async fn presence(c: &mut Ctx) -> Result {
    before(c).await?;
    let user_id = concerns::require_current_user(c)?.id;
    let now = Timestamp::from_jiff(c.now());
    let rooms = c.app().db.read(move |conn| {
        // One grants query and one batched user preload, regardless of live room count.
        let grants: Vec<(i64,i64,String)> = conn.prepare_cached("SELECT room_id,user_id,identity FROM huddle_grants WHERE revoked_at IS NULL AND last_seen_at>? AND room_id IN (SELECT room_id FROM memberships WHERE user_id=?) ORDER BY id")?.query_map(rusqlite::params![now.ago(jiff::SignedDuration::from_secs(IN_CALL_WINDOW)),user_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?.collect::<rusqlite::Result<_>>()?;
        let ids: Vec<_> = grants.iter().map(|(_,id,_)| *id).collect();
        let users = User::where_ids(conn,&ids)?;
        let mut rooms: Vec<(i64, PresentParticipants)> = Vec::new();
        for (room_id,user_id,identity) in grants {
            let Some(user) = users.iter().find(|user| user.id == user_id) else { continue };
            let index = rooms.iter().position(|(id,_)| *id == room_id).unwrap_or_else(|| { rooms.push((room_id,Vec::new())); rooms.len()-1 });
            let participants = &mut rooms[index].1;
            if let Some((_,identities)) = participants.iter_mut().find(|(user,_)| user.id == user_id) {
                identities.push(identity);
            } else {
                participants.push((user.clone(),vec![identity]));
            }
        }
        for (_, participants) in &mut rooms { participants.sort_by_key(|(user,_)|unicode::downcase(&user.name)); }
        Ok(rooms)
    }).await.map_err(db_error)?;
    let rooms: Vec<_> = rooms.into_iter().map(|(room_id,users)| json!({"room_id":room_id,"participants":users.into_iter().map(|(user,identities)| participant(c,&user,identities)).collect::<Vec<_>>()})).collect();
    c.json(StatusCode::OK, &rooms)
}
