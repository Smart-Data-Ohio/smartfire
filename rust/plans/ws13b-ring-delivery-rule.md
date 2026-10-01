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

The oracle records actual ActionCable broadcast calls. ActiveJob's test adapter
stores real serialized jobs, and the generator executes those job classes in
selected orders. It records the real WebPush pool handoff, not a reconstructed
pusher payload. The real pinned Stimulus controller consumes those recorded
frames with its timers advanced after every operation. There is no pending-ring
projection, lifecycle reducer or manually constructed expected delivery payload.

The two PRNG seeds are 388013012 (0x17209bd4) and 3620200082 (0xd7c7de92).
`ws13b_ring_matrix.rb` generates sequences; `ws13b_differential.py` regenerates,
compares and shrinks failures. The committed corpus contains the named sequence
matrix and 512 random sequences; a larger run uses the same seeds with 2,048
random sequences. Job selection indices are recorded inputs, not predictions
from Rust output. Both producer and comparator are checked by corrupting an
observed frame, push payload and banner state.
