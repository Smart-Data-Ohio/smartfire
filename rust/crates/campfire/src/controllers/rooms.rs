//! `RoomsController` (reference/app/controllers/rooms_controller.rb), plus what its subclasses
//! share: `set_room` over a `room_scope`, `ensure_can_administer`,
//! `ensure_permission_to_create_rooms`.
//!
//! The subclasses (`Rooms::OpensController` and friends) re-declare `before_action :set_room`
//! (and `ensure_can_administer`) with their own `only:`, which *replaces* the parent's callback.
//! So the actions they inherit from here but don't list (`destroy` for opens/closeds, `show` for
//! directs) run without `set_room` and raise on the nil `@room`, as in the reference.

pub mod members;
pub mod categories;
pub mod favorites;
pub mod inbound_email_addresses;
pub mod closeds;
pub mod directs;
pub mod involvements;
pub mod opens;
pub mod refreshes;
pub mod reads;
pub mod polls;
pub mod pins;
pub mod slash_commands;
pub mod message_links;
pub mod files;

use askama::Template;
use campfire_db::{Account, Message, Room, RoomType, Timeline, User};
use campfire_kit::{Ctx, Error, Redirect, Result, StatusCode, halt};

use crate::app::AppCtx;
use crate::concerns::{self, Before, before_actions, cast_integer, require_current_user};
use crate::controllers::presenters::page::{self, Rendered, db_error};
use crate::controllers::presenters::{Presenter, user_view};

/// `room_scope`: which of `Current.user.rooms` a controller may act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// `Current.user.rooms` (RoomsController)
    All,
    /// Alive open and closed rooms only (the conversion controllers).
    WithoutDirects,
    /// `Current.user.rooms.directs` (directs)
    Directs,
}

impl Scope {
    fn includes(self, room: &Room) -> bool {
        match self {
            Scope::All => true,
            Scope::WithoutDirects => matches!(room.room_type, RoomType::Open | RoomType::Closed),
            Scope::Directs => room.room_type == RoomType::Direct,
        }
    }
}

// --- Actions ------------------------------------------------------------------------------------

/// `index`: `redirect_to room_url(Current.user.rooms.last)` (inherited by the room-type
/// controllers). With no rooms, `room_url(nil)` raises.
pub async fn index(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let user_id = require_current_user(c)?.id;
    let room = c.app().db.read(move |conn| Room::last_for_user(conn, user_id)).await.map_err(db_error)?;
    let Some(room) = room else {
        return Err(Error::internal(anyhow::anyhow!("No route matches room_url(nil)")));
    };
    let url = c.url_for(&campfire_routes::room(room.id));
    c.redirect_to(&url)
}

/// `show`, also `GET /rooms/:room_id/@:message_id`.
pub async fn show(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (room, join_preview) = set_room_for_show(c, Scope::All).await?;
    concerns::remember_last_room_visited(c, room.id);
    if join_preview {
        return page::framed_page!(c, StatusCode::OK, |ctx| campfire_views::rooms::JoinPage { ctx, id:room.id, name:room.name.as_deref().unwrap_or_default() }).await;
    }
    render_show(c, room).await
}

/// `join`: open rooms can be rejoined by link; creation restrictions do not restrict joins.
pub async fn join(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let id = c.param_str("id").and_then(cast_integer);
    let room = match id {
        Some(id) => joinable_open_room(c, id).await?,
        None => None,
    };
    let Some(room) = room else { return inaccessible_room(c); };
    let user_id = require_current_user(c)?.id;
    let room_id = room.id;
    let (membership, created) = c.app().db.write(move |tx| campfire_db::Membership::join_open(tx, room_id, user_id)).await.map_err(db_error)?;
    if created {
        let partials = render_membership_sidebar(c, &room, &membership, Some(false)).await?;
        use crate::channels::broadcasts::{Partials, Stream};
        c.app().broadcasts.prepend(&Stream::user_rooms(user_id), "shared_rooms", &partials.shared_room(&room));
    }
    redirect_to_room(c, room.id)
}

/// `destroy` (RoomsController and `Rooms::DirectsController`).
pub async fn destroy(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = set_room(c, Scope::All).await?;
    ensure_can_delete(c, &room).await?;
    destroy_room(c, room).await
}

/// `destroy` inherited by `Rooms::OpensController` and `Rooms::ClosedsController`, whose
/// `set_room` doesn't run for it: `nil.destroy` raises NoMethodError.
pub async fn destroy_without_room(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    Err(Error::internal(anyhow::anyhow!("undefined method 'destroy' for nil")))
}

pub(crate) async fn destroy_room(c: &mut Ctx, room: Room) -> Result {
    let destroyed = room.clone();
    c.app().db.write(move |tx| destroyed.begin_destroy(tx)).await.map_err(db_error)?;
    audit_room(c, &room, "room.destroy", serde_json::json!({"name": room.name})).await?;
    c.app().broadcasts.room_remove(&room);
    match c.respond_to(&[&campfire_kit::format::HTML, &campfire_kit::format::JSON])? {
        f if *f == campfire_kit::format::JSON => c.json(StatusCode::OK, &serde_json::json!({"deleted": true, "room_id": room.id})),
        _ => {
            let root = c.url_for(&campfire_routes::root());
            let notice = room.name.as_deref().filter(|name| !campfire_richtext::ruby::is_blank(name)).map(|name| format!("Deleted #{name}"));
            c.redirect_to_with(&root, Redirect { notice, ..Redirect::default() })
        }
    }
}

pub async fn leave(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = set_room(c, Scope::All).await?;
    leave_room(c, room).await
}

pub(crate) async fn leave_room(c: &mut Ctx, room: Room) -> Result {
    let user = require_current_user(c)?.clone();
    let left = room.clone();
    let destroyed = c.app().db.write(move |tx| {
        let destroyed = if left.direct() {
            left.leave_direct(tx, user.id)? == campfire_db::models::direct_room::LeaveOutcome::Destroyed
        } else {
            campfire_db::Membership::find_by_room_and_user(tx.conn(), left.id, user.id)?
                .ok_or(campfire_db::Error::RecordNotFound("Membership"))?.destroy(tx)?;
            false
        };
        Ok(destroyed)
    }).await.map_err(db_error)?;
    if destroyed {
        audit_room(c, &room, "room.destroy", serde_json::json!({"name":room.name})).await?;
        c.app().broadcasts.room_remove(&room);
    } else {
        audit_room(c, &room, "room.membership.change", serde_json::json!({"revoked":[require_current_user(c)?.name]})).await?;
    }
    match c.respond_to(&[&campfire_kit::format::HTML, &campfire_kit::format::JSON])? {
        f if *f == campfire_kit::format::JSON => c.json(StatusCode::OK, &serde_json::json!({"left":true,"room_id":room.id})),
        _ => c.redirect_to(&c.url_for(&campfire_routes::root())),
    }
}

// --- Shared before-actions ----------------------------------------------------------------------

/// `set_room`: `room_scope.find_by(id: params[:room_id] || params[:id])`, or back to the root with
/// an alert.
pub async fn set_room(c: &mut Ctx, scope: Scope) -> Result<Room> {
    let user_id = require_current_user(c)?.id;
    let id = c.param_str("room_id").or_else(|| c.param_str("id")).and_then(cast_integer);
    let room = match id {
        Some(id) => c.app().db.read(move |conn| Room::find_for_user(conn, user_id, id)).await.map_err(db_error)?,
        None => None,
    };
    match room.filter(|room| scope.includes(room)) {
        Some(room) => Ok(room),
        None => inaccessible_room(c),
    }
}

fn inaccessible_room<T>(c: &mut Ctx) -> Result<T> {
    let root = c.url_for(&campfire_routes::root());
    let redirect = Redirect { alert: Some("Room not found or inaccessible".into()), ..Redirect::default() };
    halt(c.redirect_to_with(&root, redirect)?)
}

async fn joinable_open_room(c: &Ctx, id: i64) -> Result<Option<Room>> {
    c.app().db.read(move |conn| Ok(Room::find_by_id(conn, id)?.filter(|room| room.deleted_at.is_none() && room.room_type == RoomType::Open))).await.map_err(db_error)
}

/// Rails only falls back to an open-room preview for show, including the redirecting
/// open/closed namespace actions. Edit, destroy and leave remain membership-scoped.
pub(super) async fn set_room_for_show(c: &mut Ctx, scope: Scope) -> Result<(Room, bool)> {
    let user_id = require_current_user(c)?.id;
    let id = c.param_str("room_id").or_else(|| c.param_str("id")).and_then(cast_integer);
    if let Some(id) = id {
        let room = c.app().db.read(move |conn| Room::find_for_user(conn, user_id, id)).await.map_err(db_error)?;
        if let Some(room) = room.filter(|room| scope.includes(room)) { return Ok((room, false)); }
        if let Some(room) = joinable_open_room(c, id).await? { return Ok((room, true)); }
    }
    inaccessible_room(c)
}

/// `ensure_can_administer`: `head :forbidden unless Current.user.can_administer?(@room)`.
pub fn ensure_can_administer(c: &mut Ctx, room: &Room) -> Result<()> {
    let allowed = require_current_user(c)?.can_administer(Some(room.creator_id), false);
    if !allowed {
        return halt(concerns::head(StatusCode::FORBIDDEN));
    }
    Ok(())
}

/// `User#can_delete_room?`: shared group history may only be deleted by an administrator.
pub(crate) async fn ensure_can_delete(c: &mut Ctx, room: &Room) -> Result<()> {
    let id = room.id;
    let group = c.app().db.read(move |conn| Room::find(conn, id)?.direct_group_capable(conn)).await.map_err(db_error)?;
    if group {
        if !require_current_user(c)?.is_administrator() { return halt(concerns::head(StatusCode::FORBIDDEN)); }
        Ok(())
    } else {
        ensure_can_administer(c, room)
    }
}

/// Rails audits these actions after their domain transaction commits. Audit failure must not
/// undo a completed write; durable job insertion still belongs to the domain transaction.
pub(crate) async fn audit_room(c: &Ctx, room: &Room, action: &str, changes: serde_json::Value) -> Result<()> {
    let context = audit_context(c)?;
    let room = room.clone();
    let action = action.to_string();
    c.app().db.write(move |tx| record_room_audit(tx, &room, &action, changes, &context)).await.map_err(db_error)
}

pub(crate) fn audit_context(c: &Ctx) -> Result<campfire_db::models::audit_log::Context> {
    Ok(campfire_db::models::audit_log::Context {
        actor: Some(require_current_user(c)?.into()),
        ip_address: Some(c.request.remote_ip()?.to_string()),
        user_agent: c.request.user_agent().map(str::to_string),
    })
}

pub(crate) fn record_room_audit(tx: &campfire_db::Tx<'_>, room: &Room, action: &str, changes: serde_json::Value, context: &campfire_db::models::audit_log::Context) -> campfire_db::Result<()> {
    campfire_db::models::audit_log::AuditLog::record(tx, campfire_db::models::audit_log::NewAuditLog {
        action: action.into(), target: Some(room.into()), changes: Some(changes), ..Default::default()
    }, context)?;
    Ok(())
}

/// `ensure_permission_to_create_rooms`
pub async fn ensure_permission_to_create_rooms(c: &mut Ctx) -> Result<()> {
    let administrator = require_current_user(c)?.is_administrator();
    let account = c.app().db.read(Account::first).await.map_err(db_error)?;
    let restricted = account.is_some_and(|account| account.settings().restrict_room_creation_to_administrators());
    if restricted && !administrator {
        return halt(concerns::head(StatusCode::FORBIDDEN));
    }
    Ok(())
}

// --- Helpers ------------------------------------------------------------------------------------

pub(crate) fn redirect_to_room(c: &mut Ctx, room_id: i64) -> Result {
    let url = c.url_for(&campfire_routes::room(room_id));
    c.redirect_to(&url)
}

/// `params.require(:room).permit(:name)`: `Some(name)` when the name was submitted.
pub(crate) fn room_name_param(c: &Ctx) -> Result<Option<Option<String>>> {
    room_string_param(c, "name")
}

pub(super) fn room_icon_param(c: &Ctx) -> Result<Option<Option<String>>> {
    room_string_param(c, "icon_name")
}

fn room_string_param(c: &Ctx, attribute: &str) -> Result<Option<Option<String>>> {
    let room = c.params.require("room")?;
    let permitted = room.permit(&campfire_kit::permit_keys(&[attribute]));
    // ActiveModel::Type::String uses "t"/"f" for booleans; nil remains nil, and a
    // collection rejected by permit is absent (so updates leave the old value alone).
    Ok(permitted.get(attribute).map(|name| match name {
        campfire_kit::Param::Null => None,
        campfire_kit::Param::Bool(true) => Some("t".into()),
        campfire_kit::Param::Bool(false) => Some("f".into()),
        value => value.to_s(),
    }))
}

/// Room form facts, including an attempted invalid value rather than reloading the
/// persisted record. Icon preview data comes through the existing presenter resolver.
pub(super) async fn form_room(
    c: &Ctx, id: Option<i64>, name: Option<String>, icon_name: Option<String>, errors: campfire_db::Errors,
) -> Result<campfire_views::rooms::FormRoom> {
    let icon_name = Room::normalize_icon_name(icon_name.as_deref());
    let lookup_name = icon_name.clone();
    let domain = c.app().mail.config.domain.clone();
    let (icon, inbound_email) = c.app().db.read(move |conn| {
        let icon = super::presenters::accounts::resolve_room_icon(conn, lookup_name.as_deref());
        let inbound_email = if let Some(id) = id {
            let room=Room::find(conn,id)?;
            let address = domain.as_ref().zip(room.inbound_email_token.as_ref()).filter(|(_,token)| !token.chars().all(char::is_whitespace)).filter(|_| room.emailable()).map(|(domain,token)| format!("room-{token}@{domain}"));
            Some(campfire_views::rooms::InboundEmailView {id,emailable:room.emailable(),enabled:domain.is_some(),address})
        } else { None };
        Ok((icon, inbound_email))
    }).await.map_err(db_error)?;
    Ok(campfire_views::rooms::FormRoom {
        id, name, icon_name, icon, inbound_email, errors: errors.full_messages(),
        error_attributes: errors.0.iter().map(|(attribute,_)| (*attribute).to_string()).collect(),
    })
}

/// `params.fetch(:user_ids, [])` as ids `User.where(id:)` can match.
pub(crate) fn user_ids_param(c: &Ctx) -> Vec<i64> {
    c.param("user_ids").map(user_ids_from_param).unwrap_or_default()
}

/// Active Record PredicateBuilder::ArrayHandler delegates a single non-null operand
/// back to the builder, but casts multiple operands individually as integers for IN.
pub(super) fn user_ids_from_param(value: &campfire_kit::Param) -> Vec<i64> {
    fn scalar(value: &campfire_kit::Param) -> Option<i64> {
        match value {
            campfire_kit::Param::Str(value) => cast_integer(value),
            campfire_kit::Param::Number(value) => value.as_i64().or_else(|| {
                value.as_f64().filter(|id| *id >= i64::MIN as f64 && *id < -(i64::MIN as f64)).map(|id| id as i64)
            }),
            campfire_kit::Param::Bool(value) => Some(i64::from(*value)),
            _ => None,
        }
    }
    match value {
        campfire_kit::Param::Array(values) => {
            let values=values.iter().filter(|value| !matches!(value,campfire_kit::Param::Null)).collect::<Vec<_>>();
            if values.len()==1 {user_ids_from_param(values[0])}
            else {values.into_iter().filter_map(scalar).collect()}
        }
        value => scalar(value).into_iter().collect(),
    }
}

/// `User.where(id: ids)`, as ids of existing users (in id order, like the query).
pub(crate) fn existing_user_ids(conn: &campfire_db::Connection, ids: &[i64]) -> campfire_db::Result<Vec<i64>> {
    Ok(User::where_ids(conn, ids)?.into_iter().map(|user| user.id).collect())
}

/// Renders `users/sidebars/rooms/_shared` for `room` outside a request.
pub(crate) async fn render_shared_room(c: &Ctx, room: &Room) -> Result<Rendered> {
    let app = c.app().clone();
    let base_url = page::renderer_base_url(c);
    let room = room.clone();
    let html = c
        .app()
        .db
        .read(move |conn| {
            let presenter = Presenter::new(conn, &app, None);
            let sidebar_room = presenter.sidebar_room(&room);
            let account = Account::first(conn)?;
            Ok(page::render_detached_at(&app, account.as_ref(), &base_url, |ctx| {
                campfire_views::users::SidebarSharedPartial { ctx,room: sidebar_room }.render()
            }))
        })
        .await
        .map_err(db_error)?
        .map_err(Error::internal)?;
    Ok(Rendered { shared_room: Some(html), ..Rendered::default() })
}

/// Open/closed update callbacks render this identity once outside the request, then send it
/// globally (open) or to every retained member (closed). No per-viewer or session input.
pub(crate) async fn render_shared_header(c: &Ctx, room: &Room) -> Result<String> {
    let app = c.app().clone();
    let base_url = page::renderer_base_url(c);
    let room = room.clone();
    c.app().db.read(move |conn| {
        let creator = User::find(conn, room.creator_id)?;
        let header = super::presenters::rooms_directory::header(conn, &room, &creator)?;
        let account = Account::first(conn)?;
        Ok(page::render_detached_at(&app, account.as_ref(), &base_url, |ctx| campfire_views::rooms::header_identity(ctx, &header).0))
    }).await.map_err(db_error)
}

/// The involvement callback's row carries its recipient's membership and menu facts.
pub(crate) async fn render_membership_sidebar(c:&Ctx, room:&Room, membership:&campfire_db::Membership, unread:Option<bool>)->Result<Rendered> {
    let app=c.app().clone();let base_url=page::renderer_base_url(c);let room=room.clone();let membership=membership.clone();
    let html=c.app().db.read(move |conn| {
        let viewer=User::find(conn,membership.user_id)?;
        let mut row=Presenter::new(conn,&app,None).sidebar_room(&room);
        row.menu=super::presenters::accounts::room_menu(&room,Some(&membership),Some(&viewer),0,None);
        row.unread=unread.unwrap_or(false);
        let account=Account::first(conn)?;
        Ok(page::render_detached_at(&app,account.as_ref(),&base_url,|ctx|campfire_views::users::SidebarSharedPartial{ctx,room:row}.render()))
    }).await.map_err(db_error)?.map_err(Error::internal)?;
    Ok(Rendered{shared_room:Some(html),..Default::default()})
}

/// `rooms/show` with `find_messages`: the page around `params[:message_id]`, else the last page.
async fn render_show(c: &mut Ctx, room: Room) -> Result {
    let app = c.app().clone();
    let user = require_current_user(c)?.clone();
    let message_id = c.param_str("message_id").and_then(cast_integer);
    let request_host = Some(c.request.host());
    let cache_base_url = c.url_for("");
    let (show,composer) = c
        .app()
        .db
        .read(move |conn| {
            let messages = super::presenters::room_shell::find_messages(conn,room.id,message_id)?;
            let membership=campfire_db::Membership::find_by_room_and_user(conn,room.id,user.id)?.ok_or(campfire_db::Error::RecordNotFound("Membership"))?;
            let divider=super::presenters::room_shell::unread_divider(conn,&membership,&messages)?;
            let mut presenter = Presenter::new(conn, &app, request_host);
            presenter.cache_base_url = Some(cache_base_url);
            let original = Room::original(conn)?.is_some_and(|original| original.id == room.id);
            let room_gid = crate::channels::room_gid(&room).to_param();
            let drive=presenter.composer_drive_flow(&user,false)?;
            let composer=presenter.composer_facts(&room,&user,None,drive)?;
            let list=presenter.room_message_list(&messages,divider.message_id,divider.count)?;
            Ok((campfire_views::rooms::ShowView {
                shell:campfire_views::rooms::ShellComponents{message_list:Some(list),..Default::default()},scroll_to_unread_divider:divider.scroll,jump_to_unread_url:divider.jump_url,unread_divider_message_id:divider.message_id,unread_count:divider.count,
                room: presenter.room_view(&room, &user)?,
                updated_at: room.updated_at.jiff(),
                user: user_view(&app.secrets, &user),
                // The page's message fragments come from the store the render then uses.
                messages: campfire_views::fragment_cache::with(&app.fragment_cache, || presenter.messages(&messages))?,
                invitation: original && !Message::paged(conn, Timeline::Room(room.id))?,
                join_code: Account::first(conn)?.map(|account| account.join_code).unwrap_or_default(),
                messages_stream_name: rails_compat::turbo::signed_stream_name(&app.secrets, &[&room_gid, "messages"]),
                ooo_notice_members:
                    crate::controllers::presenters::status_settings::ooo_notice_members(
                        conn,
                        &app.secrets,
                        &room,
                        user.id,
                        app.db.env().now(),
                    )?,
            },composer))
        })
        .await
        .map_err(db_error)?;
    let response = super::presenters::view_context::page_or_frame(c,StatusCode::OK,
        |ctx|super::presenters::room_native::render(ctx,&show,&composer,false),
        |ctx|super::presenters::room_native::render(ctx,&show,&composer,true)).await?;
    let fragments = campfire_views::messages::MessageItem::cached_fragments(&c.app().fragment_cache, &show.messages, &c.url_for(""));
    Ok(response.with_cached_fragments(fragments))
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod parity_tests;

#[cfg(test)]
mod reads_tests;

#[cfg(test)]
mod join_tests;

#[cfg(test)]
mod channel_audits_tests;

#[cfg(test)]
mod coercions_tests;

#[cfg(test)]
mod direct_selection_tests;

#[cfg(test)]
mod icons_tests;

#[cfg(test)]
mod inbound_tests;

#[cfg(test)]
mod inbound_rails_cases;

#[cfg(test)]
mod direct_forms_tests;
#[cfg(test)]
mod direct_rename_tests;

#[cfg(test)]
mod directs_rails_cases;

#[cfg(test)]
mod rooms_rails_cases;

#[cfg(test)]
mod opens_rails_cases;

#[cfg(test)]
mod closeds_rails_cases;
#[cfg(test)]
#[path = "rooms/ws17_ooo_tests.rs"]
mod ws17_ooo_tests;

#[cfg(test)]
#[path = "rooms/members_rails_cases.rs"]
mod members_rails_cases;

#[cfg(test)]
#[path = "rooms/refreshes_rails_cases.rs"]
mod refreshes_rails_cases;

#[cfg(test)]
mod sidebars_rails_cases;

#[cfg(test)]
mod involvements_rails_cases;

#[cfg(test)]
mod reads_rails_cases;

#[cfg(test)]
mod organization_rails_support;

#[cfg(test)]
mod favorites_rails_cases;

#[cfg(test)]
mod room_categories_rails_cases;

#[cfg(test)]
mod categories_rails_cases;

#[cfg(test)]
mod switchers_rails_cases;

#[cfg(test)]
#[path = "rooms/native_integration_tests.rs"]
mod native_integration_tests;

#[cfg(test)]
mod query_probe;
