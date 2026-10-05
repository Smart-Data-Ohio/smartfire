# Rails assertion to Rust assertion map

Reference: `78b9b1546bdab4c6c1c9b8ddb94512f661289112`. Each row names an original Rails assertion call, its discriminating Rust assertion and the real test that executes it. Shared helper assertions and repeated loop cases are cited explicitly. These tables retain compiler checks as compiler checks; they do not claim matching exception classes between Ruby and the Rust type system.

## WS14e-015

Rails declaration: `test/models/event_attendance_test.rb:8` — a member holds a single response per event

Executed test: `campfire_db tests::calendar_event_test::cutover_d_events_test::cutover_d_attendance_member_holds_one_response_and_duplicate_creation_is_invalid`.

Exact original Rails fixture state, model mutations and assertion calls. Counts and activity rows use the original member/event/series scope; job assertions compare concrete class and argument identities.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event_attendance_test.rb:11](../../test/models/event_attendance_test.rb#L11)<br>`assert_equal "maybe", attendance.response` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:62](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L62)<br>`assert_eq!(attendance.response, "maybe")` |
| [test/models/event_attendance_test.rb:13](../../test/models/event_attendance_test.rb#L13)<br>`assert_equal "going", attendance.reload.response` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:65](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L65)<br>`assert_eq!( t.read(\|c\| EventAttendance::find_for(c, id("launch_party"), id("jason"))) .unwrap() .response, "going" )` |
| [test/models/event_attendance_test.rb:15](../../test/models/event_attendance_test.rb#L15)<br>`assert_raises ActiveRecord::RecordInvalid do` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:72](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L72)<br>`assert!(matches!( t.try_write(\|tx\| EventAttendance::create(tx, id("launch_party"), id("jason"), "maybe")), Err(Error::RecordInvalid(_)) ))` |

## WS14e-023

Rails declaration: `test/models/event/channel_timeline_test.rb:63` — updating an event broadcasts a card replace for each referencing message

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::d_cards::cutover_d_timeline_singleton_update_replaces_exact_two_same_room_references_and_no_cross_room`.

Exact original Rails setup through persisted model writes and signed-in room HTTP or real subscribed Cable publications. DOM assertions use parsed markup; selectors and counts retain the original scope. Broadcast checks execute the real post-commit presenter and Cable publisher.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/channel_timeline_test.rb:76](../../test/models/event/channel_timeline_test.rb#L76)<br>`assert_equal [ event ], message.events` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs:171](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs#L171)<br>`assert_eq!(event_ids(&app, same.id).await, vec![e.id])` |
| [test/models/event/channel_timeline_test.rb:79](../../test/models/event/channel_timeline_test.rb#L79)<br>`assert_empty other_message.events` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs:173](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs#L173)<br>`assert!(event_ids(&app, other.id).await.is_empty())` |
| [test/models/event/channel_timeline_test.rb:86](../../test/models/event/channel_timeline_test.rb#L86)<br>`assert_equal 2, room_count` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs:176](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs#L176)<br>`assert_eq!(refs.len(), 2)` |
| [test/models/event/channel_timeline_test.rb:88](../../test/models/event/channel_timeline_test.rb#L88)<br>`assert_broadcasts room_stream, room_count do` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs:193](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs#L193)<br>`assert_eq!( published .iter() .filter(\|(stream, _)\| stream == &stream_name(id("designers"))) .count(), refs.len() )` |
| [test/models/event/channel_timeline_test.rb:89](../../test/models/event/channel_timeline_test.rb#L89)<br>`assert_broadcasts other_stream, 0 do` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs:201](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs#L201)<br>`assert_eq!( published .iter() .filter(\|(stream, _)\| stream == &stream_name(id("watercooler"))) .count(), 0 )` |

## WS14e-024

Rails declaration: `test/models/event/channel_timeline_test.rb:95` — cancelling an event broadcasts card updates

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::d_cards::cutover_d_timeline_singleton_cancel_replaces_both_announcement_and_separate_link`.

Exact original Rails setup through persisted model writes and signed-in room HTTP or real subscribed Cable publications. DOM assertions use parsed markup; selectors and counts retain the original scope. Broadcast checks execute the real post-commit presenter and Cable publisher.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/channel_timeline_test.rb:102](../../test/models/event/channel_timeline_test.rb#L102)<br>`assert_equal [ event ], message.events` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs:247](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs#L247)<br>`assert_eq!(event_ids(&app, same.id).await, vec![e.id])` |
| [test/models/event/channel_timeline_test.rb:104](../../test/models/event/channel_timeline_test.rb#L104)<br>`assert_broadcasts room_messages_stream_name(@room),` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs:259](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs#L259)<br>`assert_eq!(published.len(), refs.len())`<br><br>[rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs:262](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs#L262)<br>`assert_eq!(stream, &stream_name(id("designers")))` |
| [test/models/event/channel_timeline_test.rb:106](../../test/models/event/channel_timeline_test.rb#L106)<br>`assert event.cancel!(actor: @organizer)` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs:256](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs#L256)<br>`assert!(cancelled)` |

## WS14e-029

Rails declaration: `test/models/event/recurrence_test.rb:299` — copying a response overwrites distinct later responses but skips cancelled occurrences

Executed test: `campfire_db tests::calendar_event_test::cutover_d_events_test::cutover_d_recurrence_response_copy_skips_api_cancelled_occurrence`.

Exact original Rails fixture state, model mutations and assertion calls. Counts and activity rows use the original member/event/series scope; job assertions compare concrete class and argument identities.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/recurrence_test.rb:308](../../test/models/event/recurrence_test.rb#L308)<br>`assert_equal "maybe", occurrences.first.response_for(users(:jason))` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:96](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L96)<br>`assert_eq!( t.read(\|c\| occurrences[0].response_for(c, Some(id("jason")))), Some("maybe".into()) )` |
| [test/models/event/recurrence_test.rb:309](../../test/models/event/recurrence_test.rb#L309)<br>`assert_equal "maybe", occurrences.second.response_for(users(:jason))` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:101](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L101)<br>`assert_eq!( t.read(\|c\| occurrences[1].response_for(c, Some(id("jason")))), Some("maybe".into()) )` |
| [test/models/event/recurrence_test.rb:310](../../test/models/event/recurrence_test.rb#L310)<br>`assert_equal "going", occurrences.third.response_for(users(:jason))` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:106](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L106)<br>`assert_eq!( t.read(\|c\| occurrences[2].response_for(c, Some(id("jason")))), Some("going".into()) )` |

## WS14e-031

Rails declaration: `test/models/event/recurrence_test.rb:416` — moving an occurrence before its previous sibling is rejected in either scope

Executed test: `campfire_db tests::calendar_event_test::cutover_d_events_test::cutover_d_recurrence_strictly_before_previous_rejects_both_scopes_and_preserves_series`.

Exact original Rails fixture state, model mutations and assertion calls. Counts and activity rows use the original member/event/series scope; job assertions compare concrete class and argument identities.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/recurrence_test.rb:424](../../test/models/event/recurrence_test.rb#L424)<br>`error = assert_raises(ActiveRecord::RecordInvalid) do` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:133](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L133)<br>`assert!(matches!(&result, Err(Error::RecordInvalid(_))))` |
| [test/models/event/recurrence_test.rb:430](../../test/models/event/recurrence_test.rb#L430)<br>`assert_equal [ "must stay between the neighbouring occurrences in its series" ], error.record.errors[:starts_at]` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:138](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L138)<br>`assert_eq!( errors.on("starts_at"), vec!["must stay between the neighbouring occurrences in its series"] )` |
| [test/models/event/recurrence_test.rb:433](../../test/models/event/recurrence_test.rb#L433)<br>`assert_equal before, head.reload.series_events.map(&:starts_at)` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:144](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L144)<br>`assert_eq!( rows(&t, h.id) .iter() .map(\|e\| e.starts_at) .collect::<Vec<_>>(), before.iter().map(\|e\| e.starts_at).collect::<Vec<_>>() )` |

## WS14e-032

Rails declaration: `test/models/event/recurrence_test.rb:436` — a single-occurrence edit past the next sibling is rejected but this and following allows it

Executed test: `campfire_db tests::calendar_event_test::cutover_d_events_test::cutover_d_recurrence_strictly_past_next_rejects_local_and_shifts_complete_following_series`.

Exact original Rails fixture state, model mutations and assertion calls. Counts and activity rows use the original member/event/series scope; job assertions compare concrete class and argument identities.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/recurrence_test.rb:442](../../test/models/event/recurrence_test.rb#L442)<br>`error = assert_raises(ActiveRecord::RecordInvalid) do` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:133](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L133)<br>`assert!(matches!(&result, Err(Error::RecordInvalid(_))))` |
| [test/models/event/recurrence_test.rb:448](../../test/models/event/recurrence_test.rb#L448)<br>`assert_equal [ "must stay between the neighbouring occurrences in its series" ], error.record.errors[:starts_at]` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:173](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L173)<br>`assert_eq!( errors.on("starts_at"), vec!["must stay between the neighbouring occurrences in its series"] )` |
| [test/models/event/recurrence_test.rb:449](../../test/models/event/recurrence_test.rb#L449)<br>`assert_equal utc(2027, 1, 8, 9, 0), occurrence.reload.starts_at` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:178](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L178)<br>`assert_eq!( t.read(\|c\| CalendarEvent::find(c, eid)).starts_at, stamp("2027-01-08 09:00:00") )` |
| [test/models/event/recurrence_test.rb:456](../../test/models/event/recurrence_test.rb#L456)<br>`assert_equal [ utc(2027, 1, 1, 9, 0), utc(2027, 1, 16, 9, 0), utc(2027, 1, 23, 9, 0), utc(2027, 1, 30, 9, 0) ],` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:184](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L184)<br>`assert_eq!( rows(&t, h.id) .iter() .map(\|e\| e.starts_at) .collect::<Vec<_>>(), [ "2027-01-01 09:00:00", "2027-01-16 09:00:00", "2027-01-23 09:00:00", "2027-01-30 09:00:00" ] .map(stamp) )` |

## WS14e-034

Rails declaration: `test/models/event/recurrence_test.rb:561` — a following time change sends one update per attendee and replaces earlier updates

Executed test: `campfire_db tests::calendar_event_test::cutover_d_events_test::cutover_d_recurrence_following_time_change_replaces_going_and_maybe_updates`.

Exact original Rails fixture state, model mutations and assertion calls. Counts and activity rows use the original member/event/series scope; job assertions compare concrete class and argument identities.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/recurrence_test.rb:573](../../test/models/event/recurrence_test.rb#L573)<br>`assert_equal "event_update", ActivityItem.find_by!(user: users(:jz), source: occurrences.second).event_type` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:226](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L226)<br>`assert_eq!(earlier.event_type, "event_update")` |
| [test/models/event/recurrence_test.rb:585](../../test/models/event/recurrence_test.rb#L585)<br>`assert_equal 1, items.count` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:241](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L241)<br>`assert_eq!(unread.len(), 1)` |
| [test/models/event/recurrence_test.rb:586](../../test/models/event/recurrence_test.rb#L586)<br>`assert_equal "event_update", items.first.event_type` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:243](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L243)<br>`assert_eq!(unread[0].0, "event_update")` |
| [test/models/event/recurrence_test.rb:587](../../test/models/event/recurrence_test.rb#L587)<br>`assert_equal occurrences.first.id, items.first.source_id` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:245](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L245)<br>`assert_eq!(unread[0].1, h.id)` |
| [test/models/event/recurrence_test.rb:589](../../test/models/event/recurrence_test.rb#L589)<br>`assert_predicate ActivityItem.find_by!(user: users(:jz), source: occurrences.second), :handled?` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:249](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L249)<br>`assert!(handled.handled_at.is_some())` |
| [test/models/event/recurrence_test.rb:590](../../test/models/event/recurrence_test.rb#L590)<br>`assert_not ActivityItem.exists?(user: @organizer, source: occurrences)` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:251](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L251)<br>`assert_eq!(t.read(\|c\|Ok(c.query_row("SELECT COUNT(*) FROM activity_items WHERE user_id=? AND source_type='Event' AND source_id IN (SELECT value FROM json_each(?))",params![id("david"),json!(sources).to_string()],\|r\|r.get::<_,i64>(0))?)),0)` |

## WS14e-035

Rails declaration: `test/models/event/recurrence_test.rb:739` — a declined attendee gets no cancellation item when an excess occurrence is cancelled

Executed test: `campfire_db tests::calendar_event_test::cutover_d_events_test::cutover_d_recurrence_shortening_notifies_going_but_not_declined_on_same_excess_event`.

Exact original Rails fixture state, model mutations and assertion calls. Counts and activity rows use the original member/event/series scope; job assertions compare concrete class and argument identities.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/recurrence_test.rb:744](../../test/models/event/recurrence_test.rb#L744)<br>`assert_equal 4, occurrences.size` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:260](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L260)<br>`assert_eq!(occurrences.len(), 4)` |
| [test/models/event/recurrence_test.rb:756](../../test/models/event/recurrence_test.rb#L756)<br>`assert_predicate excess, :cancelled?` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:277](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L277)<br>`assert!(excess.cancelled())` |
| [test/models/event/recurrence_test.rb:757](../../test/models/event/recurrence_test.rb#L757)<br>`assert_equal "event_cancelled", ActivityItem.find_by!(user: users(:jz), source: excess).event_type` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:279](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L279)<br>`assert_eq!( item(&t, excess.id, "jz").unwrap().event_type, "event_cancelled" )` |
| [test/models/event/recurrence_test.rb:758](../../test/models/event/recurrence_test.rb#L758)<br>`assert_not ActivityItem.exists?(user: users(:kevin), source: excess, event_type: "event_cancelled")` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:284](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L284)<br>`assert_eq!(t.read(\|c\|Ok(c.query_row("SELECT COUNT(*) FROM activity_items WHERE user_id=? AND source_type='Event' AND source_id=? AND event_type='event_cancelled'",params![id("kevin"),excess.id],\|r\|r.get::<_,i64>(0))?)),0)` |

## WS14e-036

Rails declaration: `test/models/event/recurrence_test.rb:761` — a rule change with a time change re-times kept occurrences by the same offset

Executed test: `campfire_db tests::calendar_event_test::cutover_d_events_test::cutover_d_recurrence_compound_time_and_until_update_moves_distinct_kept_occurrence`.

Exact original Rails fixture state, model mutations and assertion calls. Counts and activity rows use the original member/event/series scope; job assertions compare concrete class and argument identities.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/recurrence_test.rb:779](../../test/models/event/recurrence_test.rb#L779)<br>`assert_equal utc(2026, 10, 12, 11, 0), kept.starts_at` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:307](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L307)<br>`assert_eq!(kept.starts_at, stamp("2026-10-12 11:00:00"))` |
| [test/models/event/recurrence_test.rb:780](../../test/models/event/recurrence_test.rb#L780)<br>`assert_equal "declined", kept.response_for(users(:jason))` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:309](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L309)<br>`assert_eq!( t.read(\|c\| kept.response_for(c, Some(id("jason")))), Some("declined".into()) )` |

## WS14e-045

Rails declaration: `test/models/event/reminder_dispatcher_test.rb:39` — an attendee with reminders switched off keeps the invitation while others are reminded

Executed test: `campfire_db tests::calendar_event_test::cutover_d_events_test::cutover_d_reminder_preference_optout_preserves_invitation_updates_and_followup`.

Exact original Rails fixture state, model mutations and assertion calls. Counts and activity rows use the original member/event/series scope; job assertions compare concrete class and argument identities.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/reminder_dispatcher_test.rb:42](../../test/models/event/reminder_dispatcher_test.rb#L42)<br>`assert_enqueued_with(job: Event::ReminderPushJob, args: [ @event ]) do` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:329](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L329)<br>`assert_eq!( jobs(&t), vec![("Event::ReminderPushJob".into(), json!({"event_id":e.id}))] )` |
| [test/models/event/reminder_dispatcher_test.rb:46](../../test/models/event/reminder_dispatcher_test.rb#L46)<br>`assert_equal "event_invitation", ActivityItem.find_by!(user: users(:jason), source: @event).event_type` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:334](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L334)<br>`assert_eq!( item(&t, e.id, "jason").unwrap().event_type, "event_invitation" )` |
| [test/models/event/reminder_dispatcher_test.rb:47](../../test/models/event/reminder_dispatcher_test.rb#L47)<br>`assert_equal "event_reminder", ActivityItem.find_by!(user: users(:jz), source: @event).event_type` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:339](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L339)<br>`assert_eq!(item(&t, e.id, "jz").unwrap().event_type, "event_reminder")` |
| [test/models/event/reminder_dispatcher_test.rb:50](../../test/models/event/reminder_dispatcher_test.rb#L50)<br>`assert_equal "event_update", ActivityItem.find_by!(user: users(:jason), source: @event).event_type` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:350](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L350)<br>`assert_eq!(item(&t, e.id, "jason").unwrap().event_type, "event_update")` |
| [test/models/event/reminder_dispatcher_test.rb:55](../../test/models/event/reminder_dispatcher_test.rb#L55)<br>`assert_equal "event_invitation", ActivityItem.find_by!(user: users(:jason), source: follow_up).event_type` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:356](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L356)<br>`assert_eq!( item(&t, followup.id, "jason").unwrap().event_type, "event_invitation" )` |

## WS14e-046

Rails declaration: `test/models/event/reminder_dispatcher_test.rb:65` — cancelled events are skipped

Executed test: `campfire_db tests::calendar_event_test::cutover_d_events_test::cutover_d_reminder_api_cancelled_due_event_is_skipped_without_changing_cancelled_item`.

Exact original Rails fixture state, model mutations and assertion calls. Counts and activity rows use the original member/event/series scope; job assertions compare concrete class and argument identities.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/reminder_dispatcher_test.rb:69](../../test/models/event/reminder_dispatcher_test.rb#L69)<br>`assert_no_enqueued_jobs only: Event::ReminderPushJob do` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:370](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L370)<br>`assert!( jobs(&t) .iter() .all(\|(class, _)\| class != "Event::ReminderPushJob") )` |
| [test/models/event/reminder_dispatcher_test.rb:73](../../test/models/event/reminder_dispatcher_test.rb#L73)<br>`assert_nil @event.reload.reminded_at` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:376](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L376)<br>`assert!( t.read(\|c\| CalendarEvent::find(c, e.id)) .reminded_at .is_none() )` |
| [test/models/event/reminder_dispatcher_test.rb:74](../../test/models/event/reminder_dispatcher_test.rb#L74)<br>`assert_equal "event_cancelled", ActivityItem.find_by!(user: users(:jason), source: @event).event_type` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:382](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L382)<br>`assert_eq!( item(&t, e.id, "jason").unwrap().event_type, "event_cancelled" )` |

## WS14e-102

Rails declaration: `test/integration/event_cards_test.rb:9` — a room member sees the card with title, time, venue, and organizer

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::d_cards::cutover_d_cards_room_http_scopes_single_card_title_time_venue_organizer_and_attendance_frame`.

Exact original Rails setup through persisted model writes and signed-in room HTTP or real subscribed Cable publications. DOM assertions use parsed markup; selectors and counts retain the original scope. Broadcast checks execute the real post-commit presenter and Cable publisher.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/integration/event_cards_test.rb:23](../../test/integration/event_cards_test.rb#L23)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs:296](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs#L296)<br>`assert_eq!(reply.status, StatusCode::OK)` |
| [test/integration/event_cards_test.rb:26](../../test/integration/event_cards_test.rb#L26)<br>`assert_select "##{ActionView::RecordIdentifier.dom_id(message, :event_cards)}" do` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs:306](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs#L306)<br>`assert_eq!(containers.len(), 1)` |
| [test/integration/event_cards_test.rb:27](../../test/integration/event_cards_test.rb#L27)<br>`assert_select ".event-card", count: 1` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs:309](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs#L309)<br>`assert_eq!(by_class(&dom, container, "event-card").len(), 1)` |
| [test/integration/event_cards_test.rb:28](../../test/integration/event_cards_test.rb#L28)<br>`assert_select ".event-card__eyebrow", text: "Event"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs:311](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs#L311)<br>`assert_eq!( by_class(&dom, container, "event-card__eyebrow") .iter() .map(\|&n\| text(&dom, n)) .collect::<Vec<_>>(), vec!["Event"] )` |
| [test/integration/event_cards_test.rb:29](../../test/integration/event_cards_test.rb#L29)<br>`assert_select ".event-card__title", text: "Planning session"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs:319](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs#L319)<br>`assert_eq!( by_class(&dom, container, "event-card__title") .iter() .map(\|&n\| text(&dom, n)) .collect::<Vec<_>>(), vec!["Planning session"] )` |
| [test/integration/event_cards_test.rb:30](../../test/integration/event_cards_test.rb#L30)<br>`assert_select ".event-card__title a[href=?]", room_event_path(@room, event)` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs:328](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs#L328)<br>`assert_eq!( dom.descendants(title) .into_iter() .filter(\|&n\| dom.local_name(n) == Some("a") && dom.attr(n, "href") == Some(path(e.id).as_str())) .count(), 1 )` |
| [test/integration/event_cards_test.rb:31](../../test/integration/event_cards_test.rb#L31)<br>`assert_select ".event-card__meta time", count: 2` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs:342](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs#L342)<br>`assert_eq!(times.len(), 2)` |
| [test/integration/event_cards_test.rb:32](../../test/integration/event_cards_test.rb#L32)<br>`assert_select ".event-card__venue", text: /Lounge/` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs:344](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs#L344)<br>`assert!( by_class(&dom, container, "event-card__venue") .iter() .any(\|&n\| text(&dom, n).contains("Lounge")) )` |
| [test/integration/event_cards_test.rb:33](../../test/integration/event_cards_test.rb#L33)<br>`assert_select ".event-card__organizer", text: /David/` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs:350](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs#L350)<br>`assert!( by_class(&dom, container, "event-card__organizer") .iter() .any(\|&n\| text(&dom, n).contains("David")) )` |
| [test/integration/event_cards_test.rb:34](../../test/integration/event_cards_test.rb#L34)<br>`assert_select "turbo-frame[src=?]",` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs:357](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs#L357)<br>`assert_eq!( dom.descendants(container) .into_iter() .filter(\|&n\| dom.local_name(n) == Some("turbo-frame") && dom.attr(n, "src") == Some(src.as_str())) .count(), 1 )` |

## WS14e-103

Rails declaration: `test/integration/event_cards_test.rb:39` — the card shows no Join button and no live dot

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::d_cards::cutover_d_cards_room_http_voice_venue_has_no_join_or_live_dot`.

Exact original Rails setup through persisted model writes and signed-in room HTTP or real subscribed Cable publications. DOM assertions use parsed markup; selectors and counts retain the original scope. Broadcast checks execute the real post-commit presenter and Cable publisher.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/integration/event_cards_test.rb:53](../../test/integration/event_cards_test.rb#L53)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs:376](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs#L376)<br>`assert_eq!(reply.status, StatusCode::OK)` |
| [test/integration/event_cards_test.rb:54](../../test/integration/event_cards_test.rb#L54)<br>`assert_select ".event-card", minimum: 1` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs:381](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs#L381)<br>`assert!(!cards.is_empty())` |
| [test/integration/event_cards_test.rb:55](../../test/integration/event_cards_test.rb#L55)<br>`assert_select ".event-card", text: /Lounge/` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs:383](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs#L383)<br>`assert!(cards.iter().any(\|&n\| text(&dom, n).contains("Lounge")))` |
| [test/integration/event_cards_test.rb:56](../../test/integration/event_cards_test.rb#L56)<br>`assert_select ".event-card a", text: "Join", count: 0` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs:385](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs#L385)<br>`assert_eq!( cards .iter() .flat_map(\|&n\| dom.descendants(n)) .filter(\|&n\| dom.local_name(n) == Some("a") && text(&dom, n) == "Join") .count(), 0 )` |
| [test/integration/event_cards_test.rb:57](../../test/integration/event_cards_test.rb#L57)<br>`assert_select ".event-card .sidebar-item__icon", count: 0` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs:394](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs#L394)<br>`assert_eq!( cards .iter() .map(\|&n\| by_class(&dom, n, "sidebar-item__icon").len()) .sum::<usize>(), 0 )` |

## WS14e-104

Rails declaration: `test/integration/event_cards_test.rb:60` — a repeating event shows the repeating eyebrow

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::d_cards::cutover_d_cards_room_http_real_recurring_event_has_repeating_eyebrow`.

Exact original Rails setup through persisted model writes and signed-in room HTTP or real subscribed Cable publications. DOM assertions use parsed markup; selectors and counts retain the original scope. Broadcast checks execute the real post-commit presenter and Cable publisher.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/integration/event_cards_test.rb:73](../../test/integration/event_cards_test.rb#L73)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs:411](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs#L411)<br>`assert_eq!(reply.status, StatusCode::OK)` |
| [test/integration/event_cards_test.rb:74](../../test/integration/event_cards_test.rb#L74)<br>`assert_select ".event-card__eyebrow", text: "Repeating event"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs:415](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs#L415)<br>`assert!( by_class(&dom, root, "event-card__eyebrow") .iter() .any(\|&n\| text(&dom, n) == "Repeating event") )` |

## WS14e-105

Rails declaration: `test/integration/event_cards_test.rb:77` — a cancelled event shows the cancelled state

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::d_cards::cutover_d_cards_room_http_model_cancelled_event_renders_cancelled_card_and_state`.

Exact original Rails setup through persisted model writes and signed-in room HTTP or real subscribed Cable publications. DOM assertions use parsed markup; selectors and counts retain the original scope. Broadcast checks execute the real post-commit presenter and Cable publisher.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/integration/event_cards_test.rb:91](../../test/integration/event_cards_test.rb#L91)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs:442](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs#L442)<br>`assert_eq!(reply.status, StatusCode::OK)` |
| [test/integration/event_cards_test.rb:92](../../test/integration/event_cards_test.rb#L92)<br>`assert_select ".event-card--cancelled", minimum: 1` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs:446](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs#L446)<br>`assert!(!by_class(&dom, root, "event-card--cancelled").is_empty())` |
| [test/integration/event_cards_test.rb:93](../../test/integration/event_cards_test.rb#L93)<br>`assert_select ".event-card__state", text: "Cancelled"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs:448](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/d_cards.rs#L448)<br>`assert!( by_class(&dom, root, "event-card__state") .iter() .any(\|&n\| text(&dom, n) == "Cancelled") )` |

## WS14g-075

Rails declaration: `test/jobs/calendar/sync_entry_job_test.rb:466` — creating an attendance enqueues a sync

Executed test: `campfire_db tests::calendar_event_test::cutover_d_events_test::cutover_d_calendar_direct_attendance_creation_enqueues_exact_event_and_kevin`.

Exact original Rails fixture state, model mutations and assertion calls. Counts and activity rows use the original member/event/series scope; job assertions compare concrete class and argument identities.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/jobs/calendar/sync_entry_job_test.rb:467](../../test/jobs/calendar/sync_entry_job_test.rb#L467)<br>`assert_enqueued_with(job: Calendar::SyncEntryJob, args: [ @event.id, users(:kevin).id ]) do` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:394](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L394)<br>`assert_eq!( jobs(&t), vec![( "Calendar::SyncEntryJob".into(), json!({"event_id":id("launch_party"),"user_id":id("kevin")}) )] )` |

## WS14g-076

Rails declaration: `test/jobs/calendar/sync_entry_job_test.rb:472` — changing a response enqueues a sync but other saves do not

Executed test: `campfire_db tests::calendar_event_test::cutover_d_events_test::cutover_d_calendar_direct_response_update_enqueues_but_timestamp_only_save_does_not`.

Exact original Rails fixture state, model mutations and assertion calls. Counts and activity rows use the original member/event/series scope; job assertions compare concrete class and argument identities.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/jobs/calendar/sync_entry_job_test.rb:475](../../test/jobs/calendar/sync_entry_job_test.rb#L475)<br>`assert_enqueued_with(job: Calendar::SyncEntryJob, args: [ @event.id, @jason.id ]) do` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:412](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L412)<br>`assert_eq!( jobs(&t), vec![( "Calendar::SyncEntryJob".into(), json!({"event_id":id("launch_party"),"user_id":id("jason")}) )] )` |
| [test/jobs/calendar/sync_entry_job_test.rb:479](../../test/jobs/calendar/sync_entry_job_test.rb#L479)<br>`assert_no_enqueued_jobs do` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:429](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L429)<br>`assert!(jobs(&t).is_empty())` |

## WS14g-081

Rails declaration: `test/jobs/calendar/sync_entry_job_test.rb:533` — cancelling enqueues a sync for every entry

Executed test: `campfire_db tests::calendar_event_test::cutover_d_events_test::cutover_d_calendar_singleton_cancel_enqueues_each_of_two_distinct_users_entries`.

Exact original Rails fixture state, model mutations and assertion calls. Counts and activity rows use the original member/event/series scope; job assertions compare concrete class and argument identities.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/jobs/calendar/sync_entry_job_test.rb:537](../../test/jobs/calendar/sync_entry_job_test.rb#L537)<br>`assert_enqueued_jobs 2, only: Calendar::SyncEntryJob do` | [rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs:447](../../rust/crates/db/src/tests/calendar_event_test/cutover_d_events_test.rs#L447)<br>`assert_eq!( jobs(&t), vec![ ( "Calendar::SyncEntryJob".into(), json!({"event_id":id("launch_party"),"user_id":id("david")}) ), ( "Calendar::SyncEntryJob".into(), json!({"event_id":id("launch_party"),"user_id":id("jason")}) ) ] )` |
