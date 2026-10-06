# Pinned Rails ring delivery rule (d7c7de92)

Rails has no deferred Cable ring job. These are the source rules, inspected with
`git show d7c7de92:<path>`:

- `app/models/activity_item.rb:32-34,215-224`: create/update commit callbacks
  broadcast synchronously. Watched fields are read_at, handled_at, event_type and
  created_at. The callback builds the current source, room, caller, state and
  policy payload (`:172-176,234-255`). No membership, revocation or liveness check
  discards this already-committed callback.
- `app/models/huddle_grant.rb:423-430`: issuance dedupe returns before emitting
  anything. Updating last_issued_at does not invalidate a prior callback.
- `app/models/huddle_grant.rb:558-575`: banner invitations are synchronous too.
  Changing form mode does not give an older banner permission to arrive later;
  the new item/banner is the next frame in callback order.
- `app/models/huddle_grant.rb:468-484,505-511`: ending a call emits ended frames
  for unhandled items. It does not cancel handled callbacks; those callbacks
  already hid the banner. Banner-only ended frames use the current preference
  and the strict one-minute issuance window (`:531-549`).
- `app/jobs/huddle/push_invitation_job.rb:2-6`: the actual deferred job reloads
  the activity item by ID. `app/models/huddle/invitation_pusher.rb:6-10,25-44`
  resolves current associations, evaluates policy and hands the actual payload
and subscription scope to the push pool. It does not replay Cable frames.

Rust now emits Cable frames after commit, before returning from the triggering
write. Its old durable ring envelopes are retained, marked delivered, and
acknowledged without replay. Existing pre-upgrade pending envelopes retain
execution-time safety: actual emissions supersede pending envelopes across form
changes for the same caller/room/recipient, and ending a call cancels only unread
started envelopes. Handled frames remain deliverable. Tests explicitly remove
the delivered marker only to simulate persisted pre-upgrade envelopes.

The oracle records actual ActionCable broadcast calls for every affected client,
including the caller and continuing group participants. ActiveJob's test adapter
stores real serialized jobs, and the generator executes those job classes in
selected orders. It records the real WebPush pool handoff, not a reconstructed
pusher payload. Independent instances of the real pinned Stimulus controller consume each
client's own recorded frames, with timers advanced after every operation. There is no pending-ring
projection, lifecycle reducer or manually constructed expected delivery payload.

The queue inventory records **every** pending ActiveJob row after every operation:
class, complete positional arguments and absolute scheduled time. Rust compares
its corresponding pending source jobs before interpreting Rails' selected drain
indices. A complete drain must select each pending job exactly once and leave an
empty queue. Rust's already-delivered Cable envelopes are acknowledgements, not
Rails jobs; the push adapter/pool is run through its actual handoff during the
source-job execution. Unexpected Rust job classes fail instead of being ignored.

Dismissal uses the actual pinned controller, not the latest inbox row:
`app/javascript/controllers/huddle_invitation_controller.js:88-96` hides the
currently displayed invitation and returns without fetching when its readPath is
empty. The shown frame sets that path (`:149-157`); the real PATCH implementation
is `:124-133`. The recorder interleaves the real controller with Rails, and only
its observed fetch requests invoke the accessible item read action
(`app/controllers/activity_items_controller.rb:52-60,89-90,122-124`). An independent
controller replay on Rust's own frames must produce the same requests and banner
states. The separate `read` operation represents the inbox read endpoint, even
when a banner-only invitation is displayed. Hidden banners cannot be clicked.

The committed corpus contains 209 named sequences and 512 random sequences (721
in total). Its default seeds remain 388013012 and 3620200082. Fresh extended
verification uses 2,048 random sequences from seeds 20261002, 2718281828 and
4242424242, plus the 209 named sequences. `ws13b_ring_matrix.rb` generates
sequences; `ws13b_differential.py` regenerates, compares and shrinks failures.
Pass fresh seeds with `record COUNT OUTPUT SEED,SEED,...`. All eleven Rails/UI
source hashes are checked against d7c7de92. No expected payload is generated from
Rust. Eleven independent output corruptions exercise the comparison, including
extra/missing jobs, altered job class/arguments/schedule and changed UI requests.
`ws13b_queue_mutation.py` also duplicates the actual production PushInvitationJob
enqueue temporarily: unchanged Rails expectations must pass before mutation,
fail immediately at the issue step with two jobs against one, and pass after
byte-for-byte restoration.
