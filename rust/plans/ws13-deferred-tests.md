# WS13 deferred Rails test declarations

Status: partial. 548 test declarations in 33 files are listed below. The three HuddleNoticeChannel tests already ported by WS7 are verified separately in the report. These test declarations remain WS13 work. Protocol-level vector coverage of some internal-controller assertions does not count as porting their HTTP tests. WS17 owns push transport and Notifications::Policy; WS13 retains the invitation/join payload and enqueue tests. System tests below are deferred until LIVEKIT_SYSTEM_TESTS=1 with a real LiveKit server.

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

Owner: WS13. Deferred.

- a host server-mutes a speaker, revoking publish until unmuted
- muting delivers a roster to every member and a rejoin event to the target
- mute and unmute role events carry the server-muted state
- muting twice and unmuting a member who was never muted both succeed
- a repeated mute keeps the member's fresh grant and sends no rejoin
- unmuting a member who was never muted sends no rejoin
- a publish grant that survived a mute fails authorization
- disconnect drops the member from the call but keeps the membership
- an administrator who is not a host moderates a stage room
- a host cannot mute, unmute, or disconnect an administrator
- an administrator moderates another administrator
- a server-muted administrator unmutes themselves
- a server-muted host cannot unmute themselves
- speakers and listeners cannot moderate
- moderating your own session is rejected
- moderation is unreachable outside stage and voice rooms
- moderation denies outsiders, unknown memberships, and unauthenticated requests
- an administrator server-mutes a voice member, and members cannot
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

Owner: WS13. Deferred.

- a listener raises their hand and every viewer gets their own roster
- a double raise keeps the first timestamp and queue place
- raising hands is rate limited per membership
- the hand-raise rate limit resets after a minute
- a turbo-stream raise swaps the actor's own controls without navigating
- speakers and hosts cannot raise a hand
- a listener lowers their own hand
- lowering a hand that was never raised succeeds
- a host lowers another member's hand without promoting them
- an administrator member lowers another member's hand
- a listener cannot lower another member's hand
- lowering a non-member's hand is not found
- non-members get not found
- an administrator who is not a member gets not found
- hands do not exist outside stage rooms

## test/controllers/rooms/stage/roles_controller_test.rb

Owner: WS13. Deferred.

- a host promotes a listener, clearing their hand and revoking their grants
- the affected member's panel replacement carries no rejoin trigger
- a publish-boundary crossing appends a rejoin event to the member's persistent target
- a host-speaker change broadcasts roster and panel but no rejoin event and revokes nothing
- a demotion appends a rejoin event to the member's persistent target
- a turbo-stream role change replaces the roster without navigating
- a host demotes a speaker back to the audience
- a host demoting themselves is allowed unless they are the last host
- a failed last-host demotion revokes nothing
- a host who is not an administrator cannot demote an administrator
- a listener cannot change anyone's role
- a speaker cannot change anyone's role
- an administrator member manages roles without being a host
- an administrator who is not a member gets not found
- an administrator member promotes a new host when the stage has none
- non-members get not found
- changing a non-member's role is not found
- an unknown role is unprocessable
- a missing role is unprocessable
- roles do not exist outside stage rooms

## test/controllers/rooms/stage/streams_controller_test.rb

Owner: WS13. Deferred.

- a host goes live, broadcasting the badge, dot, and panels
- a speaker goes live
- a turbo-stream start swaps the actor's own panel without navigating
- a listener cannot go live
- a speaker cannot go live when the stage has no host
- a speaker goes live again after the last host leaves and a successor is promoted
- an administrator listener cannot go live
- a server-muted speaker cannot go live
- a server-muted host cannot go live
- a host without a huddle grant cannot go live
- a host whose grant was revoked cannot go live
- a host with a quiet grant cannot go live
- a host whose grant went quiet cannot go live
- an unknown quality is unprocessable
- a missing quality is unprocessable
- starting while another stream is live returns conflict naming the presenter
- the presenter stops the stream
- a speaker presenter stops their own stream
- a host stops another member's stream
- a host stop appends a stream-stopped event for the presenter
- a presenter stop appends no stream-stopped event
- an administrator member stops the stream without being a host
- a listener cannot stop the stream
- a speaker who is not the presenter cannot stop the stream
- stopping with the live stream id ends that stream
- stopping with a stale stream id ends nothing, even when another stream is live
- stopping with an unknown stream id ends nothing
- the stop control sends its stream id
- stopping with nothing live succeeds for hosts and stays silent
- stopping with nothing live is forbidden for listeners
- a turbo-stream stop swaps the actor's own panel without navigating
- non-members get not found
- an administrator who is not a member gets not found
- streams do not exist outside stage rooms
- demoting the presenter to listener ends the stream in the same transaction
- removing the presenter through the members edit ends the stream
- promoting a speaker to host keeps the grant and the live stream
- demoting a host to speaker keeps the grant and the live stream

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

Owner: WS13. Deferred.

- broadcasts the room's current stacks
- missing grants and rooms stay silent

## test/jobs/huddle/join_notice_job_test.rb

Owner: WS13. Deferred.

- a first sighting enqueues the join notice alongside the presence broadcast
- a repeat sighting enqueues no join notice
- performing the job notifies the room's members
- a missing grant is ignored

## test/jobs/huddle/push_invitation_job_test.rb

Owner: WS13 with WS17 for transport/policy integration. Deferred.

- pushes the invitation to the recipient only
- an opted-out recipient gets no push subscriptions
- a connected recipient gets no push
- missing invitations are ignored

## test/models/huddle/invitation_resolver_test.rb

Owner: WS13. Deferred.

- an unanswered invitation becomes a missed call and stays unread
- unanswered group invitations each become missed calls
- a recipient who was issued a grant since the start has their invitation handled
- a recipient seen in the call has their invitation handled
- the starter leaving before the wait elapses is a missed call
- an invitation within the wait is left alone
- an already-handled invitation is left alone
- resolving twice keeps a single missed item
- resolution can be scoped to one user

## test/models/huddle/join_notifier_test.rb

Owner: WS13. Deferred.

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

Owner: WS13 with WS17 for transport/policy integration. Deferred.

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

Owner: WS13. Deferred.

- an invitation rings a member who is not in do-not-disturb
- do-not-disturb silences the ring
- a caller allowed during do-not-disturb still rings
- a quiet check override replaces the policy
- quiet-during-meetings silences the ring during a busy interval
- a caller allowed during do-not-disturb still rings through a meeting
- out of office silences the ring unless the member keeps notifications on
- a caller allowed during do-not-disturb still rings through out of office

## test/models/huddle_grant_test.rb

Owner: WS13. Deferred.

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

Owner: WS13. Deferred.

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

Owner: WS13. Deferred.

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

Owner: WS13. Deferred.

- type predicate
- stage rooms are listed without directs but outside the voice scope
- default involvement for new members is mentions
- the room creator becomes host and every other member becomes a listener
- the creator becomes host even when they were not in the member list
- members added later become listeners
- the last host cannot be demoted
- a host demotion checks for another host after locking the room in its transaction
- a host can step down once another host exists
- non-stage rooms leave the stage columns nil
- only listeners can raise a hand, and any promotion clears it
- raising twice keeps the first timestamp
- lowering a hand that was never raised succeeds
- stage members can reach the room's messages like any channel
- deactivating a user removes their stage memberships
- deactivating the sole host ends the live session and promotes an administrator member
- deactivating the sole host promotes the earliest remaining member without an administrator
- deactivating a host promotes nobody when another host remains
- deactivating the last member of a stage leaves the emptied room alone
- destroying the last host membership ends the live session and promotes an administrator successor
- destroying the last host promotes the earliest remaining member without an administrator
- destroying a host while another host remains ends nothing and promotes nobody
- destroying a speaker ends only their own stream and grants
- destroying the last host locks the room and checks for another host inside its transaction
- live_stream reads the preloaded live stream without querying
- live_stream is nil from the preloaded association once the stream has ended
- live_stream queries fresh when streams are not preloaded

## test/models/rooms/voice_test.rb

Owner: WS13. Deferred.

- type predicate
- voices scope and channel queries include voice rooms
- default involvement for new members is mentions
- voice members can reach the room's messages like any channel
- deactivating a user removes their voice memberships

## test/models/stream_test.rb

Owner: WS13. Deferred.

- quality must be a known preset
- started_at defaults to now
- live scope only returns unended streams
- one live stream per room
- an ended stream frees the room for another
- end! is idempotent
- starting broadcasts the badge, dot, and per-viewer panel
- starting broadcasts the event venue dot
- ending broadcasts the cleared event venue dot
- ending broadcasts the cleared badge, dot, and panel
- ending twice broadcasts once
- a host stop appends a stream-stopped event to the presenter's persistent target
- a presenter stop appends no stream-stopped event
- an automatic end appends no stream-stopped event
- revoking the presenter's last grant for the room ends the stream
- revoking another member's grant leaves the stream live
- a grant revoked through authorization ends the stream
- removing the presenter's membership ends the stream
- removing the presenter's membership without grants ends the stream and broadcasts the end
- deactivating the presenter ends the stream
- deactivating the presenter ends the stream even without grants
- destroying the room destroys its streams
- revoking a grant outside a stage room runs no stream queries
- end_stale_live! ends streams whose presenter went quiet over thirty seconds ago
- end_stale_live! ends streams whose presenter was never seen
- end_stale_live! keeps streams with a recently seen presenter
- end_stale_live! ignores other memberships' grants in the room

## test/services/huddle/reconciler_test.rb

Owner: WS13. Deferred.

- one pass resolves overdue invitations, ends stale streams, and reconciles cleanup
- a resolver failure is logged and does not stop cleanup reconciliation
- a stale-stream failure is logged and does not stop cleanup reconciliation
- one pass ends a quiet presenter's live stream

## test/system/huddle_audio_test.rb

Owner: WS13. Deferred.

- the capture asks for no browser suppression while RNNoise is on
- the capture keeps browser suppression while RNNoise is off
- unmuting requests the selected device without rewriting the room defaults
- muting keeps the noise processor attached across mute and unmute
- toggling noise suppression off re-acquires the microphone with browser suppression
- toggling noise suppression on re-acquires the microphone without browser suppression
- toggling noise suppression off without a stored device re-acquires on the live device
- toggling noise suppression off while muted refreshes the stored constraints

## test/system/huddle_invitations_test.rb

Owner: WS13. Deferred.

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

Owner: WS13. Deferred.

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

Owner: WS13. Deferred.

- the channel sidebar row and header show participants and empty on revoke
- the DM sidebar row and header show the peer and empty on revoke
- the sidebar aggregate poll clears quietly expired grants
- the sidebar aggregate poll runs on connect and skips in-flight refreshes
- a removed sidebar stack clears once but keeps accepting updates while the header latches
- the aggregate poller skips while hidden and fetches on becoming visible

## test/system/huddle_roster_test.rb

Owner: WS13. Deferred.

- muting patches the local roster row instead of rebuilding it
- speaking and mute changes patch the remote row in place
- joining and leaving adds and removes roster rows only
- the mute toggle keeps a stable label with pressed state and tooltip
- the camera toggle keeps a stable label with pressed state and tooltip
- the meter stops once its track ends
- the meter skips ticks while the tab is hidden
- leaving reports after the disconnect completes

## test/system/huddles_test.rb

Owner: WS13. Deferred.

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

Owner: WS13. Deferred.

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

Owner: WS13. Deferred.

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
