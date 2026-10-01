# WS13 deferred Rails test declarations

Status: partial. This catalogue retains all 548 original test declarations in 33 files for traceability. 270 declarations now have complete assertion coverage mapped below. The other 278 remain partial or deferred. A declaration remains open until all of its assertions are ported, including notice and rendering effects. The three HuddleNoticeChannel tests already ported by WS7 are verified separately in the report. These test declarations remain WS13 work. The new internal HTTP tests execute 39 pinned Rails cases. They prove status/payload/no-store/liveness and persisted enqueue outcomes; they do not complete every assertion in the Rails controller tests below. WS17 owns push transport and Notifications::Policy; WS13 retains the invitation/join payload and enqueue tests. Thirty-five system declarations require LIVEKIT_SYSTEM_TESTS=1 with a real LiveKit server. The other 71 browser system declarations remain WS13 work, pending the public controllers/views and their browser harness.

## Continued slice coverage (after merging WS19b)

The following have verified core coverage, while the original declarations remain individually listed below. Owners remain WS13 unless the original entry identifies WS17 transport or Notifications::Policy.

- `controllers/internal_huddle_tests.rs`: original-token admission, case-insensitive Bearer scheme, expired/malformed rejection, secret enforcement on all three endpoints, configuration ordering, revoked/missing/removed grants, numeric-prefix IDs, record_seen=0, timestamp coercions, stale disconnect floors, no-store, exact authorization_payload, and HTTP enqueue rollback. 39 Rails request-response cases; additional signed-token shapes are validated by the existing 98-case protocol corpus. Running every protocol shape through HTTP and completing presence/notice assertions remain open.
- `tests/huddle_grant_test.rs`: eligibility/current relationships, random identities and reuse, never reviving revoked grants, requested-room coordinates, three real unique conflicts, concurrent issuance, 10-second sightings and strict 20-second first-sighting boundary, first-sighting jobs, leave floors, membership/session/ban/room revocation, role boundaries/server mute, last-host guard, and synchronous last-active-stage-grant stream state. Post-issuance invitations now have 49 exact Rails issuance scenarios; Stream callbacks now run; committed presence and leave/call-ended notices now have the coverage below.
- `huddle_gateway_node.mjs`: all 16 existing gateway tests run through real Rust endpoints. Ordinary responses are not fabricated; denial injections revoke SQLite rows. Only the original explicit outage/stall/malformed-response injections remain at the proxy. The test-only Rust launcher is ignored by default because it needs Node and the pinned ws package; it was explicitly run successfully in this slice.

The presence slice adds 50 exact pinned Rails participant renders (sidebar/header across five room kinds) and a real seeded app/queue/WebSocket proof of first-seen fan-out and silent rollback. The notice slice adds exact pinned Rails join/leave/call-ended payload sequences, real grant revoke/leave callbacks, registered join/push-invitation workers, retained unknown-handler replay, and a WS17 payload seam. Its push-scope corpus exercises 36 actual Rails invitation/join cases, subscription selection and the conditional throttle; the policy decisions are Rails inputs, so this does not port Notifications::Policy or prove transport delivery. Current raw pass counts and exact corpus sizes are in ws13-wave4-report.md.

The process task now runs invitations, stale streams and cleanups in Rails order with per-row commits and isolated failures. Stream create/end callbacks, explicit stopped events, last-host succession and quiet timeline delivery, role/mute grant revocation, and role/hand/moderation controllers are implemented. The stream/lifecycle corpus has 20 production Rails cases, moderation has 29 real requests, roles/hands has 34 real requests plus a real minute-bucket throttle probe, and Stage fragments have 400 byte-identical renders. Real socket tests prove committed changes, personal delivery, single rejoin behavior and silent rollback.

Passed titles are backed jointly by those differential tests, the real HTTP/Cable tests in `call_lifecycle_tests.rs` and `huddle_effects_tests.rs`, and the existing WS8a `room_test.rs` and grant tests. Counts are original declarations, not vector counts. Whole per-viewer fan-out declarations, combinations not explicitly exercised (for example another administrator as moderation target), preload/query assertions and creation/edit flows remain open even where the underlying method is implemented. Full public huddles and aggregate presence controllers, voice/stage CRUD controllers, and 11 of the original 19 owned views remain open. The extra event venue dot is verified with Stage fragments. WS17 retains policy/transport; real LiveKit system tests retain LIVEKIT_SYSTEM_TESTS=1 with a real server as the reason.

## WS13b domain slices

All 207 original declarations in the WS13b-assigned files now have complete coverage, including the inherited WS13 assertions: nine revocation, 33 grant, 38 invitation, 33 join-notifier, 27 stage, 27 stream, five voice, thirteen join-pusher, eight ring-policy, ten job and four reconciler declarations. Rendering remains byte-identical to pinned Rails; query counts/shapes and locks run against real SQLite. The merged WS17 policy, durable push adapter and registered ring worker now close the final thirteen dependencies. Neighboring mentions use WS17's full message recorder once; the temporary isolated recorder is removed. No assigned declarations remain deferred. WS13 controller and system rows retain their independent status.

PR #172 review regressions supplement these original declaration counts: queued rings now re-read recipient access and item/source state, and call-end callbacks cancel pending started frames, including already claimed workers. New probes in `reference-tools/ws13b_review_fixes.rb` run against Rails `d7c7de92`. Issuance commits before invitation callbacks; each invitation and its durable jobs share a later transaction, preserving earlier recipients when a later one fails. Gateway disconnect parsing now honors numeric offsets and preserves newer rejoin sightings; Rails rejects invalid 24-hour times. These review regressions do not change original declaration counts.

The re-review's duplicate retry ring also failed first on `e9ddf5d9`. Ring requests now identify the invitation generation by the refreshed item `created_at` (or banner grant `last_issued_at`) and reject superseded jobs. Six pinned Rails retry sequences, both drain orders, the reviewer's legitimate-retry/post-commit probes and the nine earlier regressions keep their executable assertions; original declaration counts stay unchanged.

## Rails declaration coverage by file

These are original declaration counts, not Rust test counts or individual vector counts. A declaration is closed only when all of its original assertions are covered. Each title remains below, including those now passed. Raw executable pass counts are in ws13-wave4-report.md.

| Rails file | Original | Assertions covered (passed) | Partial/deferred |
| --- | ---: | ---: | ---: |
| `test/controllers/rooms/stage/streams_controller_test.rb` | 38 | 16 | 22 |
| `test/models/huddle_invitation_test.rb` | 38 | 38 | 0 |
| `test/controllers/rooms/huddles_controller_test.rb` | 36 | 0 | 36 |
| `test/models/huddle/join_notifier_test.rb` | 33 | 33 | 0 |
| `test/models/huddle_grant_test.rb` | 33 | 33 | 0 |
| `test/system/huddles_test.rb` | 31 | 0 | 31 |
| `test/controllers/internal/huddle_controller_test.rb` | 29 | 0 | 29 |
| `test/models/rooms/stage_test.rb` | 27 | 27 | 0 |
| `test/models/stream_test.rb` | 27 | 27 | 0 |
| `test/controllers/rooms/stages_controller_test.rb` | 24 | 0 | 24 |
| `test/controllers/rooms/stage/roles_controller_test.rb` | 20 | 16 | 4 |
| `test/controllers/rooms/call_moderation_controller_test.rb` | 19 | 13 | 6 |
| `test/controllers/rooms/voices_controller_test.rb` | 18 | 0 | 18 |
| `test/controllers/rooms/stage_view_test.rb` | 16 | 0 | 16 |
| `test/system/huddle_join_notices_test.rb` | 16 | 0 | 16 |
| `test/controllers/rooms/stage/hands_controller_test.rb` | 15 | 9 | 6 |
| `test/system/stage_test.rb` | 15 | 0 | 15 |
| `test/models/huddle/join_pusher_test.rb` | 13 | 13 | 0 |
| `test/system/voice_channels_test.rb` | 12 | 0 | 12 |
| `test/system/huddle_invitations_test.rb` | 10 | 0 | 10 |
| `test/models/huddle/invitation_resolver_test.rb` | 9 | 9 | 0 |
| `test/models/huddle_revocation_test.rb` | 9 | 9 | 0 |
| `test/models/huddle/ring_policy_test.rb` | 8 | 8 | 0 |
| `test/system/huddle_audio_test.rb` | 8 | 0 | 8 |
| `test/system/huddle_roster_test.rb` | 8 | 0 | 8 |
| `test/controllers/users/huddle_presence_controller_test.rb` | 6 | 0 | 6 |
| `test/system/huddle_presence_test.rb` | 6 | 0 | 6 |
| `test/integration/huddle_presence_test.rb` | 5 | 0 | 5 |
| `test/models/rooms/voice_test.rb` | 5 | 5 | 0 |
| `test/jobs/huddle/join_notice_job_test.rb` | 4 | 4 | 0 |
| `test/jobs/huddle/push_invitation_job_test.rb` | 4 | 4 | 0 |
| `test/services/huddle/reconciler_test.rb` | 4 | 4 | 0 |
| `test/jobs/huddle/broadcast_presence_job_test.rb` | 2 | 2 | 0 |
| **Total** | **548** | **270** | **278** |

## test/controllers/internal/huddle_controller_test.rb

Owner: WS13. Deferred.

- authorizes an exact active grant from an original token
- authorizes a server-refreshed token that omits false permissions
- rejects invalid signatures, missing time claims, and expired tokens
- rejects admin, data, metadata, and unknown privileges
- rejects any publish source set other than the exact camera grant
- authorizes a stage listener token that cannot publish
- authorizes a listener token in server-refreshed shape
- rejects mismatched publish permission and sources
- rejects a publisher token missing its publish sources
- authorizes a refreshed listener token that omits sources and the publish flag
- malformed signed video grants receive a controlled denial
- a valid token without an exact database grant is forbidden
- a stale database link revokes the grant and is forbidden
- grant lookup rechecks current access without requiring or rechecking the original token
- grant lookup returns not found after revocation
- a successful authorization records liveness without changing the response
- grant lookup records liveness at most once per ten seconds
- an enforcement-only grant lookup authorizes without recording liveness
- an enforcement-only lookup still revokes a stale grant
- a steady-state grant check runs no transaction and writes nothing
- a denied lookup takes the lock and revokes exactly once
- a denied lookup does not record liveness
- a disconnect event marks the grant out of the call and refreshes presence
- a stale disconnect event keeps a newer sighting
- a disconnect event with no timestamp clears liveness
- a disconnect event with a malformed timestamp is unprocessable
- a disconnect event for an unknown grant is not found
- gateway authentication is required for every endpoint
- internal endpoints fail closed when huddles are not fully configured

## test/controllers/rooms/call_moderation_controller_test.rb

Owner: WS13. Partial: 13/19 declarations passed; the other 6 remain WS13 work.

- a host server-mutes a speaker, revoking publish until unmuted
- muting delivers a roster to every member and a rejoin event to the target
- **Passed:** mute and unmute role events carry the server-muted state
- **Passed:** muting twice and unmuting a member who was never muted both succeed
- **Passed:** a repeated mute keeps the member's fresh grant and sends no rejoin
- **Passed:** unmuting a member who was never muted sends no rejoin
- a publish grant that survived a mute fails authorization
- **Passed:** disconnect drops the member from the call but keeps the membership
- **Passed:** an administrator who is not a host moderates a stage room
- **Passed:** a host cannot mute, unmute, or disconnect an administrator
- an administrator moderates another administrator
- **Passed:** a server-muted administrator unmutes themselves
- **Passed:** a server-muted host cannot unmute themselves
- speakers and listeners cannot moderate
- **Passed:** moderating your own session is rejected
- **Passed:** moderation is unreachable outside stage and voice rooms
- **Passed:** moderation denies outsiders, unknown memberships, and unauthenticated requests
- **Passed:** an administrator server-mutes a voice member, and members cannot
- server mute only exists on stage and voice rooms

## test/controllers/rooms/huddles_controller_test.rb

Owner: WS13. Deferred.

- an authorized room member receives a narrowly scoped join token
- the active grant is reused while its random participant identity remains opaque
- direct rooms use their participant-based display name
- starting a one-to-one DM huddle invites only the other participant
- starting a channel huddle creates no invitation
- group direct rooms can start a huddle
- a nonmember cannot get a grant in a group DM and a removed member loses theirs
- GET confirms ongoing access without returning credentials
- GET denies access after room membership is revoked
- GET denies access to a soft-deleted room
- participants lists the room's in-call members without caching
- participants reflects only in-call grants
- a member removed mid-call is revoked and disappears from participants
- participants is reported for group direct rooms
- participants denies non-members, outsiders, and unauthenticated requests
- participants denies bots and inactive users
- participants requires LiveKit configuration
- GET denies access after sign out
- a nonmember cannot join a closed room
- a nonmember cannot mint voice credentials
- participants denies a member after removal
- an outsider cannot join a direct room
- an unauthenticated request receives JSON instead of a redirect
- bots cannot join
- bots cannot join through an ordinary session
- banned users cannot join
- missing LiveKit configuration is reported without minting a token
- a concurrent membership revocation receives a controlled denial
- a stage listener's token cannot publish anything
- stage speakers and hosts publish like any other participant
- voice and direct room tokens still publish
- a public URL pointing directly at the internal LiveKit address is rejected
- leaving drops the session's grants out of the call without revoking
- leaving works for voice rooms and group directs, like participants
- leaving twice, or without ever joining, still answers no content
- leaving denies non-members, bots, and unauthenticated requests

## test/controllers/rooms/stage/hands_controller_test.rb

Owner: WS13. Partial: 9/15 declarations passed; the other 6 remain WS13 work.

- a listener raises their hand and every viewer gets their own roster
- a double raise keeps the first timestamp and queue place
- **Passed:** raising hands is rate limited per membership
- **Passed:** the hand-raise rate limit resets after a minute
- **Passed:** a turbo-stream raise swaps the actor's own controls without navigating
- **Passed:** speakers and hosts cannot raise a hand
- **Passed:** a listener lowers their own hand
- **Passed:** lowering a hand that was never raised succeeds
- **Passed:** a host lowers another member's hand without promoting them
- **Passed:** an administrator member lowers another member's hand
- a listener cannot lower another member's hand
- **Passed:** lowering a non-member's hand is not found
- non-members get not found
- an administrator who is not a member gets not found
- hands do not exist outside stage rooms

## test/controllers/rooms/stage/roles_controller_test.rb

Owner: WS13. Partial: 16/20 declarations passed; the other 4 remain WS13 work.

- a host promotes a listener, clearing their hand and revoking their grants
- **Passed:** the affected member's panel replacement carries no rejoin trigger
- **Passed:** a publish-boundary crossing appends a rejoin event to the member's persistent target
- **Passed:** a host-speaker change broadcasts roster and panel but no rejoin event and revokes nothing
- **Passed:** a demotion appends a rejoin event to the member's persistent target
- **Passed:** a turbo-stream role change replaces the roster without navigating
- **Passed:** a host demotes a speaker back to the audience
- **Passed:** a host demoting themselves is allowed unless they are the last host
- **Passed:** a failed last-host demotion revokes nothing
- **Passed:** a host who is not an administrator cannot demote an administrator
- **Passed:** a listener cannot change anyone's role
- **Passed:** a speaker cannot change anyone's role
- **Passed:** an administrator member manages roles without being a host
- an administrator who is not a member gets not found
- an administrator member promotes a new host when the stage has none
- **Passed:** non-members get not found
- **Passed:** changing a non-member's role is not found
- **Passed:** an unknown role is unprocessable
- a missing role is unprocessable
- **Passed:** roles do not exist outside stage rooms

## test/controllers/rooms/stage/streams_controller_test.rb

Owner: WS13. Partial: 16/38 declarations passed; the other 22 remain WS13 work.

- a host goes live, broadcasting the badge, dot, and panels
- **Passed:** a speaker goes live
- **Passed:** a turbo-stream start swaps the actor's own panel without navigating
- a listener cannot go live
- **Passed:** a speaker cannot go live when the stage has no host
- a speaker goes live again after the last host leaves and a successor is promoted
- **Passed:** an administrator listener cannot go live
- **Passed:** a server-muted speaker cannot go live
- a server-muted host cannot go live
- **Passed:** a host without a huddle grant cannot go live
- a host whose grant was revoked cannot go live
- a host with a quiet grant cannot go live
- a host whose grant went quiet cannot go live
- **Passed:** an unknown quality is unprocessable
- a missing quality is unprocessable
- **Passed:** starting while another stream is live returns conflict naming the presenter
- the presenter stops the stream
- a speaker presenter stops their own stream
- **Passed:** a host stops another member's stream
- a host stop appends a stream-stopped event for the presenter
- a presenter stop appends no stream-stopped event
- an administrator member stops the stream without being a host
- a listener cannot stop the stream
- **Passed:** a speaker who is not the presenter cannot stop the stream
- **Passed:** stopping with the live stream id ends that stream
- **Passed:** stopping with a stale stream id ends nothing, even when another stream is live
- stopping with an unknown stream id ends nothing
- **Passed:** the stop control sends its stream id
- stopping with nothing live succeeds for hosts and stays silent
- stopping with nothing live is forbidden for listeners
- a turbo-stream stop swaps the actor's own panel without navigating
- non-members get not found
- an administrator who is not a member gets not found
- streams do not exist outside stage rooms
- **Passed:** demoting the presenter to listener ends the stream in the same transaction
- removing the presenter through the members edit ends the stream
- **Passed:** promoting a speaker to host keeps the grant and the live stream
- **Passed:** demoting a host to speaker keeps the grant and the live stream

## test/controllers/rooms/stage_view_test.rb

Owner: WS13. Deferred.

- a listener's join control hints that publishing is unavailable
- a host's join control hints that publishing is available
- a speaker's join control hints that publishing is available
- a voice channel's join control carries no publishing hint
- the layout renders the persistent role-event target for signed-in users
- a listener's stage panel renders no role forms
- a host's stage panel renders role forms
- a host sees no moderation controls on an administrator's row
- a muted administrator sees an unmute control on their own row
- an administrator sees moderation controls on every other row
- the header Live badge renders only while live
- the sidebar live dot renders only while live
- the Go live form renders for hosts and speakers
- the Go live form renders for no listener and never while live
- Stop stream renders for the presenter but not for listeners
- Stop stream renders for hosts and the presenting speaker

## test/controllers/rooms/stages_controller_test.rb

Owner: WS13. Deferred.

- show redirects to get general show
- new
- create makes the creator host and the rest listeners
- create prepends the stage row into the stage section
- create forbidden by non-admin when account restricts creation to admins
- update with membership revisions makes new members listeners
- removing a member tells them to drop the sidebar row and header stack
- a non-administrator creator can manage members of their own stage room
- only admins or creators can update
- the sole host cannot remove themselves while others remain
- create with an unknown icon re-renders the new form
- update with an unknown icon re-renders the edit form without revising members
- update with an icon normalizes the shortcode
- update clears the icon with a blank shortcode
- updating the icon replaces sidebar rows and headers for members only
- a host removes themselves once another host exists
- the sole-host check and revision run in one locked transaction
- removing everyone including the last host empties the room
- the room page and the members edit render with zero hosts
- non-members cannot see the room page or its messages
- non-members cannot reach the stage namespace
- open, closed, and voice rooms cannot be converted to stage
- stage rooms cannot be converted through the open, closed, or voice namespaces
- a direct room can't be converted to stage and have its participants revised

## test/controllers/rooms/voices_controller_test.rb

Owner: WS13. Deferred.

- show redirects to get general show
- new
- create
- create prepends the voice row into the voice section
- create forbidden by non-admin when account restricts creation to admins
- update with an icon normalizes the shortcode
- create with an unknown icon re-renders the new form
- update with an unknown icon re-renders the edit form
- update with membership revisions
- removing a member tells them to drop the sidebar row and header stack
- a non-administrator creator can manage members of their own voice room
- only admins or creators can update
- remove yourself
- non-members cannot see the room page or its messages
- non-members cannot reach the voice namespace
- open and closed rooms cannot be converted to voice
- voice rooms cannot be converted through the open or closed namespaces
- a direct room can't be converted to voice and have its participants revised

## test/controllers/users/huddle_presence_controller_test.rb

Owner: WS13. Deferred.

- returns only the current user's rooms with at least one participant
- the response runs one grants query no matter how many rooms are live
- returns an empty list when nobody is in any call
- an unauthenticated request receives JSON instead of a redirect
- bots and inactive users are denied like the huddle controller
- missing LiveKit configuration is reported

## test/integration/huddle_presence_test.rb

Owner: WS13. Deferred.

- channel header shows the live stack immediately before the join button
- quiet channel header keeps an empty stack target
- two-person DM header shows the live stack
- group DM header shows the live stack immediately before the join button
- no header stack without huddle configuration

## test/jobs/huddle/broadcast_presence_job_test.rb

Owner: WS13b. Passed: both declarations. `presence_job_fanout_and_missing_room_match_rails_counts` executes the registered durable worker with two real WebSockets, comparing all three pinned Rails stream counts and target fragments; missing grants and missing rooms emit no frames.

- **Passed:** broadcasts the room's current stacks
- **Passed:** missing grants and rooms stay silent

## test/jobs/huddle/join_notice_job_test.rb

Owner: WS13b. Passed: all four declarations. The 33 notifier sequences compare exact first/repeat-sighting enqueue IDs and perform real join callbacks; registered-worker tests verify persisted WS17 payload jobs, room-member notices and successful missing-grant discard.

- **Passed:** a first sighting enqueues the join notice alongside the presence broadcast
- **Passed:** a repeat sighting enqueues no join notice
- **Passed:** performing the job notifies the room's members
- **Passed:** a missing grant is ignored

## test/jobs/huddle/push_invitation_job_test.rb

Owner: WS13b. Passed: all four job declarations. Four actual pinned Rails job cases map to `huddle_invitation_job_test.rs`, with registered producer-worker persistence/missing-source tests. Recipient-only payloads and off/connected subscription scopes match. The merged WS17 intent handler evaluates current policy and uses its atomic durable delivery adapter; registered TLS tests verify actual wire delivery.

- **Passed:** pushes the invitation to the recipient only
- **Passed:** an opted-out recipient gets no push subscriptions
- **Passed:** a connected recipient gets no push
- **Passed:** missing invitations are ignored

## test/models/huddle/invitation_resolver_test.rb

Owner: WS13. Passed: all nine declarations, mapped to the 29-case Rails resolver corpus and `overdue_invitations_match_twenty_nine_rails_scenarios_and_are_idempotent`. Complete row snapshots cover state, unread/read/handled stamps and item counts; group and per-user cases cover both recipients; a second pass proves idempotence.

- **Passed:** an unanswered invitation becomes a missed call and stays unread
- **Passed:** unanswered group invitations each become missed calls
- **Passed:** a recipient who was issued a grant since the start has their invitation handled
- **Passed:** a recipient seen in the call has their invitation handled
- **Passed:** the starter leaving before the wait elapses is a missed call
- **Passed:** an invitation within the wait is left alone
- **Passed:** an already-handled invitation is left alone
- **Passed:** resolving twice keeps a single missed item
- **Passed:** resolution can be scoped to one user

## test/models/huddle/join_notifier_test.rb

Owner: WS13b. Passed: 33/33 declarations. The 33 notifier sequences compare exact notices, sighting jobs, subscriptions and throttle state. Actual SQLite tracing in `huddle_query_assertions_test.rs` verifies two shared user queries (bulk viewers and shared member names) and one ring query for both four- and eight-member groups. Cached inbox preferences avoid additional per-recipient user queries.

- **Passed:** an in-call DM member is told when the peer joins, and the joiner is not
- **Passed:** a member with no access to the room is told nothing
- **Passed:** an out-of-call DM member gets the banner broadcast and one push
- **Passed:** a group DM join toasts the insider and banners the outsider
- **Passed:** join fan-out loads members and rings once no matter the group size
- **Passed:** a channel join toasts the insider and tells the outsider nothing
- **Passed:** an out-of-call channel member gets no banner and no push
- **Passed:** a voice room join toasts the insider and tells the outsider nothing
- **Passed:** bots and deactivated members are told nothing
- **Passed:** a bot join notifies nobody
- **Passed:** a second device sighted while the first is listed enqueues no join notice
- **Passed:** sightings from two devices before the job runs still notify once
- **Passed:** a join job running after the joiner left notifies nobody
- **Passed:** a viewer whose ring is still live gets no join notice
- **Passed:** a viewer whose ring went stale gets the join notice again
- **Passed:** a switched-off or hidden room stays silent for the outsider
- **Passed:** an outsider with huddle invitations switched off still banners but gets no push
- **Passed:** a muted room still banners the outsider but sends no push
- **Passed:** a join after a recent in-call revoke is marked as a rejoin
- **Passed:** a rejoin after the disconnect report cleared liveness is not marked
- **Passed:** a join after a quiet revoke is not marked as a rejoin
- **Passed:** a join after the rejoin window is not marked as a rejoin
- **Passed:** a join ten seconds after the revoke is not marked as a rejoin
- **Passed:** leaving toasts the members still in the call
- **Passed:** revoking an in-call grant toasts the members still in the call
- **Passed:** leaving tells out-of-call DM members so their banner drops the leaver
- **Passed:** leaving a channel tells out-of-call members nothing
- **Passed:** the last one out of a DM dismisses every other member's banner
- **Passed:** the last one out of a channel dismisses nothing
- **Passed:** leaving a call the grant was never in toasts nobody
- **Passed:** revoking a quiet grant toasts nobody
- **Passed:** leaving while another of the leaver's grants is still in toasts nobody
- **Passed:** a leave report after a revocation does not toast twice

## test/models/huddle/join_pusher_test.rb

Owner: WS13b. Passed: all 13 declarations. `huddle_join_push_sequences_test.rs` persists the actual DND/allowance/quiet/meeting/OOO records, derives and checks WS17's current `NotificationPolicy`, then passes the original WS13 wire DTO to the real `enqueue_huddle_request` adapter. All 31 intermediate payload/subscription/throttle/timestamp states match pinned Rails. The merged registered bridge/delivery/TLS integration tests exercise the same adapter and complete decrypted payloads. No oracle policy decision is supplied to the adapter.

- **Passed:** pushes the join to the recipient's subscriptions and stamps the throttle
- **Passed:** a second push inside ten minutes is throttled
- **Passed:** a push ten minutes later goes out again
- **Passed:** a DND recipient gets no push and burns no throttle window
- **Passed:** a starred joiner still pushes through DND
- **Passed:** a recipient in quiet hours gets no push
- **Passed:** a recipient quiet in a meeting gets no push
- **Passed:** an out-of-office recipient gets no push unless they keep notifications on
- **Passed:** a connected recipient gets no push and burns no throttle window
- **Passed:** a switched-off or hidden room gets no push
- **Passed:** a muted room gets no push and burns no throttle window
- **Passed:** a recipient with huddle invitations switched off gets no push and burns no throttle window
- **Passed:** a recipient with no subscriptions burns no throttle window

## test/models/huddle/ring_policy_test.rb

Owner: WS13b. Passed: all eight declarations. `huddle_ring_policy_seam_test.rs` loads actual persisted Rails recipient/caller/allow-list/meeting/OOO contexts and derives WS17 sound policy before publishing thirteen exact invitation frames. The quiet-check override replaces policy for both original recipients. `registered_ring_worker_uses_current_ws17_policy_and_exact_rails_frames` consumes real intents through the merged production registry and verifies all eleven production-policy frames over a real WebSocket, including caller-specific allowances. No WS17 ignores or pending policy inputs remain.

- **Passed:** an invitation rings a member who is not in do-not-disturb
- **Passed:** do-not-disturb silences the ring
- **Passed:** a caller allowed during do-not-disturb still rings
- **Passed:** a quiet check override replaces the policy
- **Passed:** quiet-during-meetings silences the ring during a busy interval
- **Passed:** a caller allowed during do-not-disturb still rings through a meeting
- **Passed:** out of office silences the ring unless the member keeps notifications on
- **Passed:** a caller allowed during do-not-disturb still rings through out of office

## test/models/huddle_grant_test.rb

Owner: WS13b. Passed: 33/33 declarations. The grant sequence corpus covers domain state; `jobs/huddle_render_tests.rs` closes all fourteen presence assertions with complete pinned Rails Turbo Stream bytes, destination counts, real registered presence-job execution, repeat-sighting silence and unconfigured/deleted-room gates.

- **Passed:** an active session and membership reuse one random grant
- **Passed:** revoking and restoring room membership never resurrects the old grant
- **Passed:** issuance rejects a stale or cross-user membership
- **Passed:** issuance stops after three uniqueness conflicts
- **Passed:** issuance stamps last_issued_at on create and on reuse
- **Passed:** joining another room ends the session's in-call grant there but keeps quiet ones
- **Passed:** rejoining the same room keeps the session's grant there
- **Passed:** in_call reflects gateway liveness within twenty seconds
- **Passed:** record_seen! persists liveness at most once per ten seconds
- **Passed:** mark_out_of_call! drops liveness without revoking and refreshes presence
- **Passed:** mark_out_of_call! is silent when the grant was never seen
- **Passed:** mark_out_of_call! keeps a sighting newer than the disconnect
- **Passed:** participants_for lists distinct in-call users by name
- **Passed:** participants_for drops revoked and quiet grants
- **Passed:** issuing a voice grant refreshes the presence stacks
- **Passed:** revoking a voice grant refreshes the presence stacks
- **Passed:** first sighting in the call enqueues a presence refresh, later sightings stay silent
- **Passed:** issuing an open channel grant refreshes every sidebar and the header
- **Passed:** issuing a closed channel grant refreshes every sidebar and the header
- **Passed:** issuing a direct grant refreshes both sidebars and the header
- **Passed:** revoking a channel grant refreshes every sidebar and the header
- **Passed:** revoking a direct grant refreshes both sidebars and the header
- **Passed:** first sighting in a channel enqueues a presence refresh, later sightings stay silent
- **Passed:** no presence broadcasts without huddle configuration
- **Passed:** revoking a destroyed room's grants stays silent
- **Passed:** a stage grant records the membership role it was issued for
- **Passed:** non-stage grants record no role
- **Passed:** a stage role change revokes the member's active grants with cleanup
- **Passed:** a host-speaker change updates the grant's role in place without revoking
- **Passed:** authorize_or_revoke! revokes a grant whose issued role no longer matches
- **Passed:** rejoining after a role change issues a new grant for the new role
- **Passed:** issuing a stage grant refreshes the presence stacks
- **Passed:** revoking a stage grant refreshes the presence stacks

## test/models/huddle_invitation_test.rb

Owner: WS13b. Passed: 38/38 declarations. Stateful sequences compare grant/item attributes, huddle JSON and immediate job IDs. `jobs/huddle_neighbor_mention_test.rs` closes the neighboring mention assertion using the real signed Rails Action Text attachment and actual message creation, including eleven recipient/visibility/thread guards and recorder idempotence. The merged WS17 full message recorder now owns this hook; the temporary isolated mention implementation has been removed.

- **Passed:** issuing a grant in a one-to-one DM invites only the other participant
- **Passed:** issuing a grant never schedules a delayed job
- **Passed:** a quiet check silences the invitation payload but keeps the item
- **Passed:** a recipient with notifications off or invisible gets no invitation
- **Passed:** a recipient with huddle items switched off still gets the banner but no item
- **Passed:** a switched-off user hears one banner across reissues inside the window and a fresh one after
- **Passed:** a switched-off user is not rung again when the same session reissues its grant
- **Passed:** channel huddles create no invitation
- **Passed:** voice channel huddles create no invitation
- **Passed:** no invitation while the other participant is in the call
- **Passed:** an invitation fires when the other participant's grant went quiet
- **Passed:** reusing the same grant rings again once the dedup window has passed
- **Passed:** a second grant for the same starter does not ring again inside two minutes
- **Passed:** no ring inside the two-minute window after a missed invitation
- **Passed:** a handled invitation still suppresses the next ring inside two minutes
- **Passed:** an invitation older than two minutes re-rings through the same row
- **Passed:** a handled invitation older than two minutes re-rings through the same row
- **Passed:** a missed invitation older than two minutes re-rings through the same row
- **Passed:** revoking the starter's in-call grant broadcasts call-ended to the invitee
- **Passed:** the starter leaving the call broadcasts call-ended to the invitee
- **Passed:** revoking a quiet grant broadcasts no call-ended
- **Passed:** no call-ended when the recipient already joined
- **Passed:** a suppressed ring ends with a banner-only call-ended
- **Passed:** no banner-only call-ended when the ring long stopped
- **Passed:** the starter leaving a group call while others remain sends no call-ended
- **Passed:** revoking the starter's grant while others remain in a group call sends no call-ended
- **Passed:** call-ended fires when the last participant leaves a group call
- **Passed:** a group ring continues for remaining invitees until their ring timeout
- **Passed:** retrying after the window with a new grant reuses the unhandled item
- **Passed:** a retry after a handled attempt opens a new item
- **Passed:** a retry from the first device re-rings through its own item
- **Passed:** the same attempt reuses its item inside ten minutes and opens a new one after
- **Passed:** obtaining a grant clears the recipient's open invitations for the room
- **Passed:** joining late clears the missed item
- **Passed:** a DM with only bots besides the starter gets no invitation
- **Passed:** issuing a grant in a group DM invites every other human member
- **Passed:** group DM invitations skip bots and members who switched the room off
- **Passed:** a removed group member gets no invitation and loses their grant

## test/models/huddle_revocation_test.rb

Owner: WS13b. Passed against nine pinned Rails lifecycle scenarios in `huddle_revocation_test.rs`; five additional cleanup-failure rollback tests cover atomicity. LiveKit transport remains WS17.

- **Passed:** membership revocation persists cleanup for only that grant
- **Passed:** session removal revokes its grants in every room and leaves another session active
- **Passed:** banning a user revokes grants even though sessions are bulk deleted first
- **Passed:** deactivating a user revokes grants after memberships and sessions are bulk deleted
- **Passed:** destroying a room revokes grants and persists one room deletion
- **Passed:** removing a voice member mid-call revokes only their grant
- **Passed:** destroying a voice room revokes its grants and persists one room deletion
- **Passed:** deactivating a user ends their voice session
- **Passed:** revocation remains durable while LiveKit is unavailable

## test/models/rooms/stage_test.rb

Owner: WS13b. Passed: 27/27 declarations. Domain lifecycle sequences cover succession, membership defaults and reachability. Actual SQLite tracing checks immediate transaction acquisition before the room read and last-host check for demotion and destruction. The per-instance `stage::StageRoom` association cache reproduces live/ended preload IDs and zero-query getters; unloaded reads execute one live-stream query.

- **Passed:** type predicate
- **Passed:** stage rooms are listed without directs but outside the voice scope
- **Passed:** default involvement for new members is mentions
- **Passed:** the room creator becomes host and every other member becomes a listener
- **Passed:** the creator becomes host even when they were not in the member list
- **Passed:** members added later become listeners
- **Passed:** the last host cannot be demoted
- **Passed:** a host demotion checks for another host after locking the room in its transaction
- **Passed:** a host can step down once another host exists
- **Passed:** non-stage rooms leave the stage columns nil
- **Passed:** only listeners can raise a hand, and any promotion clears it
- **Passed:** raising twice keeps the first timestamp
- **Passed:** lowering a hand that was never raised succeeds
- **Passed:** stage members can reach the room's messages like any channel
- **Passed:** deactivating a user removes their stage memberships
- **Passed:** deactivating the sole host ends the live session and promotes an administrator member
- **Passed:** deactivating the sole host promotes the earliest remaining member without an administrator
- **Passed:** deactivating a host promotes nobody when another host remains
- **Passed:** deactivating the last member of a stage leaves the emptied room alone
- **Passed:** destroying the last host membership ends the live session and promotes an administrator successor
- **Passed:** destroying the last host promotes the earliest remaining member without an administrator
- **Passed:** destroying a host while another host remains ends nothing and promotes nobody
- **Passed:** destroying a speaker ends only their own stream and grants
- **Passed:** destroying the last host locks the room and checks for another host inside its transaction
- **Passed:** live_stream reads the preloaded live stream without querying
- **Passed:** live_stream is nil from the preloaded association once the stream has ended
- **Passed:** live_stream queries fresh when streams are not preloaded

## test/models/rooms/voice_test.rb

Owner: WS13b. Passed: 5/5 declarations. The extended domain lifecycle corpus adds voice/channel scopes, member and outsider message reachability, and membership removal on deactivation.

- **Passed:** type predicate
- **Passed:** voices scope and channel queries include voice rooms
- **Passed:** default involvement for new members is mentions
- **Passed:** voice members can reach the room's messages like any channel
- **Passed:** deactivating a user removes their voice memberships

## test/models/stream_test.rb

Owner: WS13b. Passed: 27/27 declarations. Domain lifecycle sequences cover persistence; complete pinned Rails Turbo Stream bytes cover badges, dots, personalized panels and stop events over real sockets. Actual SQLite tracing verifies that non-stage grant revocation executes zero stream queries.

- **Passed:** quality must be a known preset
- **Passed:** started_at defaults to now
- **Passed:** live scope only returns unended streams
- **Passed:** one live stream per room
- **Passed:** an ended stream frees the room for another
- **Passed:** end! is idempotent
- **Passed:** starting broadcasts the badge, dot, and per-viewer panel
- **Passed:** starting broadcasts the event venue dot
- **Passed:** ending broadcasts the cleared event venue dot
- **Passed:** ending broadcasts the cleared badge, dot, and panel
- **Passed:** ending twice broadcasts once
- **Passed:** a host stop appends a stream-stopped event to the presenter's persistent target
- **Passed:** a presenter stop appends no stream-stopped event
- **Passed:** an automatic end appends no stream-stopped event
- **Passed:** revoking the presenter's last grant for the room ends the stream
- **Passed:** revoking another member's grant leaves the stream live
- **Passed:** a grant revoked through authorization ends the stream
- **Passed:** removing the presenter's membership ends the stream
- **Passed:** removing the presenter's membership without grants ends the stream and broadcasts the end
- **Passed:** deactivating the presenter ends the stream
- **Passed:** deactivating the presenter ends the stream even without grants
- **Passed:** destroying the room destroys its streams
- **Passed:** revoking a grant outside a stage room runs no stream queries
- **Passed:** end_stale_live! ends streams whose presenter went quiet over thirty seconds ago
- **Passed:** end_stale_live! ends streams whose presenter was never seen
- **Passed:** end_stale_live! keeps streams with a recently seen presenter
- **Passed:** end_stale_live! ignores other memberships' grants in the room

## test/services/huddle/reconciler_test.rb

Owner: WS13b. Passed: all four declarations. Actual process tests compare pinned Rails phase order using SQLite triggers and a LiveKit HTTP fixture, quiet-presenter ending, per-row commits, failure continuation and exception-class-only logs. The Rails corpus retains normal, StandardError, SQL and validation-error cases.

- **Passed:** one pass resolves overdue invitations, ends stale streams, and reconciles cleanup
- **Passed:** a resolver failure is logged and does not stop cleanup reconciliation
- **Passed:** a stale-stream failure is logged and does not stop cleanup reconciliation
- **Passed:** one pass ends a quiet presenter's live stream

## test/system/huddle_audio_test.rb

Owner: WS13. Open browser system coverage; public huddle/voice/stage controllers and HTML integration must land, then run the browser harness. These cases use stubbed rooms and do not require a real LiveKit server.

- the capture asks for no browser suppression while RNNoise is on
- the capture keeps browser suppression while RNNoise is off
- unmuting requests the selected device without rewriting the room defaults
- muting keeps the noise processor attached across mute and unmute
- toggling noise suppression off re-acquires the microphone with browser suppression
- toggling noise suppression on re-acquires the microphone without browser suppression
- toggling noise suppression off without a stored device re-acquires on the live device
- toggling noise suppression off while muted refreshes the stored constraints

## test/system/huddle_invitations_test.rb

Owner: WS13. Open browser system coverage; public huddle/voice/stage controllers and HTML integration must land, then run the browser harness. These cases use stubbed rooms and do not require a real LiveKit server.

- the recipient sees an incoming huddle banner and dismissing it marks the item read
- joining from the banner marks the item handled, navigates to the DM room, and rings the huddle panel
- the banner flips to caller-left when the starter hangs up, then dismisses
- a ring stops itself after the ring timeout
- a banner-only ring stops itself after the ring timeout
- the banner stays hidden while already in the room's huddle
- an incoming call rings audibly until it is answered
- a silent invitation shows the banner without any sound
- a hidden tab raises a system notification for the call
- the ring stops when the caller leaves

## test/system/huddle_join_notices_test.rb

Owner: WS13. Open browser system coverage; public huddle/voice/stage controllers and HTML integration must land, then run the browser harness. These cases use stubbed rooms and do not require a real LiveKit server.

- an in-call member sees a join toast and hears the join sound
- rapid joins batch into one toast with one sound
- a join toast stays silent with DND on
- join and leave toasts announce through the container live region
- a leave toasts quietly in the call without the join sound
- a server mute cycle toasts neither left nor joined
- a server mute cycle stays silent when the join arrives before the leave
- a rejoin after the leave toasted toasts joined again
- a genuine leave after a join-first mute cycle still toasts
- a genuine rejoin after the leave delay toasts joined again
- a server mute cycle across a room switch stays silent
- joining a sidebar pill from another room navigates then joins
- an out-of-call member sees the banner and sidebar pill and joins from the banner
- the join banner drops each leaver and hides when empty
- the join banner reconciles its roster with presence refreshes
- the join banner clears when the huddle ends

## test/system/huddle_presence_test.rb

Owner: WS13. Open browser system coverage; public huddle/voice/stage controllers and HTML integration must land, then run the browser harness. These cases use stubbed rooms and do not require a real LiveKit server.

- the channel sidebar row and header show participants and empty on revoke
- the DM sidebar row and header show the peer and empty on revoke
- the sidebar aggregate poll clears quietly expired grants
- the sidebar aggregate poll runs on connect and skips in-flight refreshes
- a removed sidebar stack clears once but keeps accepting updates while the header latches
- the aggregate poller skips while hidden and fetches on becoming visible

## test/system/huddle_roster_test.rb

Owner: WS13. Open browser system coverage; public huddle/voice/stage controllers and HTML integration must land, then run the browser harness. These cases use stubbed rooms and do not require a real LiveKit server.

- muting patches the local roster row instead of rebuilding it
- speaking and mute changes patch the remote row in place
- joining and leaving adds and removes roster rows only
- the mute toggle keeps a stable label with pressed state and tooltip
- the camera toggle keeps a stable label with pressed state and tooltip
- the meter stops once its track ends
- the meter skips ticks while the tab is hidden
- leaving reports after the disconnect completes

## test/system/huddles_test.rb

Owner: WS13. Deferred: all 31 declarations require LIVEKIT_SYSTEM_TESTS=1 and a configured real LiveKit server (the class setup skips otherwise).

- two users exchange audio and a screen while navigating and muting
- two users exchange camera video while navigating, muting, toggling, and leaving
- the mute and camera toggles keep stable labels with pressed state and tooltips
- a camera that fails to start keeps the huddle connected and stays retryable
- a camera switched in-call falls back silently when it is unplugged
- two direct message participants exchange audio and screen while navigating and reconnecting
- a viewer enlarges a shared screen into theater mode and leaves it with escape
- the room header shares-screen button opens the shared screen
- the full screen control asks for the figure and falls back to the video element
- noise suppression runs on the microphone, can be switched off, and is remembered
- noise suppression can be switched back on without leaving the huddle
- the capture's browser suppression follows the RNNoise processor
- muting and unmuting keeps the noise suppressor on the microphone
- a second shared screen stays reachable while the first one is expanded
- a noise suppressor that fails to load still connects the huddle and stays retryable
- a browser that cannot run the noise suppressor turns the control off for good
- denied microphone leaves no ghost participant and can be retried
- denied microphone stops the device check before anything is published and can be retried
- device pickers list the fake devices and switching keeps media flowing
- an SDK device retarget keeps the stored preference and updates the picker
- the microphone meter follows the fake microphone and rests while muted
- the microphone meter restarts after a full reconnect
- the device check appears for a first join and is skipped once permissions were granted
- the connection indicator renders and the details panel shows sampled statistics
- reopening the connection panel samples bitrates from scratch
- the connection panel shows no received bitrate while alone in the call
- server removal disconnects only the targeted participant and stops their media
- the SDK reconnects through the gateway with LiveKit's refreshed token
- membership revocation rejects original and refreshed tokens even after membership is restored
- session revocation rejects both tokens and leaves the other participant connected
- server enforcement removes a revoked participant that ignores browser access checks

## test/system/stage_test.rb

Owner: WS13. Partial/deferred: 11 browser declarations remain WS13 public-controller/HTML/browser-harness work. Four declarations explicitly skip unless LIVEKIT_SYSTEM_TESTS=1 with a real server: subscribe-only listener/host publishing, role-change reconnect publishing, host server-mute/unmute reconnect, and full listener reconnect.

- stage rooms list in their own section with distinct creation controls and a stage panel
- a listener raises and lowers their hand without seeing host controls
- raised hands appear in the host's panel live, in order
- a host invites a listener to speak and moves them back to the audience
- a host lowers a raised hand without promoting
- the last host cannot demote themselves from the panel
- join stage dispatches huddle:join and toggles while connected
- a listener joins without a microphone or device check
- a demoted speaker retries as a listener without entering prejoin
- stage rooms carry ordinary text chat
- a listener joins subscribe-only while the host publishes
- inviting a listener to speak rejoins them publishing, and moving them back removes publish
- a host server-mutes a speaker and they rejoin muted, then unmutes them
- a listener survives a full reconnect and stays subscribe-only
- a muted speaker is told and rejoins without microphone prejoin

## test/system/voice_channels_test.rb

Owner: WS13. Open browser system coverage; public huddle/voice/stage controllers and HTML integration must land, then run the browser harness. These cases use stubbed rooms and do not require a real LiveKit server.

- the sidebar row and header show participants and update when a grant is revoked
- the sidebar loads once when the cable connects and reloads on reconnect
- rooms stream broadcasts survive the reconnect sidebar reload
- join voice dispatches huddle:join
- the button toggles to leave voice while connected and leaves through the panel
- presence refreshes once quiet grants expire
- leaving through the panel clears presence immediately
- voice rooms carry ordinary text chat
- removing a member drops their sidebar row and header stack without errors
- a participants 404 stops polling and clears the stack without retrying
- the voice header fits narrow phones and caps the stack
- the room page shares one participants request across its stacks
