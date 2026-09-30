//! Scoped event edits and cancellation from app/models/event.rb.
use std::collections::{BTreeMap, BTreeSet};

use jiff::{SignedDuration, civil::Date};
use rusqlite::{Connection, params};

use super::{
    CalendarEvent, MeetLinkJob, NewCalendarEvent, SyncEntryJob, attendance::EventAttendance,
    recurrence,
};
use crate::sql::query_all;
use crate::{ActivityItem, Errors, Event, Result, Timestamp, Tx};

/// Only persisted event attributes are accepted. Rails' temporary validation guards
/// have no representation in this public input.
#[derive(Debug, Clone, Default)]
pub struct EventChanges {
    pub title: Option<String>,
    pub description: Option<Option<String>>,
    pub starts_at: Option<Option<Timestamp>>,
    pub ends_at: Option<Option<Timestamp>>,
    pub time_zone: Option<String>,
    pub venue_room_id: Option<Option<i64>>,
    pub recurrence_rule: Option<Option<String>>,
    pub recurrence_until: Option<Option<Date>>,
    pub meet_link_requested: Option<bool>,
}

#[derive(Clone, Copy, Default)]
struct SaveRules {
    following: bool,
    recurrence: bool,
    skip_order: bool,
}

fn delta(new: Timestamp, old: Timestamp) -> SignedDuration {
    SignedDuration::from_micros(new.jiff().as_microsecond() - old.jiff().as_microsecond())
}

impl EventChanges {
    fn assign(&self, old: &CalendarEvent) -> NewCalendarEvent {
        NewCalendarEvent {
            room_id: old.room_id,
            organizer_id: old.organizer_id,
            title: self.title.clone().unwrap_or_else(|| old.title.clone()),
            description: self
                .description
                .clone()
                .unwrap_or_else(|| old.description.clone()),
            starts_at: self.starts_at.unwrap_or(Some(old.starts_at)),
            ends_at: self.ends_at.unwrap_or(old.ends_at),
            time_zone: self
                .time_zone
                .clone()
                .unwrap_or_else(|| old.time_zone.clone()),
            venue_room_id: self.venue_room_id.unwrap_or(old.venue_room_id),
            recurrence_rule: self
                .recurrence_rule
                .clone()
                .unwrap_or_else(|| old.recurrence_rule.clone())
                .filter(|s| !campfire_richtext::ruby::is_blank(s)),
            recurrence_until: self.recurrence_until.unwrap_or(old.recurrence_until),
            meet_link_requested: self.meet_link_requested.unwrap_or(old.meet_link_requested),
        }
    }
}

fn time_changed(old: &CalendarEvent, new: &NewCalendarEvent) -> bool {
    Some(old.starts_at) != new.starts_at
        || old.ends_at != new.ends_at
        || old.time_zone != new.time_zone
}
fn synced_changed(old: &CalendarEvent, new: &NewCalendarEvent) -> bool {
    time_changed(old, new)
        || old.title != new.title
        || old.description != new.description
        || old.venue_room_id != new.venue_room_id
}
fn recurrence_changed(old: &CalendarEvent, new: &NewCalendarEvent) -> bool {
    old.recurrence_rule != new.recurrence_rule || old.recurrence_until != new.recurrence_until
}
fn direct_message(old: &CalendarEvent) -> &'static str {
    if old.series() {
        "can only be changed from the first event in the series using This and following"
    } else {
        "can only be set when scheduling a new event"
    }
}
fn rejection(attribute: &'static str, message: &'static str) -> Result<()> {
    let mut errors = Errors::default();
    errors.add(attribute, message);
    errors.into_result()
}

impl CalendarEvent {
    fn validate_change(
        &self,
        conn: &Connection,
        a: &NewCalendarEvent,
        rules: SaveRules,
    ) -> Result<()> {
        // Pinned Rails raises on nil sibling comparisons/arithmetic. Return an
        // internal error so requests use public/500.html; never write a substitute.
        if self.series() && a.starts_at.is_none() {
            return Err(crate::Error::Other(
                "Event series nil start: Rails NoMethodError".into(),
            ));
        }
        let mut errors = Self::validate_fields(
            conn,
            a,
            self.venue_room_id != a.venue_room_id,
            !self.series() || self.series_head(),
        )?;
        if self.series_head() && a.recurrence_rule.is_none() {
            errors.add("recurrence_rule", "can't be removed from a repeating event");
        }
        let moved = a.starts_at != Some(self.starts_at);
        if self.series_head() && moved && !rules.following {
            errors.add(
                "starts_at",
                "moves the whole series: choose This and following or the entire series",
            );
        }
        if !rules.recurrence {
            if self.recurrence_rule != a.recurrence_rule {
                errors.add("recurrence_rule", direct_message(self));
            }
            if self.recurrence_until != a.recurrence_until {
                errors.add("recurrence_until", direct_message(self));
            }
        }
        if self.series()
            && moved
            && !rules.skip_order
            && let Some(start) = a.starts_at
        {
            let siblings = self.series_events(conn)?;
            let previous = siblings
                .iter()
                .filter(|e| e.id != self.id && !e.cancelled() && e.starts_at < self.starts_at)
                .max_by_key(|e| e.starts_at);
            let next = siblings
                .iter()
                .filter(|e| e.id != self.id && !e.cancelled() && e.starts_at > self.starts_at)
                .min_by_key(|e| e.starts_at);
            if previous.is_some_and(|e| start <= e.starts_at)
                || (!rules.following && next.is_some_and(|e| start >= e.starts_at))
            {
                errors.add(
                    "starts_at",
                    "must stay between the neighbouring occurrences in its series",
                );
            }
        }
        errors.into_result()
    }

    /// Plain update retains the model's recurrence and head-time guards.
    pub fn update(tx: &mut Tx<'_>, id: i64, changes: EventChanges) -> Result<Self> {
        tx.savepoint(|tx| {
            let old = Self::find(tx.conn(), id)?;
            let a = changes.assign(&old);
            let saved = old.save_attributes(tx, &a, SaveRules::default(), old.reminded_at)?;
            saved.update_callbacks(tx)?;
            Ok(saved)
        })
    }

    /// Returns whether the event's time changed, matching update_with_scope!.
    pub fn update_with_scope(
        tx: &mut Tx<'_>,
        id: i64,
        changes: EventChanges,
        scope: &str,
        actor: Option<i64>,
    ) -> Result<bool> {
        tx.savepoint(|tx| {
            let old = Self::find(tx.conn(), id)?;
            let a = changes.assign(&old);
            if old.series() && scope == "this_and_following" {
                old.update_following(tx, a, actor)
            } else {
                if recurrence_changed(&old, &a) {
                    rejection("recurrence_rule", direct_message(&old))?;
                }
                let changed = time_changed(&old, &a);
                let sync = synced_changed(&old, &a);
                let saved = old.save_attributes(
                    tx,
                    &a,
                    SaveRules::default(),
                    if changed { None } else { old.reminded_at },
                )?;
                if changed {
                    for recipient in saved
                        .notification_recipient_ids(tx.conn())?
                        .into_iter()
                        .filter(|u| Some(*u) != actor)
                    {
                        ActivityItem::refresh_unread(tx, recipient, "Event", id, "event_update")?;
                    }
                }
                if sync {
                    saved.sync_calendar_entries(tx)?;
                }
                saved.update_callbacks(tx)?;
                Ok(changed)
            }
        })
    }

    fn save_attributes(
        &self,
        tx: &mut Tx<'_>,
        a: &NewCalendarEvent,
        rules: SaveRules,
        reminded_at: Option<Timestamp>,
    ) -> Result<Self> {
        self.validate_change(tx.conn(), a, rules)?;
        let changed = synced_changed(self, a)
            || recurrence_changed(self, a)
            || self.meet_link_requested != a.meet_link_requested
            || self.reminded_at != reminded_at;
        // A parked row must be restored even for a no-op save. Rails update_columns
        // clears series_id on both the row and its dirty tracking before placement.
        let stored_series: Option<i64> =
            tx.conn()
                .query_row("SELECT series_id FROM events WHERE id=?", [self.id], |r| {
                    r.get(0)
                })?;
        if changed || stored_series != self.series_id {
            tx.conn().execute("UPDATE events SET title=?,description=?,starts_at=?,ends_at=?,time_zone=?,venue_room_id=?,series_id=?,recurrence_rule=?,recurrence_until=?,meet_link_requested=?,reminded_at=?,updated_at=? WHERE id=?",
                params![a.title,a.description,a.starts_at,a.ends_at,a.time_zone,a.venue_room_id,self.series_id,a.recurrence_rule,a.recurrence_until.map(|d| d.to_string()),a.meet_link_requested,reminded_at,if changed { tx.now() } else {self.updated_at},self.id])?;
        }
        Self::find(tx.conn(), self.id)
    }

    fn update_following(
        &self,
        tx: &mut Tx<'_>,
        a: NewCalendarEvent,
        actor: Option<i64>,
    ) -> Result<bool> {
        let rule_changed = recurrence_changed(self, &a);
        if rule_changed && !self.series_head() {
            rejection("recurrence_rule", direct_message(self))?;
        }
        let time = time_changed(self, &a);
        let rules = SaveRules {
            following: true,
            recurrence: true,
            skip_order: false,
        };
        self.validate_change(tx.conn(), &a, rules)?;
        let start = a.starts_at.expect("validated start");
        let shift = delta(start, self.starts_at);
        let end_delta = match (a.ends_at, self.ends_at) {
            (Some(new), Some(old)) if new != old => Some(delta(new, old)),
            _ => None,
        };
        let ends_added = a.ends_at.is_some() && self.ends_at.is_none();
        let ends_removed = a.ends_at.is_none() && self.ends_at.is_some();
        let followers = self.future_occurrences(tx.conn())?;
        let scope_ids: Vec<_> = std::iter::once(self.id)
            .chain(followers.iter().map(|e| e.id))
            .collect();
        let mut plans = vec![(self.clone(), a.clone())];
        for e in followers {
            let mut f = EventChanges::default().assign(&e);
            if self.title != a.title {
                f.title.clone_from(&a.title);
            }
            if self.description != a.description {
                f.description.clone_from(&a.description);
            }
            if self.venue_room_id != a.venue_room_id {
                f.venue_room_id = a.venue_room_id;
            }
            if self.meet_link_requested != a.meet_link_requested {
                f.meet_link_requested = a.meet_link_requested;
            }
            if self.time_zone != a.time_zone {
                f.time_zone.clone_from(&a.time_zone);
            }
            f.starts_at = Some(e.starts_at.since(shift));
            f.ends_at = if ends_removed {
                None
            } else if ends_added {
                Some(
                    f.starts_at
                        .expect("follower start")
                        .since(delta(a.ends_at.expect("added end"), start)),
                )
            } else {
                e.ends_at.map(|end| end.since(end_delta.unwrap_or(shift)))
            };
            if rule_changed {
                f.recurrence_rule.clone_from(&a.recurrence_rule);
                f.recurrence_until = a.recurrence_until;
            }
            plans.push((e, f));
        }
        if !shift.is_zero() && plans.len() > 1 {
            Self::park(tx, plans.iter().map(|(e, _)| e.id))?;
            plans.sort_by_key(|(e, a)| (a.starts_at, e.id));
        }
        let mut saved = Vec::new();
        let mut sync = BTreeMap::new();
        let mut callback_ids = Vec::new();
        for (old, attributes) in plans {
            sync.insert(old.id, synced_changed(&old, &attributes));
            callback_ids.push(old.id);
            let updated = old.save_attributes(
                tx,
                &attributes,
                SaveRules {
                    following: true,
                    recurrence: true,
                    skip_order: true,
                },
                if time { None } else { old.reminded_at },
            )?;
            saved.push(updated);
        }
        let head = Self::find(tx.conn(), self.id)?;
        let mut handled_sync = BTreeSet::new();
        let mut created = Vec::new();
        if rule_changed {
            let followers: Vec<_> = saved.into_iter().filter(|e| e.id != self.id).collect();
            head.rematerialize(
                tx,
                followers,
                actor,
                time,
                &sync,
                &mut handled_sync,
                &mut created,
            )?;
        }
        if time || rule_changed {
            head.announce_series(tx, &scope_ids, actor)?;
        }
        for id in &callback_ids {
            if !handled_sync.contains(id)
                && sync[id]
                && let Ok(event) = Self::find(tx.conn(), *id)
            {
                event.sync_calendar_entries(tx)?;
            }
        }
        // Event callbacks run once per transaction record, even when a follower was
        // saved twice during rematerialization. Destroyed records have no update callback.
        for id in callback_ids {
            if let Ok(event) = Self::find(tx.conn(), id) {
                event.update_callbacks(tx)?;
            }
        }
        for (event, attendees) in created {
            if event.needs_meet_link() {
                tx.emit_after_commit(Event::job(&MeetLinkJob { event_id: event.id }));
            }
            tx.emit_after_commit(Event::job(&SyncEntryJob {
                event_id: event.id,
                user_id: event.organizer_id,
            }));
            for user_id in attendees {
                tx.emit_after_commit(Event::job(&SyncEntryJob {
                    event_id: event.id,
                    user_id,
                }));
            }
        }
        Ok(time)
    }

    fn park(tx: &Tx<'_>, ids: impl IntoIterator<Item = i64>) -> Result<()> {
        for id in ids {
            tx.conn()
                .execute("UPDATE events SET series_id=NULL WHERE id=?", [id])?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn rematerialize(
        &self,
        tx: &mut Tx<'_>,
        mut later: Vec<Self>,
        actor: Option<i64>,
        time: bool,
        shift_sync: &BTreeMap<i64, bool>,
        handled_sync: &mut BTreeSet<i64>,
        created: &mut Vec<(Self, Vec<i64>)>,
    ) -> Result<()> {
        let mut slots = recurrence::slots(
            Some(self.starts_at),
            self.ends_at,
            &self.time_zone,
            self.recurrence_rule.as_deref().expect("head rule"),
            self.recurrence_until,
        )
        .into_iter()
        .skip(1)
        .collect::<Vec<_>>();
        later.sort_by_key(|e| (e.starts_at, e.id));
        let responses = Self::responses(tx.conn(), self.id)?;
        let mut protected = Vec::new();
        let mut regenerable = Vec::new();
        for e in later {
            if e.cancelled() {
                claim_slot(&mut slots, e.starts_at);
            } else if Self::responses(tx.conn(), e.id)? != responses {
                protected.push(e);
            } else {
                regenerable.push(e);
            }
        }
        let mut moves = Vec::new();
        let mut cancels = Vec::new();
        let mut destroys = Vec::new();
        // All already-matching rows claim their slots before unmatched rows move.
        protected.retain(|e| claim_slot(&mut slots, e.starts_at).is_none());
        for e in protected {
            if slots.is_empty() {
                cancels.push(e);
            } else {
                let slot = slots.remove(0);
                let rearm = time || e.starts_at != slot.0 || e.ends_at != slot.1;
                moves.push((e, slot, rearm));
            }
        }
        regenerable.retain(|e| claim_slot(&mut slots, e.starts_at).is_none());
        for e in regenerable {
            if slots.is_empty() {
                destroys.push(e);
            } else {
                moves.push((e, slots.remove(0), true));
            }
        }
        for e in destroys {
            e.destroy(tx)?;
            handled_sync.insert(e.id);
        }
        for e in cancels {
            e.cancel_one(tx, actor, true)?;
            handled_sync.insert(e.id);
        }
        Self::park(tx, moves.iter().map(|(e, _, _)| e.id))?;
        moves.sort_by_key(|(e, s, _)| (s.0, e.id));
        for (e, slot, rearm) in moves {
            let a = EventChanges {
                starts_at: Some(Some(slot.0)),
                ends_at: Some(slot.1),
                ..Default::default()
            }
            .assign(&e);
            let sync = shift_sync[&e.id] || synced_changed(&e, &a);
            let saved = e.save_attributes(
                tx,
                &a,
                SaveRules {
                    recurrence: true,
                    skip_order: true,
                    following: false,
                },
                if rearm { None } else { e.reminded_at },
            )?;
            if sync {
                saved.sync_calendar_entries(tx)?;
                handled_sync.insert(e.id);
            }
        }
        for (start, end) in slots {
            let a = NewCalendarEvent {
                starts_at: Some(start),
                ends_at: end,
                ..EventChanges::default().assign(self)
            };
            let e = Self::insert(tx, &a, Some(self.id))?;
            let mut copied = Vec::new();
            for (&user, response) in &responses {
                if EventAttendance::find_for(tx.conn(), e.id, user)?
                    .is_none_or(|a| a.response != *response)
                {
                    EventAttendance::save_response(tx, &e, user, response, false)?;
                    if user != e.organizer_id {
                        copied.push(user);
                    }
                }
            }
            created.push((e, copied));
        }
        Ok(())
    }

    fn responses(conn: &Connection, id: i64) -> Result<BTreeMap<i64, String>> {
        Ok(EventAttendance::for_event(conn, id)?
            .into_iter()
            .map(|a| (a.user_id, a.response))
            .collect())
    }

    fn series_recipient_ids(
        &self,
        conn: &Connection,
        scope_ids: &[i64],
        actor: Option<i64>,
    ) -> Result<Vec<i64>> {
        let mut users = BTreeSet::new();
        for id in scope_ids {
            users.extend(query_all(conn,"SELECT a.user_id FROM event_attendances a JOIN users u ON u.id=a.user_id JOIN memberships m ON m.user_id=u.id AND m.room_id=? WHERE a.event_id=? AND a.response IN ('going','maybe') AND u.status=0 AND u.role<>2 ORDER BY u.id",params![self.room_id,id],|r|r.get::<_,i64>(0))?);
        }
        Ok(users.into_iter().filter(|id| Some(*id) != actor).collect())
    }

    fn announce_series(
        &self,
        tx: &mut Tx<'_>,
        scope_ids: &[i64],
        actor: Option<i64>,
    ) -> Result<()> {
        let series_ids = self
            .series_events(tx.conn())?
            .into_iter()
            .map(|e| e.id)
            .collect::<BTreeSet<_>>();
        for user in self.series_recipient_ids(tx.conn(), scope_ids, actor)? {
            let ids = query_all(
                tx.conn(),
                "SELECT id,source_id FROM activity_items WHERE user_id=? AND source_type='Event' AND event_type='event_update' AND handled_at IS NULL ORDER BY id",
                [user],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)),
            )?;
            for (id, source) in ids {
                if series_ids.contains(&source) {
                    ActivityItem::find(tx.conn(), id)?.mark_handled(tx)?;
                }
            }
            ActivityItem::refresh_unread(tx, user, "Event", self.id, "event_update")?;
        }
        Ok(())
    }

    fn handle_unread(&self, tx: &mut Tx<'_>) -> Result<()> {
        let ids = query_all(
            tx.conn(),
            "SELECT id FROM activity_items WHERE source_type='Event' AND source_id=? AND read_at IS NULL AND handled_at IS NULL ORDER BY id",
            [self.id],
            |r| r.get::<_, i64>(0),
        )?;
        for id in ids {
            ActivityItem::find(tx.conn(), id)?.mark_handled(tx)?;
        }
        Ok(())
    }
    fn cancel_one(&self, tx: &mut Tx<'_>, actor: Option<i64>, notify: bool) -> Result<bool> {
        if self.cancelled() {
            return Ok(false);
        }
        self.validate_existing(tx.conn())?.into_result()?;
        tx.conn().execute(
            "UPDATE events SET cancelled_at=?,updated_at=? WHERE id=?",
            params![tx.now(), tx.now(), self.id],
        )?;
        self.handle_unread(tx)?;
        if notify {
            for user in self
                .notification_recipient_ids(tx.conn())?
                .into_iter()
                .filter(|u| Some(*u) != actor)
            {
                ActivityItem::refresh_unread(tx, user, "Event", self.id, "event_cancelled")?;
            }
        }
        for user_id in query_all(
            tx.conn(),
            "SELECT user_id FROM event_calendar_entries WHERE event_id=? ORDER BY id",
            [self.id],
            |r| r.get::<_, i64>(0),
        )? {
            tx.emit_after_commit(Event::job(&SyncEntryJob {
                event_id: self.id,
                user_id,
            }));
        }
        Ok(true)
    }

    pub fn cancel_with_scope(
        tx: &mut Tx<'_>,
        id: i64,
        scope: &str,
        actor: Option<i64>,
    ) -> Result<bool> {
        tx.savepoint(|tx| {
            let event = Self::find(tx.conn(), id)?;
            if event.cancelled() {
                return Ok(false);
            }
            let mut targets = vec![event.clone()];
            let following = event.series() && scope == "this_and_following";
            if following {
                targets.extend(
                    event
                        .future_occurrences(tx.conn())?
                        .into_iter()
                        .filter(|e| !e.cancelled()),
                );
            }
            for target in &targets {
                target.cancel_one(tx, actor, !following)?;
            }
            if following {
                for user in event.series_recipient_ids(
                    tx.conn(),
                    &targets.iter().map(|e| e.id).collect::<Vec<_>>(),
                    actor,
                )? {
                    ActivityItem::refresh_unread(tx, user, "Event", id, "event_cancelled")?;
                }
            }
            for target in targets {
                Self::find(tx.conn(), target.id)?.update_callbacks(tx)?;
            }
            Ok(true)
        })
    }
}

fn claim_slot(
    slots: &mut Vec<(Timestamp, Option<Timestamp>)>,
    start: Timestamp,
) -> Option<(Timestamp, Option<Timestamp>)> {
    slots
        .iter()
        .position(|s| s.0 == start)
        .map(|index| slots.remove(index))
}
