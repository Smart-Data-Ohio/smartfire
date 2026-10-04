# WS14 / WS15 cutover assertions still open

**Partial slice: 257 exact declarations remain without a discriminating acceptance receipt.** These remain in the cutover gate. An open record identifies missing acceptance assertions, without declaring the production path absent.

All historical receipts are retained in the inventories. This list is generated from `ledger-ws14-ws15.json`; reopened records name the exact missing Rails assertions discovered in the full closure audit.

| Record | Current ledger row | Rails declaration | Assertion still required |
|---|---|---|---|
| WS14e-001 | `rust/plans/ws14e-test-inventory.md:22` | `test/models/event_test.rb:25` | requires the end to follow the start |
| WS14e-002 | `rust/plans/ws14e-test-inventory.md:23` | `test/models/event_test.rb:33` | rejects bot and non-member organizers |
| WS14e-003 | `rust/plans/ws14e-test-inventory.md:24` | `test/models/event_test.rb:42` | rejects a soft-deleted room as the venue |
| WS14e-004 | `rust/plans/ws14e-test-inventory.md:27` | `test/models/event_test.rb:70` | members with notifications off or invisible get no invitation |
| WS14e-005 | `rust/plans/ws14e-test-inventory.md:28` | `test/models/event_test.rb:81` | members with notifications off keep their invitation through updates and cancellations |
| WS14e-006 | `rust/plans/ws14e-test-inventory.md:29` | `test/models/event_test.rb:97` | a mentions member is notified through updates and cancellations |
| WS14e-007 | `rust/plans/ws14e-test-inventory.md:30` | `test/models/event_test.rb:109` | invitations exclude bots |
| WS14e-008 | `rust/plans/ws14e-test-inventory.md:31` | `test/models/event_test.rb:118` | a time change notifies going and maybe attendees without duplicating items |
| WS14e-010 | `rust/plans/ws14e-test-inventory.md:33` | `test/models/event_test.rb:148` | a title-only edit creates no items |
| WS14e-011 | `rust/plans/ws14e-test-inventory.md:34` | `test/models/event_test.rb:157` | cancel notifies going and maybe attendees and clears the other items |
| WS14e-013 | `rust/plans/ws14e-test-inventory.md:36` | `test/models/event_test.rb:189` | event items vanish when the recipient leaves the room |
| WS14e-014 | `rust/plans/ws14e-test-inventory.md:37` | `test/models/event_test.rb:200` | deleting a room removes its events, attendances, and inbox items |
| WS14e-015 | `rust/plans/ws14e-test-inventory.md:43` | `test/models/event_attendance_test.rb:8` | test/models/event_attendance_test.rb:11: Reload a member attendance initially maybe through the attendance model path.; test/models/event_attendance_test.rb:13: Update that attendance to going and reload going without adding a second row.; test/models/event_attendance_test.rb:15: Attempt a second attendance creation for the same member/event and assert RecordInvalid, rather than an RSVP upsert. |
| WS14e-017 | `rust/plans/ws14e-test-inventory.md:51` | `test/models/event_calendar_entry_test.rb:11` | destroying an entry enqueues its remote delete |
| WS14e-018 | `rust/plans/ws14e-test-inventory.md:52` | `test/models/event_calendar_entry_test.rb:19` | delete skips the remote delete for already-reconciled rows |
| WS14e-019 | `rust/plans/ws14e-test-inventory.md:53` | `test/models/event_calendar_entry_test.rb:27` | delete_all skips remote deletes for disconnect cleanup |
| WS14e-020 | `rust/plans/ws14e-test-inventory.md:59` | `test/models/event/channel_timeline_test.rb:11` | creating an event posts exactly one announcement by the organizer with the title and URL |
| WS14e-021 | `rust/plans/ws14e-test-inventory.md:61` | `test/models/event/channel_timeline_test.rb:47` | edits and cancellations post nothing |
| WS14e-022 | `rust/plans/ws14e-test-inventory.md:62` | `test/models/event/channel_timeline_test.rb:56` | the announcement creates no inbox items |
| WS14e-023 | `rust/plans/ws14e-test-inventory.md:63` | `test/models/event/channel_timeline_test.rb:63` | test/models/event/channel_timeline_test.rb:76: For one singleton event, assert the separately created same-room message resolves exactly that Event.; test/models/event/channel_timeline_test.rb:86: Assert that this singleton has exactly two same-room referencing messages (announcement plus independently posted link).; test/models/event/channel_timeline_test.rb:88: Update that singleton title and observe exactly one replace for each of those two same-room references.; test/models/event/channel_timeline_test.rb:89: Subscribe the other room and assert zero publications after the same title update while its posted cross-room event link remains unreferenced. |
| WS14e-024 | `rust/plans/ws14e-test-inventory.md:64` | `test/models/event/channel_timeline_test.rb:95` | test/models/event/channel_timeline_test.rb:102: Create a second same-room message linking a singleton and assert its event association is exactly that Event.; test/models/event/channel_timeline_test.rb:104: Cancel that singleton and observe card replacement for both announcement and separate referencing message, with the publication count equal to the exact referencing-message count. |
| WS14e-025 | `rust/plans/ws14e-test-inventory.md:65` | `test/models/event/channel_timeline_test.rb:110` | deleting the event removes its references |
| WS14e-026 | `rust/plans/ws14e-test-inventory.md:78` | `test/models/event/recurrence_test.rb:101` | a single event has no series |
| WS14e-027 | `rust/plans/ws14e-test-inventory.md:81` | `test/models/event/recurrence_test.rb:133` | the occurrence cap and one-year range run on head updates too |
| WS14e-028 | `rust/plans/ws14e-test-inventory.md:87` | `test/models/event/recurrence_test.rb:225` | recurrence fields cannot be changed by injecting the guard flag |
| WS14e-029 | `rust/plans/ws14e-test-inventory.md:93` | `test/models/event/recurrence_test.rb:299` | test/models/event/recurrence_test.rb:310: After a head going RSVP propagates to all three occurrences, make occurrence two declined, cancel occurrence three through cancel_with_scope, then copy maybe from the head and assert the cancelled third occurrence retains its existing going RSVP. |
| WS14e-030 | `rust/plans/ws14e-test-inventory.md:95` | `test/models/event/recurrence_test.rb:336` | this event on the head accepts the form's unchanged rule values |
| WS14e-031 | `rust/plans/ws14e-test-inventory.md:100` | `test/models/event/recurrence_test.rb:416` | test/models/event/recurrence_test.rb:424: Move occurrence two strictly before occurrence one with starts_at and ends_at in each of this_event and this_and_following; assert RecordInvalid in both scopes.; test/models/event/recurrence_test.rb:430: Assert the exact starts_at neighbour-boundary error for those strictly-before inputs in both scopes.; test/models/event/recurrence_test.rb:433: Assert the full series starts_at list is unchanged after both strictly-before attempts. |
| WS14e-032 | `rust/plans/ws14e-test-inventory.md:101` | `test/models/event/recurrence_test.rb:436` | test/models/event/recurrence_test.rb:456: After rejecting a local move strictly past the next sibling, accept the same strictly-past starts_at/ends_at with this_and_following and assert every following start retains its weekly offset (the complete four-start result). |
| WS14e-033 | `rust/plans/ws14e-test-inventory.md:106` | `test/models/event/recurrence_test.rb:520` | series slots are unique among uncancelled occurrences |
| WS14e-034 | `rust/plans/ws14e-test-inventory.md:108` | `test/models/event/recurrence_test.rb:561` | test/models/event/recurrence_test.rb:573: With going Jason and maybe Jz, make a local this_event time edit to occurrence two and assert Jz has an event_update sourced to that occurrence before the next edit.; test/models/event/recurrence_test.rb:585: After the head this_and_following time edit, assert exactly one unread event item for each going and maybe attendee, including Jz.; test/models/event/recurrence_test.rb:586: Assert that the remaining unread item for the maybe attendee is event_update.; test/models/event/recurrence_test.rb:587: Assert that the maybe attendee remaining unread item is sourced to the head occurrence.; test/models/event/recurrence_test.rb:589: Assert the earlier maybe-attendee occurrence-two update is handled, rather than deleted or left unread. |
| WS14e-035 | `rust/plans/ws14e-test-inventory.md:115` | `test/models/event/recurrence_test.rb:739` | test/models/event/recurrence_test.rb:758: Put going Jz and declined Kevin on the same excess occurrence before shortening the rule and assert Kevin has no event_cancelled item for that cancelled excess occurrence. |
| WS14e-036 | `rust/plans/ws14e-test-inventory.md:116` | `test/models/event/recurrence_test.rb:761` | test/models/event/recurrence_test.rb:779: In one this_and_following update, change both starts_at/ends_at by two hours and recurrence_until; assert the distinct-RSVP kept occurrence moves from October12 09:00 to October12 11:00.; test/models/event/recurrence_test.rb:780: After that compound time plus recurrence update, assert the kept occurrence still has Jason declined. |
| WS14e-037 | `rust/plans/ws14e-test-inventory.md:122` | `test/models/event/recurrence_test.rb:915` | series order puts uncancelled occurrences first at equal times |
| WS14e-038 | `rust/plans/ws14e-test-inventory.md:123` | `test/models/event/recurrence_test.rb:932` | a rule change beyond the cap is rejected and leaves the series alone |
| WS14e-039 | `rust/plans/ws14e-test-inventory.md:130` | `test/models/event/recurrence_test.rb:1033` | a head-only series can still be re-timed through this and following |
| WS14e-040 | `rust/plans/ws14e-test-inventory.md:131` | `test/models/event/recurrence_test.rb:1051` | a single-occurrence description edit of the head still succeeds |
| WS14e-041 | `rust/plans/ws14e-test-inventory.md:142` | `test/models/event/reference_sync_test.rb:41` | a message without an event link references nothing |
| WS14e-042 | `rust/plans/ws14e-test-inventory.md:144` | `test/models/event/reference_sync_test.rb:64` | a link to a missing event creates nothing |
| WS14e-043 | `rust/plans/ws14e-test-inventory.md:145` | `test/models/event/reference_sync_test.rb:75` | editing a message to add an event link adds the reference |
| WS14e-044 | `rust/plans/ws14e-test-inventory.md:147` | `test/models/event/reference_sync_test.rb:99` | deleting the message removes its references |
| WS14e-045 | `rust/plans/ws14e-test-inventory.md:155` | `test/models/event/reminder_dispatcher_test.rb:39` | test/models/event/reminder_dispatcher_test.rb:42: For the preference-only opt-out scenario, assert the queued Event::ReminderPushJob arguments identify the actual due event, as well as its class/count.; test/models/event/reminder_dispatcher_test.rb:47: With only one attendee event_reminders=false and another eligible going/maybe attendee still opted in, assert the latter item becomes event_reminder.; test/models/event/reminder_dispatcher_test.rb:50: After the opted-out attendee keeps the invitation, change that same event time through the announcement-producing update and assert that attendee receives event_update.; test/models/event/reminder_dispatcher_test.rb:55: Create a later follow-up event in the same room and assert the opted-out attendee still receives its event_invitation. |
| WS14e-046 | `rust/plans/ws14e-test-inventory.md:157` | `test/models/event/reminder_dispatcher_test.rb:65` | test/models/event/reminder_dispatcher_test.rb:69: Cancel the due event through its actual cancellation API, clear cancellation jobs, execute the due dispatcher and assert no Event::ReminderPushJob is queued.; test/models/event/reminder_dispatcher_test.rb:73: Reload that API-cancelled event after dispatch and assert reminded_at remains null.; test/models/event/reminder_dispatcher_test.rb:74: Assert the going attendee existing item remains event_cancelled after dispatch. |
| WS14e-047 | `rust/plans/ws14e-test-inventory.md:161` | `test/models/event/reminder_dispatcher_test.rb:113` | consecutive occurrences of a series are each reminded once at their own time |
| WS14e-048 | `rust/plans/ws14e-test-inventory.md:162` | `test/models/event/reminder_dispatcher_test.rb:141` | a series starting just before midnight still builds its occurrences |
| WS14e-051 | `rust/plans/ws14e-test-inventory.md:174` | `test/models/event/reminder_pusher_test.rb:32` | the push body names the venue |
| WS14e-057 | `rust/plans/ws14e-test-inventory.md:180` | `test/models/event/reminder_pusher_test.rb:115` | an event that already ended is skipped |
| WS14e-059 | `rust/plans/ws14e-test-inventory.md:187` | `test/models/event/venue_test.rb:11` | a venue is optional |
| WS14e-060 | `rust/plans/ws14e-test-inventory.md:188` | `test/models/event/venue_test.rb:18` | a voice or Stage channel venue is valid |
| WS14e-061 | `rust/plans/ws14e-test-inventory.md:189` | `test/models/event/venue_test.rb:25` | a text channel or DM venue is rejected |
| WS14e-062 | `rust/plans/ws14e-test-inventory.md:190` | `test/models/event/venue_test.rb:35` | the organizer must belong to the venue |
| WS14e-063 | `rust/plans/ws14e-test-inventory.md:191` | `test/models/event/venue_test.rb:44` | an event in a voice channel may use its own room as the venue |
| WS14e-064 | `rust/plans/ws14e-test-inventory.md:192` | `test/models/event/venue_test.rb:51` | other edits stay valid after the organizer leaves the venue |
| WS14e-065 | `rust/plans/ws14e-test-inventory.md:193` | `test/models/event/venue_test.rb:60` | deleting the venue clears the link but keeps the event |
| WS14e-066 | `rust/plans/ws14e-test-inventory.md:194` | `test/models/event/venue_test.rb:69` | scheduling a series copies the venue to every occurrence |
| WS14e-067 | `rust/plans/ws14e-test-inventory.md:195` | `test/models/event/venue_test.rb:79` | this and following propagates a venue change |
| WS14e-068 | `rust/plans/ws14e-test-inventory.md:196` | `test/models/event/venue_test.rb:90` | this and following propagates clearing the venue |
| WS14e-069 | `rust/plans/ws14e-test-inventory.md:197` | `test/models/event/venue_test.rb:101` | a single-occurrence edit changes the venue for that occurrence only |
| WS14e-070 | `rust/plans/ws14e-test-inventory.md:198` | `test/models/event/venue_test.rb:112` | a venue-only edit creates no inbox items |
| WS14e-071 | `rust/plans/ws14e-test-inventory.md:204` | `test/controllers/rooms/events_controller_test.rb:10` | index lists upcoming, past, and cancelled events separately |
| WS14e-072 | `rust/plans/ws14e-test-inventory.md:206` | `test/controllers/rooms/events_controller_test.rb:37` | a member can create an event and members are invited |
| WS14e-073 | `rust/plans/ws14e-test-inventory.md:207` | `test/controllers/rooms/events_controller_test.rb:51` | a member can create a repeating event with one invitation per member |
| WS14e-074 | `rust/plans/ws14e-test-inventory.md:208` | `test/controllers/rooms/events_controller_test.rb:74` | create rejects a series above the occurrence cap |
| WS14e-075 | `rust/plans/ws14e-test-inventory.md:210` | `test/controllers/rooms/events_controller_test.rb:109` | index lists past occurrences individually |
| WS14e-076 | `rust/plans/ws14e-test-inventory.md:213` | `test/controllers/rooms/events_controller_test.rb:172` | updating this and following shifts later occurrences and notifies once per attendee |
| WS14e-077 | `rust/plans/ws14e-test-inventory.md:214` | `test/controllers/rooms/events_controller_test.rb:196` | updating without a scope leaves the rest of the series untouched |
| WS14e-078 | `rust/plans/ws14e-test-inventory.md:215` | `test/controllers/rooms/events_controller_test.rb:218` | changing the rule away from the first event is rejected |
| WS14e-079 | `rust/plans/ws14e-test-inventory.md:217` | `test/controllers/rooms/events_controller_test.rb:253` | an administrator who is not the organizer can use this and following, but an ordinary member cannot |
| WS14e-080 | `rust/plans/ws14e-test-inventory.md:218` | `test/controllers/rooms/events_controller_test.rb:288` | cancelling this and following cancels later occurrences with one item per attendee |
| WS14e-081 | `rust/plans/ws14e-test-inventory.md:219` | `test/controllers/rooms/events_controller_test.rb:307` | cancelling without a scope cancels only that occurrence |
| WS14e-082 | `rust/plans/ws14e-test-inventory.md:220` | `test/controllers/rooms/events_controller_test.rb:322` | cancelling this event explicitly cancels only that occurrence |
| WS14e-083 | `rust/plans/ws14e-test-inventory.md:221` | `test/controllers/rooms/events_controller_test.rb:337` | show renders cancel scopes for series occurrences and a single cancel for single events |
| WS14e-084 | `rust/plans/ws14e-test-inventory.md:222` | `test/controllers/rooms/events_controller_test.rb:356` | index issues a bounded number of queries regardless of occurrence count |
| WS14e-086 | `rust/plans/ws14e-test-inventory.md:225` | `test/controllers/rooms/events_controller_test.rb:419` | requires authentication |
| WS14e-088 | `rust/plans/ws14e-test-inventory.md:227` | `test/controllers/rooms/events_controller_test.rb:443` | the organizer can update times and attendees are notified |
| WS14e-089 | `rust/plans/ws14e-test-inventory.md:235` | `test/controllers/rooms/events_controller_test.rb:541` | a member can create an event with a venue |
| WS14e-090 | `rust/plans/ws14e-test-inventory.md:236` | `test/controllers/rooms/events_controller_test.rb:553` | the organizer can set and clear the venue |
| WS14e-091 | `rust/plans/ws14e-test-inventory.md:237` | `test/controllers/rooms/events_controller_test.rb:571` | create rejects a text channel venue |
| WS14e-092 | `rust/plans/ws14e-test-inventory.md:238` | `test/controllers/rooms/events_controller_test.rb:582` | create rejects a venue the organizer does not belong to |
| WS14e-093 | `rust/plans/ws14e-test-inventory.md:239` | `test/controllers/rooms/events_controller_test.rb:595` | update rejects a venue the organizer does not belong to |
| WS14e-094 | `rust/plans/ws14e-test-inventory.md:240` | `test/controllers/rooms/events_controller_test.rb:609` | the edit form keeps a venue the editor cannot see so an unrelated edit does not clear it |
| WS14e-095 | `rust/plans/ws14e-test-inventory.md:253` | `test/controllers/rooms/events_controller_test.rb:792` | index issues the same queries regardless of event count when venues are shared |
| WS14e-096 | `rust/plans/ws14e-test-inventory.md:258` | `test/controllers/rooms/events_controller_test.rb:863` | show renders no Meet row for a non-https link |
| WS14e-097 | `rust/plans/ws14e-test-inventory.md:268` | `test/controllers/rooms/events/attendances_controller_test.rb:49` | a response on the first event of a series is copied to every future occurrence |
| WS14e-098 | `rust/plans/ws14e-test-inventory.md:269` | `test/controllers/rooms/events/attendances_controller_test.rb:64` | a later response stays local unless apply to all future is checked |
| WS14e-099 | `rust/plans/ws14e-test-inventory.md:285` | `test/system/events_test.rb:4` | scheduling an event invites members, who respond and see it in the inbox |
| WS14e-100 | `rust/plans/ws14e-test-inventory.md:286` | `test/system/events_test.rb:48` | scheduling a repeating event invites once per member and copies the first response |
| WS14e-101 | `rust/plans/ws14e-test-inventory.md:287` | `test/system/events_test.rb:102` | scheduling an event announces it in the room with a card members respond from |
| WS14e-102 | `rust/plans/ws14e-test-inventory.md:293` | `test/integration/event_cards_test.rb:9` | test/integration/event_cards_test.rb:23: GET the actual room as a member after persisting a voice venue event and a separate referencing message; assert success.; test/integration/event_cards_test.rb:26: Scope cards to the exact referencing message DOM container.; test/integration/event_cards_test.rb:27: Assert exactly one event card in that message container.; test/integration/event_cards_test.rb:28: Assert Event eyebrow.; test/integration/event_cards_test.rb:29: Assert the persisted event title Planning session.; test/integration/event_cards_test.rb:30: Assert its title link has the correct room/event route.; test/integration/event_cards_test.rb:31: Assert exactly two start/end time elements.; test/integration/event_cards_test.rb:32: Assert the persisted voice venue Lounge appears.; test/integration/event_cards_test.rb:33: Assert organizer David appears.; test/integration/event_cards_test.rb:34: Assert exactly one attendance frame with the correct room/event/message_id src. |
| WS14e-103 | `rust/plans/ws14e-test-inventory.md:294` | `test/integration/event_cards_test.rb:39` | test/integration/event_cards_test.rb:53: GET a real room containing a persisted voice-venue event link; assert success.; test/integration/event_cards_test.rb:54: Assert at least one real room event card.; test/integration/event_cards_test.rb:55: Assert Lounge venue text in that card.; test/integration/event_cards_test.rb:56: Assert no Join anchor inside the real HTTP event card.; test/integration/event_cards_test.rb:57: Assert no live-dot sidebar-item__icon inside the real HTTP event card. |
| WS14e-104 | `rust/plans/ws14e-test-inventory.md:295` | `test/integration/event_cards_test.rb:60` | test/integration/event_cards_test.rb:73: Persist an actual weekly recurring event and link message, GET the room and assert success.; test/integration/event_cards_test.rb:74: Assert Repeating event eyebrow from actual recurring-event HTTP assembly. |
| WS14e-105 | `rust/plans/ws14e-test-inventory.md:296` | `test/integration/event_cards_test.rb:77` | test/integration/event_cards_test.rb:91: Persist an event and referencing message, cancel through the model API, GET the room and assert success.; test/integration/event_cards_test.rb:92: Assert a real HTTP event-card--cancelled card exists.; test/integration/event_cards_test.rb:93: Assert its state text is Cancelled. |
| WS14e-106 | `rust/plans/ws14e-test-inventory.md:298` | `test/integration/event_cards_test.rb:108` | a link to an event in another room stays a plain link with no card for anyone |
| WS14e-107 | `rust/plans/ws14e-test-inventory.md:299` | `test/integration/event_cards_test.rb:142` | a message without an event link renders no card |
| WS14e-108 | `rust/plans/ws14e-test-inventory.md:300` | `test/integration/event_cards_test.rb:153` | rendering a room page costs no extra queries per message with an event link |
| WS14g-004 | `rust/plans/ws14g-rails-test-map.md:148` | `test/controllers/messages_drive_attachments_test.rb:221` | viewers with and without Drive consent receive identical attachment markup |
| WS14g-005 | `rust/plans/ws14g-rails-test-map.md:149` | `test/controllers/messages_drive_attachments_test.rb:239` | edit form lists attachments as removable chips with the blank sentinel |
| WS14g-007 | `rust/plans/ws14g-rails-test-map.md:265` | `test/controllers/sudos_controller_test.rb:29` | confirming with the password verifies and audit-logs |
| WS14g-010 | `rust/plans/ws14g-rails-test-map.md:268` | `test/controllers/sudos_controller_test.rb:61` | test/controllers/sudos_controller_test.rb:62: no native assertion of TOTP registration at boot in extra_verifiers; HTTP unsupported status and hidden form only establish unavailable unenrolled TOTP. |
| WS14g-011 | `rust/plans/ws14g-rails-test-map.md:269` | `test/controllers/sudos_controller_test.rb:67` | register_verifier adds a verifier |
| WS14g-015 | `rust/plans/ws14g-rails-test-map.md:273` | `test/controllers/sudos_controller_test.rb:109` | confirming with the password still works for enrolled users |
| WS14g-016 | `rust/plans/ws14g-rails-test-map.md:274` | `test/controllers/sudos_controller_test.rb:118` | test/controllers/sudos_controller_test.rb:122,127,130: no isolated wrong-TOTP failure audit increment/verifier assertion or proof an initially unverified session remains gated; cited test rejects wrong codes only after a successful confirmation. |
| WS14g-021 | `rust/plans/ws14g-rails-test-map.md:279` | `test/controllers/sudos_controller_test.rb:196` | test/controllers/sudos_controller_test.rb:200,203,206,207: cited continuation probe injects a pending GET; it never gates GET audit_log.csv, follows the continuation, or asserts successful text/csv content. |
| WS14g-022 | `rust/plans/ws14g-rails-test-map.md:280` | `test/controllers/sudos_controller_test.rb:210` | browsing the audit log needs no confirmation |
| WS14g-023 | `rust/plans/ws14g-rails-test-map.md:281` | `test/controllers/sudos_controller_test.rb:218` | test/controllers/sudos_controller_test.rb:224: probe only asserts OK at now-899; no real join-code POST at 14 minutes with redirect to account edit. |
| WS14g-024 | `rust/plans/ws14g-rails-test-map.md:282` | `test/controllers/sudos_controller_test.rb:228` | test/controllers/sudos_controller_test.rb:234: injected probe timestamps assert FOUND without Location; no 16-minute join-code POST redirect to new_sudo. |
| WS14g-025 | `rust/plans/ws14g-rails-test-map.md:283` | `test/controllers/sudos_controller_test.rb:238` | signing in again starts unverified |
| WS14g-027 | `rust/plans/ws14g-rails-test-map.md:285` | `test/controllers/sudos_controller_test.rb:264` | the replay form rebuilds nested params |
| WS14g-034 | `rust/plans/ws14g-rails-test-map.md:298` | `test/controllers/sudos_controller_test.rb:449` | the confirmation limit lives in the shared rate-limit store, not per-process memory |
| WS14g-035 | `rust/plans/ws14g-rails-test-map.md:313` | `test/integration/drive_picker_test.rb:11` | composer omits the Drive menu item without Drive consent |
| WS14g-036 | `rust/plans/ws14g-rails-test-map.md:314` | `test/integration/drive_picker_test.rb:30` | composer carries the Drive menu item with the Drive scope |
| WS14g-037 | `rust/plans/ws14g-rails-test-map.md:318` | `test/integration/drive_share_picker_test.rb:16` | composer carries a single enhanced Drive menu item when sharing is configured |
| WS14g-038 | `rust/plans/ws14g-rails-test-map.md:319` | `test/integration/drive_share_picker_test.rb:32` | enhanced menu item needs no Drive consent and wins over the legacy picker |
| WS14g-039 | `rust/plans/ws14g-rails-test-map.md:320` | `test/integration/drive_share_picker_test.rb:44` | composer falls back to the legacy picker when sharing is not configured |
| WS14g-040 | `rust/plans/ws14g-rails-test-map.md:321` | `test/integration/drive_share_picker_test.rb:55` | composer omits every Drive menu item without sharing or Drive consent |
| WS14g-041 | `rust/plans/ws14g-rails-test-map.md:322` | `test/integration/drive_share_picker_test.rb:65` | signed-out visitors see no share metas or buttons |
| WS14g-053 | `rust/plans/ws14g-rails-test-map.md:340` | `test/jobs/calendar/disconnect_cleanup_job_test.rb:27` | refreshes an expired access token from the snapshot |
| WS14g-054 | `rust/plans/ws14g-rails-test-map.md:341` | `test/jobs/calendar/disconnect_cleanup_job_test.rb:44` | an invalid_grant skips deletes but still revokes |
| WS14g-056 | `rust/plans/ws14g-rails-test-map.md:343` | `test/jobs/calendar/disconnect_cleanup_job_test.rb:73` | a refresh 503 retries without revoking until the deletes succeed |
| WS14g-058 | `rust/plans/ws14g-rails-test-map.md:345` | `test/jobs/calendar/disconnect_cleanup_job_test.rb:113` | a revoke 5xx schedules a retry |
| WS14g-059 | `rust/plans/ws14g-rails-test-map.md:346` | `test/jobs/calendar/disconnect_cleanup_job_test.rb:128` | an exhausted retry logs the failure at error level |
| WS14g-062 | `rust/plans/ws14g-rails-test-map.md:350` | `test/jobs/calendar/disconnect_cleanup_job_test.rb:197` | an unreadable credentials blob logs a warning with the account id |
| WS14g-063 | `rust/plans/ws14g-rails-test-map.md:351` | `test/jobs/calendar/disconnect_cleanup_job_test.rb:218` | tokens never reach the job logs |
| WS14g-066 | `rust/plans/ws14g-rails-test-map.md:379` | `test/jobs/calendar/remote_delete_job_test.rb:10` | deletes the remote copy with the user's current credentials |
| WS14g-067 | `rust/plans/ws14g-rails-test-map.md:380` | `test/jobs/calendar/remote_delete_job_test.rb:19` | skips a missing account without a request |
| WS14g-068 | `rust/plans/ws14g-rails-test-map.md:381` | `test/jobs/calendar/remote_delete_job_test.rb:25` | skips a disconnected account without a request |
| WS14g-069 | `rust/plans/ws14g-rails-test-map.md:382` | `test/jobs/calendar/remote_delete_job_test.rb:33` | skips a grant without the calendar scope |
| WS14g-070 | `rust/plans/ws14g-rails-test-map.md:383` | `test/jobs/calendar/remote_delete_job_test.rb:41` | treats gone and revoked copies as success |
| WS14g-071 | `rust/plans/ws14g-rails-test-map.md:384` | `test/jobs/calendar/remote_delete_job_test.rb:53` | an invalid_grant disconnects and counts as success |
| WS14g-072 | `rust/plans/ws14g-rails-test-map.md:385` | `test/jobs/calendar/remote_delete_job_test.rb:64` | a transient failure schedules a retry |
| WS14g-073 | `rust/plans/ws14g-rails-test-map.md:386` | `test/jobs/calendar/remote_delete_job_test.rb:73` | other Google failures are logged without raising or retrying |
| WS14g-074 | `rust/plans/ws14g-rails-test-map.md:422` | `test/jobs/calendar/sync_entry_job_test.rb:444` | responding going on the first event of a series syncs every occurrence |
| WS14g-075 | `rust/plans/ws14g-rails-test-map.md:423` | `test/jobs/calendar/sync_entry_job_test.rb:466` | test/jobs/calendar/sync_entry_job_test.rb:467: no direct EventAttendance creation assertion queues Calendar::SyncEntryJob with this event/Kevin; corpus exercises respond/upsert instead. |
| WS14g-076 | `rust/plans/ws14g-rails-test-map.md:424` | `test/jobs/calendar/sync_entry_job_test.rb:472` | test/jobs/calendar/sync_entry_job_test.rb:479: no unrelated updated_at-only attendance save asserts zero jobs; unchanged response upsert is a different branch. |
| WS14g-077 | `rust/plans/ws14g-rails-test-map.md:425` | `test/jobs/calendar/sync_entry_job_test.rb:484` | updating event times enqueues syncs for connected going/maybe attendees |
| WS14g-078 | `rust/plans/ws14g-rails-test-map.md:426` | `test/jobs/calendar/sync_entry_job_test.rb:497` | an update inside a transaction enqueues only after commit |
| WS14g-079 | `rust/plans/ws14g-rails-test-map.md:427` | `test/jobs/calendar/sync_entry_job_test.rb:508` | setting or clearing the venue enqueues a sync, like a title change |
| WS14g-080 | `rust/plans/ws14g-rails-test-map.md:428` | `test/jobs/calendar/sync_entry_job_test.rb:521` | updating only the title enqueues a sync, an unchanged save does not |
| WS14g-081 | `rust/plans/ws14g-rails-test-map.md:429` | `test/jobs/calendar/sync_entry_job_test.rb:533` | test/jobs/calendar/sync_entry_job_test.rb:537: no singleton cancel with two distinct users persisted calendar entries asserts exactly two SyncEntryJob enqueues; cited series case has one entry per occurrence. |
| WS14g-083 | `rust/plans/ws14g-rails-test-map.md:431` | `test/jobs/calendar/sync_entry_job_test.rb:552` | destroying a room enqueues remote deletes for its entries |
| WS14g-084 | `rust/plans/ws14g-rails-test-map.md:432` | `test/jobs/calendar/sync_entry_job_test.rb:566` | shrinking a series enqueues remote deletes for destroyed occurrences |
| WS14g-108 | `rust/plans/ws14g-rails-test-map.md:490` | `test/models/calendar/meeting_refresh_test.rb:142` | test/models/calendar/meeting_refresh_test.rb:145-147: inject raw JSON::ParserError("unexpected token") escaping Google::Client#list_events and require real Calendar::MeetingRefresh.refresh returns :error instead of raising.; test/models/calendar/meeting_refresh_test.rb:150-151: under that same escaping-client-error input, preserve exact existing September23 10:00/11:00 busy pair and persist UNREACHABLE_MESSAGE. Current malformed HTTP corpus takes Api::Unavailable classification instead. |
| WS14g-121 | `rust/plans/ws14g-rails-test-map.md:543` | `test/models/drive_attachment_test.rb:9` | a message with attachments and no text is valid |
| WS14g-122 | `rust/plans/ws14g-rails-test-map.md:544` | `test/models/drive_attachment_test.rb:18` | a textless message without attachments is still invalid |
| WS14g-123 | `rust/plans/ws14g-rails-test-map.md:545` | `test/models/drive_attachment_test.rb:25` | an invalid file id is rejected |
| WS14g-124 | `rust/plans/ws14g-rails-test-map.md:546` | `test/models/drive_attachment_test.rb:37` | duplicate file ids on one message collapse to a single row |
| WS14g-125 | `rust/plans/ws14g-rails-test-map.md:547` | `test/models/drive_attachment_test.rb:47` | the same file id may attach to different messages |
| WS14g-126 | `rust/plans/ws14g-rails-test-map.md:548` | `test/models/drive_attachment_test.rb:58` | the 11th attachment is rejected |
| WS14g-127 | `rust/plans/ws14g-rails-test-map.md:549` | `test/models/drive_attachment_test.rb:70` | destroying the message destroys its attachments |
| WS14g-128 | `rust/plans/ws14g-rails-test-map.md:550` | `test/models/drive_attachment_test.rb:79` | url is the open link for the file id |
| WS14g-129 | `rust/plans/ws14g-rails-test-map.md:554` | `test/models/event_calendar_entry_test.rb:11` | destroying an entry enqueues its remote delete |
| WS14g-130 | `rust/plans/ws14g-rails-test-map.md:555` | `test/models/event_calendar_entry_test.rb:19` | delete skips the remote delete for already-reconciled rows |
| WS14g-131 | `rust/plans/ws14g-rails-test-map.md:556` | `test/models/event_calendar_entry_test.rb:27` | delete_all skips remote deletes for disconnect cleanup |
| WS14g-181 | `rust/plans/ws14g-rails-test-map.md:655` | `test/models/google_account_test.rb:6` | test/models/google_account_test.rb:11: A new second GoogleAccount for an already-connected user is invalid before persistence.; test/models/google_account_test.rb:12: The duplicate carries exactly user_id: ["has already been taken"]. |
| WS14g-184 | `rust/plans/ws14g-rails-test-map.md:658` | `test/models/google_account_test.rb:44` | connected, usable, and expiry predicates |
| WS14g-185 | `rust/plans/ws14g-rails-test-map.md:659` | `test/models/google_account_test.rb:61` | calendar? treats blank scopes as granted and requires calendar.events otherwise |
| WS14g-224 | `rust/plans/ws14g-rails-test-map.md:707` | `test/system/drive_attachments_test.rb:10` | attach Drive files from the picker, send textless, and remove through edit |
| WS14g-225 | `rust/plans/ws14g-rails-test-map.md:708` | `test/system/drive_attachments_test.rb:79` | edit a room message in the composer and remove one of two attachments |
| WS14g-226 | `rust/plans/ws14g-rails-test-map.md:709` | `test/system/drive_attachments_test.rb:116` | attach a Drive file from the thread composer |
| WS14g-227 | `rust/plans/ws14g-rails-test-map.md:713` | `test/system/drive_link_previews_test.rb:10` | a viewer with the Drive scope sees a picked file upgraded to a preview chip |
| WS14g-228 | `rust/plans/ws14g-rails-test-map.md:714` | `test/system/drive_link_previews_test.rb:24` | a viewer with the Drive scope keeps a plain chip for a file never picked |
| WS14g-229 | `rust/plans/ws14g-rails-test-map.md:715` | `test/system/drive_link_previews_test.rb:36` | a viewer without the Drive scope sees a plain chip and fetches nothing |
| WS14g-230 | `rust/plans/ws14g-rails-test-map.md:716` | `test/system/drive_link_previews_test.rb:51` | composer Drive picker inserts the chosen file link at the caret |
| WS14g-231 | `rust/plans/ws14g-rails-test-map.md:717` | `test/system/drive_link_previews_test.rb:88` | composer omits the Drive menu item without the Drive scope |
| WS14g-232 | `rust/plans/ws14g-rails-test-map.md:721` | `test/system/drive_share_test.rb:115` | review dialog offers attach-only and an explicit grant with names and emails |
| WS14g-233 | `rust/plans/ws14g-rails-test-map.md:722` | `test/system/drive_share_test.rb:150` | attach-only pins the chip and writes no Drive permissions |
| WS14g-234 | `rust/plans/ws14g-rails-test-map.md:723` | `test/system/drive_share_test.rb:178` | grant validates, preserves writers, and creates only missing readers |
| WS14g-235 | `rust/plans/ws14g-rails-test-map.md:724` | `test/system/drive_share_test.rb:220` | partial failure reports per recipient and retries only outstanding grants |
| WS14g-236 | `rust/plans/ws14g-rails-test-map.md:725` | `test/system/drive_share_test.rb:256` | cancelled picker selection shares nothing and can be retried |
| WS14g-237 | `rust/plans/ws14g-rails-test-map.md:726` | `test/system/drive_share_test.rb:273` | cancelled Google authorization shares nothing and can be retried |
| WS14g-238 | `rust/plans/ws14g-rails-test-map.md:727` | `test/system/drive_share_test.rb:289` | picker cancel returns quietly and the drive button works again |
| WS14g-239 | `rust/plans/ws14g-rails-test-map.md:728` | `test/system/drive_share_test.rb:307` | closed consent popup returns quietly to the composer |
| WS14g-240 | `rust/plans/ws14g-rails-test-map.md:729` | `test/system/drive_share_test.rb:324` | consent popup closed via GIS error callback returns quietly |
| WS14g-241 | `rust/plans/ws14g-rails-test-map.md:730` | `test/system/drive_share_test.rb:340` | real error dialog closes with the Close button and the drive button works again |
| WS14g-242 | `rust/plans/ws14g-rails-test-map.md:731` | `test/system/drive_share_test.rb:362` | real error dialog closes with Esc |
| WS14g-243 | `rust/plans/ws14g-rails-test-map.md:732` | `test/system/drive_share_test.rb:377` | try again re-opens the picker after a real error |
| WS14g-244 | `rust/plans/ws14g-rails-test-map.md:733` | `test/system/drive_share_test.rb:396` | script load failure offers retry without hanging the composer |
| WS14g-245 | `rust/plans/ws14g-rails-test-map.md:734` | `test/system/drive_share_test.rb:412` | first use loads scripts then continues on a fresh gesture |
| WS14g-246 | `rust/plans/ws14g-rails-test-map.md:735` | `test/system/drive_share_test.rb:426` | unshareable file disables the grant but keeps attach-only |
| WS14g-247 | `rust/plans/ws14g-rails-test-map.md:736` | `test/system/drive_share_test.rb:443` | folder selection disables the grant but keeps attach-only |
| WS14g-248 | `rust/plans/ws14g-rails-test-map.md:737` | `test/system/drive_share_test.rb:461` | expired Google session reconnects on an explicit gesture and continues |
| WS14g-249 | `rust/plans/ws14g-rails-test-map.md:738` | `test/system/drive_share_test.rb:484` | grant rejects a recipient who left mid-review and refreshes the list |
| WS14g-250 | `rust/plans/ws14g-rails-test-map.md:739` | `test/system/drive_share_test.rb:508` | thread composer grants against the parent room membership |
| WS14g-251 | `rust/plans/ws14g-rails-test-map.md:740` | `test/system/drive_share_test.rb:542` | navigation disposes the dialog, token, and picker state |
| WS14g-252 | `rust/plans/ws14g-rails-test-map.md:741` | `test/system/drive_share_test.rb:563` | mobile viewport keeps the review dialog usable |
| WS14g-253 | `rust/plans/ws14g-rails-test-map.md:742` | `test/system/drive_share_test.rb:594` | grant requires fresh review when a recipient email changes |
| WS14g-254 | `rust/plans/ws14g-rails-test-map.md:743` | `test/system/drive_share_test.rb:617` | changed identity can be re-approved against the new email |
| WS14g-255 | `rust/plans/ws14g-rails-test-map.md:744` | `test/system/drive_share_test.rb:634` | retry revalidates membership before writing |
| WS14g-256 | `rust/plans/ws14g-rails-test-map.md:745` | `test/system/drive_share_test.rb:663` | reconnect revalidates before resuming grants |
| WS14g-257 | `rust/plans/ws14g-rails-test-map.md:746` | `test/system/drive_share_test.rb:687` | cancelled authorization invalidates a delayed token callback |
| WS14g-258 | `rust/plans/ws14g-rails-test-map.md:747` | `test/system/drive_share_test.rb:716` | cancelled picker invalidates a delayed selection callback |
| WS14g-259 | `rust/plans/ws14g-rails-test-map.md:748` | `test/system/drive_share_test.rb:735` | grant is blocked when attachments are already full |
| WS14g-260 | `rust/plans/ws14g-rails-test-map.md:749` | `test/system/drive_share_test.rb:770` | mid-flight capacity loss preserves grant outcomes |
| WS14g-261 | `rust/plans/ws14g-rails-test-map.md:750` | `test/system/drive_share_test.rb:796` | rate-limited grants are classified and retry cleanly |
| WS14g-262 | `rust/plans/ws14g-rails-test-map.md:751` | `test/system/drive_share_test.rb:822` | dialog checkboxes are visible and long names wrap |
| WS14g-263 | `rust/plans/ws14g-rails-test-map.md:752` | `test/system/drive_share_test.rb:854` | retry does not write while attachment capacity is unavailable |
| WS14g-264 | `rust/plans/ws14g-rails-test-map.md:753` | `test/system/drive_share_test.rb:895` | completed outcomes stay visible through reconnect and refreshed review |
| WS14g-265 | `rust/plans/ws14g-rails-test-map.md:754` | `test/system/drive_share_test.rb:928` | reconciled access is confirmed, not claimed as newly granted |
| WS14g-266 | `rust/plans/ws14g-rails-test-map.md:755` | `test/system/drive_share_test.rb:958` | existing and reconciled access are distinguished with no new grants |
| WS14g-267 | `rust/plans/ws14g-rails-test-map.md:756` | `test/system/drive_share_test.rb:986` | server errors are classified as service failures, not policy denials |
| WS14g-269 | `rust/plans/ws14g-rails-test-map.md:764` | `test/system/meeting_status_test.rb:7` | opting in shows In a meeting for a stubbed busy interval, then clears after it ends |
| WS14g-270 | `rust/plans/ws14g-rails-test-map.md:765` | `test/system/meeting_status_test.rb:34` | the profile links to connect without a Google account |
| WS14g-271 | `rust/plans/ws14g-rails-test-map.md:769` | `test/system/out_of_office_test.rb:4` | set OOO until tomorrow, badge and DM notice show for another user, then clear it |
| WS14g-272 | `rust/plans/ws14g-rails-test-map.md:773` | `test/system/sudo_mode_test.rb:8` | one prompt, then the action continues automatically |
| WS14g-273 | `rust/plans/ws14g-rails-test-map.md:774` | `test/system/sudo_mode_test.rb:27` | a wrong password keeps the action gated |
| WS15g-002 | `rust/plans/ws15g-rails-tests.md:97` | `test/controllers/agents/github_action_delivery_test.rb:25` | test/controllers/agents/github_action_delivery_test.rb:38: Select the completed ledger row message_id after executing the actual approval-produced action job and assert it is null.; test/controllers/agents/github_action_delivery_test.rb:30: Assert the complete success ledger contract after approval.decide -> persisted Github::PerformAgentActionJob -> registered execution, rather than combining direct-consumer complete snapshots with a runner EXISTS(status) check.; test/controllers/agents/github_action_delivery_test.rb:39: From that same approval-produced job, assert the complete approval_id/action/status/returned URL metadata and the correct room/outcome together. |
| WS15g-003 | `rust/plans/ws15g-rails-tests.md:98` | `test/controllers/agents/github_action_delivery_test.rb:50` | test/controllers/agents/github_action_delivery_test.rb:58: Decide the actual approval approved, run its registered durable action job against HTTP403 with the refusal reason, then compare exactly approval_id/action/status=failed/message=GitHub refused: <response reason> and absence of URL in that same ledger row. |
| WS15g-004 | `rust/plans/ws15g-rails-tests.md:99` | `test/controllers/agents/github_action_delivery_test.rb:69` | completion appears in event polling with the github_action payload |
| WS15g-005 | `rust/plans/ws15g-rails-tests.md:100` | `test/controllers/agents/github_action_delivery_test.rb:95` | failed completions poll with the message and no url |
| WS15g-006 | `rust/plans/ws15g-rails-tests.md:101` | `test/controllers/agents/github_action_delivery_test.rb:114` | ack works on completion rows |
| WS15g-007 | `rust/plans/ws15g-rails-tests.md:102` | `test/controllers/agents/github_action_delivery_test.rb:129` | completion posts the webhook with agent and github_action keys |
| WS15g-008 | `rust/plans/ws15g-rails-tests.md:103` | `test/controllers/agents/github_action_delivery_test.rb:151` | completion rows are readable by their own agent only |
| WS15g-009 | `rust/plans/ws15g-rails-tests.md:104` | `test/controllers/agents/github_action_delivery_test.rb:173` | the ledger page lists the completion with its status |
| WS15g-010 | `rust/plans/ws15g-rails-tests.md:105` | `test/controllers/agents/github_action_delivery_test.rb:188` | the ledger page lists failures with their reason |
| WS15g-011 | `rust/plans/ws15g-rails-tests.md:134` | `test/controllers/github/connections_controller_test.rb:71` | the profile cannot edit the login while a verified account is linked |
| WS15g-012 | `rust/plans/ws15g-rails-tests.md:135` | `test/controllers/github/connections_controller_test.rb:86` | the profile edits the login again once the link is disconnected |
| WS15g-013 | `rust/plans/ws15g-rails-tests.md:140` | `test/controllers/github/connections_controller_test.rb:144` | linking never logs the pasted token |
| WS15g-014 | `rust/plans/ws15g-rails-tests.md:141` | `test/controllers/github/connections_controller_test.rb:156` | the token parameter is filtered from request logs |
| WS15g-015 | `rust/plans/ws15g-rails-tests.md:157` | `test/controllers/github/pull_request_comments_controller_test.rb:156` | posting never logs the member's token |
| WS15g-016 | `rust/plans/ws15g-rails-tests.md:177` | `test/controllers/github/pull_request_review_requests_controller_test.rb:210` | requesting never logs the member's token |
| WS15g-017 | `rust/plans/ws15g-rails-tests.md:199` | `test/controllers/github/pull_request_threads_controller_test.rb:54` | discuss reuses the winner and drops the loser when the race is lost at the unique index |
| WS15g-018 | `rust/plans/ws15g-rails-tests.md:200` | `test/controllers/github/pull_request_threads_controller_test.rb:66` | discuss reuses the winner and drops the loser when the race is lost at the validation |
| WS15g-020 | `rust/plans/ws15g-rails-tests.md:219` | `test/controllers/github/webhooks_controller_test.rb:26` | valid pull_request signature updates the record and broadcasts once |
| WS15g-021 | `rust/plans/ws15g-rails-tests.md:236` | `test/controllers/github/webhooks_controller_test.rb:220` | subscribed repositories enqueue subscription delivery and post once |
| WS15g-022 | `rust/plans/ws15g-rails-tests.md:237` | `test/controllers/github/webhooks_controller_test.rb:239` | redelivered subscription events post nothing |
| WS15g-023 | `rust/plans/ws15g-rails-tests.md:251` | `test/controllers/rooms/github/pull_request_cards_controller_test.rb:105` | a cached denial no longer applies after the member relinks |
| WS15g-024 | `rust/plans/ws15g-rails-tests.md:252` | `test/controllers/rooms/github/pull_request_cards_controller_test.rb:139` | a transport error renders the empty frame and caches nothing |
| WS15g-025 | `rust/plans/ws15g-rails-tests.md:288` | `test/helpers/github_pull_requests_helper_test.rb:35` | cache key for a message without pull requests is just the message |
| WS15g-026 | `rust/plans/ws15g-rails-tests.md:289` | `test/helpers/github_pull_requests_helper_test.rb:41` | test/helpers/github_pull_requests_helper_test.rb:50: Freeze the clock across posting the additional thread reply so all relevant timestamps stay equal, then compare the actual before/after message helper keys and assert they differ through the reply-count dependency. |
| WS15g-027 | `rust/plans/ws15g-rails-tests.md:290` | `test/helpers/github_pull_requests_helper_test.rb:53` | cache key changes when an older thread reply is deleted |
| WS15g-028 | `rust/plans/ws15g-rails-tests.md:291` | `test/helpers/github_pull_requests_helper_test.rb:66` | cache key reads the reply count without a query |
| WS15g-029 | `rust/plans/ws15g-rails-tests.md:292` | `test/helpers/github_pull_requests_helper_test.rb:78` | cache key carries the streaming flag |
| WS15g-030 | `rust/plans/ws15g-rails-tests.md:293` | `test/helpers/github_pull_requests_helper_test.rb:89` | cache key changes when a step is added to the message |
| WS15g-034 | `rust/plans/ws15g-rails-tests.md:297` | `test/helpers/github_pull_requests_helper_test.rb:151` | cache key changes when a quoted source is deleted |
| WS15g-035 | `rust/plans/ws15g-rails-tests.md:298` | `test/helpers/github_pull_requests_helper_test.rb:167` | cache key changes when a poll is voted and retracted |
| WS15g-036 | `rust/plans/ws15g-rails-tests.md:299` | `test/helpers/github_pull_requests_helper_test.rb:186` | cache key carries the system note flag |
| WS15g-037 | `rust/plans/ws15g-rails-tests.md:300` | `test/helpers/github_pull_requests_helper_test.rb:194` | cache key changes when the message is pinned and unpinned |
| WS15g-038 | `rust/plans/ws15g-rails-tests.md:301` | `test/helpers/github_pull_requests_helper_test.rb:211` | test/helpers/github_pull_requests_helper_test.rb:224: Pin a message referencing a PR, capture its helper key, update that referenced PR after the pin timestamp, unpin through the actual MessagePin API, reload and assert the helper key differs for the same newer-PR scenario. |
| WS15g-039 | `rust/plans/ws15g-rails-tests.md:302` | `test/helpers/github_pull_requests_helper_test.rb:227` | cache key changes when a referenced X post is fetched |
| WS15g-040 | `rust/plans/ws15g-rails-tests.md:303` | `test/helpers/github_pull_requests_helper_test.rb:241` | cache key changes when a referenced link embed is fetched |
| WS15g-042 | `rust/plans/ws15g-rails-tests.md:321` | `test/integration/github_pr_cards_test.rb:156` | rendering a room page costs no extra queries per message with a PR link |
| WS15g-043 | `rust/plans/ws15g-rails-tests.md:326` | `test/integration/github_pr_cards_test.rb:254` | the open-room join page leaks no card content to non-members |
| WS15g-047 | `rust/plans/ws15g-rails-tests.md:336` | `test/integration/github_pr_threads_test.rb:62` | the files summary omits the more line when everything is shown |
| WS15g-048 | `rust/plans/ws15g-rails-tests.md:337` | `test/integration/github_pr_threads_test.rb:78` | a PR thread without fetched files shows a loading summary |
| WS15g-049 | `rust/plans/ws15g-rails-tests.md:338` | `test/integration/github_pr_threads_test.rb:92` | an ordinary thread shows no PR header |
| WS15g-050 | `rust/plans/ws15g-rails-tests.md:339` | `test/integration/github_pr_threads_test.rb:105` | file paths from the API render as text |
| WS15g-053 | `rust/plans/ws15g-rails-tests.md:364` | `test/jobs/github/deliver_subscription_event_job_test.rb:77` | test/jobs/github/deliver_subscription_event_job_test.rb:89: After actual review_requested delivery creates the linked reviewer item, invoke ActivityItem::accessible_to for that reviewer and assert the exact created item is included.; test/jobs/github/deliver_subscription_event_job_test.rb:92: Delete the reviewer room membership after delivery, invoke that same inbox reader again and assert the retained persisted item is no longer accessible. |
| WS15g-054 | `rust/plans/ws15g-rails-tests.md:366` | `test/jobs/github/deliver_subscription_event_job_test.rb:111` | test/jobs/github/deliver_subscription_event_job_test.rb:125: While that reviewer github_review_requests=false, post an ordinary real message containing a mention attachment for the reviewer and assert the produced item event_type is mention. |
| WS15g-055 | `rust/plans/ws15g-rails-tests.md:381` | `test/jobs/github/deliver_subscription_event_job_test.rb:289` | test/jobs/github/deliver_subscription_event_job_test.rb:300: After actual review_requested routing creates the PR-thread reply and reviewer item, invoke ActivityItem::accessible_to for the linked reviewer and assert that exact thread-sourced item is included. |
| WS15g-056 | `rust/plans/ws15g-rails-tests.md:420` | `test/jobs/github/fetch_pull_request_job_test.rb:305` | card updates broadcast the thread header to mapped thread streams |
| WS15g-057 | `rust/plans/ws15g-rails-tests.md:421` | `test/jobs/github/fetch_pull_request_job_test.rb:328` | card updates broadcast nothing without referencing messages or mappings |
| WS15g-058 | `rust/plans/ws15g-rails-tests.md:427` | `test/jobs/github/perform_agent_action_job_test.rb:34` | approving a github action enqueues the job, denying does not |
| WS15g-059 | `rust/plans/ws15g-rails-tests.md:431` | `test/jobs/github/perform_agent_action_job_test.rb:78` | approving a non-github action enqueues nothing |
| WS15g-061 | `rust/plans/ws15g-rails-tests.md:594` | `test/models/github/write_client_test.rb:102` | network errors raise Error without logging the token |
| WS15g-062 | `rust/plans/ws15g-rails-tests.md:629` | `test/system/github_pr_write_actions_test.rb:6` | a linked member comments from a PR thread and sees the inline confirmation |
| WS15g-063 | `rust/plans/ws15g-rails-tests.md:630` | `test/system/github_pr_write_actions_test.rb:52` | a linked member requests a review from a PR thread and sees the inline confirmation |
| WS15g-064 | `rust/plans/ws15g-rails-tests.md:631` | `test/system/github_pr_write_actions_test.rb:98` | a member without a linked token sees the connect prompt in the thread |
