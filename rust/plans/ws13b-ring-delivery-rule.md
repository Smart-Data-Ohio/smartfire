# Pinned Rails ring delivery rule (d7c7de92)

Written before the third-pass production fix. All line references below are to
the actual pinned source, inspected with `git show d7c7de92:<path>`.

## Source facts

1. Rails' in-app ring is a synchronous ActionCable broadcast, not a ring job.
   ActivityItem registers create/update commit callbacks at
   `app/models/activity_item.rb:32-34`. The update callback broadcasts only when
   read_at, handled_at, event_type or created_at changed (`:215-224`). It resolves
   the item's source grant, room and caller, and emits its event type and state
   (`:234-255`). State is handled before read before unread (`:172-176`).
   A banner-only invitation broadcasts directly at
   `app/models/huddle_grant.rb:558-575`, with activityItemId=0 and empty action
   paths; it has no activity row and no item generation timestamp.
2. Issuance commits the grant before after_issued! (`huddle_grant.rb:38-79`).
   Every reused grant updates last_issued_at (`:59-62`). This is the transport
   issuance timestamp, including issuances that produce no invitation.
3. Invitation dedupe happens before any invitation emission: previous issuance
   or another same-starter grant inside the inclusive two-minute window
   (`huddle_grant.rb:423-430`), then recipient already in call or a recent item
   (`:300-302`, `:414-417`). A deduped issuance emits no replacement; it does not
   retract the previous broadcast. Eligible recipients are current active humans
   in the direct room, excluding nothing/invisible memberships (`:399-411`).
4. An item-backed retry first finds the row owned by recipient+source grant
   (`huddle_grant.rb:315-319`, `:342-343`), or an unhandled same-starter attempt
   younger than ten minutes (`:354-359`). Under the item lock, a genuine refresh
   resets source/event/read/handled/created_at (`:366-388`). Only an actual
   creation/refresh broadcasts and enqueues the push job (`:334-338`).
5. The actual deferred job is Huddle::PushInvitationJob. Its argument is an
   activity_item_id; execution reloads that item and returns if absent
   (`app/jobs/huddle/push_invitation_job.rb:2-6`). InvitationPusher resolves the
   current source/room/recipient/caller and push policy
   (`app/models/huddle/invitation_pusher.rb:6-10`, `:31-44`), selecting visible,
   disconnected, opted-in memberships (`:25-28`). It does not recheck a ring
   generation or compare last_issued_at. These are push semantics, not a Rails
   in-app ring worker.
6. Last-participant call end broadcasts an ended frame, while another active
   live participant keeps the call going (`huddle_grant.rb:468-491`). Ended
   frames do not mutate the activity item's state (`:505-527`). The suppressed
   ended path has its own one-minute banner window (`:531-549`).

## Direct Rust adaptation for its additional deferred ring queue

The immutable durable job ID identifies a particular committed invitation
emission. The logical target is an activity item, or a banner's recipient,
room and caller. A later actual emission supersedes pending emissions for that
target in the same transaction as enqueue. Dedupe creates no emission, so it
cannot supersede or invalidate the pending job. Updating grant.last_issued_at
does not change a banner's invitation identity.

Execution reads its own durable row under the writer transaction, rejecting a
missing, superseded or call-ended job even if the worker retained older arguments.
It then validates current recipient access and source relationships. Item-backed
requests retain the captured item source, created_at, event type and state; an
old mutation cannot be promoted into a later retry's unread ring. Banner metadata
can be rebuilt from the source, but its emission identity is the queue row, not
the grant's latest transport issuance. The real WS17 sound policy runs only after
these checks. Publication remains after commit; durable enqueue and supersession
remain atomic with the invitation mutation.

The sequence oracle must retain Rails' full emission stream and persisted state
for every operation. Its delayed-delivery projection selects the latest pending
emission for each logical target and clears pending starts when Rails actually
executes the guarded last-participant end callback (even if the banner ended-frame
window expired). It checks current membership and active-human eligibility, and
uses the actual source/item state and active in-call scopes at drain. This projection
models Rust's extra queue; it is not described as a nonexistent Rails ring job.
Rust must match both the unprojected enqueue decisions/payloads and the projected
drains. The matrix covers both invitation forms, 0/1/181-second reissues, pending
initial/retry/update jobs, both drain orders, removal/revoke/end, re-grant, source
changes and legitimate retries, including every earlier ring-review scenario.

## Matrix and checks

`reference-tools/ws13b_ring_matrix.rb` runs 180 sequences on the pinned image and
records every target-recipient emission, immediate frame, actual push enqueue and
full persisted invitation item. Its six source-file hashes are checked against
`git show d7c7de92`. The Rust tests compare the complete phase objects without ID,
payload or timestamp masks. Fixture setup clears the demo seed's inbox rows and
restores the caller's default invitation preference to match Rails' fresh fixtures.

- 39 universal sequences × two invitation forms × two drain orders = 156.
- Four item state mutations × three retry delays × two drain orders = 24.
- Total: 78 banner-only and 102 item-backed sequences, all passing locally.

The universal families include initial/retry reissues at 0, 1, 120, 121 and 181
seconds, two pending genuine retries, a chain of deduped reissues spanning more
than 120 seconds, delayed initial jobs, quiet/live revocation, explicit end,
recipient/caller removal and sign-out before or after delivery, immediate/late
re-grant, new-session grants and a still-live group participant. The state families
cover read, handled, missed and handled-to-unread callbacks. The three ring review
regressions are named by their corresponding initial, handled-retry and
pending-retry families; the exact third-pass reviewer test also remains unchanged.
The existing nine fix regressions, the unchanged duplicate-ring reviewer probe and
its legitimate-retry/post-commit probes retain their assertions. The six earlier
generation vectors now exercise durable row identity when draining.

A separate real-adapter failure injection rejects replacement ring insertion for
each invitation form. Supersession and the invitation mutation roll back together;
the old job still delivers exactly once, and the primary grant issuance stays
committed. The complete oracle comparator rejects injected lost-ring, duplicate
retry and wrong-room metadata outcomes.
