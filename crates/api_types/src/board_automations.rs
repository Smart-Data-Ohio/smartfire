//! A board's automations: auto-assign-by-tag rules and SLA timers, which run for every post on
//! the board. Only the board's creator and administrators may see or change them
//! (`BoardListing::can_administer`). The SPA shows them in the board's right pane at
//! `/app/r/:room_id/automations`.
//!
//! Ports `rooms/boards/automations#show`, `tag_assignments#create`, `tag_assignments#destroy`
//! and `sla_rules#update` (`crates/rooms/src/controllers/rooms/board_automations.rs` and
//! `board_automations/sla.rs`), with the same models (`BoardTagAssignment`, `BoardSlaRule`), the
//! same validations and messages, and the same `board.automation.change` audit entries. The tag
//! and SLA jobs are unchanged: these endpoints only edit the rules they read.
//!
//! Every endpoint answers 404 unless the room is a board the viewer belongs to, and 403
//! (`Forbidden`) when the viewer belongs but is neither the board's creator nor an administrator.
//! Each write answers the whole [`BoardAutomations`] as it stands after the write.
//!
//! `DELETE /api/v1/rooms/:room_id/automations/tag_rules/:id` (`tag_assignments#destroy`) removes
//! a rule, answers [`BoardAutomations`] (200) and audits `{"tag_rule":"removed","tag":…,
//! "assignee":name}`; a rule that isn't this board's is 404 with the message "Rule not found.".

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{User, WorkStatus};

/// "When a post gains this tag while it has no owner, assign it to this person": one
/// `board_tag_assignments` row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BoardTagRule {
    pub id: i64,
    /// Lower-cased, as stored.
    pub tag: String,
    /// In [`BoardAutomations::users`] (an assignee who has since left or been deactivated is
    /// still listed; the rule just stops applying).
    pub assignee_id: i64,
}

/// An SLA timer for one unfinished status: a post that has sat in `status` for
/// `nudge_after_minutes` notifies its owner, and after `escalate_after_minutes` escalates to the
/// board's creator. One `board_sla_rules` row. `done` never has one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BoardSlaTimer {
    /// `planned`, `in_progress` or `blocked`.
    pub status: WorkStatus,
    /// 1 to 43,200.
    pub nudge_after_minutes: i64,
    /// Greater than `nudge_after_minutes`, at most 43,200.
    pub escalate_after_minutes: i64,
}

/// `GET /api/v1/rooms/:room_id/automations` (`rooms/boards/automations#show`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BoardAutomations {
    pub room_id: i64,
    /// In creation order (`BoardTagAssignment::for_room`).
    pub tag_rules: Vec<BoardTagRule>,
    /// Only the statuses that have a timer, in the order planned, in progress, blocked. A status
    /// missing here is off ("Leave both blank to disable a status").
    pub sla_timers: Vec<BoardSlaTimer>,
    /// The "Assign to" picker: the board's active members, people and agents alike, by
    /// lower-cased name, as the classic form lists them. (The server still refuses an agent that
    /// can't post on the board; see [`CreateBoardTagRule`].)
    pub candidates: Vec<i64>,
    /// The rules' assignees and the candidates, once each.
    pub users: Vec<User>,
}

/// `POST /api/v1/rooms/:room_id/automations/tag_rules` (`tag_assignments#create`): add a rule.
/// Answers [`BoardAutomations`] (201) and audits
/// `{"tag_rule":"created","tag":…,"assignee":name}`.
///
/// Errors, each `Validation` with the classic message:
/// - on `tag`: "can't be blank", "is too long (maximum is 30 characters)", "is invalid" (not
///   `[a-z0-9][a-z0-9-]*` once stripped and lower-cased), "has already been taken" (a rule for the
///   tag exists on this board, whatever its case);
/// - on `assigneeId`: "must exist" (missing or no such user), "must be an active board member
///   able to own posts" (inactive, not a member, or an agent that can't read and post here).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateBoardTagRule {
    /// Stripped and lower-cased by the server.
    pub tag: String,
    /// `null` is "Choose a member" left unchosen.
    pub assignee_id: Option<i64>,
}

/// One status's row of the SLA form. Both `null` turns the status's timer off (deleting its
/// rule); one `null` is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BoardSlaTimerInput {
    pub nudge_after_minutes: Option<i64>,
    pub escalate_after_minutes: Option<i64>,
}

/// `PUT /api/v1/rooms/:room_id/automations/sla_timers` (`sla_rules#update`): the whole SLA form at
/// once. Every row is validated before anything is written; then each status's rule is created,
/// updated, deleted or left alone, with one audit entry per changed rule
/// (`{"sla_rule":"created"|"updated"|"removed",…}`, as classic). Answers [`BoardAutomations`]
/// (200).
///
/// Errors: `Validation` with `fields` keyed `planned`, `inProgress` and `blocked` (the rows that
/// failed), each holding the row's full messages ("Nudge after minutes must be greater than 0",
/// "Escalate after minutes must be after the nudge threshold", "Nudge after minutes can't be
/// blank", "… must be less than or equal to 43200"), and `message` the classic alert, e.g.
/// "Planned: Escalate after minutes must be after the nudge threshold".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateBoardSlaTimers {
    pub planned: BoardSlaTimerInput,
    pub in_progress: BoardSlaTimerInput,
    pub blocked: BoardSlaTimerInput,
}
