# Broadcast contract

Every broadcast our Rails app makes, with the workstream that owns it and where the port stands.
Later waves port a domain by making its rows `ported`, without changing the frames: action, target,
stream and partial must stay exactly as listed here.

## How to broadcast from Rust

- **Named methods.** Use `campfire::channels::Broadcasts` (`app.broadcasts`), whose methods are named
  after the Ruby ones (`message_create`, `involvement_change`, ...). Where a domain has no named
  method yet, use its primitives: `append`, `prepend`, `replace`, `update`, `remove`, `turbo` (for
  `maintain_scroll`), and `channel` (for `ActionCable.server.broadcast` to a channel's own stream).
- **Streams.** Name them with `broadcasts::Stream`: `record(gid, suffix)` for `[record, :suffix]`,
  `rooms()`, `user_rooms(id)`, `user_status(id)`, `ooo_notice(id)`, `room_messages(room)`,
  `thread_messages(id)`, and `conversation(room, message)` for `message.conversation`
  (`thread || room`). Targets use the `dom_id` helpers in the same module (`room_dom_id`,
  `message_dom_id`, `thread_dom_id`), which match `ActionView::RecordIdentifier`.
- **Rendering.** Render through `campfire_views`, or pass HTML that the domain owner has rendered
  (the `Partials` trait, or a `&str`).
- **Session-bound content.** Broadcast HTML is rendered once and sent to many users, so it must
  carry nothing session-bound. `campfire_cable::turbo::broadcast_stream_to` refuses (logs, returns
  0) any HTML with an `authenticity_token` field, a `csrf-token`/`csrf-param` meta tag, or a
  non-empty `nonce`. That matches our Rails, where `ApplicationController.render` emits no token
  and a nil nonce. Current-user state is up to the renderer: never render with a viewer.
- **Model callbacks.** A model in `campfire_db` can't reach the cable, so it emits
  `Event::Broadcast(BroadcastRequest)` from `after_commit`. Define a `#[derive(Serialize,
  Deserialize)]` struct implementing `campfire_db::Broadcast` (its `KIND` names the Ruby method),
  emit `Event::broadcast(&value)`, and add a `KIND` arm to `campfire::channels::sink::broadcast`.
  The app's `Jobs` sink delivers it on the writer thread in commit order, before any
  `DisconnectUser` emitted after it (see `RoomRemovalBroadcast`).
- **Processes.** Rails broadcasts from five processes (web, resque workers, `periodic`,
  `huddle_reconciler`, and gateway-triggered jobs) and relays them through Redis. The port runs all
  of those in the one process, so every path calls the same `Broadcasts`, which is the hub the
  sockets subscribe to. `channels::tests::hub_test` covers a booted app: a model callback through
  the `Jobs` sink, work on the job runner, and sign-out through the controller. Periodic tasks and
  the reconciler, when they land (WS3, WS13), use `app.broadcasts` the same way.

## States

- **ported**: Rust makes this broadcast today, and a test pins its frame.
- **API ready**: a named `Broadcasts` method exists; the caller or its partial doesn't yet (noted).
- **primitive**: the domain isn't in Rust yet. Its owner calls the primitives with the stream,
  target and partial listed here.
- **dead**: never called in our Ruby.

Owners: WS7 cable; WS8 core messaging (rooms, memberships, messages, threads, pins, polls, quotes);
WS11 agents; WS12 boards and activity; WS13 huddles, voice and stage; WS14 calendar, events, Drive,
meeting status and OOO; WS15 GitHub, Fizzy, X, LinkedIn and link embeds. WS6 owns the views that
the partials below need.

## The calls

There are 115 call sites in `app/`, `lib/` and `config/`, found with a search over `broadcast_{append,prepend,replace,update,remove,before,after,refresh,action,render}(_later)?_to`,
`Turbo::StreamsChannel.broadcast_*`, `ActionCable.server.broadcast`, `*Channel.broadcast_to` and a
bare `broadcast_to`. The inventory's "140" was an estimate by domain. None use `_later`.

Notation: `append [room, :messages] → [room, :messages] (messages/message)` means
`broadcast_append_to room, :messages, target: [room, :messages], partial: "messages/message"`. A
`*` marks `attributes: { maintain_scroll: true }`. `cable → name {payload}` is an
`ActionCable.server.broadcast`.

### Channels

| # | Site | Call | Owner | State |
|---|---|---|---|---|
| 1 | `app/channels/presence_channel.rb:25` | cable → `user_<id>_reads` `{room_id}` | WS7 | ported (`broadcasts::read_room`) |
| 2 | `app/channels/typing_notifications_channel.rb:26` | `broadcast_to @conversation` `{action, user: {id, name}}` (room or thread) | WS7 | ported |

### Messages (`Message::Broadcasts`, `app/models/message/broadcasts.rb`)

| # | Site | Call | Owner | State |
|---|---|---|---|---|
| 3 | `:3` `broadcast_create` | append `[conversation, :messages]` → `[conversation, :messages]` (messages/message) | WS8 | ported (`message_create`; MessagesController#create and the bot API) |
| 4 | `:10` `broadcast_stream_start` | append, as #3, no unread | WS11 | primitive (`append(Stream::conversation, conversation_messages_target, html)`) |
| 5 | `:26` `broadcast_stream_update` | replace `[conversation, :messages]` → `message` (messages/message) | WS11 | primitive (`replace(Stream::conversation, message_dom_id(m, None), html)`) |
| 6 | `:33` `broadcast_stream_final` | as #5 | WS11 | primitive |
| 7 | `:42` `broadcast_reactions_replace` | replace* `[conversation, :messages]` → `dom_id(message, :boosts)` (messages/boosts/reactions) | WS8 | API ready (`message_reactions_replace`; needs `messages/boosts/_reactions`, and the Rust boosts controller must call it, see #20) |
| 8 | `:48` `broadcast_remove` | remove `[conversation, :messages]` → `message` | WS8 | ported (`message_remove`; MessagesController#destroy and `RemoveBannedContent` for `User::Bannable`) |
| 9 | `:58` `broadcast_quote_cards_replace` | replace* `[conversation, :messages]` → `dom_id(message, :message_link_cards)` (messages/message_links/cards) | WS8 | primitive |
| 10 | `:72` `broadcast_unread_room` | cable → `user_<id>_unreads` `{roomId}` per member, muted members not mentioned left out | WS8 | ported (inside `message_create`) |

### MessagesController (`app/controllers/messages_controller.rb`)

| # | Site | Call | Owner | State |
|---|---|---|---|---|
| 11 | `:86` update | replace* `[@room, :messages]` → `[message, :presentation]` (messages/presentation) | WS8 | ported (`message_replace`) |
| 12 | `:87` update | replace* `[@room, :messages]` → `[message, :meta]` (messages/meta) | WS8 | API ready (`message_part_replace(.., "meta", html)`; needs `messages/_meta`) |
| 13 | `:92` update | replace* `[@room, :messages]` → `[message, :github_pr_cards]` (github/pull_requests/cards) | WS15 | API ready (`message_part_replace`) |
| 14 | `:93` update | … `[message, :twitter_cards]` (twitter/posts/cards) | WS15 | API ready |
| 15 | `:94` update | … `[message, :message_link_cards]` (messages/message_links/cards) | WS8 | API ready |
| 16 | `:95` update | … `[message, :fizzy_cards]` (fizzy/cards/cards) | WS15 | API ready |
| 17 | `:96` update | … `[message, :linkedin_cards]` (linkedin/posts/cards) | WS15 | API ready |
| 18 | `:97` update | … `[message, :link_embed_cards]` (link_embeds/cards) | WS15 | API ready |
| 19 | `:99` update | … `[message, :drive_attachments]` (messages/drive_attachments) | WS14 | API ready |
| 20 | `:231` reply tombstones | replace* `[reply.conversation, :messages]` → `reply` (messages/message) | WS8 | primitive |
| 21 | `:242` | cable → `user_<id>_unread_threads` `{threadId, roomId, refreshOnly: true}` | WS8 | primitive (`channel(&unread_threads::stream_name_for(id), ..)`) |

### Boosts

| # | Site | Call | Owner | State |
|---|---|---|---|---|
| 22 | `app/controllers/messages/boosts_controller.rb:54` | append* `[conversation, :messages]` → `boosts_message_<client_message_id>` (messages/boosts/boost) | WS8 | dead: our `create`/`destroy` call `broadcast_reactions` (#7). The Rust controller still does this upstream append (`boost_create`); WS8 should switch it to #7 |
| 23 | `app/controllers/messages/boosts_controller.rb:59` | remove `[conversation, :messages]` → `boost` | WS8 | dead, as #22 (Rust `boost_remove`) |

### Channel threads

| # | Site | Call | Owner | State |
|---|---|---|---|---|
| 24 | `app/controllers/channel_thread_messages_controller.rb:78` | replace* `[@thread, :messages]` → `[message, :presentation]` (messages/presentation) | WS8 | primitive (`turbo(Stream::thread_messages(id), Replace, message_dom_id(m, Some("presentation")), .., true)`) |
| 25 | `…:80` | … `[message, :meta]` (messages/meta) | WS8 | primitive |
| 26 | `…:86` | … `[message, :github_pr_cards]` (github/pull_requests/cards) | WS15 | primitive |
| 27 | `…:88` | … `[message, :twitter_cards]` (twitter/posts/cards) | WS15 | primitive |
| 28 | `…:90` | … `[message, :message_link_cards]` (messages/message_links/cards) | WS8 | primitive |
| 29 | `…:92` | … `[message, :fizzy_cards]` (fizzy/cards/cards) | WS15 | primitive |
| 30 | `…:94` | … `[message, :linkedin_cards]` (linkedin/posts/cards) | WS15 | primitive |
| 31 | `…:96` | … `[message, :link_embed_cards]` (link_embeds/cards) | WS15 | primitive |
| 32 | `…:99` | … `[message, :drive_attachments]` (messages/drive_attachments) | WS14 | primitive |
| 33 | `…:192` | replace* `[reply.conversation, :messages]` → `reply` (messages/message) | WS8 | primitive |
| 34 | `…:203` | cable → `user_<id>_unread_threads` `{threadId, roomId, refreshOnly: true}` | WS8 | primitive |
| 35 | `app/models/channel_thread.rb:882` | replace* `[message.room, :messages]` → `dom_id(message, :thread_indicator)` (messages/thread_indicator) | WS8 | primitive |
| 36 | `…:1012` | cable → `user_<id>_unreads` `{roomId}` | WS8 | primitive |
| 37 | `…:1330` | cable → `user_<id>_unread_threads` `{threadId, roomId}` | WS8 | primitive |

### Boards (`ChannelThread` on board rooms, `Rooms::BoardsController`)

| # | Site | Call | Owner | State |
|---|---|---|---|---|
| 38 | `app/models/channel_thread.rb:895` | replace `[room, :messages]` → `board_column_row_dom_id` (rooms/boards/row, `dom_suffix: :board_column_row`) | WS12 | primitive |
| 39 | `…:1002` | prepend `[room, :messages]` → `board_posts` (rooms/boards/row) | WS12 | primitive |
| 40 | `…:1024` | remove `[room, :messages]` → `board_column_row_dom_id` | WS12 | primitive |
| 41 | `…:1032` | replace `[room, :messages]` → `board_list_row_dom_id` (rooms/boards/row) | WS12 | primitive |
| 42 | `…:1038` | prepend `[room, :messages]` → `board_column_<work_status>` (rooms/boards/row, column suffix) | WS12 | primitive |
| 43 | `…:1044` | remove `[room, :messages]` → `board_list_row_dom_id` | WS12 | primitive |
| 44 | `…:1045` | remove `[room, :messages]` → `board_column_row_dom_id` | WS12 | primitive |
| 45 | `app/controllers/rooms/boards_controller.rb:71` | prepend `[user, :rooms]` → `board_rooms` (pre-rendered html per user) | WS12 | primitive |
| 46 | `…:77` | replace `[user, :rooms]` → `[room, :list]` (html) | WS12 | primitive |
| 47 | `…:80` | replace `[user, :rooms]` → `[room, :header]` (html) | WS12 | primitive |

### Rooms and the sidebar

| # | Site | Call | Owner | State |
|---|---|---|---|---|
| 48 | `app/models/membership.rb:265` | remove `[user, :rooms]` → `[room, :header_voice_participants]`, only if `Huddle.configured?` | WS7 | ported (`sink::room_removal`, via `Event::Broadcast`) |
| 49 | `app/models/membership.rb:266` | remove `[user, :rooms]` → `[room, :list]` | WS7 | ported (as #48; the golden "revoke A" step) |
| 50 | `app/controllers/rooms/opens_controller.rb:59` create | prepend `:rooms` → `shared_rooms` (users/sidebars/rooms/shared) | WS8 | ported (`open_room_create`) |
| 51 | `…:63` update | replace `:rooms` → `[room, :list]` (users/sidebars/rooms/shared) | WS8 | ported (`open_room_update`) |
| 52 | `…:64` update | replace `:rooms` → `[room, :header]` (rooms/show/header_identity) | WS8 | API ready (`open_room_update(.., Some(header))`; needs `rooms/show/_header_identity`) |
| 53 | `app/controllers/rooms/closeds_controller.rb:77` create | prepend `[user, :rooms]` → `shared_rooms` per member (html rendered once) | WS8 | ported (`closed_room_create`) |
| 54 | `…:83` update | replace `[user, :rooms]` → `[room, :list]` per member | WS8 | ported (`closed_room_update`) |
| 55 | `…:86` update | replace `[user, :rooms]` → `[room, :header]` per member (html) | WS8 | API ready (`closed_room_update(.., Some(header))`; needs the header partial) |
| 56 | `app/controllers/rooms/directs_controller.rb:79` | prepend `[user, :rooms]` → `direct_rooms` per member (users/sidebars/rooms/direct) | WS8 | ported (`direct_room_create`) |
| 57 | `app/controllers/rooms/involvements_controller.rb:39` | remove `[user, :rooms]` → `[room, :list]` (became invisible) | WS8 | ported (`involvement_change`) |
| 58 | `…:42` | prepend `[user, :rooms]` → `stage_rooms` (users/sidebars/rooms/stage) | WS8/WS13 | API ready (logic ported; `Partials::sidebar_row` for stage rooms waits on the partial) |
| 59 | `…:44` | prepend `[user, :rooms]` → `voice_rooms` (users/sidebars/rooms/voice) | WS8/WS13 | API ready, as #58 |
| 60 | `…:46` | prepend `[user, :rooms]` → `board_rooms` (users/sidebars/rooms/board) | WS8/WS12 | API ready, as #58 |
| 61 | `…:48` | prepend `[user, :rooms]` → `shared_rooms` (users/sidebars/rooms/shared) | WS8 | ported |
| 62 | `…:59` | replace `[user, :rooms]` → `[room, :list]` (users/sidebars/rooms/direct), direct room's muted transition | WS8 | ported |
| 63 | `…:62` | replace `[user, :rooms]` → `[room, :list]` (`row_partial_for(room)`, `unread:`), muted transition | WS8 | ported for shared rows; API ready for stage/voice/board rows |
| 64 | `app/controllers/rooms_controller.rb:87` join | prepend `[Current.user, :rooms]` → `shared_rooms` (users/sidebars/rooms/shared, `unread: false`) | WS8 | primitive (the join action isn't ported) |
| 65 | `app/controllers/rooms_controller.rb:214` destroy | remove `:rooms` → `[room, :list]` | WS8 | ported (`room_remove`) |
| 66 | `app/controllers/rooms/reads_controller.rb:9` | cable → `user_<id>_reads` `{room_id}` | WS8 | API ready (`broadcasts::read_room`; the controller isn't ported) |
| 67 | `app/controllers/rooms/reads_controller.rb:21` | cable → `user_<id>_unreads` `{roomId}` | WS8 | primitive (`channel`) |
| 68 | `app/models/rooms/direct.rb:202` | prepend `[member, :rooms]` → `direct_rooms` (users/sidebars/rooms/direct) | WS8 | primitive (`Partials::direct_room` renders it) |
| 69 | `app/models/rooms/direct.rb:205` | replace `[member, :rooms]` → `[room, :list]` (users/sidebars/rooms/direct) | WS8 | primitive |
| 70 | `app/models/rooms/direct.rb:209` | replace `[member, :rooms]` → `[room, :header]` (rooms/show/header_identity, `for_user:`) | WS8 | primitive |
| 71 | `app/models/message_pin.rb:63` | replace* `[room, :messages]` → `dom_id(message, :pin_badge)` (messages/pin_badge) | WS8 | primitive |
| 72 | `app/models/message_pin.rb:70` | replace* `[room, :messages]` → `dom_id(room, :pins_count)` (rooms/pins/count) | WS8 | primitive |
| 73 | `app/models/message_pin.rb:77` | replace* `[room, :messages]` → `dom_id(room, :pins_list)` (rooms/pins/list) | WS8 | primitive |
| 74 | `app/models/poll.rb:152` | replace* `[conversation, :messages]` → `dom_id(poll, :card)` (polls/poll); also from periodic `Poll.close_due!` | WS8 | primitive |

### Link cards and integrations

| # | Site | Call | Owner | State |
|---|---|---|---|---|
| 75 | `app/controllers/message_embed_suppressions_controller.rb:45` | replace* `[source.conversation, :messages]` → `[source, :linkedin_cards]` (linkedin/posts/cards) | WS15 | primitive |
| 76 | `…:47` | replace* … → `[source, :link_embed_cards]` (link_embeds/cards) | WS15 | primitive |
| 77 | `app/models/link_embed.rb:91` | replace* `[conversation, :messages]` → `dom_id(message, target)` (the card partial for `target`) | WS15 | primitive |
| 78 | `app/models/fizzy/card.rb:32` | replace* `[conversation, :messages]` → `dom_id(message, :fizzy_cards)` (fizzy/cards/cards) | WS15 | primitive |
| 79 | `app/models/github/pull_request.rb:116` | replace* `[conversation, :messages]` → `dom_id(message, :github_pr_cards)` (github/pull_requests/cards) | WS15 | primitive |
| 80 | `app/models/github/pull_request.rb:129` | replace* `[thread, :messages]` → `dom_id(thread, :github_pr_header)` (github/pull_requests/thread_header) | WS15 | primitive |
| 81 | `app/models/twitter/post.rb:79` | replace* `[conversation, :messages]` → `dom_id(message, :twitter_cards)` (twitter/posts/cards) | WS15 | primitive |
| 82 | `app/models/event/channel_timeline.rb:24` | replace* `[conversation, :messages]` → `dom_id(message, :event_cards)` (rooms/events/cards) | WS14 | primitive |

### Presence, status and activity

| # | Site | Call | Owner | State |
|---|---|---|---|---|
| 83 | `app/models/calendar/meeting_dispatcher.rb:54` (periodic) | update `[user, :status]` → `dom_id(user, :status_badge)` (users/statuses/badge) | WS14 | primitive (`update(Stream::user_status(id), ..)`) |
| 84 | `app/models/calendar/ooo_dispatcher.rb:66` (periodic) | update `[user, :ooo_notice]` → `dom_id(user, :ooo_notice)` (rooms/show/ooo_notice_line) | WS14 | primitive (`Stream::ooo_notice`) |
| 85 | `app/models/activity_item.rb:224` | cable → `user_<id>_activity` (activity payload) | WS12 | primitive (`channel(&activity::stream_name_for(id), ..)`) |

### Agents

| # | Site | Call | Owner | State |
|---|---|---|---|---|
| 86 | `app/models/agent.rb:298` | replace `agents:all` → `dom_id(agent, :status_badge)` (agents/status_badge) | WS11 | primitive (`Stream::named(agents::STREAM_NAME)`) |
| 87 | `app/models/agent.rb:304` | replace `agents:all` → `dom_id(agent, :directory_row)` (agents/directory/agent) | WS11 | primitive |
| 88 | `app/services/agents/direct_messages.rb:68` | prepend `[user, :rooms]` → `direct_rooms` (users/sidebars/rooms/direct) | WS11 | primitive |
| 89 | `app/services/agents/steps.rb:139` | replace `[conversation, :messages]` → `message` (messages/message) | WS11 | primitive |
| 90 | `app/services/agents/steps.rb:142` | replace `[thread, :messages]` → `dom_id(thread, :agent_steps)` (agent_steps/thread_steps) | WS11 | primitive |

### Huddles, voice and stage

| # | Site | Call | Owner | State |
|---|---|---|---|---|
| 91 | `app/models/huddle_grant.rb:267` | replace `[user, :rooms]` → `[room, :sidebar_voice_participants]` (html) | WS13 | primitive |
| 92 | `app/models/huddle_grant.rb:272` | replace `[room, :messages]` → `[room, :header_voice_participants]` (rooms/huddles/participants, `placement: :header`) | WS13 | primitive |
| 93 | `app/models/huddle_grant.rb:476` | cable → `user_<id>_activity` (call-ended payload) | WS13 | primitive |
| 94 | `app/models/huddle_grant.rb:537` | cable → `user_<id>_activity` `{activityItemId: 0, huddleInvitation: {eventType: "huddle_ended", ..}}` | WS13 | primitive |
| 95 | `app/models/huddle_grant.rb:562` | cable → `user_<id>_activity` `{.., huddleInvitation: {eventType: "huddle_started", .., silent}}` | WS13 | primitive |
| 96 | `app/models/huddle/join_notifier.rb:143` | cable → `user_<id>_huddle_notices` `{huddleJoinNotice: {eventType: "huddle_joined", ..}}` | WS13 | primitive (`huddle_notice::stream_name_for`) |
| 97 | `app/models/huddle/join_notifier.rb:158` | cable → `user_<id>_huddle_notices` `{huddleJoinNotice: {eventType: "huddle_left", ..}}` | WS13 | primitive |
| 98 | `app/models/huddle/join_notifier.rb:188` | cable → `user_<id>_huddle_notices` `{huddleJoinNotice: {eventType: "huddle_ended", roomId}}` | WS13 | primitive |
| 99 | `app/models/stream.rb:122` | append `[user, :rooms]` → `huddle_role_events` (rooms/stage/stream_event) | WS13 | primitive |
| 100 | `app/models/stream.rb:138` | replace `[stage_room, :messages]` → `[stage_room, :stage_live_badge]` (rooms/stage/live_badge); also the reconciler's `end_stale_live!` | WS13 | primitive |
| 101 | `app/models/stream.rb:144` | replace `[user, :rooms]` → `[stage_room, :sidebar_stage_live]` (rooms/stage/live_dot) | WS13 | primitive |
| 102 | `app/models/stream.rb:148` | replace `[user, :rooms]` → `[stage_room, :event_stage_live]` (rooms/events/venue_live_dot) | WS13 | primitive |
| 103 | `app/models/stream.rb:152` | replace `[user, :rooms]` → `[stage_room, :stage_panel]` (rooms/stage/panel_body) | WS13 | primitive |
| 104 | `app/controllers/rooms/call_moderation_controller.rb:109` | replace `[user, :rooms]` → `[room, :stage_roster]` (rooms/stage/roster, `viewer:`) | WS13 | primitive |
| 105 | `…:121` | append `[user, :rooms]` → `huddle_role_events` (rooms/stage/role_event) | WS13 | primitive |
| 106 | `app/controllers/rooms/stage/hands_controller.rb:72` | replace `[user, :rooms]` → `[room, :stage_roster]` (rooms/stage/roster) | WS13 | primitive |
| 107 | `app/controllers/rooms/stage/roles_controller.rb:71` | replace `[user, :rooms]` → `[room, :stage_roster]` (rooms/stage/roster) | WS13 | primitive |
| 108 | `…:79` | replace `[user, :rooms]` → `[room, :stage_panel]` (rooms/stage/panel_body) | WS13 | primitive |
| 109 | `…:96` | append `[user, :rooms]` → `huddle_role_events` (rooms/stage/role_event) | WS13 | primitive |
| 110 | `app/controllers/rooms/stages_controller.rb:116` | prepend `[user, :rooms]` → `stage_rooms` (html) | WS13 | primitive |
| 111 | `…:122` | replace `[user, :rooms]` → `[room, :list]` (html) | WS13 | primitive |
| 112 | `…:125` | replace `[user, :rooms]` → `[room, :header]` (html) | WS13 | primitive |
| 113 | `app/controllers/rooms/voices_controller.rb:71` | prepend `[user, :rooms]` → `voice_rooms` (html) | WS13 | primitive |
| 114 | `…:77` | replace `[user, :rooms]` → `[room, :list]` (html) | WS13 | primitive |
| 115 | `…:80` | replace `[user, :rooms]` → `[room, :header]` (html) | WS13 | primitive |

## Totals

| State | Calls |
|---|---|
| ported | 18 (#1–3, 8, 10, 11, 48–51, 53, 54, 56, 57, 61, 62, 63 in part, 65) |
| API ready | 15 (#7, 12–19, 52, 55, 58–60, 66) |
| primitive | 80 |
| dead | 2 (#22, 23) |

Streams whose names end in `:messages` or `:threads` (`GUARDED_STREAM_SUFFIXES`) are served only
through `RoomMessagesChannel`, to current members of the room (or the thread's room); the stock
`Turbo::StreamsChannel` refuses them (`RoomStreamsAreAuthorized`). The other Turbo streams above
(`:rooms`, `[user, :rooms]`, `[user, :status]`, `[user, :ooo_notice]`, `agents:all`) stay
signature-only, and the `user_<id>_*` names belong to their channels.
