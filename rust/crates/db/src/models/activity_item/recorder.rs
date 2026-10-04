//! app/services/activity_items/recorder.rb. Known sources authorize recipients;
//! the board opener caller may bypass the source check after authorizing its roster.
use crate::{ActivityItem, Error, Result, Tx, User, WorkThreadEvent};
use rusqlite::params;

#[derive(Debug, Clone, Copy)]
pub enum ActivitySource {
    Message(i64),
    WorkThreadEvent(i64),
    BoardSlaNudge(i64),
    SavedItem(i64),
    CalendarEvent(i64),
    HuddleGrant(i64),
    AgentApproval(i64),
    AgentBudgetNotice(i64),
    ScheduledMessage(i64),
    Session(i64),
    TwoFactorCredential(i64),
}

impl ActivityItem {
    /// Board creation has already authorized its roster and persisted this opener in
    /// the same transaction. Reuse those snapshots instead of reloading them per user.
    pub(crate) fn record_authorized_board_opener(
        tx: &mut Tx<'_>,
        user: &User,
        message: &crate::Message,
    ) -> Result<Option<Self>> {
        if !user.is_active() || user.is_bot() || message.creator_id == user.id {
            return Ok(None);
        }
        Self::record_authorized_with_recipient(
            tx,
            user.id,
            "Message",
            message.id,
            message.thread_id,
            "thread_activity",
            Some(user),
        )
    }

    /// Rails-compatible entry point, including the merged WS11 budget-notice reader.
    pub fn record(
        tx: &mut Tx<'_>,
        user_id: i64,
        source: ActivitySource,
        event_type: &str,
        skip_source_check: bool,
    ) -> Result<Option<Self>> {
        let event_type = super::ActivityEventType::parse(event_type)?;
        let authorization = if skip_source_check {
            super::SourceAuthorization::CallerAuthorized
        } else {
            super::SourceAuthorization::SourceRecipients
        };
        Self::record_typed(tx, user_id, source, event_type, authorization)
    }

    pub fn record_typed(
        tx: &mut Tx<'_>,
        user_id: i64,
        source: ActivitySource,
        event_type: super::ActivityEventType,
        authorization: super::SourceAuthorization,
    ) -> Result<Option<Self>> {
        let Some(facts) = source_facts(tx, source, authorization, Some(&PersistedBudgetNoticeReader))? else {
            return Ok(None);
        };
        Self::record_for_current_recipient(tx, user_id, &facts, event_type, authorization)
    }

    pub fn record_with_budget_notice_reader(
        tx: &mut Tx<'_>,
        user_id: i64,
        source: ActivitySource,
        event_type: super::ActivityEventType,
        authorization: super::SourceAuthorization,
        reader: &dyn super::AgentBudgetNoticeActivityReader,
    ) -> Result<Option<Self>> {
        let Some(facts) = source_facts(tx, source, authorization, Some(reader))? else {
            return Ok(None);
        };
        Self::record_for_current_recipient(tx, user_id, &facts, event_type, authorization)
    }

    /// An owning domain may expose another persisted polymorphic source. Source policy
    /// is explicit; a room association alone never authorizes an unknown source.
    pub fn record_from_source(
        tx: &mut Tx<'_>,
        user_id: i64,
        source: &dyn super::ActivityRecordingSource,
        event_type: super::ActivityEventType,
        authorization: super::SourceAuthorization,
    ) -> Result<Option<Self>> {
        let Some(facts) = source.recording_facts(tx.conn())? else {
            return Ok(None);
        };
        Self::record_for_current_recipient(tx, user_id, &facts, event_type, authorization)
    }

    /// Batch source authorization and active-human facts in this writer transaction.
    /// Returned items follow the requested recipient order, including idempotent repeats.
    /// Owning-domain writers keep their own callback/failure boundaries.
    pub fn record_source_for_recipients(
        tx: &mut Tx<'_>,
        recipient_ids: &[i64],
        source: &dyn super::ActivityRecordingSource,
        event_type: super::ActivityEventType,
        authorization: super::SourceAuthorization,
    ) -> Result<Vec<Self>> {
        let Some(facts) = source.recording_facts(tx.conn())? else {
            return Ok(Vec::new());
        };
        let users = User::where_ids(tx.conn(), recipient_ids)?
            .into_iter()
            .map(|user| (user.id, user))
            .collect::<std::collections::HashMap<_, _>>();
        let mut items = Vec::new();
        for id in recipient_ids {
            if let Some(user) = users.get(id)
                && let Some(item) =
                    Self::record_with_facts(tx, user, &facts, event_type, authorization)?
            {
                items.push(item);
            }
        }
        Ok(items)
    }

    fn record_for_current_recipient(
        tx: &mut Tx<'_>,
        user_id: i64,
        facts: &super::ActivityRecordingFacts,
        event_type: super::ActivityEventType,
        authorization: super::SourceAuthorization,
    ) -> Result<Option<Self>> {
        let Some(user) = User::find_by_id(tx.conn(), user_id)? else {
            return Ok(None);
        };
        Self::record_with_facts(tx, &user, facts, event_type, authorization)
    }

    fn record_with_facts(
        tx: &mut Tx<'_>,
        user: &User,
        facts: &super::ActivityRecordingFacts,
        event_type: super::ActivityEventType,
        authorization: super::SourceAuthorization,
    ) -> Result<Option<Self>> {
        if !user.is_active()
            || user.is_bot()
            || facts.creator_id == Some(user.id)
            || (authorization == super::SourceAuthorization::SourceRecipients
                && !facts.recipient_ids.contains(&user.id))
        {
            return Ok(None);
        }
        // Only these two Rails classes participate in grouping, even for custom readers.
        let thread = facts
            .thread_id
            .filter(|_| matches!(facts.source_type, "Message" | "WorkThreadEvent"));
        Self::record_authorized_with_recipient(
            tx,
            user.id,
            facts.source_type,
            facts.source_id,
            thread,
            event_type.as_str(),
            Some(user),
        )
    }

    /// WorkThreadEvent fanout keeps its authorized recipient snapshot for recording.
    /// Its recipients commit separately: broadcast eligibility must read current status.
    pub(crate) fn record_authorized_work_event(
        tx: &mut Tx<'_>,
        user: &User,
        event: &WorkThreadEvent,
        event_type: &str,
    ) -> Result<Option<Self>> {
        if !super::EVENT_TYPES.contains(&event_type) {
            return Err(Error::Other(format!(
                "Unknown activity event type: {event_type}"
            )));
        }
        if !user.is_active() || user.is_bot() {
            return Ok(None);
        }
        Self::record_authorized_with_recipient(
            tx,
            user.id,
            "WorkThreadEvent",
            event.id,
            Some(event.channel_thread_id),
            event_type,
            None,
        )
    }

    /// The newly claimed nudge authorizes its sole recipient; reuse the reviewed recorder.
    pub(crate) fn record_board_sla_nudge(
        tx: &mut Tx<'_>,
        nudge: &crate::BoardSlaNudge,
    ) -> Result<Option<Self>> {
        let Some(user) = User::find_by_id(tx.conn(), nudge.recipient_id)? else {
            return Ok(None);
        };
        Self::record_board_sla_nudge_for_recipient(tx, nudge, &user)
    }

    pub(crate) fn record_board_sla_nudge_for_recipient(
        tx: &mut Tx<'_>,
        nudge: &crate::BoardSlaNudge,
        user: &User,
    ) -> Result<Option<Self>> {
        if !user.is_active() || user.is_bot() {
            return Ok(None);
        }
        Self::record_authorized_with_recipient(
            tx,
            user.id,
            "BoardSlaNudge",
            nudge.id,
            None,
            "work_sla",
            Some(user),
        )
    }

    fn record_authorized_with_recipient(
        tx: &mut Tx<'_>,
        user_id: i64,
        source_type: &str,
        source_id: i64,
        thread_id: Option<i64>,
        event_type: &str,
        recipient: Option<&User>,
    ) -> Result<Option<Self>> {
        if let Some(thread_id) =
            thread_id.filter(|_| matches!(event_type, "thread_activity" | "work_update"))
        {
            let grouped = crate::sql::query_one(
                tx.conn(),
                "SELECT * FROM activity_items WHERE user_id=? AND handled_at IS NULL AND event_type=? AND ((source_type='Message' AND source_id IN (SELECT id FROM messages WHERE thread_id=?)) OR (source_type='WorkThreadEvent' AND source_id IN (SELECT id FROM work_thread_events WHERE channel_thread_id=?))) ORDER BY updated_at DESC,id DESC LIMIT 1",
                params![user_id, event_type, thread_id, thread_id],
                Self::from_row,
            )?;
            if let Some(before) = grouped {
                if before.source_type != source_type
                    || before.source_id != source_id
                    || before.read_at.is_some()
                    || before.updated_at != tx.now()
                {
                    tx.conn().execute("UPDATE activity_items SET source_type=?,source_id=?,read_at=NULL,updated_at=? WHERE id=?",params![source_type,source_id,tx.now(),before.id])?;
                }
                let item = Self::find(tx.conn(), before.id)?;
                // app/models/activity_item.rb: source/updated_at changes alone do not broadcast.
                if before.read_at.is_some() {
                    Self::broadcast_recorded(tx, user_id, &item, recipient)?;
                }
                return Ok(Some(item));
            }
        }
        let inserted = tx.conn().execute("INSERT INTO activity_items(user_id,source_type,source_id,event_type,created_at,updated_at) VALUES (?,?,?,?,?,?) ON CONFLICT(user_id,source_type,source_id) DO NOTHING",params![user_id,source_type,source_id,event_type,tx.now(),tx.now()])?;
        let item = Self::find_by_user_and_source(tx.conn(), user_id, source_type, source_id)?
            .ok_or(Error::RecordNotFound("ActivityItem"))?;
        if inserted == 1 {
            Self::broadcast_recorded(tx, user_id, &item, recipient)?;
        }
        Ok(Some(item))
    }

    fn broadcast_recorded(
        tx: &mut Tx<'_>,
        user_id: i64,
        item: &Self,
        recipient: Option<&User>,
    ) -> Result<()> {
        match recipient {
            Some(user) => Self::broadcast_item_for_user(tx, user, item),
            None => Self::broadcast_item(tx, user_id, item),
        }
    }
}

fn source_facts(
    tx: &Tx<'_>,
    source: ActivitySource,
    authorization: super::SourceAuthorization,
    budget_reader: Option<&dyn super::AgentBudgetNoticeActivityReader>,
) -> Result<Option<super::ActivityRecordingFacts>> {
    use super::{ActivityRecordingFacts as Facts, SourceAuthorization};
    let check = authorization == SourceAuthorization::SourceRecipients;
    let basic = |source_type, source_id, recipient_ids| Facts {
        source_type,
        source_id,
        creator_id: None,
        thread_id: None,
        recipient_ids,
    };
    let facts = match source {
        ActivitySource::Message(id) => {
            let Some(message) = crate::Message::find_by_id(tx.conn(), id)? else {
                return Ok(None);
            };
            let recipient_ids = if check {
                super::message_recorder::candidates(tx.conn(), tx.rich_text(), &message, tx.now())?
                    .recipients
                    .into_iter()
                    .map(|candidate| candidate.user_id)
                    .collect()
            } else {
                Vec::new()
            };
            Facts {
                source_type: "Message",
                source_id: id,
                creator_id: Some(message.creator_id),
                thread_id: message.thread_id,
                recipient_ids,
            }
        }
        ActivitySource::WorkThreadEvent(id) => {
            let Some(event) = WorkThreadEvent::find_by_id(tx.conn(), id)? else {
                return Ok(None);
            };
            Facts {
                source_type: "WorkThreadEvent",
                source_id: id,
                creator_id: None,
                thread_id: Some(event.channel_thread_id),
                recipient_ids: if check {
                    event.recipient_user_ids(tx.conn())?
                } else {
                    Vec::new()
                },
            }
        }
        ActivitySource::BoardSlaNudge(id) => {
            let Some(nudge) = crate::BoardSlaNudge::find_by_id(tx.conn(), id)? else {
                return Ok(None);
            };
            basic("BoardSlaNudge", id, nudge.activity_recipient_ids().to_vec())
        }
        ActivitySource::HuddleGrant(id) => {
            let Some(grant) = crate::models::huddle_grant::HuddleGrant::find_by_id(tx.conn(), id)?
            else {
                return Ok(None);
            };
            basic(
                "HuddleGrant",
                id,
                if check {
                    grant.activity_recipient_ids(tx.conn())?
                } else {
                    Vec::new()
                },
            )
        }
        ActivitySource::AgentApproval(id) => {
            let Some(approval) = crate::AgentApproval::find(tx.conn(), id)? else {
                return Ok(None);
            };
            basic(
                "AgentApproval",
                id,
                if check {
                    approval.decider_ids(tx.conn())?
                } else {
                    Vec::new()
                },
            )
        }
        ActivitySource::AgentBudgetNotice(id) => {
            let reader = budget_reader.ok_or_else(|| {
                Error::Other("AgentBudgetNotice requires its owning-domain activity reader".into())
            })?;
            let Some(recipients) = reader.recording_recipient_ids(tx.conn(), id)? else {
                return Ok(None);
            };
            basic("AgentBudgetNotice", id, recipients)
        }
        // These Rails sources do not implement activity_recipient_ids. The generic
        // recorder needs CallerAuthorized; their owning writers retain their policy.
        source => {
            let (kind, id, sql) = match source {
                ActivitySource::SavedItem(id) => {
                    ("SavedItem", id, "SELECT 1 FROM saved_items WHERE id=?")
                }
                ActivitySource::CalendarEvent(id) => {
                    ("Event", id, "SELECT 1 FROM events WHERE id=?")
                }
                ActivitySource::ScheduledMessage(id) => (
                    "ScheduledMessage",
                    id,
                    "SELECT 1 FROM scheduled_messages WHERE id=?",
                ),
                ActivitySource::Session(id) => ("Session", id, "SELECT 1 FROM sessions WHERE id=?"),
                ActivitySource::TwoFactorCredential(id) => (
                    "TwoFactorCredential",
                    id,
                    "SELECT 1 FROM two_factor_credentials WHERE id=?",
                ),
                _ => unreachable!(),
            };
            if !crate::sql::exists(tx.conn(), sql, [id])? {
                return Ok(None);
            }
            basic(kind, id, Vec::new())
        }
    };
    Ok(Some(facts))
}

/// Default consumer of WS11's typed reader. The owning budget writer and its
/// uniqueness/fanout callbacks remain in agent_posting; this adapter only reads.
struct PersistedBudgetNoticeReader;
impl super::AgentBudgetNoticeActivityReader for PersistedBudgetNoticeReader {
    fn recording_recipient_ids(&self, conn: &crate::Connection, id: i64) -> Result<Option<Vec<i64>>> {
        crate::AgentBudgetNotice::find_by_id(conn, id)?
            .map(|notice| notice.activity_recipient_ids(conn)).transpose()
    }
}

impl super::ActivityRecordingSource for crate::AgentBudgetNotice {
    fn recording_facts(&self, conn: &crate::Connection) -> Result<Option<super::ActivityRecordingFacts>> {
        let Some(notice) = crate::AgentBudgetNotice::find_by_id(conn, self.id)? else {
            return Ok(None);
        };
        Ok(Some(super::ActivityRecordingFacts {
            source_type: "AgentBudgetNotice", source_id: notice.id,
            creator_id: None, thread_id: None,
            recipient_ids: notice.activity_recipient_ids(conn)?,
        }))
    }
}
