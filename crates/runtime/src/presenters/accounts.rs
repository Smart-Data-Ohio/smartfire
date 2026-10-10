//! Shared account, user, session and sidebar facts read from database rows.
//! Request preferences and retained context are loaded by [`crate::request_context`].
//!
//! Queries the db crate doesn't have yet are written here against the reference's SQL.

pub mod bot_access;

use campfire_db::{Account, CachedStatements, Connection, Membership, PushSubscription, Room, RoomType, User};
use campfire_kit::Ctx;
use campfire_presentation::accounts::{Bot, BotAgentForm, BotForm, BotGithubAccount, BotRoom, HelpContact};
use campfire_presentation::users::{
    MentionUser, ProfileMembership, PushSubscription as PushSubscriptionView, UserSummary,
};
use campfire_presentation::Platform;
use rails_compat::Secrets;
use rails_compat::global_id::{self, GlobalId};
use rails_compat::unicode;
use rusqlite::{params, OptionalExtension};

use super::{attachments, user_summary};

/// `User::Transferable::TRANSFER_LINK_EXPIRY_DURATION`
pub const TRANSFER_LINK_EXPIRY: jiff::SignedDuration = jiff::SignedDuration::from_hours(4);

/// `user.transfer_id`: `signed_id(purpose: :transfer, expires_in: 4.hours)`.
pub fn transfer_id(secrets: &Secrets, user_id: i64, now: jiff::Timestamp) -> String {
    rails_compat::signed_id::generate(secrets, "User", user_id, Some("transfer"), Some(now + TRANSFER_LINK_EXPIRY))
}

/// `User.find_by_transfer_id(id)`: the user id, if the signature and expiry check out.
pub fn user_id_from_transfer_id(secrets: &Secrets, transfer_id: &str, now: jiff::Timestamp) -> Option<i64> {
    rails_compat::signed_id::verify(secrets, "User", transfer_id, Some("transfer"), now)
}

/// `User.from_avatar_token(sid)`'s verification half (`find_signed!(sid, purpose: :avatar)`).
pub fn user_id_from_avatar_token(secrets: &Secrets, token: &str, now: jiff::Timestamp) -> Option<i64> {
    rails_compat::signed_id::verify(secrets, "User", token, Some("avatar"), now)
}

/// `user.attachable_sgid`.
pub fn attachable_sgid(secrets: &Secrets, user_id: i64) -> String {
    global_id::attachable_sgid(secrets, &GlobalId::new("User", user_id))
}

/// `fresh_account_logo_path(size:)`: `v` is `Current.account&.updated_at&.to_fs(:number)`.
pub fn fresh_account_logo_path(account: Option<&Account>, size: Option<&str>) -> String {
    fresh_account_logo_path_in_zone(account, size, &crate::context::renderer_time_zone())
}

/// Rails loads timestamps in the request's `Time.zone`, including this cache version.
pub fn fresh_account_logo_path_in_zone(
    account: Option<&Account>,
    size: Option<&str>,
    zone: &campfire_presentation::time::Zone,
) -> String {
    let v = account.map(|account| zone.to_fs(account.updated_at.jiff(), "number"));
    campfire_routes::fresh_account_logo(v.as_deref(), size)
}

/// `platform` as the views see it (`ApplicationPlatform.new(request.user_agent)`).
pub fn platform(c: &Ctx) -> Platform {
    crate::concerns::platform(c).to_view()
}

/// `User.administrator.first`, for `accounts/_help_contact`.
pub fn help_contact(conn: &Connection) -> campfire_db::Result<Option<HelpContact>> {
    let owner: Option<(String, Option<String>)> = conn
        .query_row_cached(r#"SELECT "users".* FROM "users" WHERE "users"."role" = 1 ORDER BY "users"."id" ASC LIMIT 1"#, [], |row| {
            let user = User::from_row(row)?;
            Ok((user.display_name().to_owned(), user.email_address))
        })
        .map(Some)
        .or_else(no_rows)?;
    Ok(owner.map(|(name, email_address)| HelpContact { name, email_address: email_address.unwrap_or_default() }))
}

/// `User.none?`
pub fn no_users(conn: &Connection) -> campfire_db::Result<bool> {
    Ok(User::count(conn)? == 0)
}

/// `record.touch`: bumps `updated_at` (what `belongs_to :record, touch: true` does to an
/// attachment's record, and `Blob#touch_attachments` after analysis).
pub fn touch(
    conn: &Connection,
    table: &str,
    id: i64,
    now: campfire_db::Timestamp,
) -> campfire_db::Result<()> {
    let now = if table == "users" {
        let previous = conn
            .query_row("SELECT updated_at FROM users WHERE id=?", [id], |row| {
                row.get::<_, campfire_db::Timestamp>(0)
            })
            .optional()?;
        previous.map_or(now, |previous| now.max(previous))
    } else {
        now
    };
    conn.execute_cached(
        &format!(r#"UPDATE "{table}" SET "updated_at" = ? WHERE "{table}"."id" = ?"#),
        params![now, id],
    )?;
    Ok(())
}

fn no_rows<T>(error: rusqlite::Error) -> Result<Option<T>, rusqlite::Error> {
    if error == rusqlite::Error::QueryReturnedNoRows { Ok(None) } else { Err(error) }
}

// --- Users -----------------------------------------------------------------------------------------

/// A user for `users/_mention` and the autocompletable views.
pub fn mention_user(secrets: &Secrets, user: &User) -> MentionUser {
    MentionUser { user: user_summary(secrets, user), attachable_sgid: attachable_sgid(secrets, user.id) }
}

/// `room_display_name(room, for_user:)`: a direct room is named after its other members.
pub fn room_display_name(conn: &Connection, room: &Room, for_user: &User) -> campfire_db::Result<String> {
    if room.direct() {room.direct_display_name(conn,Some(for_user),None).map(Option::unwrap_or_default)} else {Ok(room.name.clone().unwrap_or_default())}
}

/// `Room.model_name.param_key` for the room's STI class.
pub fn room_param_key(room_type: RoomType) -> &'static str {
    match room_type {
        RoomType::Open => "rooms_open",
        RoomType::Closed => "rooms_closed",
        RoomType::Direct => "rooms_direct",
        RoomType::Voice => "rooms_voice",
        RoomType::Stage => "rooms_stage",
        RoomType::Board => "rooms_board",
    }
}

/// Users::ProfilesController#show: `Current.user.memberships.with_ordered_room.partition { |m| m.room.direct? }`,
/// returned as `(direct, shared)`.
pub fn profile_memberships(conn: &Connection, user: &User) -> campfire_db::Result<(Vec<ProfileMembership>, Vec<ProfileMembership>)> {
    let mut direct = Vec::new();
    let mut shared = Vec::new();
    for (membership, room) in Membership::with_ordered_room(conn, user.id)? {
        let view = ProfileMembership {
            room_id: room.id,
            room_param_key: room_param_key(room.room_type).to_string(),
            room_display_name: room_display_name(conn, &room, user)?,
            involvement: membership.involvement.map(|involvement| involvement.name()).unwrap_or_default().to_string(),
            direct: room.direct(),
        };
        if room.direct() { direct.push(view) } else { shared.push(view) }
    }
    Ok((direct, shared))
}

/// Whether the classic profile offers password confirmation.
pub fn profile_has_password(user: &User) -> bool {
    user
        .password_digest
        .as_deref()
        .is_some_and(|s| !campfire_richtext::ruby::is_blank(s))
}

/// The classic profile's credential and unexpired remembered devices.
pub fn profile_two_factor(
    conn: &Connection,
    id: i64,
    now: jiff::Timestamp,
    google_reauthentication: bool,
) -> campfire_db::Result<campfire_presentation::two_factor::ProfileData> {
    let credential = campfire_db::TwoFactorCredential::for_user(conn, id)?;
    let devices = campfire_db::TwoFactorRememberedDevice::for_user(conn, id)?
        .into_iter()
        .filter(|d| d.expires_at.jiff() > now)
        .map(|d| campfire_presentation::two_factor::Device {
            id: d.id,
            user_agent: d.user_agent,
            ip_address: d.ip_address,
            last_used_at: d.last_used_at.map(|t| t.jiff()),
        })
        .collect();
    Ok(campfire_presentation::two_factor::ProfileData {
        confirmed_at: credential.and_then(|c| c.confirmed_at).map(|t| t.jiff()),
        devices,
        google: google_reauthentication
            && conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM google_identities WHERE user_id=?)",
                [id],
                |r| r.get::<_, bool>(0),
            )?,
    })
}

// --- Sidebar ---------------------------------------------------------------------------------------

/// `Users::SidebarsController::DIRECT_PLACEHOLDERS`
pub const DIRECT_PLACEHOLDERS: i64 = 20;

/// Suggestions for new direct conversations, excluding current direct-room peers.
pub fn direct_placeholder_user_rows(conn: &Connection, user: &User) -> campfire_db::Result<Vec<User>> {
    let direct_room_ids: Vec<i64> = Room::for_user_of_type(conn, user.id, RoomType::Direct)?.iter().map(|room| room.id).collect();
    let mut exclude_user_ids: Vec<i64> = Vec::new();
    if !direct_room_ids.is_empty() {
        let sql = format!(
            r#"SELECT "memberships"."user_id" FROM "memberships" WHERE "memberships"."room_id" IN ({})"#,
            placeholders(direct_room_ids.len())
        );
        let mut statement = conn.prepare_cached(&sql)?;
        let ids = statement.query_map(rusqlite::params_from_iter(&direct_room_ids), |row| row.get::<_, i64>(0))?;
        for id in ids {
            let id = id?;
            if !exclude_user_ids.contains(&id) {
                exclude_user_ids.push(id);
            }
        }
    }
    exclude_user_ids.push(user.id);

    // The limit goes in the SQL: under ORDER BY, SQLite recompiles a statement with a bound LIMIT
    // every time it runs.
    let limit = (DIRECT_PLACEHOLDERS - exclude_user_ids.len() as i64).max(0);
    let sql = format!(
        r#"SELECT * FROM "users" WHERE "users"."status" = 0 AND "users"."id" NOT IN ({}) ORDER BY "users"."created_at" ASC LIMIT {limit}"#,
        placeholders(exclude_user_ids.len())
    );
    query_users(conn, &sql, rusqlite::params_from_iter(&exclude_user_ids))
}

// --- Account ---------------------------------------------------------------------------------------

/// AccountsController#account_users: `User.where(status: [ :active, :banned ])` for
/// administrators, `User.active` otherwise; `.ordered.without_bots`.
pub fn account_users(conn: &Connection, can_administer: bool) -> campfire_db::Result<Vec<User>> {
    let status = if can_administer { r#""users"."status" IN (0, 2)"# } else { r#""users"."status" = 0"# };
    let sql = format!(r#"SELECT * FROM "users" WHERE {status} AND "users"."role" != 2 ORDER BY LOWER(name)"#);
    query_users(conn, &sql, [])
}

/// Bot rows share preloaded agents and owners; their ordered room lists stay per bot.
pub fn bots(conn: &Connection, secrets: &Secrets, bots: &[User]) -> campfire_db::Result<Vec<Bot>> {
    let agents: std::collections::HashMap<_, _> =
        campfire_db::Agent::for_users(conn, &bots.iter().map(|bot| bot.id).collect::<Vec<_>>())?
            .into_iter()
            .map(|agent| (agent.user_id, agent))
            .collect();
    let mut owner_ids: Vec<_> = agents.values().filter_map(|agent| agent.owner_id).collect();
    owner_ids.sort_unstable();
    owner_ids.dedup();
    let owners: std::collections::HashMap<_, _> = if owner_ids.is_empty() {
        Default::default()
    } else {
        User::where_ids(conn, &owner_ids)?
            .into_iter()
            .map(|owner| (owner.id, owner.display_name().to_owned()))
            .collect()
    };
    bots.iter()
        .map(|bot| {
            let agent = agents.get(&bot.id);
            let owner_name = agent
                .and_then(|agent| agent.owner_id)
                .and_then(|id| owners.get(&id))
                .cloned();
            bot_with_associations(conn, secrets, bot, agent, owner_name)
        })
        .collect()
}

fn bot_with_associations(
    conn: &Connection,
    secrets: &Secrets,
    bot: &User,
    agent: Option<&campfire_db::Agent>,
    owner_name: Option<String>,
) -> campfire_db::Result<Bot> {
    let mut rooms = Room::for_user_without_directs(conn, bot.id)?;
    sort_by_lower_name(&mut rooms, |room| room.name.as_deref().unwrap_or(""));
    let icon = if attachments::attached_blob(conn, "User", bot.id, "avatar")?.is_none() {
        bot.icon_name
            .as_deref()
            .and_then(|name| super::resolve_avatar_icon(conn, name))
    } else {
        None
    };
    Ok(Bot {
        user: user_summary(secrets, bot),
        kind: agent.map(|agent| agent.kind.name().into()),
        owner_name,
        icon,
        rooms: rooms
            .into_iter()
            .map(|room| BotRoom {
                id: room.id,
                name: room.name.unwrap_or_default(),
            })
            .collect(),
    })
}

/// Read-only facts for the bot edit form. Reading a legacy bot never creates an Agent.
pub fn bot_form(conn: &Connection, app: &crate::app::AppState, base_url: &str, bot: &User, zone: &campfire_presentation::time::Zone, github_usable: bool) -> campfire_db::Result<BotForm> {
    use campfire_db::models::{agent_posting::{self, Cap}, webhook::Webhook};
    let avatar = attachments::attached_blob(conn, "User", bot.id, "avatar")?;
    let icon_name: Option<String> = conn.query_row("SELECT icon_name FROM users WHERE id=?", [bot.id], |row| row.get(0))?;
    let profile = campfire_db::Agent::for_user(conn, bot.id)?;
    let webhook = Webhook::find_by_user(conn, bot.id)?;
    // Ruby's || falls back only for nil, not for an empty agent secret.
    let agent_secret = profile.as_ref().map(|agent| agent.webhook_signing_secret(conn, &app.ar_encryption)).transpose()?.flatten();
    let signing_secret = match agent_secret {
        Some(value) => Some(value),
        None => webhook.as_ref().map(|webhook| webhook.signing_secret(&app.ar_encryption)).transpose()?.flatten(),
    };
    let budget_usage_line = profile.as_ref().map(|agent| -> campfire_db::Result<String> {
        let window = agent_posting::daily_window(campfire_db::Timestamp::from_jiff(app.clock.now()), zone.tz())?;
        let cells = [(Cap::Messages, agent.daily_message_cap, "messages"),
            (Cap::BoardPosts, agent.daily_board_post_cap, "board posts"),
            (Cap::ExternalActions, agent.daily_external_action_cap, "external actions")].into_iter().map(|(cap, limit, noun)| {
                let used = agent_posting::cap_usage(conn, agent.id, bot.id, cap, &window)?;
                Ok(limit.map(|limit| format!("{used}/{limit} {noun}")).unwrap_or_else(|| format!("{used} {noun}")))
            }).collect::<campfire_db::Result<Vec<_>>>()?;
        Ok(cells.join(" · "))
    }).transpose()?.unwrap_or_default();
    let github = crate::integrations::github::accounts::Account::for_user(conn, bot.id)?.map(|account| BotGithubAccount {
        usable: github_usable, login: account.github_login, disconnected_reason: account.disconnected_reason,
    });
    Ok(BotForm {
        name: Some(bot.name.clone()), webhook_url: bot.webhook_url(conn)?,
        avatar_attachment_url: avatar.map(|blob| format!("{base_url}{}", campfire_storage::paths::blob_redirect_path(&*app.storage.verifier, &blob, None))),
        persisted: true, icon_name: icon_name.clone(),
        icon: icon_name.as_deref().and_then(|name| super::resolve_avatar_icon(conn, name)),
        agent: profile.map(|agent| BotAgentForm {
            id: agent.id, owner_id: agent.owner_id, provider: agent.provider, runtime: agent.runtime,
            description: agent.description, daily_message_cap: agent.daily_message_cap,
            daily_board_post_cap: agent.daily_board_post_cap, daily_external_action_cap: agent.daily_external_action_cap,
            raw_caps: Default::default(), suspended: agent.suspended_at.is_some(), errors: None, error_fields: Vec::new(),
        }), budget_usage_line, signing_secret, github, ..Default::default()
    })
}

/// A push subscription with its user agent parsed like `UserAgent.parse(push_subscription.user_agent)`.
pub fn push_subscription(subscription: &PushSubscription) -> PushSubscriptionView {
    let agent = crate::concerns::user_agent::parse(subscription.user_agent.as_deref().unwrap_or(""));
    PushSubscriptionView {
        id: subscription.id,
        endpoint: subscription.endpoint.clone().unwrap_or_default(),
        browser: agent.browser(),
        version: agent.version().to_string(),
        platform: agent.platform().unwrap_or_default(),
    }
}

// --- Helpers ---------------------------------------------------------------------------------------

/// `ORDER BY LOWER(name)`: SQLite lowercases ASCII only.
pub fn sort_by_lower_name<T>(items: &mut [T], name: impl Fn(&T) -> &str) {
    items.sort_by_cached_key(|item| name(item).to_ascii_lowercase());
}

pub fn placeholders(count: usize) -> String {
    vec!["?"; count].join(", ")
}

/// Users from a `SELECT "users".*` query.
pub fn query_users(conn: &Connection, sql: &str, values: impl rusqlite::Params) -> campfire_db::Result<Vec<User>> {
    let mut statement = conn.prepare_cached(sql)?;
    let users = statement.query_map(values, User::from_row)?.collect::<Result<_, _>>()?;
    Ok(users)
}

/// `ActiveRecord::RecordNotUnique`: a unique index refused the write.
pub fn is_record_not_unique(error: &campfire_db::Error) -> bool {
    matches!(
        error,
        campfire_db::Error::Sqlite(rusqlite::Error::SqliteFailure(failure, _))
            if failure.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE || failure.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_PRIMARYKEY
    )
}

/// A permitted string attribute: `Some` when the key was given (its value may be nil).
pub fn string_attribute(params: &campfire_kit::ParamMap, key: &str) -> Option<Option<String>> {
    if !params.contains_key(key) {
        return None;
    }
    Some(params.get(key).and_then(|param| param.to_s()))
}

pub fn sidebar_direct_label(name: Option<&str>, members: &[UserSummary]) -> String {
    sidebar_direct_label_for_names(name, &members.iter().map(|m| m.name.as_str()).collect::<Vec<_>>())
}

/// [`sidebar_direct_label`] from the members' names alone.
pub fn sidebar_direct_label_for_names(name: Option<&str>, member_names: &[&str]) -> String {
    if let Some(name)=name.filter(|s|!campfire_richtext::ruby::is_blank(s)){return name.into();}
    if member_names.len()<=1 {return member_names.first().map(|n|sidebar_first_name(n)).unwrap_or_default().to_string();}
    let mut names: Vec<&str>=member_names.to_vec();
    names.sort_by_key(|n|unicode::downcase(n));
    let label=names.iter().take(3).map(|n|sidebar_first_name(n)).collect::<Vec<_>>().join(", ");
    if names.len()>3 {format!("{label} +{}",names.len()-3)}else{label}
}

fn sidebar_first_name(name: &str) -> &str {
    // Ruby String#split(" "): vertical tab separates words; NBSP does not.
    name.split([' ', '\t', '\n', '\r', '\x0b', '\x0c']).find(|part|!part.is_empty()).unwrap_or_default()
}

pub fn resolve_room_icon(conn: &Connection, name: Option<&str>) -> Option<campfire_presentation::helpers::AvatarIcon> {
    use campfire_presentation::{helpers::AvatarIcon,messages::reactions::static_icon};
    let name=name?;
    let icon=static_icon(name);
    if matches!(icon,Some(AvatarIcon::Image{brand:true,..})){return icon;}
    let title: Option<String>=conn.query_row("SELECT title FROM workspace_icons WHERE name=?",[name],|row|row.get(0)).optional().ok().flatten();
    room_icon_with_title(Some(name),title)
}

fn room_icon_with_title(name: Option<&str>, title: Option<String>) -> Option<campfire_presentation::helpers::AvatarIcon> {
    use campfire_presentation::{helpers::AvatarIcon,messages::reactions::static_icon};
    let name = name?;
    let icon = static_icon(name);
    if matches!(icon,Some(AvatarIcon::Image{brand:true,..})) { return icon; }
    title.map(|title| AvatarIcon::Image{title,url:format!("/icons/{name}"),brand:false}).or(icon)
}

pub mod audit_logs;
