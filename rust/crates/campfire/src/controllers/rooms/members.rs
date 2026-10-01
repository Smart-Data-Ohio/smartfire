//! Rooms::MembersController: uncached member facts, supplied by their domain owners.
use crate::app::AppCtx;
use crate::concerns::{self, Before, before_actions, require_current_user};
use crate::controllers::presenters::{avatar_path, page::db_error};
use campfire_db::models::user_status_settings::UserStatusSettings;
use campfire_db::models::workspace_presence_lease::Presence;
use campfire_db::{Agent, WorkspacePresenceLease};
use campfire_kit::{Ctx, Error, Result, StatusCode, format};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashSet;

#[derive(Serialize)]
struct Member {
    id: i64,
    name: String,
    avatar_url: String,
    bot: bool,
    online: bool,
    presence: String,
    status: Option<String>,
    starred: bool,
}
#[derive(Serialize)]
struct Members { members: Vec<Member> }

pub async fn index(c: &mut Ctx) -> Result {
    // The Ruby override changes request_authentication only; the shared chain still restores
    // sessions, rejects bot/agent credentials and enforces two-factor in its normal order.
    match before_actions(c, Before::default()).await {
        Err(Error::Halt(response)) if response.status == StatusCode::FOUND
            && !concerns::signed_in(c) && c.format()? == Some(&format::JSON) =>
            return Ok(c.head(StatusCode::UNAUTHORIZED)),
        result => result?,
    }
    let (_, room) = match concerns::set_room(c).await {
        Err(Error::NotFound) => return Ok(c.head(StatusCode::NOT_FOUND)),
        result => result?,
    };
    let viewer_id = require_current_user(c)?.id;
    let base_url = c.url_for("");
    let app = c.app().clone();
    let members = c.app().db.read(move |conn| {
        let ids = conn.prepare_cached("SELECT users.id FROM users JOIN memberships ON memberships.user_id=users.id WHERE memberships.room_id=? AND users.status=0 ORDER BY LOWER(users.name),users.id")?
            .query_map([room.id], |row| row.get::<_,i64>(0))?
            .collect::<std::result::Result<Vec<_>,_>>()?;
        let mut settings = UserStatusSettings::for_ids(conn, &ids)?;
        let now = campfire_db::Timestamp::from_jiff(app.clock.now());
        let leases = WorkspacePresenceLease::presence_by_user_id(conn, &ids, now)?;
        let sql = format!("SELECT starred_user_id FROM user_stars WHERE user_id=? AND starred_user_id IN ({})",std::iter::repeat_n("?",ids.len()).collect::<Vec<_>>().join(","));
        let starred = conn.prepare_cached(&sql)?.query_map(rusqlite::params_from_iter(std::iter::once(&viewer_id).chain(ids.iter())),|row|row.get::<_,i64>(0))?
            .collect::<std::result::Result<HashSet<_>,_>>()?;
        ids.into_iter().map(|id| {
            let settings = settings.remove(&id).ok_or(campfire_db::Error::RecordNotFound("User"))?;
            let user = &settings.user;
            let agent = if user.is_bot() { Agent::for_user(conn,id)? } else { None };
            let (presence, online, status) = if let Some(agent) = agent {
                let live = agent.suspended_at.is_none() && agent.last_seen_at.is_some();
                let text = agent.working_presence_text(now).or_else(|| agent.status_note.as_deref().filter(|value|!campfire_richtext::ruby::is_blank(value)));
                let status = text.map(str::to_string).unwrap_or_else(|| {
                    let human = agent.status.replace('_'," ");
                    let mut chars = human.chars();
                    chars.next().map(|first| first.to_uppercase().chain(chars).collect()).unwrap_or_default()
                });
                (if live { "agent" } else { "offline" }.to_string(), live, Some(status))
            } else {
                let presence = settings.effective_presence(leases.get(&id).copied().unwrap_or(Presence::Offline));
                (match presence {Presence::Online=>"online",Presence::Idle=>"idle",Presence::Offline=>"offline",Presence::Dnd=>"dnd"}.to_string(),presence!=Presence::Offline,settings.status_text_display(now))
            };
            Ok(Member {id,name:user.name.clone(),avatar_url:format!("{base_url}{}",avatar_path(&app.secrets,user)),bot:user.is_bot(),online,presence,status,starred:starred.contains(&id)})
        }).collect::<campfire_db::Result<Vec<_>>>()
    }).await.map_err(db_error)?;
    c.no_store();
    c.set_header("pragma","no-cache");
    let body=serde_json::to_string(&Members { members }).map_err(Error::internal)?;
    // Rack::ETag digests the uncached JSON bytes, rather than the kit's fragment-piece key.
    c.set_header("etag", &format!("W/\"{}\"", &format!("{:x}",Sha256::digest(body.as_bytes()))[..32]));
    Ok(c.render(StatusCode::OK,&format::JSON,body))
}
