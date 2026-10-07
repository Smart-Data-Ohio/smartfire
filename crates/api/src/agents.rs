//! The S4 agent endpoints on `/api/v1` (`campfire_api_types::agents` documents each one): the
//! directory, an agent's profile, its approval requests, and a human's decision on one. Each reads
//! what the classic pages read (`agents/directory#index`, the bot branch of `users#show`,
//! `agents/approvals#for_agent`) and decides through `agent_approvals#update`'s own path, so
//! the authorization, the checks and the callbacks are the classic ones.

use std::collections::{BTreeSet, HashMap};

use campfire_api_types as api;
use campfire_app::app::{AppCtx, AppState};
use campfire_controllers::controllers::agent_approvals::{self as classic, Decided};
use campfire_db::models::agent_posting::{self, Cap};
use campfire_db::models::agent_profile::{self, DirectoryRecord};
use campfire_db::{Agent, AgentApproval, Connection, Membership, Room, Timestamp, User};
use campfire_kit::{Ctx, Error, Result, StatusCode};
use campfire_web::concerns::{self, AuthenticatedBy};
use campfire_web::controllers::presenters::page::db_error;
use campfire_web::controllers::presenters::{accounts, view_context};

use crate::dto;
use crate::endpoints::{before_actions, body, now};
use crate::error::{fail, validation};

endpoint!(
    /// `GET /api/v1/agents`
    index => list_agents
);
endpoint!(
    /// `GET /api/v1/agents/:agent_id`
    show => show_agent
);
endpoint!(
    /// `GET /api/v1/agents/:agent_id/approvals`
    approvals => list_approvals
);
endpoint!(
    /// `PATCH /api/v1/agent_approvals/:id`
    decide => decide_approval
);
endpoint!(
    /// `GET /api/v1/agents/:agent_id/events`
    events => list_events
);

/// `agents/approvals#for_agent`' page size.
const PAGE: usize = 50;

/// A stored word as its wire enum (the enums' serde names are the stored values).
fn wire<T: serde::de::DeserializeOwned>(value: &str) -> campfire_db::Result<T> {
    serde_json::from_value(serde_json::Value::String(value.to_owned())).map_err(|_| {
        campfire_db::Error::Other(format!("an agent value the contract doesn't know: {value}"))
    })
}

fn present(value: Option<&str>) -> Option<String> {
    value
        .filter(|value| !campfire_richtext::ruby::is_blank(value))
        .map(str::to_string)
}

/// The signed-in human, or `None` for a bot or an agent token (which never use this API).
pub(crate) fn human(c: &Ctx) -> Result<Option<User>> {
    let viewer = concerns::require_current_user(c)?;
    Ok((!viewer.is_bot()
        && !matches!(
            concerns::authenticated_by(c),
            AuthenticatedBy::AgentToken | AuthenticatedBy::BotKey
        ))
    .then(|| viewer.clone()))
}

fn agent_id(c: &Ctx, key: &str) -> Result<i64> {
    c.param_str(key)
        .and_then(concerns::cast_integer)
        .ok_or(Error::NotFound)
}

fn directory_row(record: &DirectoryRecord) -> api::AgentDirectoryRow {
    api::AgentDirectoryRow {
        agent_id: record.id,
        user_id: record.user.id,
        kind: dto::agent_kind(&record.kind),
        owner_id: record.owner.as_ref().map(|owner| owner.id),
        status: dto::agent_status(&record.status),
        status_note: record.status_note.clone(),
        suspended: record.suspended,
        created_at: dto::time(record.created_at),
        status_changed_at: record.status_changed_at.map(dto::time),
        last_seen_at: record.last_seen_at.map(dto::time),
    }
}

/// The directory row of one agent, whatever its user's state (as `directory_agent` reads it).
fn agent_row(agent: &Agent) -> api::AgentDirectoryRow {
    api::AgentDirectoryRow {
        agent_id: agent.id,
        user_id: agent.user_id,
        kind: dto::agent_kind(agent.kind.name()),
        owner_id: agent.owner_id,
        status: dto::agent_status(&agent.status),
        status_note: agent.status_note.clone(),
        suspended: agent.suspended_at.is_some(),
        created_at: dto::time(agent.created_at),
        status_changed_at: agent.status_changed_at.map(dto::time),
        last_seen_at: agent.last_seen_at.map(dto::time),
    }
}

async fn list_agents(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    if human(c)?.is_none() {
        return Err(fail(
            c,
            api::ApiError::Forbidden {
                message: "Forbidden".into(),
            },
        ));
    }
    let (secrets, now) = (c.app().secrets.clone(), now(c));
    let directory = c
        .app()
        .db
        .read(move |conn| {
            let records = agent_profile::for_directory(conn)?;
            let people = records
                .iter()
                .flat_map(|record| {
                    std::iter::once(record.user.id).chain(record.owner.as_ref().map(|o| o.id))
                })
                .collect::<BTreeSet<_>>();
            Ok(api::AgentDirectory {
                agents: records.iter().map(directory_row).collect(),
                users: dto::users(conn, &secrets, people, now)?,
            })
        })
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &directory)
}

/// The agent's rooms the viewer shares, by lower-cased name, and how many others it's in (the
/// classic profile's room list).
fn profile_rooms(
    conn: &Connection,
    agent: &Agent,
    viewer: &User,
) -> campfire_db::Result<(Vec<api::AgentProfileRoom>, i64)> {
    let mut rooms = Room::for_user(conn, agent.user_id)?;
    accounts::sort_by_lower_name(&mut rooms, |room| room.name.as_deref().unwrap_or(""));
    let total = rooms.len() as i64;
    let mut shared = Vec::new();
    for room in rooms {
        if Membership::find_by_room_and_user(conn, room.id, viewer.id)?.is_some() {
            shared.push(api::AgentProfileRoom {
                room_id: room.id,
                name: accounts::room_display_name(conn, &room, viewer)?,
            });
        }
    }
    let hidden = total - shared.len() as i64;
    Ok((shared, hidden))
}

fn grants(conn: &Connection, agent: &Agent) -> campfire_db::Result<api::AgentGrants> {
    Ok(match agent.grant_lines(conn)? {
        None => api::AgentGrants {
            legacy: true,
            grants: Vec::new(),
        },
        Some(lines) => api::AgentGrants {
            legacy: false,
            grants: lines
                .into_iter()
                .map(|line| {
                    Ok(api::AgentGrant {
                        capability: wire(line.capability)?,
                        workspace_wide: line.workspace_wide,
                        room_count: line.room_count,
                    })
                })
                .collect::<campfire_db::Result<_>>()?,
        },
    })
}

fn management(
    conn: &Connection,
    agent: &Agent,
    now: Timestamp,
    zone: &jiff::tz::TimeZone,
) -> campfire_db::Result<api::AgentManagement> {
    let counts = agent.activity_counts(conn, now)?;
    let window = agent_posting::daily_window(now, zone)?;
    let budget_usage = [
        (
            Cap::Messages,
            api::AgentBudgetCap::Messages,
            agent.daily_message_cap,
        ),
        (
            Cap::BoardPosts,
            api::AgentBudgetCap::BoardPosts,
            agent.daily_board_post_cap,
        ),
        (
            Cap::ExternalActions,
            api::AgentBudgetCap::ExternalActions,
            agent.daily_external_action_cap,
        ),
    ]
    .into_iter()
    .map(|(cap, wire_cap, limit)| {
        Ok(api::AgentBudgetUsage {
            cap: wire_cap,
            used: agent_posting::cap_usage(conn, agent.id, agent.user_id, cap, &window)?,
            limit,
        })
    })
    .collect::<campfire_db::Result<_>>()?;
    Ok(api::AgentManagement {
        activity_summary: api::AgentActivitySummary {
            delivered: counts.delivered,
            acknowledged: counts.acknowledged,
            posted: counts.posted,
            suppressed: counts.suppressed,
        },
        budget_usage,
    })
}

async fn show_agent(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let Some(viewer) = human(c)? else {
        return Err(Error::NotFound);
    };
    let id = agent_id(c, "agent_id")?;
    let zone = view_context::time_zone(c).await?;
    let (secrets, now) = (c.app().secrets.clone(), now(c));
    let profile = c
        .app()
        .db
        .read(move |conn| {
            let Some(agent) = Agent::find(conn, id)? else {
                return Ok(None);
            };
            let manages = viewer.is_administrator() || agent.owner_id == Some(viewer.id);
            let (rooms, hidden_room_count) = profile_rooms(conn, &agent, &viewer)?;
            Ok(Some(api::AgentProfile {
                agent: agent_row(&agent),
                provider: present(agent.provider.as_deref()),
                runtime: present(agent.runtime.as_deref()),
                description: present(agent.description.as_deref()),
                rooms,
                hidden_room_count,
                grants: manages.then(|| grants(conn, &agent)).transpose()?,
                management: manages
                    .then(|| management(conn, &agent, now, zone.tz()))
                    .transpose()?,
                users: dto::users(
                    conn,
                    &secrets,
                    std::iter::once(agent.user_id).chain(agent.owner_id),
                    now,
                )?,
            }))
        })
        .await
        .map_err(db_error)?
        .ok_or(Error::NotFound)?;
    c.json(StatusCode::OK, &profile)
}

/// Builds [`api::AgentApproval`]s for one viewer: their room names and their right to approve
/// (`approvable_by`, read once per kind of action) and deny (`decidable_by`).
pub(crate) struct ApprovalCards<'a> {
    conn: &'a Connection,
    viewer: &'a User,
    rooms: HashMap<i64, Option<String>>,
    approvable: HashMap<(i64, bool), bool>,
    deniable: HashMap<i64, bool>,
}

impl<'a> ApprovalCards<'a> {
    pub(crate) fn new(conn: &'a Connection, viewer: &'a User) -> Self {
        Self {
            conn,
            viewer,
            rooms: HashMap::new(),
            approvable: HashMap::new(),
            deniable: HashMap::new(),
        }
    }

    /// The viewer-relative room name, only for an administrator or a member of the room (the
    /// contract's gate: an owner outside the room sees no name).
    fn room_name(&mut self, room_id: i64) -> campfire_db::Result<Option<String>> {
        if let Some(name) = self.rooms.get(&room_id) {
            return Ok(name.clone());
        }
        let name = match Room::find_by_id(self.conn, room_id)? {
            Some(room)
                if self.viewer.is_administrator()
                    || Membership::find_by_room_and_user(self.conn, room_id, self.viewer.id)?
                        .is_some() =>
            {
                Some(accounts::room_display_name(self.conn, &room, self.viewer)?)
            }
            _ => None,
        };
        self.rooms.insert(room_id, name.clone());
        Ok(name)
    }

    pub(crate) fn card(
        &mut self,
        approval: &AgentApproval,
        agent_user_id: i64,
        now: Timestamp,
    ) -> campfire_db::Result<api::AgentApproval> {
        let admin_only = approval.github_action() || approval.fizzy_action();
        let approvable = match self.approvable.get(&(approval.agent_id, admin_only)) {
            Some(allowed) => *allowed,
            None => {
                let allowed = approval.approvable_by(self.conn, self.viewer)?;
                self.approvable
                    .insert((approval.agent_id, admin_only), allowed);
                allowed
            }
        };
        let deniable = match self.deniable.get(&approval.agent_id) {
            Some(allowed) => *allowed,
            None => {
                let allowed = approval.decidable_by(self.conn, self.viewer)?;
                self.deniable.insert(approval.agent_id, allowed);
                allowed
            }
        };
        let room_name = match approval.room_id {
            Some(room_id) => self.room_name(room_id)?,
            None => None,
        };
        Ok(api::AgentApproval {
            id: approval.id,
            agent_id: approval.agent_id,
            agent_user_id,
            room_id: approval.room_id,
            room_name,
            action: approval.action.clone(),
            summary: approval.summary.clone(),
            status: wire(approval.effective_status(now))?,
            expires_at: dto::time(approval.expires_at),
            created_at: dto::time(approval.created_at),
            decided_by_id: approval.decided_by_id,
            decided_at: approval.decided_at.map(dto::time),
            decision_note: present(approval.decision_note.as_deref()),
            github_login: present(approval.github_login.as_deref()),
            fizzy_user_name: present(approval.fizzy_user_name.as_deref()),
            admin_only,
            approvable,
            deniable,
        })
    }
}

/// The approvals page's audience: an active human administrator or the agent's owner, while the
/// agent's user is active. Anyone else gets a 404, as in the classic app.
fn manages_approvals(conn: &Connection, agent: &Agent, viewer: &User) -> campfire_db::Result<bool> {
    let Some(bot) = User::find_by_id(conn, agent.user_id)? else {
        return Ok(false);
    };
    Ok(bot.is_active()
        && viewer.is_active()
        && !viewer.is_bot()
        && (viewer.is_administrator() || agent.owner_id == Some(viewer.id)))
}

async fn list_approvals(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let Some(viewer) = human(c)? else {
        return Err(Error::NotFound);
    };
    let id = agent_id(c, "agent_id")?;
    let filter = c
        .param_str("status")
        .filter(|status| campfire_db::models::agent_approval::STATUSES.contains(status))
        .map(str::to_owned);
    let before = match c.param_str("before").filter(|raw| !raw.is_empty()) {
        None => None,
        Some(raw) => match crate::cursor::decode_id(raw) {
            Some(id) => Some(id),
            None => return Err(fail(c, validation("before", "is invalid"))),
        },
    };
    let lookup = viewer.clone();
    let agent = c
        .app()
        .db
        .read(move |conn| {
            let Some(agent) = Agent::find(conn, id)? else {
                return Ok(None);
            };
            Ok(manages_approvals(conn, &agent, &lookup)?.then_some(agent))
        })
        .await
        .map_err(db_error)?
        .ok_or(Error::NotFound)?;
    let status = filter.clone();
    let selected_at = now(c);
    let mut rows = c
        .app()
        .db
        .read(move |conn| {
            AgentApproval::history_page_before(conn, id, status.as_deref(), selected_at, before)
        })
        .await
        .map_err(db_error)?;
    let more = rows.len() > PAGE;
    rows.truncate(PAGE);
    let next_cursor = rows
        .last()
        .filter(|_| more)
        .map(|row| crate::cursor::encode_id(row.id));
    // Listing settles the overdue requests on the page first, as the classic page does: the
    // guarded expiry write reloads each one in the writer.
    if rows
        .iter()
        .any(|row| row.status == "pending" && row.expires_at <= selected_at)
    {
        rows = c
            .app()
            .db
            .write(move |tx| {
                for row in &mut rows {
                    if row.status == "pending" && row.expires_at <= tx.now() {
                        row.expire_if_due(tx)?;
                    }
                }
                Ok(rows)
            })
            .await
            .map_err(db_error)?;
    }
    let now = now(c);
    if let Some(status) = filter
        .as_deref()
        .filter(|status| matches!(*status, "pending" | "expired"))
    {
        rows.retain(|row| row.effective_status(now) == status);
    }
    let secrets = c.app().secrets.clone();
    let page = c
        .app()
        .db
        .read(move |conn| {
            let mut cards = ApprovalCards::new(conn, &viewer);
            let approvals = rows
                .iter()
                .map(|row| cards.card(row, agent.user_id, now))
                .collect::<campfire_db::Result<Vec<_>>>()?;
            let people = std::iter::once(agent.user_id)
                .chain(rows.iter().filter_map(|row| row.decided_by_id))
                .collect::<BTreeSet<_>>();
            Ok(api::AgentApprovalPage {
                approvals,
                users: dto::users(conn, &secrets, people, now)?,
                next_cursor,
            })
        })
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &page)
}

/// One approval as `viewer` sees it, with the page's `users`; `None` once they may not decide it.
pub(crate) fn approval_update(
    conn: &Connection,
    app: &AppState,
    approval_id: i64,
    viewer: &User,
    now: Timestamp,
) -> campfire_db::Result<Option<api::ApprovalUpdated>> {
    let Some(approval) = AgentApproval::find(conn, approval_id)? else {
        return Ok(None);
    };
    let Some(agent) = Agent::find(conn, approval.agent_id)? else {
        return Ok(None);
    };
    if !manages_approvals(conn, &agent, viewer)? {
        return Ok(None);
    }
    let card = ApprovalCards::new(conn, viewer).card(&approval, agent.user_id, now)?;
    Ok(Some(api::ApprovalUpdated {
        users: dto::users(
            conn,
            &app.secrets,
            std::iter::once(agent.user_id).chain(approval.decided_by_id),
            now,
        )?,
        approval: card,
    }))
}

async fn decide_approval(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let Some(viewer) = human(c)? else {
        return Err(Error::NotFound);
    };
    let id = agent_id(c, "id")?;
    let request: api::DecideApproval = match body(c).await {
        Ok(request) => request,
        // An unknown decision doesn't decode: the classic check's message, on `decision`.
        Err(_) => {
            return Err(fail(
                c,
                api::ApiError::Validation {
                    message: "Decision must be approved or denied".into(),
                    fields: [(
                        "decision".to_string(),
                        vec!["must be approved or denied".to_string()],
                    )]
                    .into(),
                },
            ));
        }
    };
    let decision = match request.decision {
        api::ApprovalDecision::Approved => "approved",
        api::ApprovalDecision::Denied => "denied",
    };
    match classic::decide(c, viewer.clone(), id, decision, request.note).await? {
        Decided::NotFound => Err(Error::NotFound),
        Decided::Refused {
            status, message, ..
        } => Err(fail(
            c,
            if status == StatusCode::FORBIDDEN {
                api::ApiError::Forbidden { message }
            } else {
                api::ApiError::Validation {
                    message: message.clone(),
                    fields: [("base".to_string(), vec![message])].into(),
                }
            },
        )),
        Decided::Applied { approval, .. } => {
            let (app, now) = (c.app().clone(), now(c));
            let id = approval.id;
            let updated = c
                .app()
                .db
                .read(move |conn| approval_update(conn, &app, id, &viewer, now))
                .await
                .map_err(db_error)?
                .ok_or(Error::NotFound)?;
            c.json(StatusCode::OK, &updated.approval)
        }
    }
}

/// The ledger's outcome filter (`agents/history#ledger`).
const OUTCOMES: [&str; 4] = ["pending", "delivered", "acknowledged", "suppressed"];

/// `history`'s 140-character cut, ending "..." when cut.
fn cut(text: &str) -> String {
    campfire_views::helpers::application::truncate(text, 140, "...")
}

fn metadata_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// What one viewer may read of an agent's ledger entries, per room.
struct LedgerGates<'a> {
    conn: &'a Connection,
    agent: &'a Agent,
    viewer: &'a User,
    rooms: HashMap<i64, Option<Room>>,
    member: HashMap<i64, bool>,
    readable: HashMap<i64, bool>,
}

impl LedgerGates<'_> {
    fn room(&mut self, room_id: i64) -> campfire_db::Result<Option<Room>> {
        if let Some(room) = self.rooms.get(&room_id) {
            return Ok(room.clone());
        }
        let room = Room::find_by_id(self.conn, room_id)?;
        self.rooms.insert(room_id, room.clone());
        Ok(room)
    }

    /// An administrator, or a member of the room: the contract's gate on the room-scoped text.
    fn sees_room(&mut self, room_id: i64) -> campfire_db::Result<bool> {
        if self.viewer.is_administrator() {
            return Ok(true);
        }
        if let Some(member) = self.member.get(&room_id) {
            return Ok(*member);
        }
        let member =
            Membership::find_by_room_and_user(self.conn, room_id, self.viewer.id)?.is_some();
        self.member.insert(room_id, member);
        Ok(member)
    }

    /// The classic content gate: the agent is in the message's room with `read_messages` there,
    /// and the viewer sees the room.
    fn reads_content(&mut self, room_id: i64) -> campfire_db::Result<bool> {
        if let Some(allowed) = self.readable.get(&room_id) {
            return Ok(*allowed);
        }
        let allowed = Membership::find_by_room_and_user(self.conn, room_id, self.agent.user_id)?
            .is_some()
            && self.agent.can(self.conn, "read_messages", Some(room_id))?
            && self.sees_room(room_id)?;
        self.readable.insert(room_id, allowed);
        Ok(allowed)
    }
}

fn ledger_event(
    gates: &mut LedgerGates<'_>,
    event: campfire_db::models::agent_delivery::AgentEvent,
    rich_text: &dyn campfire_db::RichText,
) -> campfire_db::Result<Option<api::AgentLedgerEvent>> {
    let Ok(event_type) = wire::<api::AgentLedgerEventType>(&event.event_type) else {
        tracing::warn!(id = event.id, event_type = %event.event_type, "ledger: unknown event type");
        return Ok(None);
    };
    let outcome = event
        .outcome
        .as_deref()
        .and_then(|outcome| wire::<api::AgentDeliveryOutcome>(outcome).ok());
    let webhook_status = wire::<api::AgentWebhookStatus>(&event.webhook_status)
        .unwrap_or(api::AgentWebhookStatus::None);
    // Entries with no room are never gated.
    let (room_name, open) = match event.room_id {
        None => (None, true),
        Some(room_id) => {
            let open = gates.sees_room(room_id)?;
            let name = match gates.room(room_id)? {
                Some(room) if open => Some(accounts::room_display_name(
                    gates.conn,
                    &room,
                    gates.viewer,
                )?),
                _ => None,
            };
            (name, open)
        }
    };
    let content = match event
        .message_id
        .map(|id| campfire_db::Message::find_by_id(gates.conn, id))
        .transpose()?
        .flatten()
    {
        Some(message) if gates.reads_content(message.room_id)? => {
            Some(cut(&message.plain_text_body(gates.conn, rich_text)?))
        }
        _ => None,
    };
    let text = |key: &str| {
        event
            .metadata
            .get(key)
            .filter(|value| !value.is_null())
            .map(metadata_text)
    };
    let external = (matches!(
        event_type,
        api::AgentLedgerEventType::GithubActionCompleted
            | api::AgentLedgerEventType::FizzyActionCompleted
    ) && event.metadata.is_object())
    .then(|| api::AgentExternalResult {
        action: text("action"),
        status: text("status"),
        message: present(text("message").as_deref()).filter(|_| open),
    });
    let handoff_summary = (event_type == api::AgentLedgerEventType::WorkHandedOff && open)
        .then(|| {
            event
                .metadata
                .get("handoff")
                .filter(|handoff| handoff.is_object())
                .map(|handoff| {
                    cut(&handoff
                        .get("summary")
                        .map(metadata_text)
                        .unwrap_or_default())
                })
        })
        .flatten();
    Ok(Some(api::AgentLedgerEvent {
        id: event.id,
        event_type,
        outcome,
        created_at: dto::time(event.created_at),
        room_id: event.room_id,
        room_name,
        actor_id: event.actor_id,
        message_id: event.message_id,
        hop: event.hop(),
        detail: present(event.detail.as_deref()).filter(|_| open),
        webhook_status,
        webhook_attempts: event.webhook_attempts,
        webhook_last_error: present(event.webhook_last_error.as_deref()),
        external,
        handoff_summary,
        content,
    }))
}

async fn list_events(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let Some(viewer) = human(c)? else {
        return Err(Error::NotFound);
    };
    let id = agent_id(c, "agent_id")?;
    let agent = c
        .app()
        .db
        .read(move |conn| Agent::find(conn, id))
        .await
        .map_err(db_error)?
        .ok_or(Error::NotFound)?;
    if !viewer.is_administrator() && agent.owner_id != Some(viewer.id) {
        return Err(fail(
            c,
            api::ApiError::Forbidden {
                message: "Forbidden".into(),
            },
        ));
    }
    let outcome = c
        .param_str("outcome")
        .filter(|outcome| OUTCOMES.contains(outcome))
        .map(str::to_owned);
    let before = match c.param_str("before").filter(|raw| !raw.is_empty()) {
        None => None,
        Some(raw) => match crate::cursor::decode(raw) {
            Some(key) => Some(key),
            None => return Err(fail(c, validation("before", "is invalid"))),
        },
    };
    let (secrets, now) = (c.app().secrets.clone(), now(c));
    let rich_text = c.app().db.env().rich_text.clone();
    let page = c
        .app()
        .db
        .read(move |conn| {
            let mut rows = campfire_db::models::agent_delivery::AgentEvent::history_page_before(
                conn,
                agent.id,
                outcome.as_deref(),
                before,
            )?;
            let more = rows.len() > PAGE;
            rows.truncate(PAGE);
            let next_cursor = rows
                .last()
                .filter(|_| more)
                .map(|row| crate::cursor::encode(row.created_at, row.id));
            let people = std::iter::once(agent.user_id)
                .chain(rows.iter().filter_map(|row| row.actor_id))
                .collect::<BTreeSet<_>>();
            let mut gates = LedgerGates {
                conn,
                agent: &agent,
                viewer: &viewer,
                rooms: HashMap::new(),
                member: HashMap::new(),
                readable: HashMap::new(),
            };
            let mut events = Vec::with_capacity(rows.len());
            for row in rows {
                if let Some(event) = ledger_event(&mut gates, row, &*rich_text)? {
                    events.push(event);
                }
            }
            Ok(api::AgentLedgerPage {
                events,
                users: dto::users(conn, &secrets, people, now)?,
                next_cursor,
            })
        })
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &page)
}
