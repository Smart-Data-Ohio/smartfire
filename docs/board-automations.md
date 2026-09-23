# Board automations

Each board carries its own automations: tag auto-assignment, per-status
SLA timers with escalation, and a daily stale-work digest. They are
configured per board by the board's creator or an administrator, from
**Automations** on the board page or the board settings page. Other
members see no link and get 403 on the page. Every configuration change
is recorded in the [audit log](audit-log.md) as
`board.automation.change`.

## Auto-assign by tag

A tag rule assigns new posts to one person or agent: when a post gains
the tag while it has no owner, the rule's target becomes the owner. A
rule never overrides an existing assignment — human or agent. The
assignment is recorded in Work history with no actor ("Auto-assigned by
board tag rule"), produces the usual work-assignment inbox items, and
writes the agent's `work_assigned` ledger event when the target is an
agent.

The target must be an active board member able to own posts (an agent
additionally needs an active `Agent` row and `post_messages` in the
board). Grants are re-checked when the rule fires, so a target that
lost access is silently skipped.

## SLA timers

Each work status (`Planned`, `In progress`, `Blocked`, `Done`) may carry
one rule: a nudge threshold and a later escalation threshold, in minutes
(up to 30 days). A post sitting in the status past the nudge time
notifies its owner; past the escalation time it escalates to the board's
creator. `Blocked` is handled the same way as every other status. Each
stage fires once per status crossing: changing the status restarts both
timers.

Recipients:

- The nudge goes to the post's owner when that is an active human
  member, to the human behind an agent owner otherwise, and to the
  board's creator when the post is unassigned.
- The escalation always goes to the board's creator.

Every candidate must be an active human member of the board; without
one the stage stays unclaimed and retries on a later sweep.

A nudge is an `SLA breach` activity inbox item sourced on the claim,
plus a push notification through the standard
[notification policy](notifications.md) (DND and quiet hours apply; the
push carries no sender, like a reminder). The inbox item links to the
post and reads "Sitting in \<status> for \<age\>", prefixed with
"Escalated:" for escalations. Losing board membership hides the item,
like other work items.

## Stale-work digest

Once a day each board with SLA rules posts one digest of the open posts
sitting in a ruled status past the rule's nudge time, stalest first
(capped at 20 listed posts). `Done` posts never appear, and boards with
no stale posts post nothing. The digest goes out as one quiet system
note (`messages.system_note`) — never a normal message — so it renders
without unread, push, agent delivery, inbox, or search noise. The
latest digest also renders on the board page under the header. Titles
and names are escaped as plain text.

## Running and idempotency

Both dispatchers run from `Periodic::Runner` (`board sla nudges` every
5 minutes, `board stale digests` every hour):

- Each SLA stage claims a `board_sla_nudges` row — unique per thread,
  status, stage, and status entry time — before notifying, so a stage
  fires exactly once per crossing even across runner restarts or two
  runners racing. A later status change carries a new entry time, which
  is a new claim.
- Each digest claims a `board_stale_digests` row — unique per board and
  day — before posting, so a digest goes out at most once a day.

The status entry time is `channel_threads.work_status_changed_at`,
stamped on every save that changes `work_status` (backfilled from
`updated_at` for existing rows).
