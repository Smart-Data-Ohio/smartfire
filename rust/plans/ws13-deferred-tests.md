# WS13 deferred Rails test declarations

Status: partial. All 548 original declarations remain inventoried across 33 Rails files: 290 passed, 258 open. All 226 WS13 controller/integration declarations have complete assertions. WS13b owns the 216 model/job/service declarations, whose historical 37 passed and 179 open counts remain unchanged pending PR #172; that branch has not been merged. WS17's public seams stay unchanged. Eleven Stage, eight audio-processing and eight roster browser declarations now pass against real Rust pages, JavaScript, sessions and Cable. The other 79 system declarations remain open: 44 ordinary browser cases and 35 real LiveKit cases (31 huddles, four Stage), individually retained below. All 38 complete room pages, including both Designers pages, match the frozen Rails fixtures plus the approved #163 application-layout action. No screenshot or pixel work remains. Counts refer to complete original declarations, not vector or Rust-test counts.

## Continued slice coverage (after merging WS19b)

Historical domain coverage predates the WS13b split and is retained below. WS13b now owns the model/job/service files; WS13 owns controllers, integration and system files. WS17 retains transport and Notifications::Policy.

- `controllers/internal_huddle_tests.rs`: original-token admission, case-insensitive Bearer scheme, expired/malformed rejection, secret enforcement on all three endpoints, configuration ordering, revoked/missing/removed grants, numeric-prefix IDs, record_seen=0, timestamp coercions, stale disconnect floors, no-store, exact authorization_payload, and HTTP enqueue rollback. 39 Rails request-response cases; additional signed-token shapes are validated by the existing 98-case protocol corpus. Every protocol shape now also executes through HTTP; liveness and disconnect presence assertions are complete. SQL statement and transaction boundaries now have execution-level assertions.
- `tests/huddle_grant_test.rs`: eligibility/current relationships, random identities and reuse, never reviving revoked grants, requested-room coordinates, three real unique conflicts, concurrent issuance, 10-second sightings and strict 20-second first-sighting boundary, first-sighting jobs, leave floors, membership/session/ban/room revocation, role boundaries/server mute, last-host guard, and synchronous last-active-stage-grant stream state. Post-issuance invitations now have 49 exact Rails issuance scenarios; Stream callbacks now run; committed presence and leave/call-ended notices now have the coverage below.
- `huddle_gateway_node.mjs`: all 16 existing gateway tests run through real Rust endpoints. Ordinary responses are not fabricated; denial injections revoke SQLite rows. Only the original explicit outage/stall/malformed-response injections remain at the proxy. The test-only Rust launcher is ignored by default because it needs Node and the pinned ws package; it was explicitly run successfully in this slice.

The presence slice adds 50 exact pinned Rails participant renders (sidebar/header across five room kinds) and a real seeded app/queue/WebSocket proof of first-seen fan-out and silent rollback. The notice slice adds exact pinned Rails join/leave/call-ended payload sequences, real grant revoke/leave callbacks, registered join/push-invitation workers, retained unknown-handler replay, and a WS17 payload seam. Its push-scope corpus exercises 36 actual Rails invitation/join cases, subscription selection and the conditional throttle; the policy decisions are Rails inputs, so this does not port Notifications::Policy or prove transport delivery. Current raw pass counts and exact corpus sizes are in ws13-wave4-report.md.

The process task now runs invitations, stale streams and cleanups in Rails order with per-row commits and isolated failures. Stream create/end callbacks, explicit stopped events, last-host succession and quiet timeline delivery, role/mute grant revocation, and role/hand/moderation controllers are implemented. The stream/lifecycle corpus has 20 production Rails cases, moderation has 29 real requests, roles/hands has 34 real requests plus a real minute-bucket throttle probe, and Stage fragments have 400 byte-identical renders. Real socket tests prove committed changes, personal delivery, single rejoin behavior and silent rollback.

Controller and integration coverage is complete: all 226 original WS13 declarations have full assertions. This continuation closes the final 26 with actual HTTP/Cable effects, executed SQLite statement and transaction traces, and a deterministic membership-revocation race. Earlier partial counts in prior reports are historical. See the per-file table and retained titles below for the current counts.

The complete room header has 28 byte-identical renders; voice/Stage new/edit pages have 14. The new Markdown composer, member/thread panels and poll builder have 30 complete Rails renders across Closed, named/unnamed Direct, Voice and Stage rooms and Drive modes. The complete sidebar frame has 17 cold Rails renders over the parity seed, using the exact post-#163 sidebar source. It includes workspace identity/destinations/tools, shared/Board/Voice/Stage/DM rows, favourites, category controls, placeholders, room menus and the profile-card trigger. Request tests also exercise persisted placement and the administrator-only group-DM delete flag. The surrounding room page, pending-message shell, first-paint preloads, unread-divider/jump controls and per-viewer live OOO notices are integrated: 38 complete pages plus 38 complete component renders. Actual seeded/populated pages compare their room/header/shell adapters and native message renderer, including on-page and off-page unread boundaries. Both Designers captures are accepted with main’s GitHub cards and the native quote-child bridge. Shared/direct row refresh and callback adapters now use the same native composition as the request frame. Another 16 standalone sidebar rows and six recipient-specific group-DM headers match Rails byte for byte. Real socket/request tests cover rename delivery, recipient menu flags, JSON mute/read effects and live OOO updates/clears; Voice/Stage callback signatures are unchanged. The two Designers fixtures and approved post-#163 application-layout/status-action reconciliation are complete.

Runtime recent-search, Drive and sound metadata adapters are integrated with 23 recorded persisted-state cases and eight Picker configurations. WS13b owns model/job/service cases; WS17 owns policy/transport. Twenty-seven system cases now pass; 79 remain for end-to-end testing, including 35 LiveKit cases. No newly completed controller/integration title was inferred from render-vector counts.


## Rails declaration coverage by file

These are original declaration counts, not Rust test counts or individual vector counts. A declaration is closed only when all of its original assertions are covered. Each title remains below, including those now passed. Raw executable pass counts are in ws13-wave4-report.md.

| Rails file | Original | Assertions covered (passed) | Partial/deferred |
| --- | ---: | ---: | ---: |
| `test/controllers/rooms/stage/streams_controller_test.rb` | 38 | 38 | 0 |
| `test/models/huddle_invitation_test.rb` | 38 | 0 | 38 |
| `test/controllers/rooms/huddles_controller_test.rb` | 36 | 36 | 0 |
| `test/models/huddle/join_notifier_test.rb` | 33 | 0 | 33 |
| `test/models/huddle_grant_test.rb` | 33 | 0 | 33 |
| `test/system/huddles_test.rb` | 31 | 0 | 31 |
| `test/controllers/internal/huddle_controller_test.rb` | 29 | 29 | 0 |
| `test/models/rooms/stage_test.rb` | 27 | 12 | 15 |
| `test/models/stream_test.rb` | 27 | 14 | 13 |
| `test/controllers/rooms/stages_controller_test.rb` | 24 | 24 | 0 |
| `test/controllers/rooms/stage/roles_controller_test.rb` | 20 | 20 | 0 |
| `test/controllers/rooms/call_moderation_controller_test.rb` | 19 | 19 | 0 |
| `test/controllers/rooms/voices_controller_test.rb` | 18 | 18 | 0 |
| `test/controllers/rooms/stage_view_test.rb` | 16 | 16 | 0 |
| `test/system/huddle_join_notices_test.rb` | 16 | 0 | 16 |
| `test/controllers/rooms/stage/hands_controller_test.rb` | 15 | 15 | 0 |
| `test/system/stage_test.rb` | 15 | 11 | 4 |
| `test/models/huddle/join_pusher_test.rb` | 13 | 0 | 13 |
| `test/system/voice_channels_test.rb` | 12 | 0 | 12 |
| `test/system/huddle_invitations_test.rb` | 10 | 0 | 10 |
| `test/models/huddle/invitation_resolver_test.rb` | 9 | 9 | 0 |
| `test/models/huddle_revocation_test.rb` | 9 | 0 | 9 |
| `test/models/huddle/ring_policy_test.rb` | 8 | 0 | 8 |
| `test/system/huddle_audio_test.rb` | 8 | 8 | 0 |
| `test/system/huddle_roster_test.rb` | 8 | 8 | 0 |
| `test/controllers/users/huddle_presence_controller_test.rb` | 6 | 6 | 0 |
| `test/system/huddle_presence_test.rb` | 6 | 0 | 6 |
| `test/integration/huddle_presence_test.rb` | 5 | 5 | 0 |
| `test/models/rooms/voice_test.rb` | 5 | 2 | 3 |
| `test/jobs/huddle/join_notice_job_test.rb` | 4 | 0 | 4 |
| `test/jobs/huddle/push_invitation_job_test.rb` | 4 | 0 | 4 |
| `test/services/huddle/reconciler_test.rb` | 4 | 0 | 4 |
| `test/jobs/huddle/broadcast_presence_job_test.rb` | 2 | 0 | 2 |
| **Total** | **548** | **290** | **258** |

## test/controllers/internal/huddle_controller_test.rb

Owner: WS13. Complete: 29/29 original declarations passed through real HTTP, Cable and SQLite assertions.

- **Passed:** authorizes an exact active grant from an original token
- **Passed:** authorizes a server-refreshed token that omits false permissions
- **Passed:** rejects invalid signatures, missing time claims, and expired tokens
- **Passed:** rejects admin, data, metadata, and unknown privileges
- **Passed:** rejects any publish source set other than the exact camera grant
- **Passed:** authorizes a stage listener token that cannot publish
- **Passed:** authorizes a listener token in server-refreshed shape
- **Passed:** rejects mismatched publish permission and sources
- **Passed:** rejects a publisher token missing its publish sources
- **Passed:** authorizes a refreshed listener token that omits sources and the publish flag
- **Passed:** malformed signed video grants receive a controlled denial
- **Passed:** a valid token without an exact database grant is forbidden
- **Passed:** a stale database link revokes the grant and is forbidden
- **Passed:** grant lookup rechecks current access without requiring or rechecking the original token
- **Passed:** grant lookup returns not found after revocation
- **Passed:** a successful authorization records liveness without changing the response
- **Passed:** grant lookup records liveness at most once per ten seconds
- **Passed:** an enforcement-only grant lookup authorizes without recording liveness
- **Passed:** an enforcement-only lookup still revokes a stale grant
- **Passed:** a steady-state grant check runs no transaction and writes nothing
- **Passed:** a denied lookup takes the lock and revokes exactly once
- **Passed:** a denied lookup does not record liveness
- **Passed:** a disconnect event marks the grant out of the call and refreshes presence
- **Passed:** a stale disconnect event keeps a newer sighting
- **Passed:** a disconnect event with no timestamp clears liveness
- **Passed:** a disconnect event with a malformed timestamp is unprocessable
- **Passed:** a disconnect event for an unknown grant is not found
- **Passed:** gateway authentication is required for every endpoint
- **Passed:** internal endpoints fail closed when huddles are not fully configured

## test/controllers/rooms/call_moderation_controller_test.rb

Owner: WS13. Complete: 19/19 original declarations passed through real HTTP, Cable and SQLite assertions.

- **Passed:** a host server-mutes a speaker, revoking publish until unmuted
- **Passed:** muting delivers a roster to every member and a rejoin event to the target
- **Passed:** mute and unmute role events carry the server-muted state
- **Passed:** muting twice and unmuting a member who was never muted both succeed
- **Passed:** a repeated mute keeps the member's fresh grant and sends no rejoin
- **Passed:** unmuting a member who was never muted sends no rejoin
- **Passed:** a publish grant that survived a mute fails authorization
- **Passed:** disconnect drops the member from the call but keeps the membership
- **Passed:** an administrator who is not a host moderates a stage room
- **Passed:** a host cannot mute, unmute, or disconnect an administrator
- **Passed:** an administrator moderates another administrator
- **Passed:** a server-muted administrator unmutes themselves
- **Passed:** a server-muted host cannot unmute themselves
- **Passed:** speakers and listeners cannot moderate
- **Passed:** moderating your own session is rejected
- **Passed:** moderation is unreachable outside stage and voice rooms
- **Passed:** moderation denies outsiders, unknown memberships, and unauthenticated requests
- **Passed:** an administrator server-mutes a voice member, and members cannot
- **Passed:** server mute only exists on stage and voice rooms

## test/controllers/rooms/huddles_controller_test.rb

Owner: WS13. Complete: 36/36 original declarations passed through real HTTP, Cable and SQLite assertions.

- **Passed:** an authorized room member receives a narrowly scoped join token
- **Passed:** the active grant is reused while its random participant identity remains opaque
- **Passed:** direct rooms use their participant-based display name
- **Passed:** starting a one-to-one DM huddle invites only the other participant
- **Passed:** starting a channel huddle creates no invitation
- **Passed:** group direct rooms can start a huddle
- **Passed:** a nonmember cannot get a grant in a group DM and a removed member loses theirs
- **Passed:** GET confirms ongoing access without returning credentials
- **Passed:** GET denies access after room membership is revoked
- **Passed:** GET denies access to a soft-deleted room
- **Passed:** participants lists the room's in-call members without caching
- **Passed:** participants reflects only in-call grants
- **Passed:** a member removed mid-call is revoked and disappears from participants
- **Passed:** participants is reported for group direct rooms
- **Passed:** participants denies non-members, outsiders, and unauthenticated requests
- **Passed:** participants denies bots and inactive users
- **Passed:** participants requires LiveKit configuration
- **Passed:** GET denies access after sign out
- **Passed:** a nonmember cannot join a closed room
- **Passed:** a nonmember cannot mint voice credentials
- **Passed:** participants denies a member after removal
- **Passed:** an outsider cannot join a direct room
- **Passed:** an unauthenticated request receives JSON instead of a redirect
- **Passed:** bots cannot join
- **Passed:** bots cannot join through an ordinary session
- **Passed:** banned users cannot join
- **Passed:** missing LiveKit configuration is reported without minting a token
- **Passed:** a concurrent membership revocation receives a controlled denial
- **Passed:** a stage listener's token cannot publish anything
- **Passed:** stage speakers and hosts publish like any other participant
- **Passed:** voice and direct room tokens still publish
- **Passed:** a public URL pointing directly at the internal LiveKit address is rejected
- **Passed:** leaving drops the session's grants out of the call without revoking
- **Passed:** leaving works for voice rooms and group directs, like participants
- **Passed:** leaving twice, or without ever joining, still answers no content
- **Passed:** leaving denies non-members, bots, and unauthenticated requests

## test/controllers/rooms/stage/hands_controller_test.rb

Owner: WS13. Complete: 15/15 original declarations passed through real HTTP, Cable and SQLite assertions.

- **Passed:** a listener raises their hand and every viewer gets their own roster
- **Passed:** a double raise keeps the first timestamp and queue place
- **Passed:** raising hands is rate limited per membership
- **Passed:** the hand-raise rate limit resets after a minute
- **Passed:** a turbo-stream raise swaps the actor's own controls without navigating
- **Passed:** speakers and hosts cannot raise a hand
- **Passed:** a listener lowers their own hand
- **Passed:** lowering a hand that was never raised succeeds
- **Passed:** a host lowers another member's hand without promoting them
- **Passed:** an administrator member lowers another member's hand
- **Passed:** a listener cannot lower another member's hand
- **Passed:** lowering a non-member's hand is not found
- **Passed:** non-members get not found
- **Passed:** an administrator who is not a member gets not found
- **Passed:** hands do not exist outside stage rooms

## test/controllers/rooms/stage/roles_controller_test.rb

Owner: WS13. Complete: 20/20 original declarations passed through real HTTP, Cable and SQLite assertions.

- **Passed:** a host promotes a listener, clearing their hand and revoking their grants
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
- **Passed:** an administrator who is not a member gets not found
- **Passed:** an administrator member promotes a new host when the stage has none
- **Passed:** non-members get not found
- **Passed:** changing a non-member's role is not found
- **Passed:** an unknown role is unprocessable
- **Passed:** a missing role is unprocessable
- **Passed:** roles do not exist outside stage rooms

## test/controllers/rooms/stage/streams_controller_test.rb

Owner: WS13. Complete: 38/38 original declarations passed through real HTTP, Cable and SQLite assertions.

- **Passed:** a host goes live, broadcasting the badge, dot, and panels
- **Passed:** a speaker goes live
- **Passed:** a turbo-stream start swaps the actor's own panel without navigating
- **Passed:** a listener cannot go live
- **Passed:** a speaker cannot go live when the stage has no host
- **Passed:** a speaker goes live again after the last host leaves and a successor is promoted
- **Passed:** an administrator listener cannot go live
- **Passed:** a server-muted speaker cannot go live
- **Passed:** a server-muted host cannot go live
- **Passed:** a host without a huddle grant cannot go live
- **Passed:** a host whose grant was revoked cannot go live
- **Passed:** a host with a quiet grant cannot go live
- **Passed:** a host whose grant went quiet cannot go live
- **Passed:** an unknown quality is unprocessable
- **Passed:** a missing quality is unprocessable
- **Passed:** starting while another stream is live returns conflict naming the presenter
- **Passed:** the presenter stops the stream
- **Passed:** a speaker presenter stops their own stream
- **Passed:** a host stops another member's stream
- **Passed:** a host stop appends a stream-stopped event for the presenter
- **Passed:** a presenter stop appends no stream-stopped event
- **Passed:** an administrator member stops the stream without being a host
- **Passed:** a listener cannot stop the stream
- **Passed:** a speaker who is not the presenter cannot stop the stream
- **Passed:** stopping with the live stream id ends that stream
- **Passed:** stopping with a stale stream id ends nothing, even when another stream is live
- **Passed:** stopping with an unknown stream id ends nothing
- **Passed:** the stop control sends its stream id
- **Passed:** stopping with nothing live succeeds for hosts and stays silent
- **Passed:** stopping with nothing live is forbidden for listeners
- **Passed:** a turbo-stream stop swaps the actor's own panel without navigating
- **Passed:** non-members get not found
- **Passed:** an administrator who is not a member gets not found
- **Passed:** streams do not exist outside stage rooms
- **Passed:** demoting the presenter to listener ends the stream in the same transaction
- **Passed:** removing the presenter through the members edit ends the stream
- **Passed:** promoting a speaker to host keeps the grant and the live stream
- **Passed:** demoting a host to speaker keeps the grant and the live stream

## test/controllers/rooms/stage_view_test.rb

Owner: WS13. Complete: 16/16 original declarations passed through real HTTP, Cable and SQLite assertions.

- **Passed:** a listener's join control hints that publishing is unavailable
- **Passed:** a host's join control hints that publishing is available
- **Passed:** a speaker's join control hints that publishing is available
- **Passed:** a voice channel's join control carries no publishing hint
- **Passed:** the layout renders the persistent role-event target for signed-in users
- **Passed:** a listener's stage panel renders no role forms
- **Passed:** a host's stage panel renders role forms
- **Passed:** a host sees no moderation controls on an administrator's row
- **Passed:** a muted administrator sees an unmute control on their own row
- **Passed:** an administrator sees moderation controls on every other row
- **Passed:** the header Live badge renders only while live
- **Passed:** the sidebar live dot renders only while live
- **Passed:** the Go live form renders for hosts and speakers
- **Passed:** the Go live form renders for no listener and never while live
- **Passed:** Stop stream renders for the presenter but not for listeners
- **Passed:** Stop stream renders for hosts and the presenting speaker

## test/controllers/rooms/stages_controller_test.rb

Owner: WS13. Complete: 24/24 original declarations passed through real HTTP, Cable and SQLite assertions.

- **Passed:** show redirects to get general show
- **Passed:** new
- **Passed:** create makes the creator host and the rest listeners
- **Passed:** create prepends the stage row into the stage section
- **Passed:** create forbidden by non-admin when account restricts creation to admins
- **Passed:** update with membership revisions makes new members listeners
- **Passed:** removing a member tells them to drop the sidebar row and header stack
- **Passed:** a non-administrator creator can manage members of their own stage room
- **Passed:** only admins or creators can update
- **Passed:** the sole host cannot remove themselves while others remain
- **Passed:** create with an unknown icon re-renders the new form
- **Passed:** update with an unknown icon re-renders the edit form without revising members
- **Passed:** update with an icon normalizes the shortcode
- **Passed:** update clears the icon with a blank shortcode
- **Passed:** updating the icon replaces sidebar rows and headers for members only
- **Passed:** a host removes themselves once another host exists
- **Passed:** the sole-host check and revision run in one locked transaction
- **Passed:** removing everyone including the last host empties the room
- **Passed:** the room page and the members edit render with zero hosts
- **Passed:** non-members cannot see the room page or its messages
- **Passed:** non-members cannot reach the stage namespace
- **Passed:** open, closed, and voice rooms cannot be converted to stage
- **Passed:** stage rooms cannot be converted through the open, closed, or voice namespaces
- **Passed:** a direct room can't be converted to stage and have its participants revised

## test/controllers/rooms/voices_controller_test.rb

Owner: WS13. Complete: 18/18 original declarations passed through real HTTP, Cable and SQLite assertions.

- **Passed:** show redirects to get general show
- **Passed:** new
- **Passed:** create
- **Passed:** create prepends the voice row into the voice section
- **Passed:** create forbidden by non-admin when account restricts creation to admins
- **Passed:** update with an icon normalizes the shortcode
- **Passed:** create with an unknown icon re-renders the new form
- **Passed:** update with an unknown icon re-renders the edit form
- **Passed:** update with membership revisions
- **Passed:** removing a member tells them to drop the sidebar row and header stack
- **Passed:** a non-administrator creator can manage members of their own voice room
- **Passed:** only admins or creators can update
- **Passed:** remove yourself
- **Passed:** non-members cannot see the room page or its messages
- **Passed:** non-members cannot reach the voice namespace
- **Passed:** open and closed rooms cannot be converted to voice
- **Passed:** voice rooms cannot be converted through the open or closed namespaces
- **Passed:** a direct room can't be converted to voice and have its participants revised

## test/controllers/users/huddle_presence_controller_test.rb

Owner: WS13. Complete: 6/6 original declarations passed through real HTTP, Cable and SQLite assertions.

- **Passed:** returns only the current user's rooms with at least one participant
- **Passed:** the response runs one grants query no matter how many rooms are live
- **Passed:** returns an empty list when nobody is in any call
- **Passed:** an unauthenticated request receives JSON instead of a redirect
- **Passed:** bots and inactive users are denied like the huddle controller
- **Passed:** missing LiveKit configuration is reported

## test/integration/huddle_presence_test.rb

Owner: WS13. Complete: 5/5 original declarations passed through real HTTP, Cable and SQLite assertions.

- **Passed:** channel header shows the live stack immediately before the join button
- **Passed:** quiet channel header keeps an empty stack target
- **Passed:** two-person DM header shows the live stack
- **Passed:** group DM header shows the live stack immediately before the join button
- **Passed:** no header stack without huddle configuration

## test/jobs/huddle/broadcast_presence_job_test.rb

Owner: WS13b. Deferred.

- broadcasts the room's current stacks
- missing grants and rooms stay silent

## test/jobs/huddle/join_notice_job_test.rb

Owner: WS13b. Deferred.

- a first sighting enqueues the join notice alongside the presence broadcast
- a repeat sighting enqueues no join notice
- performing the job notifies the room's members
- a missing grant is ignored

## test/jobs/huddle/push_invitation_job_test.rb

Owner: WS13b with WS17 for transport/policy integration. Deferred.

- pushes the invitation to the recipient only
- an opted-out recipient gets no push subscriptions
- a connected recipient gets no push
- missing invitations are ignored

## test/models/huddle/invitation_resolver_test.rb

Owner: WS13b. Passed: all nine declarations, mapped to the 29-case Rails resolver corpus and `overdue_invitations_match_twenty_nine_rails_scenarios_and_are_idempotent`. Complete row snapshots cover state, unread/read/handled stamps and item counts; group and per-user cases cover both recipients; a second pass proves idempotence.

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

Owner: WS13b. Deferred.

- an in-call DM member is told when the peer joins, and the joiner is not
- a member with no access to the room is told nothing
- an out-of-call DM member gets the banner broadcast and one push
- a group DM join toasts the insider and banners the outsider
- join fan-out loads members and rings once no matter the group size
- a channel join toasts the insider and tells the outsider nothing
- an out-of-call channel member gets no banner and no push
- a voice room join toasts the insider and tells the outsider nothing
- bots and deactivated members are told nothing
- a bot join notifies nobody
- a second device sighted while the first is listed enqueues no join notice
- sightings from two devices before the job runs still notify once
- a join job running after the joiner left notifies nobody
- a viewer whose ring is still live gets no join notice
- a viewer whose ring went stale gets the join notice again
- a switched-off or hidden room stays silent for the outsider
- an outsider with huddle invitations switched off still banners but gets no push
- a muted room still banners the outsider but sends no push
- a join after a recent in-call revoke is marked as a rejoin
- a rejoin after the disconnect report cleared liveness is not marked
- a join after a quiet revoke is not marked as a rejoin
- a join after the rejoin window is not marked as a rejoin
- a join ten seconds after the revoke is not marked as a rejoin
- leaving toasts the members still in the call
- revoking an in-call grant toasts the members still in the call
- leaving tells out-of-call DM members so their banner drops the leaver
- leaving a channel tells out-of-call members nothing
- the last one out of a DM dismisses every other member's banner
- the last one out of a channel dismisses nothing
- leaving a call the grant was never in toasts nobody
- revoking a quiet grant toasts nobody
- leaving while another of the leaver's grants is still in toasts nobody
- a leave report after a revocation does not toast twice

## test/models/huddle/join_pusher_test.rb

Owner: WS13b with WS17 for transport/policy integration. Deferred.

- pushes the join to the recipient's subscriptions and stamps the throttle
- a second push inside ten minutes is throttled
- a push ten minutes later goes out again
- a DND recipient gets no push and burns no throttle window
- a starred joiner still pushes through DND
- a recipient in quiet hours gets no push
- a recipient quiet in a meeting gets no push
- an out-of-office recipient gets no push unless they keep notifications on
- a connected recipient gets no push and burns no throttle window
- a switched-off or hidden room gets no push
- a muted room gets no push and burns no throttle window
- a recipient with huddle invitations switched off gets no push and burns no throttle window
- a recipient with no subscriptions burns no throttle window

## test/models/huddle/ring_policy_test.rb

Owner: WS13b. Deferred.

- an invitation rings a member who is not in do-not-disturb
- do-not-disturb silences the ring
- a caller allowed during do-not-disturb still rings
- a quiet check override replaces the policy
- quiet-during-meetings silences the ring during a busy interval
- a caller allowed during do-not-disturb still rings through a meeting
- out of office silences the ring unless the member keeps notifications on
- a caller allowed during do-not-disturb still rings through out of office

## test/models/huddle_grant_test.rb

Owner: WS13b. Deferred.

- an active session and membership reuse one random grant
- revoking and restoring room membership never resurrects the old grant
- issuance rejects a stale or cross-user membership
- issuance stops after three uniqueness conflicts
- issuance stamps last_issued_at on create and on reuse
- joining another room ends the session's in-call grant there but keeps quiet ones
- rejoining the same room keeps the session's grant there
- in_call reflects gateway liveness within twenty seconds
- record_seen! persists liveness at most once per ten seconds
- mark_out_of_call! drops liveness without revoking and refreshes presence
- mark_out_of_call! is silent when the grant was never seen
- mark_out_of_call! keeps a sighting newer than the disconnect
- participants_for lists distinct in-call users by name
- participants_for drops revoked and quiet grants
- issuing a voice grant refreshes the presence stacks
- revoking a voice grant refreshes the presence stacks
- first sighting in the call enqueues a presence refresh, later sightings stay silent
- issuing an open channel grant refreshes every sidebar and the header
- issuing a closed channel grant refreshes every sidebar and the header
- issuing a direct grant refreshes both sidebars and the header
- revoking a channel grant refreshes every sidebar and the header
- revoking a direct grant refreshes both sidebars and the header
- first sighting in a channel enqueues a presence refresh, later sightings stay silent
- no presence broadcasts without huddle configuration
- revoking a destroyed room's grants stays silent
- a stage grant records the membership role it was issued for
- non-stage grants record no role
- a stage role change revokes the member's active grants with cleanup
- a host-speaker change updates the grant's role in place without revoking
- authorize_or_revoke! revokes a grant whose issued role no longer matches
- rejoining after a role change issues a new grant for the new role
- issuing a stage grant refreshes the presence stacks
- revoking a stage grant refreshes the presence stacks

## test/models/huddle_invitation_test.rb

Owner: WS13b. Deferred.

- issuing a grant in a one-to-one DM invites only the other participant
- issuing a grant never schedules a delayed job
- a quiet check silences the invitation payload but keeps the item
- a recipient with notifications off or invisible gets no invitation
- a recipient with huddle items switched off still gets the banner but no item
- a switched-off user hears one banner across reissues inside the window and a fresh one after
- a switched-off user is not rung again when the same session reissues its grant
- channel huddles create no invitation
- voice channel huddles create no invitation
- no invitation while the other participant is in the call
- an invitation fires when the other participant's grant went quiet
- reusing the same grant rings again once the dedup window has passed
- a second grant for the same starter does not ring again inside two minutes
- no ring inside the two-minute window after a missed invitation
- a handled invitation still suppresses the next ring inside two minutes
- an invitation older than two minutes re-rings through the same row
- a handled invitation older than two minutes re-rings through the same row
- a missed invitation older than two minutes re-rings through the same row
- revoking the starter's in-call grant broadcasts call-ended to the invitee
- the starter leaving the call broadcasts call-ended to the invitee
- revoking a quiet grant broadcasts no call-ended
- no call-ended when the recipient already joined
- a suppressed ring ends with a banner-only call-ended
- no banner-only call-ended when the ring long stopped
- the starter leaving a group call while others remain sends no call-ended
- revoking the starter's grant while others remain in a group call sends no call-ended
- call-ended fires when the last participant leaves a group call
- a group ring continues for remaining invitees until their ring timeout
- retrying after the window with a new grant reuses the unhandled item
- a retry after a handled attempt opens a new item
- a retry from the first device re-rings through its own item
- the same attempt reuses its item inside ten minutes and opens a new one after
- obtaining a grant clears the recipient's open invitations for the room
- joining late clears the missed item
- a DM with only bots besides the starter gets no invitation
- issuing a grant in a group DM invites every other human member
- group DM invitations skip bots and members who switched the room off
- a removed group member gets no invitation and loses their grant

## test/models/huddle_revocation_test.rb

Owner: WS13b. Deferred.

- membership revocation persists cleanup for only that grant
- session removal revokes its grants in every room and leaves another session active
- banning a user revokes grants even though sessions are bulk deleted first
- deactivating a user revokes grants after memberships and sessions are bulk deleted
- destroying a room revokes grants and persists one room deletion
- removing a voice member mid-call revokes only their grant
- destroying a voice room revokes its grants and persists one room deletion
- deactivating a user ends their voice session
- revocation remains durable while LiveKit is unavailable

## test/models/rooms/stage_test.rb

Owner: WS13b. Partial: 12/27 declarations passed against the hand, lifecycle and participation corpora plus existing WS8a room tests; the other 15 remain open.

- **Passed:** type predicate
- stage rooms are listed without directs but outside the voice scope
- **Passed:** default involvement for new members is mentions
- **Passed:** the room creator becomes host and every other member becomes a listener
- **Passed:** the creator becomes host even when they were not in the member list
- members added later become listeners
- **Passed:** the last host cannot be demoted
- a host demotion checks for another host after locking the room in its transaction
- **Passed:** a host can step down once another host exists
- **Passed:** non-stage rooms leave the stage columns nil
- **Passed:** only listeners can raise a hand, and any promotion clears it
- **Passed:** raising twice keeps the first timestamp
- **Passed:** lowering a hand that was never raised succeeds
- stage members can reach the room's messages like any channel
- **Passed:** deactivating a user removes their stage memberships
- deactivating the sole host ends the live session and promotes an administrator member
- deactivating the sole host promotes the earliest remaining member without an administrator
- deactivating a host promotes nobody when another host remains
- deactivating the last member of a stage leaves the emptied room alone
- destroying the last host membership ends the live session and promotes an administrator successor
- **Passed:** destroying the last host promotes the earliest remaining member without an administrator
- destroying a host while another host remains ends nothing and promotes nobody
- destroying a speaker ends only their own stream and grants
- destroying the last host locks the room and checks for another host inside its transaction
- live_stream reads the preloaded live stream without querying
- live_stream is nil from the preloaded association once the stream has ended
- live_stream queries fresh when streams are not preloaded

## test/models/rooms/voice_test.rb

Owner: WS13b. Partial: 2/5 declarations passed; the other 3 remain WS13b work.

- **Passed:** type predicate
- voices scope and channel queries include voice rooms
- **Passed:** default involvement for new members is mentions
- voice members can reach the room's messages like any channel
- deactivating a user removes their voice memberships

## test/models/stream_test.rb

Owner: WS13b. Partial: 14/27 declarations passed; the other 13 remain WS13b work.

- **Passed:** quality must be a known preset
- **Passed:** started_at defaults to now
- live scope only returns unended streams
- **Passed:** one live stream per room
- **Passed:** an ended stream frees the room for another
- **Passed:** end! is idempotent
- starting broadcasts the badge, dot, and per-viewer panel
- **Passed:** starting broadcasts the event venue dot
- **Passed:** ending broadcasts the cleared event venue dot
- ending broadcasts the cleared badge, dot, and panel
- ending twice broadcasts once
- a host stop appends a stream-stopped event to the presenter's persistent target
- a presenter stop appends no stream-stopped event
- an automatic end appends no stream-stopped event
- **Passed:** revoking the presenter's last grant for the room ends the stream
- revoking another member's grant leaves the stream live
- a grant revoked through authorization ends the stream
- removing the presenter's membership ends the stream
- **Passed:** removing the presenter's membership without grants ends the stream and broadcasts the end
- deactivating the presenter ends the stream
- **Passed:** deactivating the presenter ends the stream even without grants
- destroying the room destroys its streams
- revoking a grant outside a stage room runs no stream queries
- **Passed:** end_stale_live! ends streams whose presenter went quiet over thirty seconds ago
- **Passed:** end_stale_live! ends streams whose presenter was never seen
- **Passed:** end_stale_live! keeps streams with a recently seen presenter
- **Passed:** end_stale_live! ignores other memberships' grants in the room

## test/services/huddle/reconciler_test.rb

Owner: WS13b. Deferred.

- one pass resolves overdue invitations, ends stale streams, and reconciles cleanup
- a resolver failure is logged and does not stop cleanup reconciliation
- a stale-stream failure is logged and does not stop cleanup reconciliation
- one pass ends a quiet presenter's live stream

## test/system/huddle_audio_test.rb

Owner: WS13. Complete: 8/8 original declarations passed against the actual seeded Designers room, production JavaScript and the original recorded SDK/audio-track stub. The assertions retain capture constraints, device preference, room defaults, processor attachment, restarts and muted-track behavior.

- **Passed:** the capture asks for no browser suppression while RNNoise is on
- **Passed:** the capture keeps browser suppression while RNNoise is off
- **Passed:** unmuting requests the selected device without rewriting the room defaults
- **Passed:** muting keeps the noise processor attached across mute and unmute
- **Passed:** toggling noise suppression off re-acquires the microphone with browser suppression
- **Passed:** toggling noise suppression on re-acquires the microphone without browser suppression
- **Passed:** toggling noise suppression off without a stored device re-acquires on the live device
- **Passed:** toggling noise suppression off while muted refreshes the stored constraints

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

Owner: WS13. Complete: 8/8 original declarations passed against real Rust pages, signed sessions, assets and Cable. The SDK/analyser fixture stays at the original Rails boundary; all roster identity, button-state, meter timing and leave-order assertions are retained.

- **Passed:** muting patches the local roster row instead of rebuilding it
- **Passed:** speaking and mute changes patch the remote row in place
- **Passed:** joining and leaving adds and removes roster rows only
- **Passed:** the mute toggle keeps a stable label with pressed state and tooltip
- **Passed:** the camera toggle keeps a stable label with pressed state and tooltip
- **Passed:** the meter stops once its track ends
- **Passed:** the meter skips ticks while the tab is hidden
- **Passed:** leaving reports after the disconnect completes

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

Owner: WS13. Complete browser coverage: 11/15 original declarations passed with their full assertions. Four declarations remain open because they require a real LiveKit server: subscribe-only listener/host publishing, role-change reconnect publishing, host server-mute/unmute reconnect, and full listener reconnect.

- **Passed:** stage rooms list in their own section with distinct creation controls and a stage panel
- **Passed:** a listener raises and lowers their hand without seeing host controls
- **Passed:** raised hands appear in the host's panel live, in order
- **Passed:** a host invites a listener to speak and moves them back to the audience
- **Passed:** a host lowers a raised hand without promoting
- **Passed:** the last host cannot demote themselves from the panel
- **Passed:** join stage dispatches huddle:join and toggles while connected
- **Passed:** a listener joins without a microphone or device check
- **Passed:** a demoted speaker retries as a listener without entering prejoin
- **Passed:** stage rooms carry ordinary text chat
- a listener joins subscribe-only while the host publishes
- inviting a listener to speak rejoins them publishing, and moving them back removes publish
- a host server-mutes a speaker and they rejoin muted, then unmutes them
- a listener survives a full reconnect and stays subscribe-only
- **Passed:** a muted speaker is told and rejoins without microphone prejoin

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

## Current continuation evidence

`stream_controller_tests.rs` closes the 22 previously open stream declarations with exact HTTP bodies, global stream-count guards, outsider/admin/type isolation, presenter and stale-id behavior, member-edit teardown and last-host succession. Its real Cable test checks one room badge and three per-user replacements, explicit presenter-stop silence, a moderator's single stopped event, and a barrier proving silent no-op stop without sleeping or changing timing thresholds. `stage_page_tests.rs` checks the 15 page declarations through the actual generic room endpoint. Domain/model/job/service files remain WS13b-owned.

The 35 real LiveKit declarations remain individually inventoried above: 31 in `test/system/huddles_test.rb`, four in `test/system/stage_test.rb`. Their browser WebRTC/media/reconnect assertions cannot be expressed by recorded responses from the injected administrative LiveKit client. Recorded client/Twirp and gateway contracts remain covered; 52 ordinary browser declarations remain open, rather than being reclassified as LiveKit-dependent; 19 ordinary declarations are complete.

## Remaining controller and integration closure

The nine tests in `remaining_call_tests.rs`, `remaining_query_tests.rs` and `remaining_presence_tests.rs` close the last 26 original declarations. Query probes trace executed cached statements, retaining the configured reader count. Their isolated app shuts down its independent background workers before measuring controller SQL; the test suite still runs four tests concurrently. A barrier holds the room read after membership scope while a separate write commits membership revocation, then verifies the issue path returns the controlled JSON denial without a grant. Stage edits prove the host read and membership insert sit between one immediate begin and commit. Steady-state admission runs no transaction or writes and at most eight statements; denial performs exactly one grant update and cleanup insert. Presence batches one grants query and one user preload across three live rooms. Real Cable frames prove exact hand/promotion/mute fan-out, personalization, cleanup and unchanged host grants, with a clock-controlled double raise preserving queue order. All five integration declarations inspect actual GET HTML for channel, direct and group stacks, quiet targets, adjacency and disabled configuration.

All 226 controller/integration declarations are complete. WS13b retains the 216 domain declarations and unchanged historic 37 passed / 179 open. Nineteen of 106 system declarations now pass. The remaining 52 ordinary browser cases and 35 real LiveKit cases remain individually inventoried, as requested.

## Room composition continuation

`room_composition_tests.rs` compares 30 complete Rails fragment renders for the Markdown composer (without Drive, legacy Drive and the configured Picker), member panel, thread panel and poll builder across closed, named/unnamed direct, voice and Stage rooms. These native templates include reply context, autocomplete/slash-command wiring, the schedule-send dialog, shared multi-select controls and tracked-work thread controls. Generic room GETs now integrate them, with the shared thread-panel label loaded separately from the viewer's DM composer label. This fragment slice was followed by the complete 38-page acceptance set below. The parent sidebar is covered in the following continuation. No system declaration is reclassified as passed.

## Complete sidebar composition continuation

The parent sidebar is now native Rust markup with plain request inputs. The Rails recorder captures the full turbo frame, not a selected region, and does not normalize HTML. Each configuration scenario clears its isolated Rails cache before rendering, so enabled startup configurations do not reuse disabled-process fragments. DM avatar association order and collection whitespace match Rails; the label is independently sorted. Existing WS13b/WS17 signatures are unchanged. The exact #163 parent template is tracked in a private oracle build context, making regeneration possible from a fresh clone without merging main or using untracked files.

## Designers, layout and Stage browser continuation

Both previously retained Designers renders are now in the 38-page acceptance set. The parent quote renderer keeps cross-room sources in viewer-authorized lazy frames and renders same-room native cards; complete card partials are recorded independently. The exact approved #163 application-layout source is part of the private oracle context and its SHA256 is checked against `2e20b24c`.

`parity/system/ws13` explicitly runs eleven complete original Stage declarations in the pinned Playwright image against the production Rust router/assets/Cable over signed sessions. The private fixture/state routes exist only in the ignored external-test launcher, never in the shipped binary. Microphone and credential failure injections remain at the same boundary as the original declarations. The synthetic join/leave declaration holds external SDK signaling pending so the isolated capture proxy cannot misroute its TLS connection to Rust HTTP; its original event, label, no-second-join and idle assertions remain intact. The original selector and explicit broadcast/connection waits are unchanged. Selenium's empty inline-stack visibility is checked through CSS display/visibility because Playwright's nonzero-box predicate disagrees for Rails' whitespace-only target. No screenshots or image comparisons are used.

The previous continuation paragraphs are historical: Designers and application-layout reconciliation are closed, and Stage ordinary cases have advanced from 0/11 to 11/11. Eight audio-processing and eight roster declarations also pass in the same browser acceptance run. The current table and retained title markers are authoritative; 79/106 system declarations remain, including all 35 real LiveKit declarations.

## Audio browser continuation

The eight audio declarations use the real Designers seed room (the fixture asserts its ID) and signed Rust sessions, with the actual Rails SDK/track stub transcribed verbatim apart from named JavaScript arguments. No product JavaScript, LiveKit policy, transport seam or original assertion was changed. The ordinary browser total is now 19 passed, 52 remaining; the real LiveKit total remains 0 passed, 35 remaining. Browser acceptance runs explicitly rather than being silently skipped as part of the ordinary workspace suite.

The roster SDK/analyser fixture is copied from the pinned Rails helper. Browser contexts now close their individual forwarding proxies with the context: cancelled page requests cannot carry into later declarations. The shared fixture explicitly waits for its Stimulus huddle controller within the existing two-second selector budget before installing the original SDK fixture. These are harness lifecycle fixes; no original assertion, concurrency or explicit wait is changed.
