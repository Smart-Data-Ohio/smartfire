# Generic activity recorder

The Rails-compatible `ActivityItem::record(tx, user_id, ActivitySource, event_type, skip_source_check)`
now handles Message, WorkThreadEvent, BoardSlaNudge, SavedItem, CalendarEvent (Rails Event),
HuddleGrant, AgentApproval, ScheduledMessage, Session and TwoFactorCredential.
`ActivityEventType::parse` validates the event before source/recipient checks;
`record_typed` takes that validated type and `SourceAuthorization::{SourceRecipients,CallerAuthorized}`.

A persisted source does not acquire authorization just because it is displayable in the inbox.
SavedItem, Event, ScheduledMessage, Session and TwoFactorCredential have no Rails
`activity_recipient_ids` hook: the generic API requires caller authorization for them.
HuddleGrant uses its current direct-room roster, Approval its current deciders, and the other
existing sources retain their own hooks. Every recipient must be a current active human;
creator exclusion remains in force even with caller authorization.

`ActivityRecordingSource::recording_facts(&Connection) -> Result<Option<ActivityRecordingFacts>>`
is the owning-domain contract for other polymorphic sources. None means unsaved/deleted.
Facts contain the persisted base-class identity, optional creator and thread, and current
recipient IDs. Implementations verify those facts on the supplied writer connection;
request-supplied or request-cached authority is not a source contract. Only Message and
WorkThreadEvent may group. `record_from_source` accepts the contract; the batch form
`record_source_for_recipients` loads source facts and users once per locked transaction.

The narrow **AgentBudgetNoticeActivityReader** seam belongs to WS11:

```rust
fn recording_recipient_ids(
    &self, conn: &Connection, notice_id: i64,
) -> Result<Option<Vec<i64>>>;
```

Some verifies current notice persistence and returns its current owner, or active human
administrators when the owner association is absent. The recorder applies its active-human
filter. `ActivityItem::record_with_budget_notice_reader(tx, user_id,
ActivitySource::AgentBudgetNotice(id), event_type, authorization, &reader)` consumes it.
The plain entry point requires this explicit reader for budget notices; it never duplicates
WS11's notice writer, audience SQL or callback lifecycle. WS12 tests implement a consumer
against actual fixture rows and compare the complete recorded facts with Rails, including
owner changes and administrator fallback.

Source+recipient identity is unique. Repeated non-grouping calls preserve the first event
kind and read/handled/timestamp state. Grouping reuses the reviewed dirty-column writer and
operation-snapshot broadcasts. Rollback discards rows and commit broadcasts together.
Existing reminder, event, huddle, approval, budget, scheduled-message and security writers
retain their original callbacks and failure boundaries.

Evidence: `users/generic_recorder.rb` and `vectors/ws12_generic_recorder.json`: 135 complete
source/recipient/authorization vectors, caller-authorized message grouping/idempotency,
and handled-source idempotency; source SHA checks, no response masks. Seven Rust regressions
in `ws12_generic_recorder_test.rs`, including 10/100-recipient source/user preload counts.
The initial runtime control records nothing for the newly supported sources; a second
control authorizes all sources and overwrites existing read/handled/event facts. Both fail.

Nullable direct-room involvement remains eligible, matching Rails: only explicit nothing/invisible values silence the source hook. The dedicated nullable-involvement regression fails against the original NOT IN predicate and passes with the explicit NULL clause.
