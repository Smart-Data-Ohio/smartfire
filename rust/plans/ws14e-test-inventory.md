# WS14e Rails test inventory (partial)

Pinned reference: `d7c7de92`. Covered means the named scenario has a discriminating Rust assertion in this slice; it does not mean full HTTP, HTML, callback failure, or transport parity. Deferred scenarios stay with WS14e continuation unless another owner is named. Calendar jobs/models stay with WS14g; push transport/policy stays with WS17.

## test/models/event_test.rb (18)

| Rails test | Coverage or deferral |
|---|---|
| `test/models/event_test.rb:9` — requires a title, start time, and time zone | Covered: `event_validations_match_rails_messages` |
| `test/models/event_test.rb:18` — rejects unknown time zones | Covered: `event_validations_match_rails_messages` |
| `test/models/event_test.rb:25` — requires the end to follow the start | Covered: `event_validations_match_rails_messages (equal end; earlier end still deferred)` |
| `test/models/event_test.rb:33` — rejects bot and non-member organizers | Deferred: WS14e continuation |
| `test/models/event_test.rb:42` — rejects a soft-deleted room as the venue | Deferred: WS14e continuation |
| `test/models/event_test.rb:53` — records the organizer as going | Covered: `series_materializes_organizer_attendance_one_invitation_and_one_announcement` |
| `test/models/event_test.rb:59` — invites every other active human member | Covered: `series_materializes_organizer_attendance_one_invitation_and_one_announcement` |
| `test/models/event_test.rb:70` — members with notifications off or invisible get no invitation | Deferred: WS14e continuation |
| `test/models/event_test.rb:81` — members with notifications off keep their invitation through updates and cancellations | Deferred: WS14e continuation |
| `test/models/event_test.rb:97` — a mentions member is notified through updates and cancellations | Deferred: WS14e continuation |
| `test/models/event_test.rb:109` — invitations exclude bots | Deferred: WS14e continuation |
| `test/models/event_test.rb:118` — a time change notifies going and maybe attendees without duplicating items | Deferred: WS14e continuation |
| `test/models/event_test.rb:138` — a time change resets the reminder | Deferred: WS14e continuation |
| `test/models/event_test.rb:148` — a title-only edit creates no items | Deferred: WS14e continuation |
| `test/models/event_test.rb:157` — cancel notifies going and maybe attendees and clears the other items | Deferred: WS14e continuation |
| `test/models/event_test.rb:176` — cancelling twice is a no-op | Deferred: WS14e continuation |
| `test/models/event_test.rb:189` — event items vanish when the recipient leaves the room | Deferred: WS14e continuation |
| `test/models/event_test.rb:200` — deleting a room removes its events, attendances, and inbox items | Deferred: WS14e continuation |

## test/models/event_attendance_test.rb (3)

| Rails test | Coverage or deferral |
|---|---|
| `test/models/event_attendance_test.rb:8` — a member holds a single response per event | Deferred: WS14e continuation |
| `test/models/event_attendance_test.rb:20` — rejects unknown responses | Deferred: WS14e continuation |
| `test/models/event_attendance_test.rb:26` — rejects bots, non-members, and responses to cancelled events | Covered: `ws14e_security_rsvp_rejects_nonmembers_bots_and_cancelled_events` |

## test/models/event_calendar_entry_test.rb (3)

| Rails test | Coverage or deferral |
|---|---|
| `test/models/event_calendar_entry_test.rb:11` — destroying an entry enqueues its remote delete | Deferred: WS14g |
| `test/models/event_calendar_entry_test.rb:19` — delete skips the remote delete for already-reconciled rows | Deferred: WS14g |
| `test/models/event_calendar_entry_test.rb:27` — delete_all skips remote deletes for disconnect cleanup | Deferred: WS14g |

## test/models/event/channel_timeline_test.rb (7)

| Rails test | Coverage or deferral |
|---|---|
| `test/models/event/channel_timeline_test.rb:11` — creating an event posts exactly one announcement by the organizer with the title and URL | Covered: `series_materializes_organizer_attendance_one_invitation_and_one_announcement (series only; singleton exact count deferred)` |
| `test/models/event/channel_timeline_test.rb:27` — a repeating series announces once for the head and occurrences never announce | Covered: `series_materializes_organizer_attendance_one_invitation_and_one_announcement` |
| `test/models/event/channel_timeline_test.rb:47` — edits and cancellations post nothing | Deferred: WS14e continuation |
| `test/models/event/channel_timeline_test.rb:56` — the announcement creates no inbox items | Deferred: WS14e continuation |
| `test/models/event/channel_timeline_test.rb:63` — updating an event broadcasts a card replace for each referencing message | Deferred: WS14e continuation |
| `test/models/event/channel_timeline_test.rb:95` — cancelling an event broadcasts card updates | Deferred: WS14e continuation |
| `test/models/event/channel_timeline_test.rb:110` — deleting the event removes its references | Deferred: WS14e continuation |

## test/models/event/recurrence_test.rb (63)

| Rails test | Coverage or deferral |
|---|---|
| `test/models/event/recurrence_test.rb:11` — daily generation advances calendar days and keeps the duration | Covered: `pinned_rails_recurrence_vectors` |
| `test/models/event/recurrence_test.rb:24` — weekly generation advances seven days up to and including the end date | Covered: `pinned_rails_recurrence_vectors` |
| `test/models/event/recurrence_test.rb:33` — biweekly generation advances fourteen days | Covered: `pinned_rails_recurrence_vectors` |
| `test/models/event/recurrence_test.rb:42` — monthly generation anchors to the head day of month | Covered: `pinned_rails_recurrence_vectors` |
| `test/models/event/recurrence_test.rb:51` — monthly from the 31st falls back to the last day then returns to the 31st | Covered: `pinned_rails_recurrence_vectors` |
| `test/models/event/recurrence_test.rb:63` — weekly across a daylight-saving change keeps the local wall-clock time | Covered: `pinned_rails_recurrence_vectors` |
| `test/models/event/recurrence_test.rb:81` — materializing links every occurrence to the head and records the organizer as going | Covered: `series_materializes_organizer_attendance_one_invitation_and_one_announcement` |
| `test/models/event/recurrence_test.rb:101` — a single event has no series | Deferred: WS14e continuation |
| `test/models/event/recurrence_test.rb:111` — more than 52 occurrences is rejected with an earlier-end-date message | Covered: `recurrence_range_cap_and_blank_rule_match_rails` |
| `test/models/event/recurrence_test.rb:124` — exactly 52 occurrences is allowed | Covered: `recurrence_range_cap_and_blank_rule_match_rails` |
| `test/models/event/recurrence_test.rb:133` — the occurrence cap and one-year range run on head updates too | Deferred: WS14e continuation |
| `test/models/event/recurrence_test.rb:145` — the end date is required when a rule is set | Covered: `recurrence_range_cap_and_blank_rule_match_rails` |
| `test/models/event/recurrence_test.rb:155` — the end date must be after the start date | Covered: `recurrence_range_cap_and_blank_rule_match_rails` |
| `test/models/event/recurrence_test.rb:169` — the end date must be at most one year after the start date | Covered: `recurrence_range_cap_and_blank_rule_match_rails` |
| `test/models/event/recurrence_test.rb:186` — unknown rules are rejected and blank rules normalize to nil | Covered: `event_validations_match_rails_messages; recurrence_range_cap_and_blank_rule_match_rails` |
| `test/models/event/recurrence_test.rb:201` — recurrence fields cannot be changed through plain update | Covered: `scoped_recurrence_guards_do_not_escape_the_operation` |
| `test/models/event/recurrence_test.rb:225` — recurrence fields cannot be changed by injecting the guard flag | Deferred: WS14e continuation |
| `test/models/event/recurrence_test.rb:236` — the recurrence guard does not persist past a scoped update | Covered: `scoped_recurrence_guards_do_not_escape_the_operation` |
| `test/models/event/recurrence_test.rb:249` — a series sends one invitation per invitee, attached to the first event | Covered: `series_materializes_organizer_attendance_one_invitation_and_one_announcement` |
| `test/models/event/recurrence_test.rb:264` — a response on the first event is copied to every future occurrence | Covered: `head_response_copies_and_follower_response_stays_local_until_requested` |
| `test/models/event/recurrence_test.rb:275` — a response on a later occurrence stays local without the checkbox | Covered: `head_response_copies_and_follower_response_stays_local_until_requested` |
| `test/models/event/recurrence_test.rb:286` — apply to all future copies the response to that occurrence and every later one | Covered: `head_response_copies_and_follower_response_stays_local_until_requested` |
| `test/models/event/recurrence_test.rb:299` — copying a response overwrites distinct later responses but skips cancelled occurrences | Deferred: WS14e continuation |
| `test/models/event/recurrence_test.rb:313` — this event leaves its siblings untouched | Covered: `event_scoped_operations_match_rails_vectors (local title)` |
| `test/models/event/recurrence_test.rb:336` — this event on the head accepts the form's unchanged rule values | Deferred: WS14e continuation |
| `test/models/event/recurrence_test.rb:350` — this event is the default scope | Covered: `event_scoped_operations_match_rails_vectors (default scope)` |
| `test/models/event/recurrence_test.rb:360` — this and following shifts later occurrences by the same offset and copies the title | Covered: `event_scoped_operations_match_rails_vectors (following title and following move)` |
| `test/models/event/recurrence_test.rb:380` — moving an occurrence later shifts every original follower by the same offset | Covered: `event_scoped_operations_match_rails_vectors (following move onto next slot)` |
| `test/models/event/recurrence_test.rb:398` — moving an occurrence onto its head's slot is rejected in either scope | Covered: `event_scoped_operations_match_rails_vectors (previous bound both scopes)` |
| `test/models/event/recurrence_test.rb:416` — moving an occurrence before its previous sibling is rejected in either scope | Deferred: WS14e continuation |
| `test/models/event/recurrence_test.rb:436` — a single-occurrence edit past the next sibling is rejected but this and following allows it | Deferred: WS14e continuation |
| `test/models/event/recurrence_test.rb:460` — a re-time between the neighbouring occurrences still passes | Covered: `event_scoped_operations_match_rails_vectors (local between neighbours)` |
| `test/models/event/recurrence_test.rb:475` — moving an occurrence earlier with this and following never touches previous occurrences | Covered: `event_scoped_operations_match_rails_vectors (earlier following move)` |
| `test/models/event/recurrence_test.rb:491` — this and following can shift an occurrence exactly onto the next active slot | Covered: `event_scoped_operations_match_rails_vectors (following move onto next slot)` |
| `test/models/event/recurrence_test.rb:506` — this and following can shift the head exactly onto the next active slot | Covered: `event_scoped_operations_match_rails_vectors (entire series onto next slot)` |
| `test/models/event/recurrence_test.rb:520` — series slots are unique among uncancelled occurrences | Deferred: WS14e continuation |
| `test/models/event/recurrence_test.rb:539` — a starts-only change preserves later durations and an ends-only change extends them | Covered: `event_scoped_operations_match_rails_vectors (only start and only end)` |
| `test/models/event/recurrence_test.rb:561` — a following time change sends one update per attendee and replaces earlier updates | Deferred: WS14e continuation |
| `test/models/event/recurrence_test.rb:593` — a series update replaces read-but-unhandled updates on other occurrences | Covered: `series_notifications_replace_read_updates_and_rearm_every_reminder` |
| `test/models/event/recurrence_test.rb:624` — a following title-only edit is silent but still copies the title | Covered: `event_scoped_operations_match_rails_vectors (following title)` |
| `test/models/event/recurrence_test.rb:638` — rule changes are rejected away from the first event with this and following | Covered: `event_scoped_operations_match_rails_vectors (follower rule guard)` |
| `test/models/event/recurrence_test.rb:663` — extending the end date reuses matching occurrences and copies responses to new ones | Covered: `event_scoped_operations_match_rails_vectors (extend copies responses)` |
| `test/models/event/recurrence_test.rb:684` — a rule change moves occurrences with distinct responses onto the new pattern's slots | Covered: `event_scoped_operations_match_rails_vectors (protect distinct RSVP)` |
| `test/models/event/recurrence_test.rb:710` — shortening weekly to daily cancels distinct occurrences beyond the new slots | Covered: `event_scoped_operations_match_rails_vectors (cancel excess protected)` |
| `test/models/event/recurrence_test.rb:739` — a declined attendee gets no cancellation item when an excess occurrence is cancelled | Deferred: WS14e continuation |
| `test/models/event/recurrence_test.rb:761` — a rule change with a time change re-times kept occurrences by the same offset | Deferred: WS14e continuation |
| `test/models/event/recurrence_test.rb:783` — shrinking the series moves distinct occurrences onto the remaining slots and leaves cancelled ones | Covered: `event_scoped_operations_match_rails_vectors (shrink keeps cancelled beyond range)` |
| `test/models/event/recurrence_test.rb:806` — shrinking the series destroys regenerable occurrences beyond the new end | Covered: `event_scoped_operations_match_rails_vectors (protect distinct RSVP and cancelled keeps slot)` |
| `test/models/event/recurrence_test.rb:821` — a cancelled occurrence keeps its slot when the series shrinks | Covered: `event_scoped_operations_match_rails_vectors (cancelled keeps slot)` |
| `test/models/event/recurrence_test.rb:847` — rematerialization can move a protected occurrence onto another planned mover's slot | Covered: `event_scoped_operations_match_rails_vectors (monthly parking collision)` |
| `test/models/event/recurrence_test.rb:876` — a failure during rematerialization placement leaves every row with its original series and time | Covered: `rematerialization_failure_restores_original_series_rows` |
| `test/models/event/recurrence_test.rb:915` — series order puts uncancelled occurrences first at equal times | Deferred: WS14e continuation |
| `test/models/event/recurrence_test.rb:932` — a rule change beyond the cap is rejected and leaves the series alone | Deferred: WS14e continuation |
| `test/models/event/recurrence_test.rb:949` — cancelling this event touches only that occurrence | Covered: `event_scoped_operations_match_rails_vectors (cancel local)` |
| `test/models/event/recurrence_test.rb:960` — cancelling this and following sends one item per attendee on the earliest occurrence | Covered: `event_scoped_operations_match_rails_vectors (cancel following)` |
| `test/models/event/recurrence_test.rb:981` — cancelling the series from the first event cancels every occurrence | Covered: `event_scoped_operations_match_rails_vectors (cancel all)` |
| `test/models/event/recurrence_test.rb:996` — cancelling an already cancelled occurrence is a no-op | Covered: `event_scoped_operations_match_rails_vectors (cancel twice)` |
| `test/models/event/recurrence_test.rb:1007` — cancel and update scopes default to this event for unknown values | Covered: `event_scoped_operations_match_rails_vectors (unknown scopes)` |
| `test/models/event/recurrence_test.rb:1018` — a single-occurrence time edit of the head is rejected | Covered: `event_scoped_operations_match_rails_vectors (head time guard)` |
| `test/models/event/recurrence_test.rb:1033` — a head-only series can still be re-timed through this and following | Deferred: WS14e continuation |
| `test/models/event/recurrence_test.rb:1051` — a single-occurrence description edit of the head still succeeds | Deferred: WS14e continuation |
| `test/models/event/recurrence_test.rb:1061` — a plain update of the head start time is rejected | Covered: `scoped_recurrence_guards_do_not_escape_the_operation` |
| `test/models/event/recurrence_test.rb:1073` — a follower save failure clears the scoped flags and rolls the transaction back | Covered: `event_scoped_placement_failure_rolls_back_parking_and_jobs` |

## test/models/event/reference_sync_test.rb (9)

| Rails test | Coverage or deferral |
|---|---|
| `test/models/event/reference_sync_test.rb:9` — a message with an event link gains a reference | Covered: `references_follow_message_edits_and_reject_other_rooms` |
| `test/models/event/reference_sync_test.rb:21` — absolute URLs on any host match | Covered: `references_follow_message_edits_and_reject_other_rooms` |
| `test/models/event/reference_sync_test.rb:31` — duplicate URLs in one message create a single reference | Covered: `references_follow_message_edits_and_reject_other_rooms` |
| `test/models/event/reference_sync_test.rb:41` — a message without an event link references nothing | Deferred: WS14e continuation |
| `test/models/event/reference_sync_test.rb:49` — a link to an event in another room creates nothing | Covered: `references_follow_message_edits_and_reject_other_rooms` |
| `test/models/event/reference_sync_test.rb:64` — a link to a missing event creates nothing | Deferred: WS14e continuation |
| `test/models/event/reference_sync_test.rb:75` — editing a message to add an event link adds the reference | Deferred: WS14e continuation |
| `test/models/event/reference_sync_test.rb:86` — editing a message to remove an event link drops the reference | Covered: `references_follow_message_edits_and_reject_other_rooms` |
| `test/models/event/reference_sync_test.rb:99` — deleting the message removes its references | Deferred: WS14e continuation |

## test/models/event/reminder_dispatcher_test.rb (14)

| Rails test | Coverage or deferral |
|---|---|
| `test/models/event/reminder_dispatcher_test.rb:15` — a due event reminds each going and maybe attendee once and enqueues push | Covered: `reminders_notify_going_and_maybe_once_refreshing_existing_items` |
| `test/models/event/reminder_dispatcher_test.rb:30` — members with notifications off keep their invitation and get no reminder | Covered: `room_involvement_and_inbox_preferences_control_reminder_items` |
| `test/models/event/reminder_dispatcher_test.rb:39` — an attendee with reminders switched off keeps the invitation while others are reminded | Deferred: WS14e continuation |
| `test/models/event/reminder_dispatcher_test.rb:58` — declined attendees keep their invitation and get no reminder | Covered: `reminders_notify_going_and_maybe_once_refreshing_existing_items` |
| `test/models/event/reminder_dispatcher_test.rb:65` — cancelled events are skipped | Covered: `ended_cancelled_and_soft_deleted_rooms_stay_silent (dispatcher only; cancellation API deferred)` |
| `test/models/event/reminder_dispatcher_test.rb:77` — only events starting soon are due | Covered: `due_window_is_inclusive_and_stale_claims_are_silent` |
| `test/models/event/reminder_dispatcher_test.rb:88` — recently started events are still reminded but old ones are not | Covered: `due_window_is_inclusive_and_stale_claims_are_silent` |
| `test/models/event/reminder_dispatcher_test.rb:102` — a second run creates nothing | Covered: `reminders_notify_going_and_maybe_once_refreshing_existing_items` |
| `test/models/event/reminder_dispatcher_test.rb:113` — consecutive occurrences of a series are each reminded once at their own time | Deferred: WS14e continuation |
| `test/models/event/reminder_dispatcher_test.rb:141` — a series starting just before midnight still builds its occurrences | Deferred: WS14e continuation |
| `test/models/event/reminder_dispatcher_test.rb:156` — events in soft-deleted rooms are skipped | Covered: `ended_cancelled_and_soft_deleted_rooms_stay_silent` |
| `test/models/event/reminder_dispatcher_test.rb:170` — a stale event after runner downtime is claimed without reminding or pushing | Covered: `due_window_is_inclusive_and_stale_claims_are_silent` |
| `test/models/event/reminder_dispatcher_test.rb:189` — an ended event is claimed without reminding | Covered: `ended_cancelled_and_soft_deleted_rooms_stay_silent` |
| `test/models/event/reminder_dispatcher_test.rb:205` — one failing event does not stop the others | Covered: `ws14e_one_invalid_event_does_not_stop_other_reminders` |

## test/models/event/reminder_pusher_test.rb (10)

| Rails test | Coverage or deferral |
|---|---|
| `test/models/event/reminder_pusher_test.rb:4` — pushes the reminder to going and maybe attendees who are still members | Deferred: WS17 transport/policy; WS14e integration |
| `test/models/event/reminder_pusher_test.rb:20` — push reminders ignore the event_reminders inbox switch | Deferred: WS17 transport/policy; WS14e integration |
| `test/models/event/reminder_pusher_test.rb:32` — the push body names the venue | Deferred: WS17 transport/policy; WS14e integration |
| `test/models/event/reminder_pusher_test.rb:45` — a direct room reminder is titled by the organizer | Deferred: WS17 transport/policy; WS14e integration |
| `test/models/event/reminder_pusher_test.rb:60` — the push body counts down the actual minutes | Deferred: WS17 transport/policy; WS14e integration |
| `test/models/event/reminder_pusher_test.rb:73` — the push body uses the singular minute | Deferred: WS17 transport/policy; WS14e integration |
| `test/models/event/reminder_pusher_test.rb:86` — an event starting now says so | Deferred: WS17 transport/policy; WS14e integration |
| `test/models/event/reminder_pusher_test.rb:101` — a recently started event still says starting now | Deferred: WS17 transport/policy; WS14e integration |
| `test/models/event/reminder_pusher_test.rb:115` — an event that already ended is skipped | Deferred: WS17 transport/policy; WS14e integration |
| `test/models/event/reminder_pusher_test.rb:126` — an event that started long ago is skipped | Deferred: WS17 transport/policy; WS14e integration |

## test/models/event/venue_test.rb (12)

| Rails test | Coverage or deferral |
|---|---|
| `test/models/event/venue_test.rb:11` — a venue is optional | Deferred: WS14e continuation |
| `test/models/event/venue_test.rb:18` — a voice or Stage channel venue is valid | Deferred: WS14e continuation |
| `test/models/event/venue_test.rb:25` — a text channel or DM venue is rejected | Deferred: WS14e continuation |
| `test/models/event/venue_test.rb:35` — the organizer must belong to the venue | Deferred: WS14e continuation |
| `test/models/event/venue_test.rb:44` — an event in a voice channel may use its own room as the venue | Deferred: WS14e continuation |
| `test/models/event/venue_test.rb:51` — other edits stay valid after the organizer leaves the venue | Deferred: WS14e continuation |
| `test/models/event/venue_test.rb:60` — deleting the venue clears the link but keeps the event | Deferred: WS14e continuation |
| `test/models/event/venue_test.rb:69` — scheduling a series copies the venue to every occurrence | Deferred: WS14e continuation |
| `test/models/event/venue_test.rb:79` — this and following propagates a venue change | Deferred: WS14e continuation |
| `test/models/event/venue_test.rb:90` — this and following propagates clearing the venue | Deferred: WS14e continuation |
| `test/models/event/venue_test.rb:101` — a single-occurrence edit changes the venue for that occurrence only | Deferred: WS14e continuation |
| `test/models/event/venue_test.rb:112` — a venue-only edit creates no inbox items | Deferred: WS14e continuation |

## test/controllers/rooms/events_controller_test.rb (55)

| Rails test | Coverage or deferral |
|---|---|
| `test/controllers/rooms/events_controller_test.rb:10` — index lists upcoming, past, and cancelled events separately | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:24` — show renders for members and 404s for non-members | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:37` — a member can create an event and members are invited | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:51` — a member can create a repeating event with one invitation per member | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:74` — create rejects a series above the occurrence cap | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:88` — index shows a series once with its repeat label and remaining count | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:109` — index lists past occurrences individually | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:126` — show renders the series banner with previous and next occurrence links | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:150` — edit offers a scope on series occurrences and the rule only on the first event | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:172` — updating this and following shifts later occurrences and notifies once per attendee | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:196` — updating without a scope leaves the rest of the series untouched | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:218` — changing the rule away from the first event is rejected | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:233` — non-organizers cannot edit or cancel series occurrences | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:253` — an administrator who is not the organizer can use this and following, but an ordinary member cannot | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:288` — cancelling this and following cancels later occurrences with one item per attendee | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:307` — cancelling without a scope cancels only that occurrence | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:322` — cancelling this event explicitly cancels only that occurrence | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:337` — show renders cancel scopes for series occurrences and a single cancel for single events | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:356` — index issues a bounded number of queries regardless of occurrence count | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:393` — create renders errors for invalid events | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:401` — bots are denied | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:419` — requires authentication | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:427` — only the organizer or an administrator can edit | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:443` — the organizer can update times and attendees are notified | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:457` — saving the edit form from another time zone does not move the event | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:482` — show prints the scheduled zone next to the localized time | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:489` — cancelled events cannot be edited | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:500` — non-organizers cannot cancel | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:509` — the organizer can cancel and cancelling twice is a no-op | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:521` — show notes the Google Calendar copy when an entry exists for the viewer | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:533` — show hides another member's Google Calendar copy | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:541` — a member can create an event with a venue | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:553` — the organizer can set and clear the venue | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:571` — create rejects a text channel venue | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:582` — create rejects a venue the organizer does not belong to | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:595` — update rejects a venue the organizer does not belong to | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:609` — the edit form keeps a venue the editor cannot see so an unrelated edit does not clear it | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:627` — the new form does not list another member's venue | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:636` — the form lists only the member's voice and Stage channels, grouped by kind | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:659` — show links the venue with a Join button for venue members | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:672` — show names the venue without a link for non-members | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:686` — index rows show the venue | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:697` — show renders the live dot for a stage venue with a live stream | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:711` — show renders no live pip for a stage venue with an ended stream | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:725` — show hides the live dot from members who do not belong to the stage venue | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:745` — show never renders a live dot for a voice venue | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:755` — index rows render the live dot for a live stage venue | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:768` — index rows render no live pip for a stage venue with an ended stream | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:782` — index rows never render a live dot for a voice venue | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:792` — index issues the same queries regardless of event count when venues are shared | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:826` — show and index omit the Where line without a venue | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:838` — creating with a Meet request stores the flag | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:846` — show renders the Meet link when present | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:856` — show renders no Meet row without a link | Deferred: WS14e continuation |
| `test/controllers/rooms/events_controller_test.rb:863` — show renders no Meet row for a non-https link | Deferred: WS14e continuation |

## test/controllers/rooms/events/attendances_controller_test.rb (16)

| Rails test | Coverage or deferral |
|---|---|
| `test/controllers/rooms/events/attendances_controller_test.rb:10` — a member can respond and change their response | Deferred: WS14e continuation |
| `test/controllers/rooms/events/attendances_controller_test.rb:23` — non-members get a 404 | Deferred: WS14e continuation |
| `test/controllers/rooms/events/attendances_controller_test.rb:32` — bots are denied | Deferred: WS14e continuation |
| `test/controllers/rooms/events/attendances_controller_test.rb:40` — cancelled events reject responses | Deferred: WS14e continuation |
| `test/controllers/rooms/events/attendances_controller_test.rb:49` — a response on the first event of a series is copied to every future occurrence | Deferred: WS14e continuation |
| `test/controllers/rooms/events/attendances_controller_test.rb:64` — a later response stays local unless apply to all future is checked | Deferred: WS14e continuation |
| `test/controllers/rooms/events/attendances_controller_test.rb:87` — show offers apply to all future on later occurrences with a successor | Deferred: WS14e continuation |
| `test/controllers/rooms/events/attendances_controller_test.rb:111` — unknown responses are rejected | Deferred: WS14e continuation |
| `test/controllers/rooms/events/attendances_controller_test.rb:118` — show renders the attendance frame with the response controls for members | Deferred: WS14e continuation |
| `test/controllers/rooms/events/attendances_controller_test.rb:136` — show 404s for non-members | Deferred: WS14e continuation |
| `test/controllers/rooms/events/attendances_controller_test.rb:149` — show offers apply to all future occurrences on the series head and hides it on the last occurrence | Deferred: WS14e continuation |
| `test/controllers/rooms/events/attendances_controller_test.rb:172` — show reports the closed state for cancelled events | Deferred: WS14e continuation |
| `test/controllers/rooms/events/attendances_controller_test.rb:187` — responding from the frame re-renders the frame instead of redirecting | Deferred: WS14e continuation |
| `test/controllers/rooms/events/attendances_controller_test.rb:206` — a frame response from a non-member 404s | Deferred: WS14e continuation |
| `test/controllers/rooms/events/attendances_controller_test.rb:222` — a frame response with an unknown choice re-renders the frame with an alert | Deferred: WS14e continuation |
| `test/controllers/rooms/events/attendances_controller_test.rb:238` — a frame response to a cancelled event re-renders the frame with an alert | Deferred: WS14e continuation |

## test/system/events_test.rb (3)

| Rails test | Coverage or deferral |
|---|---|
| `test/system/events_test.rb:4` — scheduling an event invites members, who respond and see it in the inbox | Deferred: WS14e continuation |
| `test/system/events_test.rb:48` — scheduling a repeating event invites once per member and copies the first response | Deferred: WS14e continuation |
| `test/system/events_test.rb:102` — scheduling an event announces it in the room with a card members respond from | Deferred: WS14e continuation |

## test/integration/event_cards_test.rb (8)

| Rails test | Coverage or deferral |
|---|---|
| `test/integration/event_cards_test.rb:9` — a room member sees the card with title, time, venue, and organizer | Deferred: WS14e continuation |
| `test/integration/event_cards_test.rb:39` — the card shows no Join button and no live dot | Deferred: WS14e continuation |
| `test/integration/event_cards_test.rb:60` — a repeating event shows the repeating eyebrow | Deferred: WS14e continuation |
| `test/integration/event_cards_test.rb:77` — a cancelled event shows the cancelled state | Deferred: WS14e continuation |
| `test/integration/event_cards_test.rb:96` — the scheduling announcement renders its card in the room | Deferred: WS14e continuation |
| `test/integration/event_cards_test.rb:108` — a link to an event in another room stays a plain link with no card for anyone | Deferred: WS14e continuation |
| `test/integration/event_cards_test.rb:142` — a message without an event link renders no card | Deferred: WS14e continuation |
| `test/integration/event_cards_test.rb:153` — rendering a room page costs no extra queries per message with an event link | Deferred: WS14e continuation |

## Counts

- `test/models/event_test.rb`: 18
- `test/models/event_attendance_test.rb`: 3
- `test/models/event_calendar_entry_test.rb`: 3
- `test/models/event/channel_timeline_test.rb`: 7
- `test/models/event/recurrence_test.rb`: 63
- `test/models/event/reference_sync_test.rb`: 9
- `test/models/event/reminder_dispatcher_test.rb`: 14
- `test/models/event/reminder_pusher_test.rb`: 10
- `test/models/event/venue_test.rb`: 12
- `test/controllers/rooms/events_controller_test.rb`: 55
- `test/controllers/rooms/events/attendances_controller_test.rb`: 16
- `test/system/events_test.rb`: 3
- `test/integration/event_cards_test.rb`: 8

Total named Rails tests inventoried: 221. Includes the Calendar entry model (WS14g) and event-card integration file beyond the brief's controller/model/system split.
