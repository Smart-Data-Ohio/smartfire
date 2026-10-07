//! `/api/v1/settings` (S7): the signed-in person's settings, the SPA's twin of the classic
//! profile page. Every write runs the classic controller's save path (`users/profiles#update`,
//! `users/notification_settings#update`, `users/statuses#update`, `users/dnd_allowances`,
//! `users/sessions`, `users/push_subscriptions`), so the rows, audit entries, jobs and
//! broadcasts are the classic ones. Only the request (typed JSON instead of a form) and the
//! answer (the settings again instead of a redirect) differ.

use axum::Router;
use axum::routing::{get, patch, post, put};
use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_db::models::audit_log::{Actor, Context};
use campfire_db::models::user::profile_settings::{self, INBOX_KEYS};
use campfire_db::models::user_status_settings::clock_time_to_minutes;
use campfire_db::{
    DndAllowedUser, Errors, PushSubscription, Session, User, UserChanges, UserStatusSettings,
};
use campfire_kit::{Ctx, Error, Kit, Result, StatusCode, action, unparsed_action};
use campfire_views::users::ConnectionPanel;
use campfire_web::authentication;
use campfire_web::concerns::{self, Authentication, Before, current_session};
use campfire_web::controllers::presenters::attachments::{self, Assignment, Record};
use campfire_web::controllers::presenters::page::db_error;
use campfire_web::controllers::presenters::{self, profile_sections};
use serde::de::DeserializeOwned;

use crate::dto::time;
use crate::error::{fail, not_found, prepare, respond, validation};

/// The largest settings body read: a profile with a long bio, or twenty keyword alerts.
const BODY_LIMIT: usize = 64 * 1024;

/// The settings routes, merged into [`crate::routes`].
pub fn routes() -> Router<Kit> {
    Router::new()
        .route("/api/v1/settings", get(action(show)))
        .route(
            "/api/v1/settings/profile",
            patch(unparsed_action(update_profile)),
        )
        .route(
            "/api/v1/settings/avatar",
            put(unparsed_action(update_avatar)).delete(unparsed_action(remove_avatar)),
        )
        .route(
            "/api/v1/settings/appearance",
            patch(unparsed_action(update_appearance)),
        )
        .route(
            "/api/v1/settings/calls",
            patch(unparsed_action(update_calls)),
        )
        .route(
            "/api/v1/settings/notifications",
            patch(unparsed_action(update_notifications)),
        )
        .route(
            "/api/v1/settings/status",
            patch(unparsed_action(update_status)),
        )
        .route(
            "/api/v1/settings/dnd_allowances/{user_id}",
            post(action(allow_through_dnd)).delete(action(disallow_through_dnd)),
        )
        .route("/api/v1/settings/sessions", get(action(sessions)))
        .route(
            "/api/v1/settings/sessions/revoke_others",
            post(action(revoke_other_sessions)),
        )
        .route(
            "/api/v1/settings/sessions/{id}",
            axum::routing::delete(action(revoke_session)),
        )
        .route(
            "/api/v1/settings/push_subscriptions",
            get(action(push_subscriptions)),
        )
        .route(
            "/api/v1/settings/push_subscriptions/{id}",
            axum::routing::delete(action(remove_push_subscription)),
        )
}

macro_rules! endpoint {
    ($(#[$doc:meta])* $name:ident => $body:ident) => {
        $(#[$doc])*
        pub async fn $name(c: &mut Ctx) -> Result {
            prepare(c);
            let result = $body(c).await;
            respond(c, result)
        }
    };
}

endpoint!(
    /// `GET /api/v1/settings`
    show => show_settings
);
endpoint!(
    /// `PATCH /api/v1/settings/profile`
    update_profile => save_profile
);
endpoint!(
    /// `PUT /api/v1/settings/avatar`
    update_avatar => attach_avatar
);
endpoint!(
    /// `DELETE /api/v1/settings/avatar`
    remove_avatar => detach_avatar
);
endpoint!(
    /// `PATCH /api/v1/settings/appearance`
    update_appearance => save_appearance
);
endpoint!(
    /// `PATCH /api/v1/settings/calls`
    update_calls => save_calls
);
endpoint!(
    /// `PATCH /api/v1/settings/notifications`
    update_notifications => save_notifications
);
endpoint!(
    /// `PATCH /api/v1/settings/status`
    update_status => save_status
);
endpoint!(
    /// `POST /api/v1/settings/dnd_allowances/:user_id`
    allow_through_dnd => create_dnd_allowance
);
endpoint!(
    /// `DELETE /api/v1/settings/dnd_allowances/:user_id`
    disallow_through_dnd => destroy_dnd_allowance
);
endpoint!(
    /// `GET /api/v1/settings/sessions`
    sessions => index_sessions
);
endpoint!(
    /// `DELETE /api/v1/settings/sessions/:id`
    revoke_session => destroy_session
);
endpoint!(
    /// `POST /api/v1/settings/sessions/revoke_others`
    revoke_other_sessions => destroy_other_sessions
);
endpoint!(
    /// `GET /api/v1/settings/push_subscriptions`
    push_subscriptions => index_push_subscriptions
);
endpoint!(
    /// `DELETE /api/v1/settings/push_subscriptions/:id`
    remove_push_subscription => destroy_push_subscription
);

async fn before_actions(c: &mut Ctx) -> Result<()> {
    concerns::before_actions(
        c,
        Before {
            authentication: Authentication::JsonUnauthorized,
            ..Before::default()
        },
    )
    .await
}

/// The signed-in person, as the before-actions loaded them.
async fn viewer(c: &mut Ctx) -> Result<User> {
    before_actions(c).await?;
    Ok(concerns::require_current_user(c)?.clone())
}

/// The JSON body as `T`; anything else is a 422.
async fn body<T: DeserializeOwned>(c: &mut Ctx) -> Result<T> {
    let bytes = c.read_body(BODY_LIMIT).await;
    serde_json::from_slice(&bytes).map_err(|error| {
        fail(
            c,
            api::ApiError::Validation {
                message: format!("The request body isn't valid: {error}"),
                fields: Default::default(),
            },
        )
    })
}

/// A rejected change, in the envelope the database's validation errors take.
fn invalid(errors: Errors) -> Error {
    Error::internal(campfire_db::Error::RecordInvalid(errors))
}

/// `two_factor#audit_context`: who did it, from where.
fn audit_context(c: &Ctx) -> Result<Context> {
    Ok(Context {
        actor: concerns::current_user(c).map(Actor::from),
        ip_address: Some(c.request.remote_ip()?.to_string()),
        user_agent: c.request.user_agent().map(str::to_string),
    })
}

/// The answer to a read and to every settings write: the settings as they now are.
async fn reply(c: &mut Ctx, id: i64) -> Result {
    let settings = load(c, id).await?;
    c.json(StatusCode::OK, &settings)
}

async fn show_settings(c: &mut Ctx) -> Result {
    let user = viewer(c).await?;
    reply(c, user.id).await
}

/// Every section of the classic profile page, from the presenters that page renders from.
async fn load(c: &mut Ctx, id: i64) -> Result<api::Settings> {
    let now = c.now();
    let secrets = c.app().secrets.clone();
    let (user, status, sections, appearance, avatar_attached) = c
        .app()
        .db
        .read(move |conn| {
            let user = User::find(conn, id)?;
            let status = UserStatusSettings::find(conn, id)?;
            let sections = profile_sections::load(conn, id, now)?;
            let appearance = profile_settings::appearance(conn, id)?;
            let attached = attachments::attached_blob(conn, "User", id, "avatar")?.is_some();
            Ok((user, status, sections, appearance, attached))
        })
        .await
        .map_err(db_error)?;
    let fizzy = presenters::fizzy_profile::connection(c.app(), id)
        .await
        .map_err(Error::internal)?;
    let github = presenters::github::connection(c.app(), id)
        .await
        .map_err(Error::internal)?;
    let google = &sections.google;
    let now = campfire_db::Timestamp::from_jiff(now);
    Ok(api::Settings {
        profile: api::ProfileSettings {
            user_id: user.id,
            name: user.name.clone(),
            email_address: user.email_address.clone(),
            bio: user.bio.clone(),
            avatar_url: presenters::avatar_path(&secrets, &user),
            avatar_attached,
            has_password: user
                .password_digest
                .as_deref()
                .is_some_and(|digest| !campfire_richtext::ruby::is_blank(digest)),
            github_login: sections.github_login.clone(),
            github_verified: sections.github_verified,
            bot: user.is_bot(),
        },
        appearance: api::AppearanceSettings {
            theme: theme(&appearance.theme),
            text_size: text_size(&appearance.text_size),
            time_zone: appearance
                .time_zone
                .as_deref()
                .and_then(profile_settings::zone_identifier),
            time_zones: campfire_views::users::profile_time_zones()
                .iter()
                .map(|(label, value)| api::TimeZoneChoice {
                    label: label.clone(),
                    value: value.clone(),
                })
                .collect(),
        },
        notifications: api::NotificationSettings {
            dnd_enabled: sections.notifications.manual_dnd,
            quiet_hours_enabled: sections.notifications.quiet_hours,
            quiet_hours_start: sections.notifications.quiet_start.clone(),
            quiet_hours_end: sections.notifications.quiet_end.clone(),
            meeting_dnd_enabled: sections.notifications.meeting_dnd,
            ooo_notify_enabled: sections.notifications.ooo_notify,
            allowed_people: sections
                .notifications
                .allowed_people
                .iter()
                .map(|(user_id, name)| api::DndAllowedPerson {
                    user_id: *user_id,
                    name: name.clone(),
                })
                .collect(),
            keyword_alerts: sections
                .notifications
                .keywords
                .lines()
                .map(str::to_string)
                .collect(),
            inbox: sections
                .inbox
                .iter()
                .map(|switch| api::InboxSwitch {
                    key: switch.key.clone(),
                    label: switch.label.to_string(),
                    description: switch.description.to_string(),
                    enabled: switch.enabled,
                })
                .collect(),
        },
        status: api::StatusSettings {
            presence_setting: match status.presence_setting.as_str() {
                "dnd" => api::PresenceSetting::Dnd,
                "invisible" => api::PresenceSetting::Invisible,
                _ => api::PresenceSetting::Auto,
            },
            custom_status_emoji: status.custom_status_emoji.clone(),
            custom_status_text: status.custom_status_text.clone(),
            custom_status_expires_at: status.custom_status_expires_at.map(time),
            meeting_status_enabled: status.meeting_status_enabled,
            ooo_calendar_enabled: status.ooo_calendar_enabled,
            ooo_until: status.ooo_until_effective(now).map(time),
            ooo_manual: status.manual_ooo_active(now),
            ooo_note: status.ooo_note.clone(),
            calendar_error: sections.status.fetch_error.clone(),
        },
        calls: api::CallSettings {
            voice_mode: match sections.voice_mode.as_str() {
                "push_to_talk" => api::VoiceMode::PushToTalk,
                _ => api::VoiceMode::VoiceActivity,
            },
            push_to_talk_key: sections.push_to_talk_key.clone(),
        },
        integrations: api::IntegrationSettings {
            google: api::GoogleIntegration {
                sign_in_configured: c.app().google.sign_in().config.configured(),
                identity_email: google.identity_email.clone(),
                calendar_configured: c.app().google.api().config.configured(),
                connected: google.connected,
                calendar: google.calendar,
                drive: google.drive,
                email: google.account_exists.then(|| google.email.clone()),
            },
            github: if github.usable {
                api::Connection::Connected {
                    name: github.login.clone(),
                    workspace: None,
                    app_token: github.app_token,
                }
            } else if github.linked {
                api::Connection::Rejected {
                    reason: github.reason.clone(),
                }
            } else {
                api::Connection::Missing
            },
            github_app_configured: github.app_configured,
            fizzy: connection(fizzy),
            manage_path: "/users/me/profile".into(),
            slack_import_path: "/slack/imports".into(),
        },
    })
}

fn connection(panel: ConnectionPanel) -> api::Connection {
    match panel {
        ConnectionPanel::Missing => api::Connection::Missing,
        ConnectionPanel::Rejected { reason } => api::Connection::Rejected { reason },
        ConnectionPanel::Connected {
            name,
            workspace,
            app_token,
        } => api::Connection::Connected {
            name,
            workspace,
            app_token,
        },
    }
}

fn theme(value: &str) -> api::Theme {
    match value {
        "light" => api::Theme::Light,
        "dark" => api::Theme::Dark,
        _ => api::Theme::System,
    }
}

fn text_size(value: &str) -> api::TextSize {
    match value {
        "smaller" => api::TextSize::Smaller,
        "small" => api::TextSize::Small,
        "large" => api::TextSize::Large,
        "larger" => api::TextSize::Larger,
        _ => api::TextSize::Default,
    }
}

fn theme_value(theme: api::Theme) -> String {
    match theme {
        api::Theme::Light => "light",
        api::Theme::Dark => "dark",
        api::Theme::System => "system",
    }
    .into()
}

fn text_size_value(size: api::TextSize) -> String {
    match size {
        api::TextSize::Smaller => "smaller",
        api::TextSize::Small => "small",
        api::TextSize::Default => "default",
        api::TextSize::Large => "large",
        api::TextSize::Larger => "larger",
    }
    .into()
}

fn voice_mode_value(mode: api::VoiceMode) -> String {
    match mode {
        api::VoiceMode::VoiceActivity => "voice_activity",
        api::VoiceMode::PushToTalk => "push_to_talk",
    }
    .into()
}

/// The write `users/profiles#update` makes: the user's columns (with the email and password
/// audits) and the profile settings, in one transaction.
async fn write_profile(
    c: &mut Ctx,
    user: User,
    changes: UserChanges,
    settings: profile_settings::Changes,
    email_changing: bool,
    password_changing: bool,
) -> Result<()> {
    let audit = audit_context(c)?;
    c.app()
        .db
        .write(move |tx| {
            let mut user = user;
            authentication::update_profile(
                tx,
                &mut user,
                changes,
                email_changing,
                password_changing,
                &audit,
            )?;
            profile_settings::update(tx, user.id, settings)
        })
        .await
        .map_err(Error::internal)
}

async fn save_profile(c: &mut Ctx) -> Result {
    let user = viewer(c).await?;
    let update: api::UpdateProfile = body(c).await?;
    let id = user.id;
    // A linked GitHub account owns the username.
    let github_verified = c
        .app()
        .db
        .read(move |conn| {
            Ok(
                campfire_app::integrations::github::accounts::Account::for_user(conn, id)?
                    .is_some_and(|account| account.connected()),
            )
        })
        .await
        .map_err(db_error)?;
    let email_changing = update
        .email_address
        .as_deref()
        .is_some_and(|email| user.email_change_requested(email));
    if email_changing {
        let password = update.current_password.clone();
        let missing = password
            .as_deref()
            .is_none_or(campfire_richtext::ruby::is_blank);
        let existing = user.clone();
        let confirmed = tokio::task::spawn_blocking(move || {
            existing.current_password_confirmed(&password.unwrap_or_default())
        })
        .await
        .map_err(Error::internal)?;
        if !confirmed {
            let message = if missing {
                "is required to change your email address"
            } else {
                "is incorrect"
            };
            return Err(fail(c, validation("currentPassword", message)));
        }
    }
    let password_changing = update
        .password
        .as_deref()
        .is_some_and(|password| !campfire_richtext::ruby::is_blank(password));
    let changes = UserChanges {
        name: update.name,
        email_address: update.email_address.map(Some),
        // `password=` ignores a blank password.
        password_digest: concerns::password_digest(
            c,
            update.password.filter(|password| !password.is_empty()),
        )
        .await?,
        bio: update.bio.map(Some),
        ..UserChanges::default()
    };
    let settings = profile_settings::Changes {
        github_login: if github_verified {
            None
        } else {
            update.github_login
        },
        ..profile_settings::Changes::default()
    };
    write_profile(
        c,
        user,
        changes,
        settings,
        email_changing,
        password_changing,
    )
    .await?;
    reply(c, id).await
}

async fn attach_avatar(c: &mut Ctx) -> Result {
    let user = viewer(c).await?;
    let update: api::UpdateAvatar = body(c).await?;
    let verified = campfire_storage::paths::verify_signed_blob_id(
        &*c.app().storage.verifier,
        &update.signed_id,
        c.app().clock.now(),
    );
    if verified.is_none() {
        return Err(fail(c, validation("signedId", "isn't an uploaded file")));
    }
    assign_avatar(c, user.id, Assignment::Signed(update.signed_id)).await
}

async fn detach_avatar(c: &mut Ctx) -> Result {
    let user = viewer(c).await?;
    assign_avatar(c, user.id, Assignment::Delete).await
}

async fn assign_avatar(c: &mut Ctx, id: i64, avatar: Assignment) -> Result {
    let avatar = avatar.stage(c.app()).await?;
    c.app()
        .db
        .write(move |tx| attachments::assign(tx, Record::user(id), "avatar", avatar))
        .await
        .map_err(Error::internal)?;
    reply(c, id).await
}

async fn save_appearance(c: &mut Ctx) -> Result {
    let user = viewer(c).await?;
    let update: api::UpdateAppearance = body(c).await?;
    let id = user.id;
    let changes = UserChanges {
        time_zone_explicit: update.time_zone.is_some().then_some(true),
        time_zone: update.time_zone.clone().map(Some),
        ..UserChanges::default()
    };
    let settings = profile_settings::Changes {
        theme: update.theme.map(theme_value),
        text_size: update.text_size.map(text_size_value),
        time_zone: update.time_zone,
        ..profile_settings::Changes::default()
    };
    write_profile(c, user, changes, settings, false, false).await?;
    reply(c, id).await
}

async fn save_calls(c: &mut Ctx) -> Result {
    let user = viewer(c).await?;
    let update: api::UpdateCalls = body(c).await?;
    let id = user.id;
    let settings = profile_settings::Changes {
        voice_mode: update.voice_mode.map(voice_mode_value),
        push_to_talk_key: update.push_to_talk_key,
        ..profile_settings::Changes::default()
    };
    write_profile(c, user, UserChanges::default(), settings, false, false).await?;
    reply(c, id).await
}

async fn save_notifications(c: &mut Ctx) -> Result {
    let user = viewer(c).await?;
    let update: api::UpdateNotifications = body(c).await?;
    let id = user.id;
    let mut settings = c
        .app()
        .db
        .read(move |conn| UserStatusSettings::find(conn, id))
        .await
        .map_err(db_error)?;
    for (value, field) in [
        (update.dnd_enabled, &mut settings.dnd_enabled),
        (
            update.quiet_hours_enabled,
            &mut settings.quiet_hours_enabled,
        ),
        (
            update.meeting_dnd_enabled,
            &mut settings.meeting_dnd_enabled,
        ),
        (update.ooo_notify_enabled, &mut settings.ooo_notify_enabled),
    ] {
        if let Some(value) = value {
            *field = value;
        }
    }
    if let Some(start) = &update.quiet_hours_start {
        settings.quiet_hours_start_minute = clock_time_to_minutes(start);
    }
    if let Some(end) = &update.quiet_hours_end {
        settings.quiet_hours_end_minute = clock_time_to_minutes(end);
    }
    if update.dnd_enabled.is_some() {
        settings.reconcile_dnd_timer(c.app().db.env().now());
    }
    // The classic form posts each switch as "1" or "0"; store what it stores.
    let inbox = update.inbox.map(|switches| {
        serde_json::Value::Object(
            switches
                .into_iter()
                .filter(|(key, _)| INBOX_KEYS.contains(&key.as_str()))
                .map(|(key, on)| (key, serde_json::Value::from(if on { "1" } else { "0" })))
                .collect(),
        )
    });
    let keywords = update.keyword_alerts;
    c.app()
        .db
        .write(move |tx| {
            settings.save_with_keywords(tx, keywords.as_deref())?;
            if inbox.is_some() {
                profile_settings::update(
                    tx,
                    id,
                    profile_settings::Changes {
                        inbox_preferences: inbox,
                        ..profile_settings::Changes::default()
                    },
                )?;
            }
            Ok(())
        })
        .await
        .map_err(Error::internal)?;
    reply(c, id).await
}

async fn save_status(c: &mut Ctx) -> Result {
    let user = viewer(c).await?;
    let update: api::UpdateStatus = body(c).await?;
    let id = user.id;
    let mut settings = c
        .app()
        .db
        .read(move |conn| UserStatusSettings::find(conn, id))
        .await
        .map_err(db_error)?;
    let original = (
        settings.presence_setting.clone(),
        settings.custom_status_emoji.clone(),
        settings.custom_status_text.clone(),
        settings.custom_status_expires_at,
    );
    let now = c.app().db.env().now();
    if let Some(presence) = update.presence_setting {
        settings.presence_setting = match presence {
            api::PresenceSetting::Auto => "auto",
            api::PresenceSetting::Dnd => "dnd",
            api::PresenceSetting::Invisible => "invisible",
        }
        .into();
    }
    if let Some(emoji) = update.custom_status_emoji {
        settings.custom_status_emoji = Some(emoji);
    }
    if let Some(text) = update.custom_status_text {
        settings.custom_status_text = Some(text);
    }
    if let Some(expiry) = update.custom_status_expires_in {
        let preset = match expiry {
            api::StatusExpiry::Minutes30 => "minutes_30",
            api::StatusExpiry::Hour1 => "hour_1",
            api::StatusExpiry::Hours4 => "hours_4",
            api::StatusExpiry::Today => "today",
            api::StatusExpiry::Week => "week",
            api::StatusExpiry::Never => "never",
        };
        settings
            .set_custom_status_expires_in(preset, now)
            .map_err(Error::internal)?;
    }
    if let Some(on) = update.meeting_status_enabled {
        settings.meeting_status_enabled = on;
    }
    if let Some(on) = update.ooo_calendar_enabled {
        settings.ooo_calendar_enabled = on;
    }
    let present = |value: Option<String>| value.filter(|s| !campfire_richtext::ruby::is_blank(s));
    if update.clear_custom_status == Some(true) {
        settings.custom_status_emoji = None;
        settings.custom_status_text = None;
        settings.custom_status_expires_at = None;
    }
    if update.clear_ooo == Some(true) {
        settings.ooo_until = None;
        settings.ooo_note = None;
    } else if let Some(preset) = update.ooo_preset {
        let preset = match preset {
            api::OooPreset::Tomorrow => "tomorrow",
            api::OooPreset::Monday => "monday",
            api::OooPreset::Week => "week",
            api::OooPreset::Custom => "custom",
        };
        let end = settings
            .ooo_preset_until(preset, update.ooo_until_custom.as_deref(), now)
            .map_err(Error::internal)?;
        match end {
            Some(end) if end > now => {
                settings.ooo_until = Some(end);
                settings.ooo_note = present(update.ooo_note);
            }
            _ => {
                let mut errors = Errors::default();
                errors.add("ooo_until", "needs a future date and time");
                return Err(invalid(errors));
            }
        }
    } else if let Some(note) = update.ooo_note {
        settings.ooo_note = present(Some(note));
    }
    // `broadcast_status_change` checks only the status attributes.
    let changed = original
        != (
            settings.presence_setting.clone(),
            settings.custom_status_emoji.clone(),
            settings.custom_status_text.clone(),
            settings.custom_status_expires_at,
        );
    c.app()
        .db
        .write(move |tx| {
            settings.save_status(tx)?;
            if changed {
                settings.announce_badge(tx)?;
            }
            Ok(())
        })
        .await
        .map_err(Error::internal)?;
    reply(c, id).await
}

async fn create_dnd_allowance(c: &mut Ctx) -> Result {
    change_dnd_allowance(c, true).await
}

async fn destroy_dnd_allowance(c: &mut Ctx) -> Result {
    change_dnd_allowance(c, false).await
}

/// `users/dnd_allowances`: an active human other than the person.
async fn change_dnd_allowance(c: &mut Ctx, create: bool) -> Result {
    let owner = viewer(c).await?.id;
    let Some(target) = c.param_str("user_id").and_then(concerns::cast_integer) else {
        return Err(fail(c, not_found()));
    };
    let person = c
        .app()
        .db
        .read(move |conn| User::find_by_id(conn, target))
        .await
        .map_err(db_error)?
        .filter(|person| person.is_active() && !person.is_bot());
    let Some(person) = person else {
        return Err(fail(c, not_found()));
    };
    if person.id == owner {
        return Err(fail(c, validation("userId", "can't be you")));
    }
    let result = c
        .app()
        .db
        .write(move |tx| {
            if create {
                DndAllowedUser::find_or_create(tx, owner, target)?;
            } else {
                DndAllowedUser::remove(tx, owner, target)?;
            }
            Ok(())
        })
        .await;
    if let Err(error) = result
        && !error.is_record_not_unique()
    {
        return Err(Error::internal(error));
    }
    reply(c, owner).await
}

/// The sessions page's headers: never cached.
async fn session_viewer(c: &mut Ctx) -> Result<User> {
    let user = viewer(c).await?;
    c.set_header("pragma", "no-cache");
    Ok(user)
}

async fn session_list(c: &mut Ctx, user: User, notice: Option<String>) -> Result {
    let current_id = current_session(c).map(|session| session.id);
    let now = campfire_db::Timestamp::from_jiff(c.now());
    let timeout = c.app().config.admin_session_idle_timeout;
    let sessions = c
        .app()
        .db
        .read(move |conn| authentication::visible_sessions(conn, &user, timeout, now))
        .await
        .map_err(db_error)?;
    let list = api::SessionList {
        sessions: sessions
            .into_iter()
            .map(|session| api::SessionInfo {
                id: session.id,
                current: Some(session.id) == current_id,
                description: authentication::device_description(&session),
                ip_address: session.ip_address.clone(),
                last_active_at: time(session.last_active_at),
                created_at: time(session.created_at),
            })
            .collect(),
        notice,
    };
    c.json(StatusCode::OK, &list)
}

async fn index_sessions(c: &mut Ctx) -> Result {
    let user = session_viewer(c).await?;
    session_list(c, user, None).await
}

async fn destroy_session(c: &mut Ctx) -> Result {
    let user = session_viewer(c).await?;
    let id = concerns::ruby_to_i(c.param_str("id").unwrap_or(""));
    let user_id = user.id;
    let session = c
        .app()
        .db
        .read(move |conn| {
            Ok(Session::for_user(conn, user_id)?
                .into_iter()
                .find(|session| session.id == id))
        })
        .await
        .map_err(db_error)?;
    let Some(session) = session else {
        return Err(fail(c, not_found()));
    };
    if current_session(c).is_some_and(|current| current.id == id) {
        // Signing this browser out, as the classic page does: its push subscription goes too.
        if let Some(endpoint) = c
            .param_str("push_subscription_endpoint")
            .map(str::to_string)
        {
            c.app()
                .db
                .write(move |tx| PushSubscription::destroy_by_endpoint(tx, user_id, &endpoint))
                .await
                .map_err(Error::internal)?;
        }
        concerns::terminate_current_session(c).await?;
        return Err(fail(
            c,
            api::ApiError::Unauthorized {
                message: "Signed out".into(),
            },
        ));
    }
    let context = audit_context(c)?;
    let revoker = user.clone();
    c.app()
        .db
        .write(move |tx| authentication::revoke_session(tx, &revoker, &session, &context))
        .await
        .map_err(Error::internal)?;
    session_list(c, user, Some("Signed out that session.".into())).await
}

async fn destroy_other_sessions(c: &mut Ctx) -> Result {
    let user = session_viewer(c).await?;
    let Some(current) = current_session(c).map(|session| session.id) else {
        return Err(fail(c, not_found()));
    };
    let context = audit_context(c)?;
    let revoker = user.clone();
    let count = c
        .app()
        .db
        .write(move |tx| authentication::revoke_other_sessions(tx, &revoker, current, &context))
        .await
        .map_err(Error::internal)?;
    let notice = match count {
        0 => "No other sessions to sign out.".to_string(),
        1 => "Signed out 1 other session.".to_string(),
        _ => format!("Signed out {count} other sessions."),
    };
    session_list(c, user, Some(notice)).await
}

async fn push_subscription_list(c: &mut Ctx, user_id: i64) -> Result {
    let subscriptions = c
        .app()
        .db
        .read(move |conn| PushSubscription::for_user(conn, user_id))
        .await
        .map_err(db_error)?;
    let list = api::PushSubscriptionList {
        push_subscriptions: subscriptions
            .iter()
            .map(|subscription| {
                let view = presenters::accounts::push_subscription(subscription);
                api::PushSubscriptionInfo {
                    id: view.id,
                    endpoint: view.endpoint,
                    browser: view.browser,
                    version: view.version,
                    platform: view.platform,
                }
            })
            .collect(),
    };
    c.json(StatusCode::OK, &list)
}

async fn index_push_subscriptions(c: &mut Ctx) -> Result {
    let user = viewer(c).await?;
    push_subscription_list(c, user.id).await
}

/// `@push_subscriptions.destroy_by(id: params[:id])`: someone else's or a gone one is a no-op.
async fn destroy_push_subscription(c: &mut Ctx) -> Result {
    let user_id = viewer(c).await?.id;
    if let Some(id) = c.param_str("id").and_then(concerns::cast_integer) {
        c.app()
            .db
            .write(move |tx| match PushSubscription::find(tx.conn(), id) {
                Ok(subscription) if subscription.user_id == user_id => subscription.destroy(tx),
                Ok(_) | Err(campfire_db::Error::RecordNotFound(_)) => Ok(()),
                Err(error) => Err(error),
            })
            .await
            .map_err(Error::internal)?;
    }
    push_subscription_list(c, user_id).await
}
