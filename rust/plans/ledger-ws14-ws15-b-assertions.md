# Rails assertion to Rust assertion map

Reference: `d7c7de92`. Each row names an original Rails assertion call, its discriminating Rust assertion and the real test that executes it. Shared helper assertions and repeated loop cases are cited explicitly. These tables retain compiler checks as compiler checks; they do not claim matching exception classes between Ruby and the Rust type system.

## WS14e-001

Rails declaration: `test/models/event_test.rb:25` — requires the end to follow the start

Executed test: `campfire_db tests::calendar_event_test::cutover_event_test::cutover_event_end_must_follow_start`.

The same Rust assertion executes -3600s, equality and no-end cases separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event_test.rb:28](../../test/models/event_test.rb#L28)<br>`assert_not @room.events.build(organizer: @organizer, title: "Ends early", starts_at:, ends_at: starts_at - 1.hour, time_zone: "UTC").valid?` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:56](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L56)<br>`assert_eq!( t.try_write(move \|tx\| CalendarEvent::create(tx, a)).is_ok(), delta.is_none() )` |
| [test/models/event_test.rb:29](../../test/models/event_test.rb#L29)<br>`assert_not @room.events.build(organizer: @organizer, title: "Ends same", starts_at:, ends_at: starts_at, time_zone: "UTC").valid?` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:56](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L56)<br>`assert_eq!( t.try_write(move \|tx\| CalendarEvent::create(tx, a)).is_ok(), delta.is_none() )` |
| [test/models/event_test.rb:30](../../test/models/event_test.rb#L30)<br>`assert @room.events.build(organizer: @organizer, title: "No end", starts_at:, time_zone: "UTC").valid?` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:56](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L56)<br>`assert_eq!( t.try_write(move \|tx\| CalendarEvent::create(tx, a)).is_ok(), delta.is_none() )` |

## WS14e-002

Rails declaration: `test/models/event_test.rb:33` — rejects bot and non-member organizers

Executed test: `campfire_db tests::calendar_event_test::cutover_event_test::cutover_event_rejects_bot_and_nonmember_organizers`.

The assertion executes for the fixture bot and a newly created nonmember.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event_test.rb:36](../../test/models/event_test.rb#L36)<br>`assert_not @room.events.build(organizer: users(:bender), title: "Bot party", starts_at:, time_zone: "UTC").valid?` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:79](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L79)<br>`assert!(matches!( t.try_write(move \|tx\| CalendarEvent::create(tx, a)), Err(Error::RecordInvalid(ref errors)) if errors.on("organizer") == vec!["must be an active human member of the room"] ))`<br><br>[rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:87](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L87)<br>`assert!( matches!(t.try_write(move \|tx\| CalendarEvent::create(tx, a)), Err(Error::RecordInvalid(ref errors)) if errors.on("organizer") == vec!["must be an active human member of the room"]) )` |
| [test/models/event_test.rb:39](../../test/models/event_test.rb#L39)<br>`assert_not @room.events.build(organizer: outsider, title: "Outsider party", starts_at:, time_zone: "UTC").valid?` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:79](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L79)<br>`assert!(matches!( t.try_write(move \|tx\| CalendarEvent::create(tx, a)), Err(Error::RecordInvalid(ref errors)) if errors.on("organizer") == vec!["must be an active human member of the room"] ))` |

## WS14e-003

Rails declaration: `test/models/event_test.rb:42` — rejects a soft-deleted room as the venue

Executed test: `campfire_db tests::calendar_event_test::cutover_event_test::cutover_event_rejects_soft_deleted_venue`.

The first assertion discriminates RecordInvalid directly; the second checks the exact venue errors.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event_test.rb:49](../../test/models/event_test.rb#L49)<br>`assert_not event.valid?` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:114](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L114)<br>`assert!(matches!(&result, Err(Error::RecordInvalid(_))))` |
| [test/models/event_test.rb:50](../../test/models/event_test.rb#L50)<br>`assert_equal [ "must be a voice or Stage channel you belong to" ], event.errors[:venue]` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:119](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L119)<br>`assert_eq!( errors.on("venue"), vec!["must be a voice or Stage channel you belong to"] )` |

## WS14e-004

Rails declaration: `test/models/event_test.rb:70` — members with notifications off or invisible get no invitation

Executed test: `campfire_db tests::calendar_event_test::cutover_event_test::cutover_event_off_and_invisible_members_get_no_invitation`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event_test.rb:76](../../test/models/event_test.rb#L76)<br>`assert_not ActivityItem.exists?(user: users(:jason), source: event)` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:130](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L130)<br>`assert!(item(&t, e.id, "jason").is_none())` |
| [test/models/event_test.rb:77](../../test/models/event_test.rb#L77)<br>`assert_not ActivityItem.exists?(user: users(:jz), source: event)` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:131](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L131)<br>`assert!(item(&t, e.id, "jz").is_none())` |
| [test/models/event_test.rb:78](../../test/models/event_test.rb#L78)<br>`assert_equal "event_invitation", ActivityItem.find_by!(user: users(:kevin), source: event).event_type` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:132](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L132)<br>`assert_eq!( item(&t, e.id, "kevin").unwrap().event_type, "event_invitation" )` |

## WS14e-005

Rails declaration: `test/models/event_test.rb:81` — members with notifications off keep their invitation through updates and cancellations

Executed test: `campfire_db tests::calendar_event_test::cutover_event_test::cutover_event_off_member_keeps_invitation_on_update_and_cancel`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event_test.rb:89](../../test/models/event_test.rb#L89)<br>`assert_equal "event_invitation", jason_item.event_type` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:145](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L145)<br>`assert_eq!( item(&t, e.id, "jason").unwrap().event_type, "event_invitation" )` |
| [test/models/event_test.rb:91](../../test/models/event_test.rb#L91)<br>`assert event.cancel!(actor: @organizer)` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:38](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L38)<br>`assert!(t.write(move \|tx\| CalendarEvent::cancel_with_scope( tx, event_id, "this_event", Some(id("david")) )))` |
| [test/models/event_test.rb:93](../../test/models/event_test.rb#L93)<br>`assert_equal "event_invitation", jason_item.reload.event_type` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:152](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L152)<br>`assert_eq!(retained.event_type, "event_invitation")` |
| [test/models/event_test.rb:94](../../test/models/event_test.rb#L94)<br>`assert_not ActivityItem.exists?(user: users(:jason), source: event, event_type: "event_cancelled")` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:153](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L153)<br>`assert_eq!(t.read(\|c\|Ok(c.query_row("SELECT COUNT(*) FROM activity_items WHERE source_type='Event' AND source_id=? AND user_id=? AND event_type='event_cancelled'",params![e.id,id("jason")],\|r\|r.get::<_,i64>(0))?)),0)` |

## WS14e-006

Rails declaration: `test/models/event_test.rb:97` — a mentions member is notified through updates and cancellations

Executed test: `campfire_db tests::calendar_event_test::cutover_event_test::cutover_event_mentions_member_receives_update_and_cancel`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event_test.rb:103](../../test/models/event_test.rb#L103)<br>`assert_equal "event_update", ActivityItem.find_by!(user: users(:jason), source: event).event_type` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:162](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L162)<br>`assert_eq!(item(&t, e.id, "jason").unwrap().event_type, "event_update")` |
| [test/models/event_test.rb:105](../../test/models/event_test.rb#L105)<br>`assert event.cancel!(actor: @organizer)` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:38](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L38)<br>`assert!(t.write(move \|tx\| CalendarEvent::cancel_with_scope( tx, event_id, "this_event", Some(id("david")) )))` |
| [test/models/event_test.rb:106](../../test/models/event_test.rb#L106)<br>`assert_equal "event_cancelled", ActivityItem.find_by!(user: users(:jason), source: event).event_type` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:164](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L164)<br>`assert_eq!( item(&t, e.id, "jason").unwrap().event_type, "event_cancelled" )` |

## WS14e-007

Rails declaration: `test/models/event_test.rb:109` — invitations exclude bots

Executed test: `campfire_db tests::calendar_event_test::cutover_event_test::cutover_event_invitations_exclude_bots`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event_test.rb:114](../../test/models/event_test.rb#L114)<br>`assert_equal "event_invitation", ActivityItem.find_by!(user: users(:jason), source: event).event_type` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:176](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L176)<br>`assert_eq!( item(&t, e.id, "jason").unwrap().event_type, "event_invitation" )` |
| [test/models/event_test.rb:115](../../test/models/event_test.rb#L115)<br>`assert_not ActivityItem.exists?(user: users(:bender), source: event)` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:180](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L180)<br>`assert!(item(&t, e.id, "bender").is_none())` |

## WS14e-008

Rails declaration: `test/models/event_test.rb:118` — a time change notifies going and maybe attendees without duplicating items

Executed test: `campfire_db tests::calendar_event_test::cutover_event_test::cutover_event_time_change_notifies_going_and_maybe_once`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event_test.rb:128](../../test/models/event_test.rb#L128)<br>`assert_equal 1, items.count` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:191](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L191)<br>`assert_eq!(event_item_count(&t, e.id, id(who)), 1)` |
| [test/models/event_test.rb:129](../../test/models/event_test.rb#L129)<br>`assert_equal "event_update", items.first.event_type` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:193](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L193)<br>`assert_eq!(i.event_type, "event_update")` |
| [test/models/event_test.rb:130](../../test/models/event_test.rb#L130)<br>`assert_predicate items.first, :unread?` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:194](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L194)<br>`assert!(i.unread())` |
| [test/models/event_test.rb:134](../../test/models/event_test.rb#L134)<br>`assert_equal "event_invitation", kevin_item.event_type` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:196](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L196)<br>`assert_eq!( item(&t, e.id, "kevin").unwrap().event_type, "event_invitation" )` |
| [test/models/event_test.rb:135](../../test/models/event_test.rb#L135)<br>`assert_not ActivityItem.exists?(user: @organizer, source: event)` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:200](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L200)<br>`assert!(item(&t, e.id, "david").is_none())` |

## WS14e-010

Rails declaration: `test/models/event_test.rb:148` — a title-only edit creates no items

Executed test: `campfire_db tests::calendar_event_test::cutover_event_test::cutover_event_title_only_edit_creates_no_items`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event_test.rb:151](../../test/models/event_test.rb#L151)<br>`assert_no_difference -> { ActivityItem.where(source: event).count } do` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:229](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L229)<br>`assert_eq!(event_count(), before)` |
| [test/models/event_test.rb:154](../../test/models/event_test.rb#L154)<br>`assert_equal "Renamed", event.reload.title` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:230](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L230)<br>`assert_eq!(t.read(\|c\| CalendarEvent::find(c, e.id)).title, "Renamed")` |

## WS14e-011

Rails declaration: `test/models/event_test.rb:157` — cancel notifies going and maybe attendees and clears the other items

Executed test: `campfire_db tests::calendar_event_test::cutover_event_test::cutover_event_cancel_notifies_going_maybe_and_handles_declined`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event_test.rb:163](../../test/models/event_test.rb#L163)<br>`assert event.cancel!(actor: @organizer)` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:38](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L38)<br>`assert!(t.write(move \|tx\| CalendarEvent::cancel_with_scope( tx, event_id, "this_event", Some(id("david")) )))` |
| [test/models/event_test.rb:164](../../test/models/event_test.rb#L164)<br>`assert_predicate event.reload, :cancelled?` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:240](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L240)<br>`assert!(t.read(\|c\| CalendarEvent::find(c, e.id)).cancelled())` |
| [test/models/event_test.rb:168](../../test/models/event_test.rb#L168)<br>`assert_equal 1, items.count` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:242](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L242)<br>`assert_eq!(event_item_count(&t, e.id, id(who)), 1)` |
| [test/models/event_test.rb:169](../../test/models/event_test.rb#L169)<br>`assert_equal "event_cancelled", items.first.event_type` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:244](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L244)<br>`assert_eq!(i.event_type, "event_cancelled")` |
| [test/models/event_test.rb:170](../../test/models/event_test.rb#L170)<br>`assert_predicate items.first, :unread?` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:245](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L245)<br>`assert!(i.unread())` |
| [test/models/event_test.rb:173](../../test/models/event_test.rb#L173)<br>`assert_predicate ActivityItem.find_by!(user: users(:kevin), source: event), :handled?` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:247](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L247)<br>`assert!(item(&t, e.id, "kevin").unwrap().handled())` |

## WS14e-013

Rails declaration: `test/models/event_test.rb:189` — event items vanish when the recipient leaves the room

Executed test: `campfire_db tests::calendar_event_test::cutover_event_test::cutover_event_items_inaccessible_after_member_leaves`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event_test.rb:193](../../test/models/event_test.rb#L193)<br>`assert_includes ActivityItem.accessible_to(users(:jason)), item` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:254](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L254)<br>`assert!( t.read(\|c\| ActivityItem::accessible_to(c, &User::find(c, id("jason"))?)) .iter() .any(\|i\| i.id == original.id) )` |
| [test/models/event_test.rb:197](../../test/models/event_test.rb#L197)<br>`assert_not ActivityItem.accessible_to(users(:jason)).exists?(item.id)` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:264](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L264)<br>`assert!( !t.read(\|c\| ActivityItem::accessible_to(c, &User::find(c, id("jason"))?)) .iter() .any(\|i\| i.id == original.id) )` |

## WS14e-014

Rails declaration: `test/models/event_test.rb:200` — deleting a room removes its events, attendances, and inbox items

Executed test: `campfire_db tests::calendar_event_test::cutover_event_test::cutover_event_room_deletion_removes_events_attendances_and_items`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event_test.rb:204](../../test/models/event_test.rb#L204)<br>`assert_not_empty item_ids` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:292](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L292)<br>`assert!(!ids.is_empty())` |
| [test/models/event_test.rb:208](../../test/models/event_test.rb#L208)<br>`assert_empty Event.where(id: event.id)` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:308](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L308)<br>`assert!(t.db.read_blocking(\|c\| CalendarEvent::find(c, eid)).is_err())` |
| [test/models/event_test.rb:209](../../test/models/event_test.rb#L209)<br>`assert_empty EventAttendance.where(id: attendance_ids)` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:318](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L318)<br>`assert_eq!( t.read(\|c\| Ok(c.query_row( "SELECT COUNT(*) FROM event_attendances WHERE id=?", [aid], \|r\| r.get::<_, i64>(0) )?)), 0 )` |
| [test/models/event_test.rb:210](../../test/models/event_test.rb#L210)<br>`assert_empty ActivityItem.where(id: item_ids)` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:328](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L328)<br>`assert!(t.db.read_blocking(\|c\| ActivityItem::find(c, iid)).is_err())` |

## WS14e-017

Rails declaration: `test/models/event_calendar_entry_test.rb:11` — destroying an entry enqueues its remote delete

Executed test: `campfire_db tests::calendar_event_test::cutover_entry_test::cutover_entry_destroy_enqueues_captured_remote_identity_after_commit`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event_calendar_entry_test.rb:14](../../test/models/event_calendar_entry_test.rb#L14)<br>`assert_enqueued_with(job: Calendar::RemoteDeleteJob, args: [ @david.id, "orphan-id" ]) do` | [rust/crates/db/src/tests/calendar_event_test/cutover_entry_test.rs:40](../../rust/crates/db/src/tests/calendar_event_test/cutover_entry_test.rs#L40)<br>`assert_eq!( deletes(&t), vec![serde_json::json!([id("david"), "orphan-id"])] )` |

## WS14e-018

Rails declaration: `test/models/event_calendar_entry_test.rb:19` — delete skips the remote delete for already-reconciled rows

Executed test: `campfire_db tests::calendar_event_test::cutover_entry_test::cutover_entry_delete_skips_remote_delete`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event_calendar_entry_test.rb:22](../../test/models/event_calendar_entry_test.rb#L22)<br>`assert_no_enqueued_jobs(only: Calendar::RemoteDeleteJob) do` | [rust/crates/db/src/tests/calendar_event_test/cutover_entry_test.rs:51](../../rust/crates/db/src/tests/calendar_event_test/cutover_entry_test.rs#L51)<br>`assert!(deletes(&t).is_empty())` |

## WS14e-019

Rails declaration: `test/models/event_calendar_entry_test.rb:27` — delete_all skips remote deletes for disconnect cleanup

Executed test: `campfire_db tests::calendar_event_test::cutover_entry_test::cutover_entry_delete_all_skips_remote_delete_and_preserves_other_users`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event_calendar_entry_test.rb:30](../../test/models/event_calendar_entry_test.rb#L30)<br>`assert_no_enqueued_jobs(only: Calendar::RemoteDeleteJob) do` | [rust/crates/db/src/tests/calendar_event_test/cutover_entry_test.rs:67](../../rust/crates/db/src/tests/calendar_event_test/cutover_entry_test.rs#L67)<br>`assert!(deletes(&t).is_empty())` |

## WS14e-020

Rails declaration: `test/models/event/channel_timeline_test.rb:11` — creating an event posts exactly one announcement by the organizer with the title and URL

Executed test: `campfire_db tests::calendar_event_test::cutover_timeline_test::cutover_timeline_singleton_has_one_organizer_announcement_with_title_url_and_reference`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/channel_timeline_test.rb:14](../../test/models/event/channel_timeline_test.rb#L14)<br>`assert_difference -> { @room.root_messages.count }, 1 do` | [rust/crates/db/src/tests/calendar_event_test/cutover_timeline_test.rs:34](../../rust/crates/db/src/tests/calendar_event_test/cutover_timeline_test.rs#L34)<br>`assert_eq!(roots(&t), before + 1)` |
| [test/models/event/channel_timeline_test.rb:21](../../test/models/event/channel_timeline_test.rb#L21)<br>`assert_equal @organizer, announcement.creator` | [rust/crates/db/src/tests/calendar_event_test/cutover_timeline_test.rs:36](../../rust/crates/db/src/tests/calendar_event_test/cutover_timeline_test.rs#L36)<br>`assert_eq!(m.creator_id, id("david"))` |
| [test/models/event/channel_timeline_test.rb:22](../../test/models/event/channel_timeline_test.rb#L22)<br>`assert_equal "Scheduled an event: Planning session\n/rooms/#{@room.id}/events/#{event.id}",` | [rust/crates/db/src/tests/calendar_event_test/cutover_timeline_test.rs:37](../../rust/crates/db/src/tests/calendar_event_test/cutover_timeline_test.rs#L37)<br>`assert_eq!( m.markdown_source, Some(format!( "Scheduled an event: Planning session\n/rooms/{}/events/{}", e.room_id, e.id )) )` |
| [test/models/event/channel_timeline_test.rb:24](../../test/models/event/channel_timeline_test.rb#L24)<br>`assert_equal [ event ], announcement.events` | [rust/crates/db/src/tests/calendar_event_test/cutover_timeline_test.rs:52](../../rust/crates/db/src/tests/calendar_event_test/cutover_timeline_test.rs#L52)<br>`assert_eq!( t.read(\|c\| Ok(CalendarEvent::for_message_ids(c, &[m.id])? .remove(&m.id) .unwrap_or_default() .into_iter() .map(\|e\| e.id) .collect::<Vec<_>>())), vec![e.id] )` |

## WS14e-021

Rails declaration: `test/models/event/channel_timeline_test.rb:47` — edits and cancellations post nothing

Executed test: `campfire_db tests::calendar_event_test::cutover_timeline_test::cutover_timeline_edits_and_cancels_post_no_messages`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/channel_timeline_test.rb:50](../../test/models/event/channel_timeline_test.rb#L50)<br>`assert_no_difference -> { Message.count } do` | [rust/crates/db/src/tests/calendar_event_test/cutover_timeline_test.rs:82](../../rust/crates/db/src/tests/calendar_event_test/cutover_timeline_test.rs#L82)<br>`assert_eq!(count(&t, "messages"), before)` |
| [test/models/event/channel_timeline_test.rb:52](../../test/models/event/channel_timeline_test.rb#L52)<br>`assert event.cancel!(actor: @organizer)` | [rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs:38](../../rust/crates/db/src/tests/calendar_event_test/cutover_event_test.rs#L38)<br>`assert!(t.write(move \|tx\| CalendarEvent::cancel_with_scope( tx, event_id, "this_event", Some(id("david")) )))` |

## WS14e-022

Rails declaration: `test/models/event/channel_timeline_test.rb:56` — the announcement creates no inbox items

Executed test: `campfire_db tests::calendar_event_test::cutover_timeline_test::cutover_timeline_announcement_has_no_inbox_items`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/channel_timeline_test.rb:60](../../test/models/event/channel_timeline_test.rb#L60)<br>`assert_empty ActivityItem.where(source: announcement)` | [rust/crates/db/src/tests/calendar_event_test/cutover_timeline_test.rs:89](../../rust/crates/db/src/tests/calendar_event_test/cutover_timeline_test.rs#L89)<br>`assert_eq!( t.read(\|c\| Ok(c.query_row( "SELECT COUNT(*) FROM activity_items WHERE source_type='Message' AND source_id=?", [m.id], \|r\| r.get::<_, i64>(0) )?)), 0 )` |

## WS14e-025

Rails declaration: `test/models/event/channel_timeline_test.rb:110` — deleting the event removes its references

Executed test: `campfire_db tests::calendar_event_test::cutover_timeline_test::cutover_timeline_event_destroy_removes_references_but_preserves_message`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/channel_timeline_test.rb:117](../../test/models/event/channel_timeline_test.rb#L117)<br>`assert_not_empty EventReference.where(message_id: message.id, event_id: event.id)` | [rust/crates/db/src/tests/calendar_event_test/cutover_timeline_test.rs:123](../../rust/crates/db/src/tests/calendar_event_test/cutover_timeline_test.rs#L123)<br>`assert_eq!(refs(), 1)` |
| [test/models/event/channel_timeline_test.rb:121](../../test/models/event/channel_timeline_test.rb#L121)<br>`assert_empty EventReference.where(message_id: message.id, event_id: event.id)` | [rust/crates/db/src/tests/calendar_event_test/cutover_timeline_test.rs:126](../../rust/crates/db/src/tests/calendar_event_test/cutover_timeline_test.rs#L126)<br>`assert_eq!(refs(), 0)` |
| [test/models/event/channel_timeline_test.rb:122](../../test/models/event/channel_timeline_test.rb#L122)<br>`assert Message.exists?(message.id)` | [rust/crates/db/src/tests/calendar_event_test/cutover_timeline_test.rs:127](../../rust/crates/db/src/tests/calendar_event_test/cutover_timeline_test.rs#L127)<br>`assert!(t.read(\|c\| Message::find(c, m.id)).id == m.id)` |

## WS14e-026

Rails declaration: `test/models/event/recurrence_test.rb:101` — a single event has no series

Executed test: `campfire_db tests::calendar_event_test::cutover_recurrence_test::cutover_recurrence_singleton_has_no_series_or_neighbors`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/recurrence_test.rb:104](../../test/models/event/recurrence_test.rb#L104)<br>`assert_not event.series?` | [rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:26](../../rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs#L26)<br>`assert!(!e.series())` |
| [test/models/event/recurrence_test.rb:105](../../test/models/event/recurrence_test.rb#L105)<br>`assert_not event.series_head?` | [rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:27](../../rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs#L27)<br>`assert!(!e.series_head())` |
| [test/models/event/recurrence_test.rb:106](../../test/models/event/recurrence_test.rb#L106)<br>`assert_nil event.next_occurrence` | [rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:28](../../rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs#L28)<br>`assert!(t.read(\|c\| e.next_occurrence(c)).is_none())` |
| [test/models/event/recurrence_test.rb:107](../../test/models/event/recurrence_test.rb#L107)<br>`assert_nil event.previous_occurrence` | [rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:29](../../rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs#L29)<br>`assert!(t.read(\|c\| e.previous_occurrence(c)).is_none())` |
| [test/models/event/recurrence_test.rb:108](../../test/models/event/recurrence_test.rb#L108)<br>`assert_empty event.future_occurrences` | [rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:30](../../rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs#L30)<br>`assert!(t.read(\|c\| e.future_occurrences(c)).is_empty())` |

## WS14e-027

Rails declaration: `test/models/event/recurrence_test.rb:133` — the occurrence cap and one-year range run on head updates too

Executed test: `campfire_db tests::calendar_event_test::cutover_recurrence_test::cutover_recurrence_head_plain_update_checks_one_year_cap`.

The first assertion discriminates RecordInvalid directly; the second checks the exact recurrence error.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/recurrence_test.rb:138](../../test/models/event/recurrence_test.rb#L138)<br>`error = assert_raises(ActiveRecord::RecordInvalid) do` | [rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:47](../../rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs#L47)<br>`assert!(matches!(&result, Err(Error::RecordInvalid(_))))` |
| [test/models/event/recurrence_test.rb:141](../../test/models/event/recurrence_test.rb#L141)<br>`assert_match(/at most one year/, error.record.errors[:recurrence_until].join)` | [rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:52](../../rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs#L52)<br>`assert!( errors .on("recurrence_until") .iter() .any(\|e\| e.contains("at most one year")) )` |
| [test/models/event/recurrence_test.rb:142](../../test/models/event/recurrence_test.rb#L142)<br>`assert_equal Date.new(2026, 10, 19), head.reload.recurrence_until` | [rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:58](../../rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs#L58)<br>`assert_eq!( t.read(\|c\| CalendarEvent::find(c, h.id)).recurrence_until, h.recurrence_until )` |

## WS14e-028

Rails declaration: `test/models/event/recurrence_test.rb:225` — recurrence fields cannot be changed by injecting the guard flag

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::cutover_recurrence_guard_flag_injection_cannot_change_the_rule`.

Unknown model attributes are rejected at the typed EventChanges boundary (compile-fail E0560); the real HTTP path additionally proves no persisted rule change. No Rails exception-class equivalence is claimed for the Rust type system.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/recurrence_test.rb:230](../../test/models/event/recurrence_test.rb#L230)<br>`assert_raises(ActiveModel::UnknownAttributeError) do` | [rust/crates/db/src/models/calendar_event/changes.rs:20](../../rust/crates/db/src/models/calendar_event/changes.rs#L20)<br>`/// &#96;&#96;&#96;compile_fail,E0560 /// use campfire_db::models::calendar_event::changes::EventChanges; /// let changes = EventChanges { /// allow_recurrence_mutation: true, /// recurrence_rule: Some(Some("daily".into())), /// ..Default::default() /// }; /// &#96;&#96;&#96;` |
| [test/models/event/recurrence_test.rb:233](../../test/models/event/recurrence_test.rb#L233)<br>`assert_equal "weekly", head.reload.recurrence_rule` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover.rs:32](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover.rs#L32)<br>`assert_eq!( app.db() .read(move \|c\| CalendarEvent::find(c, head.id)) .await .unwrap() .recurrence_rule .as_deref(), Some("weekly") )` |

## WS14e-030

Rails declaration: `test/models/event/recurrence_test.rb:336` — this event on the head accepts the form's unchanged rule values

Executed test: `campfire_db tests::calendar_event_test::cutover_recurrence_test::cutover_recurrence_head_local_edit_accepts_unchanged_rule_values`.

The setup asserts three occurrences; the follower assertion iterates both the second and third.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/recurrence_test.rb:345](../../test/models/event/recurrence_test.rb#L345)<br>`assert_equal "Renamed", occurrences.first.reload.title` | [rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:81](../../rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs#L81)<br>`assert_eq!(after[0].title, "Renamed")` |
| [test/models/event/recurrence_test.rb:346](../../test/models/event/recurrence_test.rb#L346)<br>`assert_equal "Planning session", occurrences.second.reload.title` | [rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:83](../../rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs#L83)<br>`assert_eq!(e.title, "Planning session")` |
| [test/models/event/recurrence_test.rb:347](../../test/models/event/recurrence_test.rb#L347)<br>`assert_equal "Planning session", occurrences.third.reload.title` | [rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:83](../../rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs#L83)<br>`assert_eq!(e.title, "Planning session")` |

## WS14e-033

Rails declaration: `test/models/event/recurrence_test.rb:520` — series slots are unique among uncancelled occurrences

Executed test: `campfire_db tests::calendar_event_test::cutover_recurrence_test::cutover_recurrence_active_series_slots_have_unique_database_constraint`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/recurrence_test.rb:526](../../test/models/event/recurrence_test.rb#L526)<br>`assert index, "expected the index_events_on_series_slot index to exist"` | [rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:97](../../rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs#L97)<br>`assert!(index.is_some())` |
| [test/models/event/recurrence_test.rb:527](../../test/models/event/recurrence_test.rb#L527)<br>`assert index.unique` | [rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:98](../../rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs#L98)<br>`assert_eq!(index, Some(true))` |
| [test/models/event/recurrence_test.rb:530](../../test/models/event/recurrence_test.rb#L530)<br>`assert_raises(ActiveRecord::RecordNotUnique) do` | [rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:115](../../rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs#L115)<br>`assert!( matches!(result,Err(Error::Sqlite(rusqlite::Error::SqliteFailure(e,_))) if e.extended_code==rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE) )` |

## WS14e-037

Rails declaration: `test/models/event/recurrence_test.rb:915` — series order puts uncancelled occurrences first at equal times

Executed test: `campfire_db tests::calendar_event_test::cutover_recurrence_test::cutover_recurrence_equal_time_orders_active_before_cancelled`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/recurrence_test.rb:928](../../test/models/event/recurrence_test.rb#L928)<br>`assert_equal [ occurrences.first.id, occurrences.third.id, occurrences.second.id ], current.map(&:id)` | [rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:135](../../rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs#L135)<br>`assert_eq!( rows(&t, &h).iter().map(\|e\| e.id).collect::<Vec<_>>(), vec![h.id, before[2].id, before[1].id] )` |
| [test/models/event/recurrence_test.rb:929](../../test/models/event/recurrence_test.rb#L929)<br>`assert_equal occurrences.third.id, occurrences.first.next_occurrence.id` | [rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:139](../../rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs#L139)<br>`assert_eq!(t.read(\|c\| h.next_occurrence(c)).unwrap().id, before[2].id)` |

## WS14e-038

Rails declaration: `test/models/event/recurrence_test.rb:932` — a rule change beyond the cap is rejected and leaves the series alone

Executed test: `campfire_db tests::calendar_event_test::cutover_recurrence_test::cutover_recurrence_rule_over_cap_rejects_and_preserves_original_series`.

The first assertion discriminates RecordInvalid directly; the second checks the exact recurrence error.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/recurrence_test.rb:938](../../test/models/event/recurrence_test.rb#L938)<br>`error = assert_raises(ActiveRecord::RecordInvalid) do` | [rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:160](../../rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs#L160)<br>`assert!(matches!(&result, Err(Error::RecordInvalid(_))))` |
| [test/models/event/recurrence_test.rb:944](../../test/models/event/recurrence_test.rb#L944)<br>`assert_match(/pick an earlier end date/, error.record.errors[:recurrence_until].join)` | [rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:165](../../rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs#L165)<br>`assert!( errors .on("recurrence_until") .iter() .any(\|e\| e.contains("pick an earlier end date")) )` |
| [test/models/event/recurrence_test.rb:945](../../test/models/event/recurrence_test.rb#L945)<br>`assert_equal before_ids, head.reload.series_events.ids` | [rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:171](../../rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs#L171)<br>`assert_eq!( rows(&t, &h).iter().map(\|e\| e.id).collect::<Vec<_>>(), before.iter().map(\|e\| e.id).collect::<Vec<_>>() )` |
| [test/models/event/recurrence_test.rb:946](../../test/models/event/recurrence_test.rb#L946)<br>`assert_equal "weekly", head.recurrence_rule` | [rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:175](../../rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs#L175)<br>`assert_eq!( t.read(\|c\| CalendarEvent::find(c, h.id)).recurrence_rule, Some("weekly".into()) )` |

## WS14e-039

Rails declaration: `test/models/event/recurrence_test.rb:1033` — a head-only series can still be re-timed through this and following

Executed test: `campfire_db tests::calendar_event_test::cutover_recurrence_test::cutover_recurrence_head_only_series_retimes_through_following`.

Rails private @following_reorder has no mutable Rust counterpart. Its reset is tested by the guarded observable behavior: the immediately following plain head retime is RecordInvalid and leaves the start unchanged. event-guard-reset.rb checks the Ruby flag plus this consequence against pinned Rails (event.rb:315-319).

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/recurrence_test.rb:1037](../../test/models/event/recurrence_test.rb#L1037)<br>`assert_equal [ head.id ], head.series_events.pluck(:id)` | [rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:184](../../rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs#L184)<br>`assert_eq!( rows(&t, &h).iter().map(\|e\| e.id).collect::<Vec<_>>(), vec![h.id] )` |
| [test/models/event/recurrence_test.rb:1044](../../test/models/event/recurrence_test.rb#L1044)<br>`assert_equal utc(2027, 1, 1, 10, 0), head.reload.starts_at` | [rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:199](../../rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs#L199)<br>`assert_eq!(fresh.starts_at, stamp("2027-01-01 10:00:00"))` |
| [test/models/event/recurrence_test.rb:1045](../../test/models/event/recurrence_test.rb#L1045)<br>`assert_equal utc(2027, 1, 1, 11, 0), head.ends_at` | [rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:200](../../rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs#L200)<br>`assert_eq!(fresh.ends_at, Some(stamp("2027-01-01 11:00:00")))` |
| [test/models/event/recurrence_test.rb:1046](../../test/models/event/recurrence_test.rb#L1046)<br>`assert_equal head.id, head.series_id` | [rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:201](../../rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs#L201)<br>`assert_eq!(fresh.series_id, Some(h.id))` |
| [test/models/event/recurrence_test.rb:1047](../../test/models/event/recurrence_test.rb#L1047)<br>`assert_equal [ head.id ], head.series_events.pluck(:id)` | [rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:202](../../rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs#L202)<br>`assert_eq!( rows(&t, &h).iter().map(\|e\| e.id).collect::<Vec<_>>(), vec![h.id] )` |
| [test/models/event/recurrence_test.rb:1048](../../test/models/event/recurrence_test.rb#L1048)<br>`assert_not head.instance_variable_get(:@following_reorder)` | [rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:220](../../rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs#L220)<br>`assert!( matches!(result, Err(Error::RecordInvalid(ref errors)) if errors.on("starts_at") == vec!["moves the whole series: choose This and following or the entire series"]) )`<br><br>[rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:223](../../rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs#L223)<br>`assert_eq!( t.read(\|c\| CalendarEvent::find(c, eid)).starts_at, fresh.starts_at )` |

## WS14e-040

Rails declaration: `test/models/event/recurrence_test.rb:1051` — a single-occurrence description edit of the head still succeeds

Executed test: `campfire_db tests::calendar_event_test::cutover_recurrence_test::cutover_recurrence_head_description_edit_stays_local`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/recurrence_test.rb:1057](../../test/models/event/recurrence_test.rb#L1057)<br>`assert_equal "Head note", head.reload.description` | [rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:242](../../rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs#L242)<br>`assert_eq!(after[0].description, Some("Head note".into()))` |
| [test/models/event/recurrence_test.rb:1058](../../test/models/event/recurrence_test.rb#L1058)<br>`assert_nil occurrences.second.reload.description` | [rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs:243](../../rust/crates/db/src/tests/calendar_event_test/cutover_recurrence_test.rs#L243)<br>`assert!(after[1].description.is_none())` |

## WS14e-041

Rails declaration: `test/models/event/reference_sync_test.rb:41` — a message without an event link references nothing

Executed test: `campfire_db tests::calendar_event_test::cutover_reference_test::cutover_reference_message_without_link_has_no_events`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/reference_sync_test.rb:46](../../test/models/event/reference_sync_test.rb#L46)<br>`assert_empty message.events` | [rust/crates/db/src/tests/calendar_event_test/cutover_reference_test.rs:45](../../rust/crates/db/src/tests/calendar_event_test/cutover_reference_test.rs#L45)<br>`assert!(event_ids(&t, m.id).is_empty())` |

## WS14e-042

Rails declaration: `test/models/event/reference_sync_test.rb:64` — a link to a missing event creates nothing

Executed test: `campfire_db tests::calendar_event_test::cutover_reference_test::cutover_reference_missing_event_creates_no_references`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/reference_sync_test.rb:71](../../test/models/event/reference_sync_test.rb#L71)<br>`assert_empty message.events` | [rust/crates/db/src/tests/calendar_event_test/cutover_reference_test.rs:56](../../rust/crates/db/src/tests/calendar_event_test/cutover_reference_test.rs#L56)<br>`assert!(event_ids(&t, m.id).is_empty())` |
| [test/models/event/reference_sync_test.rb:72](../../test/models/event/reference_sync_test.rb#L72)<br>`assert_empty message.event_references` | [rust/crates/db/src/tests/calendar_event_test/cutover_reference_test.rs:55](../../rust/crates/db/src/tests/calendar_event_test/cutover_reference_test.rs#L55)<br>`assert!(references(&t, m.id).is_empty())` |

## WS14e-043

Rails declaration: `test/models/event/reference_sync_test.rb:75` — editing a message to add an event link adds the reference

Executed test: `campfire_db tests::calendar_event_test::cutover_reference_test::cutover_reference_edit_adds_the_exact_event`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/reference_sync_test.rb:79](../../test/models/event/reference_sync_test.rb#L79)<br>`assert_empty message.events` | [rust/crates/db/src/tests/calendar_event_test/cutover_reference_test.rs:63](../../rust/crates/db/src/tests/calendar_event_test/cutover_reference_test.rs#L63)<br>`assert!(event_ids(&t, m.id).is_empty())` |
| [test/models/event/reference_sync_test.rb:83](../../test/models/event/reference_sync_test.rb#L83)<br>`assert_equal [ @event ], message.reload.events` | [rust/crates/db/src/tests/calendar_event_test/cutover_reference_test.rs:79](../../rust/crates/db/src/tests/calendar_event_test/cutover_reference_test.rs#L79)<br>`assert_eq!(event_ids(&t, mid), vec![id("launch_party")])` |

## WS14e-044

Rails declaration: `test/models/event/reference_sync_test.rb:99` — deleting the message removes its references

Executed test: `campfire_db tests::calendar_event_test::cutover_reference_test::cutover_reference_message_destroy_removes_join_and_preserves_event`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/reference_sync_test.rb:105](../../test/models/event/reference_sync_test.rb#L105)<br>`assert_equal 1, EventReference.where(message_id: message.id).count` | [rust/crates/db/src/tests/calendar_event_test/cutover_reference_test.rs:93](../../rust/crates/db/src/tests/calendar_event_test/cutover_reference_test.rs#L93)<br>`assert_eq!(references(&t, m.id), vec![id("launch_party")])` |
| [test/models/event/reference_sync_test.rb:109](../../test/models/event/reference_sync_test.rb#L109)<br>`assert_empty EventReference.where(message_id: message.id)` | [rust/crates/db/src/tests/calendar_event_test/cutover_reference_test.rs:96](../../rust/crates/db/src/tests/calendar_event_test/cutover_reference_test.rs#L96)<br>`assert!(references(&t, mid).is_empty())` |
| [test/models/event/reference_sync_test.rb:110](../../test/models/event/reference_sync_test.rb#L110)<br>`assert ::Event.exists?(@event.id)` | [rust/crates/db/src/tests/calendar_event_test/cutover_reference_test.rs:97](../../rust/crates/db/src/tests/calendar_event_test/cutover_reference_test.rs#L97)<br>`assert_eq!( t.read(\|c\| CalendarEvent::find(c, id("launch_party"))).id, id("launch_party") )` |

## WS14e-047

Rails declaration: `test/models/event/reminder_dispatcher_test.rb:113` — consecutive occurrences of a series are each reminded once at their own time

Executed test: `campfire_db tests::calendar_event_test::cutover_reminder_test::cutover_reminder_consecutive_occurrences_are_claimed_once_at_own_times`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/reminder_dispatcher_test.rb:123](../../test/models/event/reminder_dispatcher_test.rb#L123)<br>`assert_equal 2, occurrences.size` | [rust/crates/db/src/tests/calendar_event_test/cutover_reminder_test.rs:20](../../rust/crates/db/src/tests/calendar_event_test/cutover_reminder_test.rs#L20)<br>`assert_eq!(rows.len(), 2)` |
| [test/models/event/reminder_dispatcher_test.rb:129](../../test/models/event/reminder_dispatcher_test.rb#L129)<br>`assert_not_nil occurrences.first.reload.reminded_at` | [rust/crates/db/src/tests/calendar_event_test/cutover_reminder_test.rs:38](../../rust/crates/db/src/tests/calendar_event_test/cutover_reminder_test.rs#L38)<br>`assert_eq!( t.read(\|c\| CalendarEvent::find(c, first)).reminded_at, Some(t.now()) )` |
| [test/models/event/reminder_dispatcher_test.rb:130](../../test/models/event/reminder_dispatcher_test.rb#L130)<br>`assert_nil occurrences.second.reload.reminded_at` | [rust/crates/db/src/tests/calendar_event_test/cutover_reminder_test.rs:42](../../rust/crates/db/src/tests/calendar_event_test/cutover_reminder_test.rs#L42)<br>`assert!( t.read(\|c\| CalendarEvent::find(c, second)) .reminded_at .is_none() )` |
| [test/models/event/reminder_dispatcher_test.rb:131](../../test/models/event/reminder_dispatcher_test.rb#L131)<br>`assert_equal "event_reminder", ActivityItem.find_by!(user: users(:jason), source: occurrences.first).event_type` | [rust/crates/db/src/tests/calendar_event_test/cutover_reminder_test.rs:47](../../rust/crates/db/src/tests/calendar_event_test/cutover_reminder_test.rs#L47)<br>`assert_eq!( item(&t, first, "jason").unwrap().event_type, "event_reminder" )` |
| [test/models/event/reminder_dispatcher_test.rb:137](../../test/models/event/reminder_dispatcher_test.rb#L137)<br>`assert_not_nil occurrences.second.reload.reminded_at` | [rust/crates/db/src/tests/calendar_event_test/cutover_reminder_test.rs:54](../../rust/crates/db/src/tests/calendar_event_test/cutover_reminder_test.rs#L54)<br>`assert_eq!( t.read(\|c\| CalendarEvent::find(c, second)).reminded_at, Some(t.now()) )` |
| [test/models/event/reminder_dispatcher_test.rb:138](../../test/models/event/reminder_dispatcher_test.rb#L138)<br>`assert_equal "event_reminder", ActivityItem.find_by!(user: users(:jason), source: occurrences.second).event_type` | [rust/crates/db/src/tests/calendar_event_test/cutover_reminder_test.rs:58](../../rust/crates/db/src/tests/calendar_event_test/cutover_reminder_test.rs#L58)<br>`assert_eq!( item(&t, second, "jason").unwrap().event_type, "event_reminder" )` |

## WS14e-048

Rails declaration: `test/models/event/reminder_dispatcher_test.rb:141` — a series starting just before midnight still builds its occurrences

Executed test: `campfire_db tests::calendar_event_test::cutover_reminder_test::cutover_reminder_start_crossing_midnight_builds_two_occurrences`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/reminder_dispatcher_test.rb:151](../../test/models/event/reminder_dispatcher_test.rb#L151)<br>`assert_equal 2, series.series_events.count` | [rust/crates/db/src/tests/calendar_event_test/cutover_reminder_test.rs:82](../../rust/crates/db/src/tests/calendar_event_test/cutover_reminder_test.rs#L82)<br>`assert_eq!(rows.len(), 2)` |
| [test/models/event/reminder_dispatcher_test.rb:152](../../test/models/event/reminder_dispatcher_test.rb#L152)<br>`assert_equal Date.new(2026, 9, 23), starts_at.in_time_zone("UTC").to_date` | [rust/crates/db/src/tests/calendar_event_test/cutover_reminder_test.rs:83](../../rust/crates/db/src/tests/calendar_event_test/cutover_reminder_test.rs#L83)<br>`assert_eq!( rows[0].starts_at, Timestamp::parse_db("2026-09-23 00:00:00").unwrap() )` |

## WS14e-059

Rails declaration: `test/models/event/venue_test.rb:11` — a venue is optional

Executed test: `campfire_db tests::calendar_event_test::cutover_venue_test::cutover_venue_optional_is_valid_and_absent`.

The Rust assertion executes the real create; a validation error aborts before the saved venue can be compared.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/venue_test.rb:14](../../test/models/event/venue_test.rb#L14)<br>`assert event.valid?` | [rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs:72](../../rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs#L72)<br>`assert_eq!(event(&t, None, false).venue_room_id, None)` |
| [test/models/event/venue_test.rb:15](../../test/models/event/venue_test.rb#L15)<br>`assert_nil event.venue` | [rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs:72](../../rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs#L72)<br>`assert_eq!(event(&t, None, false).venue_room_id, None)` |

## WS14e-060

Rails declaration: `test/models/event/venue_test.rb:18` — a voice or Stage channel venue is valid

Executed test: `campfire_db tests::calendar_event_test::cutover_venue_test::cutover_venue_voice_and_stage_are_valid`.

The saved-ID assertion executes separately for Voice and Stage, failing if either model create is rejected.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/venue_test.rb:19](../../test/models/event/venue_test.rb#L19)<br>`assert @room.events.build(organizer: @organizer, title: "Voice meetup",` | [rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs:79](../../rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs#L79)<br>`assert_eq!(event(&t, Some(vid), false).venue_room_id, Some(vid))` |
| [test/models/event/venue_test.rb:21](../../test/models/event/venue_test.rb#L21)<br>`assert @room.events.build(organizer: @organizer, title: "Stage meetup",` | [rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs:79](../../rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs#L79)<br>`assert_eq!(event(&t, Some(vid), false).venue_room_id, Some(vid))` |

## WS14e-061

Rails declaration: `test/models/event/venue_test.rb:25` — a text channel or DM venue is rejected

Executed test: `campfire_db tests::calendar_event_test::cutover_venue_test::cutover_venue_text_and_direct_are_rejected`.

The invalid helper asserts RecordInvalid directly, then the exact venue errors for both text and DM.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/venue_test.rb:30](../../test/models/event/venue_test.rb#L30)<br>`assert_not event.valid?` | [rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs:60](../../rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs#L60)<br>`assert!(matches!(&result, Err(Error::RecordInvalid(_))))` |
| [test/models/event/venue_test.rb:31](../../test/models/event/venue_test.rb#L31)<br>`assert_equal [ "must be a voice or Stage channel you belong to" ], event.errors[:venue]` | [rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs:64](../../rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs#L64)<br>`assert_eq!( e.on("venue"), vec!["must be a voice or Stage channel you belong to"] )` |

## WS14e-062

Rails declaration: `test/models/event/venue_test.rb:35` — the organizer must belong to the venue

Executed test: `campfire_db tests::calendar_event_test::cutover_venue_test::cutover_venue_organizer_membership_is_required`.

The invalid helper asserts RecordInvalid directly, then the exact venue errors for the nonmember organizer.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/venue_test.rb:40](../../test/models/event/venue_test.rb#L40)<br>`assert_not event.valid?` | [rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs:60](../../rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs#L60)<br>`assert!(matches!(&result, Err(Error::RecordInvalid(_))))` |
| [test/models/event/venue_test.rb:41](../../test/models/event/venue_test.rb#L41)<br>`assert_equal [ "must be a voice or Stage channel you belong to" ], event.errors[:venue]` | [rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs:64](../../rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs#L64)<br>`assert_eq!( e.on("venue"), vec!["must be a voice or Stage channel you belong to"] )` |

## WS14e-063

Rails declaration: `test/models/event/venue_test.rb:44` — an event in a voice channel may use its own room as the venue

Executed test: `campfire_db tests::calendar_event_test::cutover_venue_test::cutover_venue_voice_event_can_use_own_room`.

The real create is evaluated inside this assertion; a rejected self-room venue fails before its saved ID can be compared.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/venue_test.rb:48](../../test/models/event/venue_test.rb#L48)<br>`assert event.valid?` | [rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs:112](../../rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs#L112)<br>`assert_eq!( t.write(move \|tx\| CalendarEvent::create(tx, a)) .venue_room_id, Some(vid) )` |

## WS14e-064

Rails declaration: `test/models/event/venue_test.rb:51` — other edits stay valid after the organizer leaves the venue

Executed test: `campfire_db tests::calendar_event_test::cutover_venue_test::cutover_venue_unrelated_edits_remain_valid_after_organizer_leaves`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/venue_test.rb:55](../../test/models/event/venue_test.rb#L55)<br>`assert_equal false, event.update_with_announcement!({ title: "Renamed" }, actor: @organizer)` | [rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs:129](../../rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs#L129)<br>`assert!(!t.write(move \|tx\| CalendarEvent::update_with_scope( tx, eid, EventChanges { title: Some("Renamed".into()), ..Default::default() }, "this_event", Some(id("david")) )))` |
| [test/models/event/venue_test.rb:56](../../test/models/event/venue_test.rb#L56)<br>`assert_equal "Renamed", event.reload.title` | [rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs:140](../../rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs#L140)<br>`assert_eq!(saved.title, "Renamed")` |
| [test/models/event/venue_test.rb:57](../../test/models/event/venue_test.rb#L57)<br>`assert_equal @voice.id, event.venue_room_id` | [rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs:141](../../rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs#L141)<br>`assert_eq!(saved.venue_room_id, Some(vid))` |

## WS14e-065

Rails declaration: `test/models/event/venue_test.rb:60` — deleting the venue clears the link but keeps the event

Executed test: `campfire_db tests::calendar_event_test::cutover_venue_test::cutover_venue_destroy_clears_link_and_preserves_event`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/venue_test.rb:65](../../test/models/event/venue_test.rb#L65)<br>`assert_nil event.reload.venue_room_id` | [rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs:160](../../rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs#L160)<br>`assert_eq!(saved.venue_room_id, None)` |
| [test/models/event/venue_test.rb:66](../../test/models/event/venue_test.rb#L66)<br>`assert_equal "Planning session", event.title` | [rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs:161](../../rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs#L161)<br>`assert_eq!(saved.title, "Planning session")` |

## WS14e-066

Rails declaration: `test/models/event/venue_test.rb:69` — scheduling a series copies the venue to every occurrence

Executed test: `campfire_db tests::calendar_event_test::cutover_venue_test::cutover_venue_series_copies_to_all_occurrences`.

Exact three-element venue vector discriminates both occurrence count and each occurrence venue.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/venue_test.rb:73](../../test/models/event/venue_test.rb#L73)<br>`assert_equal 3, occurrences.size` | [rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs:168](../../rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs#L168)<br>`assert_eq!(venues(&t, &head), vec![Some(vid); 3])` |
| [test/models/event/venue_test.rb:75](../../test/models/event/venue_test.rb#L75)<br>`assert_equal @voice.id, occurrence.venue_room_id` | [rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs:168](../../rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs#L168)<br>`assert_eq!(venues(&t, &head), vec![Some(vid); 3])` |

## WS14e-067

Rails declaration: `test/models/event/venue_test.rb:79` — this and following propagates a venue change

Executed test: `campfire_db tests::calendar_event_test::cutover_venue_test::cutover_venue_following_propagates_change`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/venue_test.rb:86](../../test/models/event/venue_test.rb#L86)<br>`assert_equal @stage.id, occurrence.reload.venue_room_id` | [rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs:177](../../rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs#L177)<br>`assert_eq!(venues(&t, &head), vec![Some(stage); 3])` |

## WS14e-068

Rails declaration: `test/models/event/venue_test.rb:90` — this and following propagates clearing the venue

Executed test: `campfire_db tests::calendar_event_test::cutover_venue_test::cutover_venue_following_propagates_clear`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/venue_test.rb:97](../../test/models/event/venue_test.rb#L97)<br>`assert_nil occurrence.reload.venue_room_id` | [rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs:185](../../rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs#L185)<br>`assert_eq!(venues(&t, &head), vec![None; 3])` |

## WS14e-069

Rails declaration: `test/models/event/venue_test.rb:101` — a single-occurrence edit changes the venue for that occurrence only

Executed test: `campfire_db tests::calendar_event_test::cutover_venue_test::cutover_venue_local_edit_changes_only_one_occurrence`.

The vector compares head, second and third in series order.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/venue_test.rb:107](../../test/models/event/venue_test.rb#L107)<br>`assert_equal @voice.id, occurrences.first.reload.venue_room_id` | [rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs:195](../../rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs#L195)<br>`assert_eq!(venues(&t, &head), vec![Some(vid), Some(stage), Some(vid)])` |
| [test/models/event/venue_test.rb:108](../../test/models/event/venue_test.rb#L108)<br>`assert_equal @stage.id, occurrences.second.reload.venue_room_id` | [rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs:195](../../rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs#L195)<br>`assert_eq!(venues(&t, &head), vec![Some(vid), Some(stage), Some(vid)])` |
| [test/models/event/venue_test.rb:109](../../test/models/event/venue_test.rb#L109)<br>`assert_equal @voice.id, occurrences.third.reload.venue_room_id` | [rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs:195](../../rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs#L195)<br>`assert_eq!(venues(&t, &head), vec![Some(vid), Some(stage), Some(vid)])` |

## WS14e-070

Rails declaration: `test/models/event/venue_test.rb:112` — a venue-only edit creates no inbox items

Executed test: `campfire_db tests::calendar_event_test::cutover_venue_test::cutover_venue_only_edit_creates_no_inbox_items`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/venue_test.rb:115](../../test/models/event/venue_test.rb#L115)<br>`assert_no_difference -> { ActivityItem.where(source: event).count } do` | [rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs:211](../../rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs#L211)<br>`assert_eq!( t.read(\|c\| Ok(c.query_row( "SELECT COUNT(*) FROM activity_items WHERE source_type='Event' AND source_id=?", [e.id], \|r\| r.get::<_, i64>(0) )?)), n )` |
| [test/models/event/venue_test.rb:118](../../test/models/event/venue_test.rb#L118)<br>`assert_equal @stage.id, event.reload.venue_room_id` | [rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs:219](../../rust/crates/db/src/tests/calendar_event_test/cutover_venue_test.rs#L219)<br>`assert_eq!( t.read(\|c\| CalendarEvent::find(c, e.id)).venue_room_id, Some(stage) )` |

## WS14e-071

Rails declaration: `test/controllers/rooms/events_controller_test.rb:10` — index lists upcoming, past, and cancelled events separately

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::controllers::cutover_events_index_separates_upcoming_past_and_cancelled`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/rooms/events_controller_test.rb:15](../../test/controllers/rooms/events_controller_test.rb#L15)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:27](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L27)<br>`assert_eq!(reply.status, StatusCode::OK)` |
| [test/controllers/rooms/events_controller_test.rb:18](../../test/controllers/rooms/events_controller_test.rb#L18)<br>`assert_includes upcoming, "Launch party planning"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:31](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L31)<br>`assert!(upcoming.contains("Launch party planning"))` |
| [test/controllers/rooms/events_controller_test.rb:19](../../test/controllers/rooms/events_controller_test.rb#L19)<br>`assert_not_includes upcoming, "Old kickoff"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:32](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L32)<br>`assert!(!upcoming.contains("Old kickoff"))` |
| [test/controllers/rooms/events_controller_test.rb:20](../../test/controllers/rooms/events_controller_test.rb#L20)<br>`assert_includes past, "Old kickoff"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:33](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L33)<br>`assert!(past.contains("Old kickoff"))` |
| [test/controllers/rooms/events_controller_test.rb:21](../../test/controllers/rooms/events_controller_test.rb#L21)<br>`assert_includes cancelled, "Sprint retro"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:34](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L34)<br>`assert!(cancelled.contains("Sprint retro"))` |

## WS14e-072

Rails declaration: `test/controllers/rooms/events_controller_test.rb:37` — a member can create an event and members are invited

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::controllers::cutover_events_member_create_invites_and_parses_posted_zone`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/rooms/events_controller_test.rb:38](../../test/controllers/rooms/events_controller_test.rb#L38)<br>`assert_difference -> { Event.count } do` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:42](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L42)<br>`assert_eq!(count(&app).await, n + 1)` |
| [test/controllers/rooms/events_controller_test.rb:45](../../test/controllers/rooms/events_controller_test.rb#L45)<br>`assert_redirected_to room_event_path(@room, event)` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:163](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L163)<br>`assert_eq!(reply.status, StatusCode::FOUND)`<br><br>[rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:164](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L164)<br>`assert_eq!( reply.location(), Some(format!("http://campfire.test{}", path(eid)).as_str()) )` |
| [test/controllers/rooms/events_controller_test.rb:46](../../test/controllers/rooms/events_controller_test.rb#L46)<br>`assert_equal users(:david), event.organizer` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:54](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L54)<br>`assert_eq!(e.organizer_id, DAVID)` |
| [test/controllers/rooms/events_controller_test.rb:47](../../test/controllers/rooms/events_controller_test.rb#L47)<br>`assert_equal ActiveSupport::TimeZone["America/New_York"].parse("2026-09-25T15:30"), event.starts_at` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:55](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L55)<br>`assert_eq!( e.starts_at, Timestamp::parse_db("2026-09-25 19:30:00").unwrap() )` |
| [test/controllers/rooms/events_controller_test.rb:48](../../test/controllers/rooms/events_controller_test.rb#L48)<br>`assert_equal "event_invitation", ActivityItem.find_by!(user: users(:jason), source: event).event_type` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:59](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L59)<br>`assert_eq!(item(&app, e.id, JASON).await.event_type, "event_invitation")` |

## WS14e-073

Rails declaration: `test/controllers/rooms/events_controller_test.rb:51` — a member can create a repeating event with one invitation per member

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::controllers::cutover_events_series_create_invites_once_per_member`.

One exact invitation tuple per member discriminates both item count and source identity.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/rooms/events_controller_test.rb:52](../../test/controllers/rooms/events_controller_test.rb#L52)<br>`assert_difference -> { Event.count }, 3 do` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:67](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L67)<br>`assert_eq!(count(&app).await, n + 3)` |
| [test/controllers/rooms/events_controller_test.rb:62](../../test/controllers/rooms/events_controller_test.rb#L62)<br>`assert_redirected_to room_event_path(@room, head)` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:163](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L163)<br>`assert_eq!(reply.status, StatusCode::FOUND)`<br><br>[rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:164](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L164)<br>`assert_eq!( reply.location(), Some(format!("http://campfire.test{}", path(eid)).as_str()) )` |
| [test/controllers/rooms/events_controller_test.rb:63](../../test/controllers/rooms/events_controller_test.rb#L63)<br>`assert_equal head.id, head.series_id` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:81](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L81)<br>`assert_eq!(head.series_id, Some(head.id))` |
| [test/controllers/rooms/events_controller_test.rb:65](../../test/controllers/rooms/events_controller_test.rb#L65)<br>`assert_equal 3, occurrences.size` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:82](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L82)<br>`assert_eq!(rows(&app, head.id).await.len(), 3)` |
| [test/controllers/rooms/events_controller_test.rb:69](../../test/controllers/rooms/events_controller_test.rb#L69)<br>`assert_equal 1, items.count` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:84](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L84)<br>`assert_eq!( item_rows(&app, head.id, who, None).await, vec![(head.id, "event_invitation".into())] )` |
| [test/controllers/rooms/events_controller_test.rb:70](../../test/controllers/rooms/events_controller_test.rb#L70)<br>`assert_equal head.id, items.first.source_id` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:84](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L84)<br>`assert_eq!( item_rows(&app, head.id, who, None).await, vec![(head.id, "event_invitation".into())] )` |

## WS14e-074

Rails declaration: `test/controllers/rooms/events_controller_test.rb:74` — create rejects a series above the occurrence cap

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::controllers::cutover_events_create_above_cap_renders_error_without_writes`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/rooms/events_controller_test.rb:75](../../test/controllers/rooms/events_controller_test.rb#L75)<br>`assert_no_difference -> { Event.count } do` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:98](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L98)<br>`assert_eq!(count(&app).await, n)` |
| [test/controllers/rooms/events_controller_test.rb:84](../../test/controllers/rooms/events_controller_test.rb#L84)<br>`assert_response :unprocessable_content` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:96](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L96)<br>`assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY)` |
| [test/controllers/rooms/events_controller_test.rb:85](../../test/controllers/rooms/events_controller_test.rb#L85)<br>`assert_includes response.body, "pick an earlier end date"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:97](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L97)<br>`assert!(reply.text().contains("pick an earlier end date"))` |

## WS14e-075

Rails declaration: `test/controllers/rooms/events_controller_test.rb:109` — index lists past occurrences individually

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::controllers::cutover_events_index_lists_each_past_occurrence`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/rooms/events_controller_test.rb:115](../../test/controllers/rooms/events_controller_test.rb#L115)<br>`assert_equal 3, occurrences.size` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:123](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L123)<br>`assert_eq!(rs.len(), 3)` |
| [test/controllers/rooms/events_controller_test.rb:119](../../test/controllers/rooms/events_controller_test.rb#L119)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:126](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L126)<br>`assert_eq!(reply.status, StatusCode::OK)` |
| [test/controllers/rooms/events_controller_test.rb:122](../../test/controllers/rooms/events_controller_test.rb#L122)<br>`assert_includes rest, room_event_path(@room, occurrence)` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:130](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L130)<br>`assert!(rest.contains(&path(e.id)))` |

## WS14e-076

Rails declaration: `test/controllers/rooms/events_controller_test.rb:172` — updating this and following shifts later occurrences and notifies once per attendee

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::controllers::cutover_events_following_shift_notifies_once_per_attendee`.

The exact item tuple discriminates count, event_type and source identity.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/rooms/events_controller_test.rb:187](../../test/controllers/rooms/events_controller_test.rb#L187)<br>`assert_redirected_to room_event_path(@room, head)` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:163](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L163)<br>`assert_eq!(reply.status, StatusCode::FOUND)`<br><br>[rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:164](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L164)<br>`assert_eq!( reply.location(), Some(format!("http://campfire.test{}", path(eid)).as_str()) )` |
| [test/controllers/rooms/events_controller_test.rb:188](../../test/controllers/rooms/events_controller_test.rb#L188)<br>`assert_equal ActiveSupport::TimeZone["UTC"].local(2026, 10, 2, 16, 30), occurrences.second.reload.starts_at` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:150](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L150)<br>`assert_eq!( rs[1].starts_at, Timestamp::parse_db("2026-10-02 16:30:00").unwrap() )` |
| [test/controllers/rooms/events_controller_test.rb:189](../../test/controllers/rooms/events_controller_test.rb#L189)<br>`assert_equal ActiveSupport::TimeZone["UTC"].local(2026, 10, 9, 16, 30), occurrences.third.reload.starts_at` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:154](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L154)<br>`assert_eq!( rs[2].starts_at, Timestamp::parse_db("2026-10-09 16:30:00").unwrap() )` |
| [test/controllers/rooms/events_controller_test.rb:191](../../test/controllers/rooms/events_controller_test.rb#L191)<br>`assert_equal 1, items.count` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:158](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L158)<br>`assert_eq!( item_rows(&app, head.id, JASON, None).await, vec![(head.id, "event_update".into())] )` |
| [test/controllers/rooms/events_controller_test.rb:192](../../test/controllers/rooms/events_controller_test.rb#L192)<br>`assert_equal "event_update", items.first.event_type` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:158](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L158)<br>`assert_eq!( item_rows(&app, head.id, JASON, None).await, vec![(head.id, "event_update".into())] )` |
| [test/controllers/rooms/events_controller_test.rb:193](../../test/controllers/rooms/events_controller_test.rb#L193)<br>`assert_equal head.id, items.first.source_id` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:158](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L158)<br>`assert_eq!( item_rows(&app, head.id, JASON, None).await, vec![(head.id, "event_update".into())] )` |

## WS14e-077

Rails declaration: `test/controllers/rooms/events_controller_test.rb:196` — updating without a scope leaves the rest of the series untouched

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::controllers::cutover_events_default_update_leaves_other_occurrences_untouched`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/rooms/events_controller_test.rb:213](../../test/controllers/rooms/events_controller_test.rb#L213)<br>`assert_redirected_to room_event_path(@room, occurrences.second)` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:163](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L163)<br>`assert_eq!(reply.status, StatusCode::FOUND)`<br><br>[rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:164](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L164)<br>`assert_eq!( reply.location(), Some(format!("http://campfire.test{}", path(eid)).as_str()) )` |
| [test/controllers/rooms/events_controller_test.rb:214](../../test/controllers/rooms/events_controller_test.rb#L214)<br>`assert_equal "Weekly planning", occurrences.first.reload.title` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:173](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L173)<br>`assert_eq!(saved[0].title, "Weekly planning")` |
| [test/controllers/rooms/events_controller_test.rb:215](../../test/controllers/rooms/events_controller_test.rb#L215)<br>`assert_equal "Weekly planning", occurrences.third.reload.title` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:174](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L174)<br>`assert_eq!(saved[2].title, "Weekly planning")` |

## WS14e-078

Rails declaration: `test/controllers/rooms/events_controller_test.rb:218` — changing the rule away from the first event is rejected

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::controllers::cutover_events_follower_rule_change_is_rejected`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/rooms/events_controller_test.rb:229](../../test/controllers/rooms/events_controller_test.rb#L229)<br>`assert_response :unprocessable_content` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:185](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L185)<br>`assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY)` |
| [test/controllers/rooms/events_controller_test.rb:230](../../test/controllers/rooms/events_controller_test.rb#L230)<br>`assert_includes response.body, "first event"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:186](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L186)<br>`assert!(reply.text().contains("first event"))` |

## WS14e-079

Rails declaration: `test/controllers/rooms/events_controller_test.rb:253` — an administrator who is not the organizer can use this and following, but an ordinary member cannot

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::controllers::cutover_events_admin_following_edit_succeeds_and_member_edit_is_denied`.

Ordered title vector discriminates each head/second/third title separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/rooms/events_controller_test.rb:273](../../test/controllers/rooms/events_controller_test.rb#L273)<br>`assert_redirected_to room_event_path(@room, occurrence)` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:163](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L163)<br>`assert_eq!(reply.status, StatusCode::FOUND)`<br><br>[rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:164](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L164)<br>`assert_eq!( reply.location(), Some(format!("http://campfire.test{}", path(eid)).as_str()) )` |
| [test/controllers/rooms/events_controller_test.rb:274](../../test/controllers/rooms/events_controller_test.rb#L274)<br>`assert_equal "Weekly planning", occurrences.first.reload.title` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:202](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L202)<br>`assert_eq!( saved.iter().map(\|e\| e.title.as_str()).collect::<Vec<_>>(), vec!["Weekly planning", "Renamed", "Renamed"] )` |
| [test/controllers/rooms/events_controller_test.rb:275](../../test/controllers/rooms/events_controller_test.rb#L275)<br>`assert_equal "Renamed", occurrence.reload.title` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:202](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L202)<br>`assert_eq!( saved.iter().map(\|e\| e.title.as_str()).collect::<Vec<_>>(), vec!["Weekly planning", "Renamed", "Renamed"] )` |
| [test/controllers/rooms/events_controller_test.rb:276](../../test/controllers/rooms/events_controller_test.rb#L276)<br>`assert_equal "Renamed", occurrences.third.reload.title` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:202](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L202)<br>`assert_eq!( saved.iter().map(\|e\| e.title.as_str()).collect::<Vec<_>>(), vec!["Weekly planning", "Renamed", "Renamed"] )` |
| [test/controllers/rooms/events_controller_test.rb:284](../../test/controllers/rooms/events_controller_test.rb#L284)<br>`assert_response :forbidden` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:214](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L214)<br>`assert_eq!(denied.status, StatusCode::FORBIDDEN)` |
| [test/controllers/rooms/events_controller_test.rb:285](../../test/controllers/rooms/events_controller_test.rb#L285)<br>`assert_equal "Renamed", occurrence.reload.title` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:215](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L215)<br>`assert_eq!(find(&app, eid).await.title, "Renamed")` |

## WS14e-080

Rails declaration: `test/controllers/rooms/events_controller_test.rb:288` — cancelling this and following cancels later occurrences with one item per attendee

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::controllers::cutover_events_following_cancel_cancels_later_and_notifies_once`.

Ordered cancellation vector discriminates each occurrence; exact item tuple discriminates count and source.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/rooms/events_controller_test.rb:298](../../test/controllers/rooms/events_controller_test.rb#L298)<br>`assert_redirected_to room_event_path(@room, occurrences.second)` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:163](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L163)<br>`assert_eq!(reply.status, StatusCode::FOUND)`<br><br>[rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:164](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L164)<br>`assert_eq!( reply.location(), Some(format!("http://campfire.test{}", path(eid)).as_str()) )` |
| [test/controllers/rooms/events_controller_test.rb:299](../../test/controllers/rooms/events_controller_test.rb#L299)<br>`assert_not_predicate occurrences.first.reload, :cancelled?` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:233](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L233)<br>`assert_eq!( saved.iter().map(\|e\| e.cancelled()).collect::<Vec<_>>(), vec![false, true, true] )` |
| [test/controllers/rooms/events_controller_test.rb:300](../../test/controllers/rooms/events_controller_test.rb#L300)<br>`assert_predicate occurrences.second.reload, :cancelled?` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:233](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L233)<br>`assert_eq!( saved.iter().map(\|e\| e.cancelled()).collect::<Vec<_>>(), vec![false, true, true] )` |
| [test/controllers/rooms/events_controller_test.rb:301](../../test/controllers/rooms/events_controller_test.rb#L301)<br>`assert_predicate occurrences.third.reload, :cancelled?` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:233](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L233)<br>`assert_eq!( saved.iter().map(\|e\| e.cancelled()).collect::<Vec<_>>(), vec![false, true, true] )` |
| [test/controllers/rooms/events_controller_test.rb:303](../../test/controllers/rooms/events_controller_test.rb#L303)<br>`assert_equal 1, items.count` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:237](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L237)<br>`assert_eq!( item_rows(&app, head.id, JASON, Some("event_cancelled")).await, vec![(rs[1].id, "event_cancelled".into())] )` |
| [test/controllers/rooms/events_controller_test.rb:304](../../test/controllers/rooms/events_controller_test.rb#L304)<br>`assert_equal occurrences.second.id, items.first.source_id` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:237](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L237)<br>`assert_eq!( item_rows(&app, head.id, JASON, Some("event_cancelled")).await, vec![(rs[1].id, "event_cancelled".into())] )` |

## WS14e-081

Rails declaration: `test/controllers/rooms/events_controller_test.rb:307` — cancelling without a scope cancels only that occurrence

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::controllers::cutover_events_default_cancel_is_local`.

local_cancel(None) executes the shared redirect and ordered cancellation assertions.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/rooms/events_controller_test.rb:316](../../test/controllers/rooms/events_controller_test.rb#L316)<br>`assert_redirected_to room_event_path(@room, occurrences.second)` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:163](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L163)<br>`assert_eq!(reply.status, StatusCode::FOUND)`<br><br>[rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:164](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L164)<br>`assert_eq!( reply.location(), Some(format!("http://campfire.test{}", path(eid)).as_str()) )` |
| [test/controllers/rooms/events_controller_test.rb:317](../../test/controllers/rooms/events_controller_test.rb#L317)<br>`assert_not_predicate occurrences.first.reload, :cancelled?` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:257](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L257)<br>`assert_eq!( rows(&app, head.id) .await .iter() .map(\|e\| e.cancelled()) .collect::<Vec<_>>(), vec![false, true, false] )` |
| [test/controllers/rooms/events_controller_test.rb:318](../../test/controllers/rooms/events_controller_test.rb#L318)<br>`assert_predicate occurrences.second.reload, :cancelled?` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:257](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L257)<br>`assert_eq!( rows(&app, head.id) .await .iter() .map(\|e\| e.cancelled()) .collect::<Vec<_>>(), vec![false, true, false] )` |
| [test/controllers/rooms/events_controller_test.rb:319](../../test/controllers/rooms/events_controller_test.rb#L319)<br>`assert_not_predicate occurrences.third.reload, :cancelled?` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:257](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L257)<br>`assert_eq!( rows(&app, head.id) .await .iter() .map(\|e\| e.cancelled()) .collect::<Vec<_>>(), vec![false, true, false] )` |

## WS14e-082

Rails declaration: `test/controllers/rooms/events_controller_test.rb:322` — cancelling this event explicitly cancels only that occurrence

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::controllers::cutover_events_explicit_this_event_cancel_is_local`.

local_cancel(Some("this_event")) executes the shared redirect and ordered cancellation assertions.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/rooms/events_controller_test.rb:331](../../test/controllers/rooms/events_controller_test.rb#L331)<br>`assert_redirected_to room_event_path(@room, occurrences.second)` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:163](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L163)<br>`assert_eq!(reply.status, StatusCode::FOUND)`<br><br>[rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:164](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L164)<br>`assert_eq!( reply.location(), Some(format!("http://campfire.test{}", path(eid)).as_str()) )` |
| [test/controllers/rooms/events_controller_test.rb:332](../../test/controllers/rooms/events_controller_test.rb#L332)<br>`assert_not_predicate occurrences.first.reload, :cancelled?` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:257](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L257)<br>`assert_eq!( rows(&app, head.id) .await .iter() .map(\|e\| e.cancelled()) .collect::<Vec<_>>(), vec![false, true, false] )` |
| [test/controllers/rooms/events_controller_test.rb:333](../../test/controllers/rooms/events_controller_test.rb#L333)<br>`assert_predicate occurrences.second.reload, :cancelled?` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:257](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L257)<br>`assert_eq!( rows(&app, head.id) .await .iter() .map(\|e\| e.cancelled()) .collect::<Vec<_>>(), vec![false, true, false] )` |
| [test/controllers/rooms/events_controller_test.rb:334](../../test/controllers/rooms/events_controller_test.rb#L334)<br>`assert_not_predicate occurrences.third.reload, :cancelled?` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:257](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L257)<br>`assert_eq!( rows(&app, head.id) .await .iter() .map(\|e\| e.cancelled()) .collect::<Vec<_>>(), vec![false, true, false] )` |

## WS14e-083

Rails declaration: `test/controllers/rooms/events_controller_test.rb:337` — show renders cancel scopes for series occurrences and a single cancel for single events

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::controllers::cutover_events_show_cancel_scope_inputs_only_for_series`.

Input-count assertion executes once for each cancel_scope value; the second GET verifies the singleton.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/rooms/events_controller_test.rb:345](../../test/controllers/rooms/events_controller_test.rb#L345)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:281](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L281)<br>`assert_eq!(reply.status, StatusCode::OK)` |
| [test/controllers/rooms/events_controller_test.rb:346](../../test/controllers/rooms/events_controller_test.rb#L346)<br>`assert_select "input[name=cancel_scope][value=this_event]", count: 1` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:285](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L285)<br>`assert_eq!( inputs .find_iter(&html) .filter(\|m\| m.as_str().contains("name=\"cancel_scope\"") && m.as_str().contains(&format!("value=\"{value}\""))) .count(), 1 )` |
| [test/controllers/rooms/events_controller_test.rb:347](../../test/controllers/rooms/events_controller_test.rb#L347)<br>`assert_select "input[name=cancel_scope][value=this_and_following]", count: 1` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:285](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L285)<br>`assert_eq!( inputs .find_iter(&html) .filter(\|m\| m.as_str().contains("name=\"cancel_scope\"") && m.as_str().contains(&format!("value=\"{value}\""))) .count(), 1 )` |
| [test/controllers/rooms/events_controller_test.rb:351](../../test/controllers/rooms/events_controller_test.rb#L351)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:295](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L295)<br>`assert_eq!(reply.status, StatusCode::OK)` |
| [test/controllers/rooms/events_controller_test.rb:352](../../test/controllers/rooms/events_controller_test.rb#L352)<br>`assert_select "input[name=cancel_scope]", count: 0` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:296](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L296)<br>`assert!(!reply.text().contains("name=\"cancel_scope\""))` |
| [test/controllers/rooms/events_controller_test.rb:353](../../test/controllers/rooms/events_controller_test.rb#L353)<br>`assert_includes response.body, "Cancel event"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:297](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L297)<br>`assert!(reply.text().contains("Cancel event"))` |

## WS14e-084

Rails declaration: `test/controllers/rooms/events_controller_test.rb:356` — index issues a bounded number of queries regardless of occurrence count

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::controllers::cutover_events_index_query_count_is_independent_of_occurrence_count`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/rooms/events_controller_test.rb:363](../../test/controllers/rooms/events_controller_test.rb#L363)<br>`assert_equal 3, heads.first.series_events.count` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:306](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L306)<br>`assert_eq!(rows(&app, heads[0].id).await.len(), 3)` |
| [test/controllers/rooms/events_controller_test.rb:366](../../test/controllers/rooms/events_controller_test.rb#L366)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:308](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L308)<br>`assert_eq!(david.get(&index_path()).await.status, StatusCode::OK)` |
| [test/controllers/rooms/events_controller_test.rb:374](../../test/controllers/rooms/events_controller_test.rb#L374)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:172](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L172)<br>`assert_eq!(reply.status, StatusCode::OK)` |
| [test/controllers/rooms/events_controller_test.rb:382](../../test/controllers/rooms/events_controller_test.rb#L382)<br>`assert_equal 8, heads.first.reload.series_events.count` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:326](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L326)<br>`assert_eq!(rows(&app, head.id).await.len(), 8)` |
| [test/controllers/rooms/events_controller_test.rb:387](../../test/controllers/rooms/events_controller_test.rb#L387)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:172](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L172)<br>`assert_eq!(reply.status, StatusCode::OK)` |
| [test/controllers/rooms/events_controller_test.rb:390](../../test/controllers/rooms/events_controller_test.rb#L390)<br>`assert_equal small, large` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:329](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L329)<br>`assert_eq!(small, large, "event index SQL must stay bounded")` |

## WS14e-086

Rails declaration: `test/controllers/rooms/events_controller_test.rb:419` — requires authentication

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::controllers::cutover_events_signed_out_index_redirects_to_sign_in`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/rooms/events_controller_test.rb:424](../../test/controllers/rooms/events_controller_test.rb#L424)<br>`assert_redirected_to new_session_url` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:336](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L336)<br>`assert_eq!(reply.status, StatusCode::FOUND)`<br><br>[rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:337](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L337)<br>`assert_eq!(reply.location(), Some("http://campfire.test/session/new"))` |

## WS14e-088

Rails declaration: `test/controllers/rooms/events_controller_test.rb:443` — the organizer can update times and attendees are notified

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::controllers::cutover_events_update_uses_existing_zone_and_notifies_attendees`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/rooms/events_controller_test.rb:450](../../test/controllers/rooms/events_controller_test.rb#L450)<br>`assert_redirected_to room_event_path(@room, @event)` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:163](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L163)<br>`assert_eq!(reply.status, StatusCode::FOUND)`<br><br>[rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:164](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L164)<br>`assert_eq!( reply.location(), Some(format!("http://campfire.test{}", path(eid)).as_str()) )` |
| [test/controllers/rooms/events_controller_test.rb:452](../../test/controllers/rooms/events_controller_test.rb#L452)<br>`assert_equal ActiveSupport::TimeZone["America/New_York"].parse("2026-09-26 15:30"), @event.reload.starts_at` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:348](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L348)<br>`assert_eq!( e.starts_at, Timestamp::parse_db("2026-09-26 19:30:00").unwrap() )` |
| [test/controllers/rooms/events_controller_test.rb:453](../../test/controllers/rooms/events_controller_test.rb#L453)<br>`assert_equal "America/New_York", @event.time_zone` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:352](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L352)<br>`assert_eq!(e.time_zone, "America/New_York")` |
| [test/controllers/rooms/events_controller_test.rb:454](../../test/controllers/rooms/events_controller_test.rb#L454)<br>`assert_equal "event_update", ActivityItem.find_by!(user: users(:kevin), source: @event).event_type` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:353](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L353)<br>`assert_eq!(item(&app, eid, KEVIN).await.event_type, "event_update")` |

## WS14e-089

Rails declaration: `test/controllers/rooms/events_controller_test.rb:541` — a member can create an event with a venue

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::controllers::cutover_events_create_with_venue_persists_the_room`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/rooms/events_controller_test.rb:549](../../test/controllers/rooms/events_controller_test.rb#L549)<br>`assert_redirected_to room_event_path(@room, event)` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:163](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L163)<br>`assert_eq!(reply.status, StatusCode::FOUND)`<br><br>[rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:164](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L164)<br>`assert_eq!( reply.location(), Some(format!("http://campfire.test{}", path(eid)).as_str()) )` |
| [test/controllers/rooms/events_controller_test.rb:550](../../test/controllers/rooms/events_controller_test.rb#L550)<br>`assert_equal voice.id, event.venue_room_id` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:374](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L374)<br>`assert_eq!(e.venue_room_id, Some(vid))` |

## WS14e-090

Rails declaration: `test/controllers/rooms/events_controller_test.rb:553` — the organizer can set and clear the venue

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::controllers::cutover_events_update_sets_and_clears_venue`.

The redirect and saved-venue assertions execute for both setting the ID and posting the blank sentinel.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/rooms/events_controller_test.rb:562](../../test/controllers/rooms/events_controller_test.rb#L562)<br>`assert_redirected_to room_event_path(@room, @event)` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:163](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L163)<br>`assert_eq!(reply.status, StatusCode::FOUND)`<br><br>[rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:164](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L164)<br>`assert_eq!( reply.location(), Some(format!("http://campfire.test{}", path(eid)).as_str()) )` |
| [test/controllers/rooms/events_controller_test.rb:563](../../test/controllers/rooms/events_controller_test.rb#L563)<br>`assert_equal voice.id, @event.reload.venue_room_id` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:396](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L396)<br>`assert_eq!( find(&app, eid).await.venue_room_id, if v == json!("") { None } else { Some(vid) } )` |
| [test/controllers/rooms/events_controller_test.rb:567](../../test/controllers/rooms/events_controller_test.rb#L567)<br>`assert_redirected_to room_event_path(@room, @event)` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:163](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L163)<br>`assert_eq!(reply.status, StatusCode::FOUND)`<br><br>[rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:164](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L164)<br>`assert_eq!( reply.location(), Some(format!("http://campfire.test{}", path(eid)).as_str()) )` |
| [test/controllers/rooms/events_controller_test.rb:568](../../test/controllers/rooms/events_controller_test.rb#L568)<br>`assert_nil @event.reload.venue_room_id` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:396](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L396)<br>`assert_eq!( find(&app, eid).await.venue_room_id, if v == json!("") { None } else { Some(vid) } )` |

## WS14e-091

Rails declaration: `test/controllers/rooms/events_controller_test.rb:571` — create rejects a text channel venue

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::controllers::cutover_events_create_rejects_text_venue_without_writes`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/rooms/events_controller_test.rb:572](../../test/controllers/rooms/events_controller_test.rb#L572)<br>`assert_no_difference -> { Event.count } do` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:418](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L418)<br>`assert_eq!(count(&app).await, n)` |
| [test/controllers/rooms/events_controller_test.rb:578](../../test/controllers/rooms/events_controller_test.rb#L578)<br>`assert_response :unprocessable_content` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:412](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L412)<br>`assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY)` |
| [test/controllers/rooms/events_controller_test.rb:579](../../test/controllers/rooms/events_controller_test.rb#L579)<br>`assert_includes response.body, "must be a voice or Stage channel you belong to"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:413](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L413)<br>`assert!( reply .text() .contains("must be a voice or Stage channel you belong to") )` |

## WS14e-092

Rails declaration: `test/controllers/rooms/events_controller_test.rb:582` — create rejects a venue the organizer does not belong to

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::controllers::cutover_events_create_rejects_nonmember_venue_without_writes`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/rooms/events_controller_test.rb:585](../../test/controllers/rooms/events_controller_test.rb#L585)<br>`assert_no_difference -> { Event.count } do` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:418](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L418)<br>`assert_eq!(count(&app).await, n)` |
| [test/controllers/rooms/events_controller_test.rb:591](../../test/controllers/rooms/events_controller_test.rb#L591)<br>`assert_response :unprocessable_content` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:412](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L412)<br>`assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY)` |
| [test/controllers/rooms/events_controller_test.rb:592](../../test/controllers/rooms/events_controller_test.rb#L592)<br>`assert_includes response.body, "must be a voice or Stage channel you belong to"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:413](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L413)<br>`assert!( reply .text() .contains("must be a voice or Stage channel you belong to") )` |

## WS14e-093

Rails declaration: `test/controllers/rooms/events_controller_test.rb:595` — update rejects a venue the organizer does not belong to

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::controllers::cutover_events_update_rejects_nonmember_venue_and_keeps_original`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/rooms/events_controller_test.rb:604](../../test/controllers/rooms/events_controller_test.rb#L604)<br>`assert_response :unprocessable_content` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:445](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L445)<br>`assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY)` |
| [test/controllers/rooms/events_controller_test.rb:605](../../test/controllers/rooms/events_controller_test.rb#L605)<br>`assert_includes response.body, "must be a voice or Stage channel you belong to"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:446](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L446)<br>`assert!( reply .text() .contains("must be a voice or Stage channel you belong to") )` |
| [test/controllers/rooms/events_controller_test.rb:606](../../test/controllers/rooms/events_controller_test.rb#L606)<br>`assert_nil @event.reload.venue_room_id` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:452](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L452)<br>`assert_eq!(e.venue_room_id, None)` |

## WS14e-094

Rails declaration: `test/controllers/rooms/events_controller_test.rb:609` — the edit form keeps a venue the editor cannot see so an unrelated edit does not clear it

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::controllers::cutover_events_editor_keeps_hidden_venue_during_unrelated_edit`.

The exact option text is located inside the Voice optgroup of the venue select; the linked assertions require its value and selected attributes.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/rooms/events_controller_test.rb:616](../../test/controllers/rooms/events_controller_test.rb#L616)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:463](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L463)<br>`assert_eq!(shown.status, StatusCode::OK)` |
| [test/controllers/rooms/events_controller_test.rb:617](../../test/controllers/rooms/events_controller_test.rb#L617)<br>`assert_select "select[name='event[venue_room_id]'] optgroup[label='Voice'] option[value='#{venue.id}'][selected]", "Design sync"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:472](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L472)<br>`assert!(select.contains("label=\"Voice\""))`<br><br>[rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:482](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L482)<br>`assert!(option.contains(&format!("value=\"{vid}\"")))`<br><br>[rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:483](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L483)<br>`assert!(option.contains("selected=\"selected\""))` |
| [test/controllers/rooms/events_controller_test.rb:623](../../test/controllers/rooms/events_controller_test.rb#L623)<br>`assert_response :redirect` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:163](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L163)<br>`assert_eq!(reply.status, StatusCode::FOUND)` |
| [test/controllers/rooms/events_controller_test.rb:624](../../test/controllers/rooms/events_controller_test.rb#L624)<br>`assert_equal venue.id, @event.reload.venue_room_id` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:487](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L487)<br>`assert_eq!(find(&app, eid).await.venue_room_id, Some(vid))` |

## WS14e-095

Rails declaration: `test/controllers/rooms/events_controller_test.rb:792` — index issues the same queries regardless of event count when venues are shared

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::controllers::cutover_events_shared_venue_index_query_count_is_independent_of_event_count`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/rooms/events_controller_test.rb:801](../../test/controllers/rooms/events_controller_test.rb#L801)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:515](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L515)<br>`assert_eq!(david.get(&index_path()).await.status, StatusCode::OK)` |
| [test/controllers/rooms/events_controller_test.rb:809](../../test/controllers/rooms/events_controller_test.rb#L809)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:172](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L172)<br>`assert_eq!(reply.status, StatusCode::OK)` |
| [test/controllers/rooms/events_controller_test.rb:820](../../test/controllers/rooms/events_controller_test.rb#L820)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:172](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L172)<br>`assert_eq!(reply.status, StatusCode::OK)` |
| [test/controllers/rooms/events_controller_test.rb:823](../../test/controllers/rooms/events_controller_test.rb#L823)<br>`assert_equal small, large` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:540](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L540)<br>`assert_eq!(small, large, "shared venue SQL must stay bounded")` |

## WS14e-096

Rails declaration: `test/controllers/rooms/events_controller_test.rb:863` — show renders no Meet row for a non-https link

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::controllers::cutover_events_show_hides_non_https_meet_link`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/rooms/events_controller_test.rb:868](../../test/controllers/rooms/events_controller_test.rb#L868)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:552](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L552)<br>`assert_eq!(reply.status, StatusCode::OK)` |
| [test/controllers/rooms/events_controller_test.rb:869](../../test/controllers/rooms/events_controller_test.rb#L869)<br>`assert_not_includes response.body, "Join Google Meet"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:553](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L553)<br>`assert!(!reply.text().contains("Join Google Meet"))` |
| [test/controllers/rooms/events_controller_test.rb:870](../../test/controllers/rooms/events_controller_test.rb#L870)<br>`assert_not_includes response.body, "javascript:"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs:554](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/controllers.rs#L554)<br>`assert!(!reply.text().contains("javascript:"))` |

## WS14e-097

Rails declaration: `test/controllers/rooms/events/attendances_controller_test.rb:49` — a response on the first event of a series is copied to every future occurrence

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::attendances::cutover_attendance_head_response_copies_to_each_future_occurrence`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/rooms/events/attendances_controller_test.rb:58](../../test/controllers/rooms/events/attendances_controller_test.rb#L58)<br>`assert_redirected_to room_event_path(@room, head)` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:163](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L163)<br>`assert_eq!(reply.status, StatusCode::FOUND)`<br><br>[rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:164](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L164)<br>`assert_eq!( reply.location(), Some(format!("http://campfire.test{}", path(eid)).as_str()) )` |
| [test/controllers/rooms/events/attendances_controller_test.rb:60](../../test/controllers/rooms/events/attendances_controller_test.rb#L60)<br>`assert_equal "going", occurrence.response_for(users(:kevin))` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/attendances.rs:30](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/attendances.rs#L30)<br>`assert_eq!( responses(&app, head.id).await, vec![Some("going".into()); 3] )` |

## WS14e-098

Rails declaration: `test/controllers/rooms/events/attendances_controller_test.rb:64` — a later response stays local unless apply to all future is checked

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::attendances::cutover_attendance_later_response_is_local_until_apply_future_checked`.

The ordered response vectors compare head/second/third after each real PATCH.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/rooms/events/attendances_controller_test.rb:73](../../test/controllers/rooms/events/attendances_controller_test.rb#L73)<br>`assert_redirected_to room_event_path(@room, occurrences.second)` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:163](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L163)<br>`assert_eq!(reply.status, StatusCode::FOUND)`<br><br>[rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:164](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L164)<br>`assert_eq!( reply.location(), Some(format!("http://campfire.test{}", path(eid)).as_str()) )` |
| [test/controllers/rooms/events/attendances_controller_test.rb:74](../../test/controllers/rooms/events/attendances_controller_test.rb#L74)<br>`assert_nil occurrences.first.response_for(users(:kevin))` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/attendances.rs:50](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/attendances.rs#L50)<br>`assert_eq!( responses(&app, head.id).await, vec![None, Some("going".into()), None] )` |
| [test/controllers/rooms/events/attendances_controller_test.rb:75](../../test/controllers/rooms/events/attendances_controller_test.rb#L75)<br>`assert_equal "going", occurrences.second.response_for(users(:kevin))` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/attendances.rs:50](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/attendances.rs#L50)<br>`assert_eq!( responses(&app, head.id).await, vec![None, Some("going".into()), None] )` |
| [test/controllers/rooms/events/attendances_controller_test.rb:76](../../test/controllers/rooms/events/attendances_controller_test.rb#L76)<br>`assert_nil occurrences.third.response_for(users(:kevin))` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/attendances.rs:50](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/attendances.rs#L50)<br>`assert_eq!( responses(&app, head.id).await, vec![None, Some("going".into()), None] )` |
| [test/controllers/rooms/events/attendances_controller_test.rb:81](../../test/controllers/rooms/events/attendances_controller_test.rb#L81)<br>`assert_redirected_to room_event_path(@room, occurrences.second)` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:163](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L163)<br>`assert_eq!(reply.status, StatusCode::FOUND)`<br><br>[rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:164](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L164)<br>`assert_eq!( reply.location(), Some(format!("http://campfire.test{}", path(eid)).as_str()) )` |
| [test/controllers/rooms/events/attendances_controller_test.rb:82](../../test/controllers/rooms/events/attendances_controller_test.rb#L82)<br>`assert_nil occurrences.first.response_for(users(:kevin))` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/attendances.rs:62](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/attendances.rs#L62)<br>`assert_eq!( responses(&app, head.id).await, vec![None, Some("maybe".into()), Some("maybe".into())] )` |
| [test/controllers/rooms/events/attendances_controller_test.rb:83](../../test/controllers/rooms/events/attendances_controller_test.rb#L83)<br>`assert_equal "maybe", occurrences.second.response_for(users(:kevin))` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/attendances.rs:62](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/attendances.rs#L62)<br>`assert_eq!( responses(&app, head.id).await, vec![None, Some("maybe".into()), Some("maybe".into())] )` |
| [test/controllers/rooms/events/attendances_controller_test.rb:84](../../test/controllers/rooms/events/attendances_controller_test.rb#L84)<br>`assert_equal "maybe", occurrences.third.response_for(users(:kevin))` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/attendances.rs:62](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/attendances.rs#L62)<br>`assert_eq!( responses(&app, head.id).await, vec![None, Some("maybe".into()), Some("maybe".into())] )` |

## WS14e-099

Rails declaration: `test/system/events_test.rb:4` — scheduling an event invites members, who respond and see it in the inbox

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::interactions::cutover_interaction_schedule_invitation_inbox_open_and_response`.

Shared schedule/inbox helpers execute in this test. Assertions compare the server DOM/state reached by the original interaction; no pixel receipt is claimed.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/system/events_test.rb:12](../../test/system/events_test.rb#L12)<br>`assert_selector "h1", text: "Events"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs:13](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs#L13)<br>`assert!(headings(&index.text()).iter().any(\|h\| h == "Events"))` |
| [test/system/events_test.rb:20](../../test/system/events_test.rb#L20)<br>`assert_selector "h1", text: "Launch retro"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs:54](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs#L54)<br>`assert!(headings(&shown.text()).contains(&find(app, eid).await.title))` |
| [test/system/events_test.rb:21](../../test/system/events_test.rb#L21)<br>`assert_text "Bring your notes."` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs:56](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs#L56)<br>`assert!(shown.text().contains(description))` |
| [test/system/events_test.rb:22](../../test/system/events_test.rb#L22)<br>`assert_text "Currently: Going"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs:58](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs#L58)<br>`assert!(visible_text(&shown.text()).contains("Currently: Going"))` |
| [test/system/events_test.rb:25](../../test/system/events_test.rb#L25)<br>`assert_text "Launch retro"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs:66](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs#L66)<br>`assert!(index.text().contains(&find(app, eid).await.title))` |
| [test/system/events_test.rb:36](../../test/system/events_test.rb#L36)<br>`assert_text "Event invitation"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs:87](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs#L87)<br>`assert!(itemhtml.contains("Event invitation"))` |
| [test/system/events_test.rb:37](../../test/system/events_test.rb#L37)<br>`assert_text "Launch retro"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs:88](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs#L88)<br>`assert!(itemhtml.contains(&find(app, eid).await.title))` |
| [test/system/events_test.rb:42](../../test/system/events_test.rb#L42)<br>`assert_text "Currently: Going"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs:115](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs#L115)<br>`assert!(visible_text(&show.text()).contains("Currently: Going"))` |
| [test/system/events_test.rb:45](../../test/system/events_test.rb#L45)<br>`assert_equal "going", event.reload.response_for(users(:jason))` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs:122](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs#L122)<br>`assert_eq!( app.db() .read(move \|c\| CalendarEvent::find(c, eid)?.response_for(c, Some(JASON))) .await .unwrap() .as_deref(), Some("going") )` |

## WS14e-100

Rails declaration: `test/system/events_test.rb:48` — scheduling a repeating event invites once per member and copies the first response

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::interactions::cutover_interaction_repeating_schedule_invites_once_and_copies_response`.

Shared schedule/inbox helpers execute with repeating=true. Assertions compare the server DOM/state reached by the original interaction; no pixel receipt is claimed.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/system/events_test.rb:68](../../test/system/events_test.rb#L68)<br>`assert_selector "h1", text: "Weekly planning"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs:54](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs#L54)<br>`assert!(headings(&shown.text()).contains(&find(app, eid).await.title))` |
| [test/system/events_test.rb:69](../../test/system/events_test.rb#L69)<br>`assert_text "Part of a series"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs:61](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs#L61)<br>`assert!(shown.text().contains("Part of a series"))` |
| [test/system/events_test.rb:70](../../test/system/events_test.rb#L70)<br>`assert_text "Next occurrence"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs:62](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs#L62)<br>`assert!(shown.text().contains("Next occurrence"))` |
| [test/system/events_test.rb:73](../../test/system/events_test.rb#L73)<br>`assert_text "Repeats weekly"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs:68](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs#L68)<br>`assert!(index.text().contains("Repeats weekly"))` |
| [test/system/events_test.rb:74](../../test/system/events_test.rb#L74)<br>`assert_text "3 occurrences remaining"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs:69](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs#L69)<br>`assert!(index.text().contains("3 occurrences remaining"))` |
| [test/system/events_test.rb:79](../../test/system/events_test.rb#L79)<br>`assert_equal 3, occurrences.size` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs:135](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs#L135)<br>`assert_eq!(rows(&app, eid).await.len(), 3)` |
| [test/system/events_test.rb:80](../../test/system/events_test.rb#L80)<br>`assert_equal 1, ActivityItem.where(user: users(:jason), source: occurrences).count` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs:136](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs#L136)<br>`assert_eq!(item_rows(&app, eid, JASON, None).await.len(), 1)` |
| [test/system/events_test.rb:88](../../test/system/events_test.rb#L88)<br>`assert_text "Event invitation"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs:87](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs#L87)<br>`assert!(itemhtml.contains("Event invitation"))` |
| [test/system/events_test.rb:89](../../test/system/events_test.rb#L89)<br>`assert_text "repeats weekly until"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs:90](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs#L90)<br>`assert!(itemhtml.contains("repeats weekly until"))` |
| [test/system/events_test.rb:94](../../test/system/events_test.rb#L94)<br>`assert_text "Currently: Going"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs:115](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs#L115)<br>`assert!(visible_text(&show.text()).contains("Currently: Going"))` |
| [test/system/events_test.rb:98](../../test/system/events_test.rb#L98)<br>`assert_equal "going", occurrence.response_for(users(:jason))` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs:138](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs#L138)<br>`assert_eq!( app.db() .read(move \|c\| CalendarEvent::find(c, eid)? .series_events(c)? .iter() .map(\|e\| e.response_for(c, Some(JASON))) .collect::<campfire_db::Result<Vec<_>>>()) .await .unwrap(), vec![Some("going".into()); 3] )` |

## WS14e-101

Rails declaration: `test/system/events_test.rb:102` — scheduling an event announces it in the room with a card members respond from

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::interactions::cutover_interaction_announcement_card_response_stays_in_requested_frame`.

Card text is scoped to the unique Card session card. The room path invariant is checked through the real Turbo response: OK, no Location/Turbo-Location redirect, exact requested frame. Shared Rails assets consume that frame; no browser pixel receipt is claimed.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/system/events_test.rb:115](../../test/system/events_test.rb#L115)<br>`assert_selector "h1", text: "Card session"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs:54](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs#L54)<br>`assert!(headings(&shown.text()).contains(&find(app, eid).await.title))` |
| [test/system/events_test.rb:124](../../test/system/events_test.rb#L124)<br>`assert_text "Scheduled an event: Card session"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs:159](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs#L159)<br>`assert!(room.text().contains("Scheduled an event: Card session"))` |
| [test/system/events_test.rb:127](../../test/system/events_test.rb#L127)<br>`assert_text "Organized by David"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs:165](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs#L165)<br>`assert!(visible_text(&matching_cards[0]).contains("Organized by David"))` |
| [test/system/events_test.rb:128](../../test/system/events_test.rb#L128)<br>`assert_text "No response yet"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs:173](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs#L173)<br>`assert!(loaded.text().contains("No response yet"))` |
| [test/system/events_test.rb:132](../../test/system/events_test.rb#L132)<br>`assert_text "Currently: Going"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs:189](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs#L189)<br>`assert!(saved.text().contains("Currently: <strong>Going</strong>"))` |
| [test/system/events_test.rb:136](../../test/system/events_test.rb#L136)<br>`assert_current_path room_path(room)` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs:185](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs#L185)<br>`assert_eq!(saved.status, StatusCode::OK)`<br><br>[rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs:186](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs#L186)<br>`assert_eq!(saved.location(), None)`<br><br>[rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs:187](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs#L187)<br>`assert!(saved.headers.get("turbo-location").is_none())`<br><br>[rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs:188](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs#L188)<br>`assert!(saved.text().contains(&format!("id=\"{frame}\"")))` |
| [test/system/events_test.rb:137](../../test/system/events_test.rb#L137)<br>`assert_text "Scheduled an event: Card session"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs:191](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs#L191)<br>`assert!(room.text().contains("Scheduled an event: Card session"))` |
| [test/system/events_test.rb:140](../../test/system/events_test.rb#L140)<br>`assert_equal "going", event.reload.response_for(users(:jason))` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs:192](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/interactions.rs#L192)<br>`assert_eq!( app.db() .read(move \|c\| CalendarEvent::find(c, eid)?.response_for(c, Some(JASON))) .await .unwrap() .as_deref(), Some("going") )` |

## WS14e-106

Rails declaration: `test/integration/event_cards_test.rb:108` — a link to an event in another room stays a plain link with no card for anyone

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::cards::cutover_cards_foreign_event_link_is_plain_for_members_and_nonmembers`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/integration/event_cards_test.rb:120](../../test/integration/event_cards_test.rb#L120)<br>`assert_empty message.events` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/cards.rs:45](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/cards.rs#L45)<br>`assert!( app.db() .read(move \|c\| CalendarEvent::for_message_ids(c, &[mid])) .await .unwrap() .get(&mid) .is_none_or(Vec::is_empty) )` |
| [test/integration/event_cards_test.rb:127](../../test/integration/event_cards_test.rb#L127)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/cards.rs:56](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/cards.rs#L56)<br>`assert_eq!(reply.status, StatusCode::OK)` |
| [test/integration/event_cards_test.rb:128](../../test/integration/event_cards_test.rb#L128)<br>`assert_select ".event-card", count: 0` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/cards.rs:57](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/cards.rs#L57)<br>`assert!(cards(&reply.text()).is_empty())` |
| [test/integration/event_cards_test.rb:129](../../test/integration/event_cards_test.rb#L129)<br>`assert_select "a[href=?]", event_url, minimum: 1` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/cards.rs:58](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/cards.rs#L58)<br>`assert!(reply.text().contains(&format!("href=\"{url}\"")))` |
| [test/integration/event_cards_test.rb:130](../../test/integration/event_cards_test.rb#L130)<br>`assert_not_includes response.body, "Secret planning"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/cards.rs:59](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/cards.rs#L59)<br>`assert!(!reply.text().contains("Secret planning"))` |
| [test/integration/event_cards_test.rb:137](../../test/integration/event_cards_test.rb#L137)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/cards.rs:62](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/cards.rs#L62)<br>`assert_eq!(reply.status, StatusCode::OK)` |
| [test/integration/event_cards_test.rb:138](../../test/integration/event_cards_test.rb#L138)<br>`assert_select ".event-card", count: 0` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/cards.rs:63](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/cards.rs#L63)<br>`assert!(cards(&reply.text()).is_empty())` |
| [test/integration/event_cards_test.rb:139](../../test/integration/event_cards_test.rb#L139)<br>`assert_not_includes response.body, "Secret planning"` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/cards.rs:64](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/cards.rs#L64)<br>`assert!(!reply.text().contains("Secret planning"))` |

## WS14e-107

Rails declaration: `test/integration/event_cards_test.rb:142` — a message without an event link renders no card

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::cards::cutover_cards_message_without_event_link_has_no_card`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/integration/event_cards_test.rb:149](../../test/integration/event_cards_test.rb#L149)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/cards.rs:72](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/cards.rs#L72)<br>`assert_eq!(reply.status, StatusCode::OK)` |
| [test/integration/event_cards_test.rb:150](../../test/integration/event_cards_test.rb#L150)<br>`assert_select ".event-card", count: 0` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/cards.rs:73](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/cards.rs#L73)<br>`assert!(cards(&reply.text()).is_empty())` |

## WS14e-108

Rails declaration: `test/integration/event_cards_test.rb:153` — rendering a room page costs no extra queries per message with an event link

Executed test: `campfire::bin/campfire controllers::rooms::events::tests::cutover::cards::cutover_cards_whole_room_query_count_is_independent_of_event_link_count`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/integration/event_cards_test.rb:157](../../test/integration/event_cards_test.rb#L157)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/cards.rs:108](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/cards.rs#L108)<br>`assert_eq!(david.get(&url).await.status, StatusCode::OK)` |
| [test/integration/event_cards_test.rb:164](../../test/integration/event_cards_test.rb#L164)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:172](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L172)<br>`assert_eq!(reply.status, StatusCode::OK)` |
| [test/integration/event_cards_test.rb:169](../../test/integration/event_cards_test.rb#L169)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs:172](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/support.rs#L172)<br>`assert_eq!(reply.status, StatusCode::OK)` |
| [test/integration/event_cards_test.rb:171](../../test/integration/event_cards_test.rb#L171)<br>`assert_equal small, large,` | [rust/crates/campfire/src/controllers/rooms/events/tests/cutover/cards.rs:112](../../rust/crates/campfire/src/controllers/rooms/events/tests/cutover/cards.rs#L112)<br>`assert_eq!( small, large, "room page must have O(1) queries per event-link message" )` |

## WS14g-129

Rails declaration: `test/models/event_calendar_entry_test.rb:11` — destroying an entry enqueues its remote delete

Executed test: `campfire_db tests::calendar_event_test::cutover_entry_test::cutover_entry_destroy_enqueues_captured_remote_identity_after_commit`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event_calendar_entry_test.rb:14](../../test/models/event_calendar_entry_test.rb#L14)<br>`assert_enqueued_with(job: Calendar::RemoteDeleteJob, args: [ @david.id, "orphan-id" ]) do` | [rust/crates/db/src/tests/calendar_event_test/cutover_entry_test.rs:40](../../rust/crates/db/src/tests/calendar_event_test/cutover_entry_test.rs#L40)<br>`assert_eq!( deletes(&t), vec![serde_json::json!([id("david"), "orphan-id"])] )` |

## WS14g-130

Rails declaration: `test/models/event_calendar_entry_test.rb:19` — delete skips the remote delete for already-reconciled rows

Executed test: `campfire_db tests::calendar_event_test::cutover_entry_test::cutover_entry_delete_skips_remote_delete`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event_calendar_entry_test.rb:22](../../test/models/event_calendar_entry_test.rb#L22)<br>`assert_no_enqueued_jobs(only: Calendar::RemoteDeleteJob) do` | [rust/crates/db/src/tests/calendar_event_test/cutover_entry_test.rs:51](../../rust/crates/db/src/tests/calendar_event_test/cutover_entry_test.rs#L51)<br>`assert!(deletes(&t).is_empty())` |

## WS14g-131

Rails declaration: `test/models/event_calendar_entry_test.rb:27` — delete_all skips remote deletes for disconnect cleanup

Executed test: `campfire_db tests::calendar_event_test::cutover_entry_test::cutover_entry_delete_all_skips_remote_delete_and_preserves_other_users`.

Each cited assertion executes through this named real model/HTTP test. Shared helpers and loops execute the original control cases.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event_calendar_entry_test.rb:30](../../test/models/event_calendar_entry_test.rb#L30)<br>`assert_no_enqueued_jobs(only: Calendar::RemoteDeleteJob) do` | [rust/crates/db/src/tests/calendar_event_test/cutover_entry_test.rs:67](../../rust/crates/db/src/tests/calendar_event_test/cutover_entry_test.rs#L67)<br>`assert!(deletes(&t).is_empty())` |

