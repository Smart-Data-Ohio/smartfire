# WS14e Event APIs consumed by WS14g

Owner: WS14e. Reference: Smartfire Rails `d7c7de92`, `Event#respond!`,
`EventAttendance` commit callbacks, and `Calendar::MeetLink#provision!`'s
`event.update!(meet_link: link)`. API signatures are stable for WS14g consumers.

```rust
pub fn respond(
    tx: &mut Tx<'_>,
    event_id: i64,
    user_id: i64,
    response: &str,
    apply_to_future: bool,
) -> Result<EventAttendance>;

pub fn save_meet_link(
    tx: &mut Tx<'_>,
    event_id: i64,
    meet_link: Option<String>,
) -> Result<CalendarEvent>;
```

Call both inside `Database::write` / `write_blocking`. They use the existing writer
transaction and return domain facts/errors, without HTML or remote I/O. Missing
events return RecordNotFound. WS14g owns job registration, Google requests, retry
policy and the decision to invoke these APIs.

`respond` returns the selected occurrence's attendance. A series head copies its
response to every active later occurrence; a follower stays local unless
`apply_to_future` is true. The future set uses `(starts_at, id)` ordering and skips
cancelled occurrences. Responses are going/maybe/declined. The user must exist,
be an active human and belong to the event room; the selected event must be open.
An unchanged response keeps its timestamp and has no Sync callback. Each changed
attendance has one `Calendar::SyncEntryJob` commit callback, with
`{event_id, user_id}`, even when separate response calls save it repeatedly in
one transaction. A subsequently destroyed attendance has no surviving callback.

Attendance callback jobs are collected by record identity and persisted before
the triggering writer COMMIT. A queue insert failure rolls back every affected
response and job; callbacks/publication happen only after commit. Explicit job
enqueues outside these record callbacks are not globally coalesced. The API's
savepoint also restores its rows and callbacks if an operation fails.

`save_meet_link` is an internal Calendar writer API. The attribute stays absent
from EventChanges and controller strong parameters. It runs normal persisted
event validation, including organizer eligibility and recurrence/head guards.
Rails has no model URL validation here: nil, blank and non-HTTPS values are
stored as supplied; the renderer's existing safe HTTPS filter controls exposure.
A changed link refreshes updated_at; an unchanged link keeps that timestamp.
Both saves register one EventCards replacement per referencing message after
commit, including unchanged saves. A populated link adds no SyncEntry or Meet
job. Clearing a requested link (or storing blank) retains the normal
`needs_meet_link?` callback and atomically enqueues `Calendar::MeetLinkJob` with
`{event_id}`. Cancelled events can be internally saved but have no Meet retry.
Repeated internal saves coalesce the Meet callback and evaluate the final pending
state before commit; blank-to-populated saves therefore have no provisioning
job. Validation, storage or queue failure leaves the link and callbacks unchanged.

Evidence: `reference-tools/events/calendar_api.{rb,sh}` executes 18 response and
18 Meet-link states against pinned Rails, including transaction rollback,
follower failure, duplicate saves, unchanged saves and validation failures.
`tests/calendar_event_test/calendar_api_test.rs` compares rows/timestamps, ordered
jobs and card descriptions; `jobs/tests/event_tests.rs` uses real durable queue
insert rejection, and the seeded HTTP test rejects user Meet-link injection.
The real-socket differential includes internal Meet-link card delivery.

Still outside these APIs' current coverage: broader Event create/update callback
ordering and instance-identity cases across repeated general event API saves, and multiple-recipient
invitation failure. Those remain explicit WS14e deferrals. WS14g's inbound and
Meet consumers must exercise these APIs with their own remote/job tests.
