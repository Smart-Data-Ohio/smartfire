# Broadcast contract

Every broadcast our Rails app makes, with the workstream that owns it and where the port stands.
Later waves port a domain by making its rows `ported`, without changing the frames: action, target,
stream and partial must stay exactly as listed here.

## Current JSON publication

Use `campfire_app::cable::Broadcasts` for domain events. `campfire_api::install` always
installs the JSON renderer and sync socket at boot. Model callbacks enqueue typed broadcast
requests after commit; `campfire_channels::channels::sink` publishes them in commit order,
preserving viewer scope, unread changes and delete-before-disconnect ordering.
Action Cable framing and authentication remain; HTML stream publication is retired.

The census below records the former Rails producers for provenance, not current renderer APIs.

## States

- **ported**: Rust makes this broadcast today and a test pins its envelope; full rendered HTML parity remains partial where the view is inherited.
- **API ready**: a named `Broadcasts` method exists; the caller or its partial doesn't yet (noted).
- **waiting on its domain**: the domain isn't in Rust yet. Its owner calls the primitives with the stream,
  target and partial listed here.
- **dead**: never called in our Ruby.

Owners: WS7 cable; WS8 core messaging (rooms, memberships, messages, threads, pins, polls, quotes);
WS11 agents; WS12 boards and activity; WS13 huddles, voice and stage; WS14 calendar, events, Drive,
meeting status and OOO; WS15 GitHub, Fizzy, X, LinkedIn and link embeds. WS6 owns the views that
the partials below need.

## The calls

There are 115 call sites in `app/`, `lib/` and `config/`, found with a search over `broadcast_{append,prepend,replace,update,remove,before,after,refresh,action,render}(_later)?_to`,
`Turbo::StreamsChannel.broadcast_*`, `ActionCable.server.broadcast`, `*Channel.broadcast_to` and a
bare `broadcast_to`. The lead and independent review confirmed 115 primitive call sites at reference pin `fec615be`; the brief's 140 figure was incorrect. None use `_later`.

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
| 4 | `:10` `broadcast_stream_start` | append, as #3, no unread | WS11 | waiting on its domain (`append(Stream::conversation, conversation_messages_target, html)`) |
| 5 | `:26` `broadcast_stream_update` | replace `[conversation, :messages]` → `message` (messages/message) | WS11 | waiting on its domain (`replace(Stream::conversation, message_dom_id(m, None), html)`) |
| 6 | `:33` `broadcast_stream_final` | as #5 | WS11 | waiting on its domain |
| 7 | `:42` `broadcast_reactions_replace` | replace* `[conversation, :messages]` → `dom_id(message, :boosts)` (messages/boosts/reactions) | WS8 | API ready (`message_reactions_replace`; needs `messages/boosts/_reactions`, and the Rust boosts controller must call it, see #20) |
| 8 | `:48` `broadcast_remove` | remove `[conversation, :messages]` → `message` | WS8 | ported (`message_remove`; MessagesController#destroy and `RemoveBannedContent` for `User::Bannable`) |
| 9 | `:58` `broadcast_quote_cards_replace` | replace* `[conversation, :messages]` → `dom_id(message, :message_link_cards)` (messages/message_links/cards) | WS8 | waiting on its domain |
| 10 | `:72` `broadcast_unread_room` | cable → `user_<id>_unreads` `{roomId}` per member, muted members not mentioned left out | WS8 | ported (inside `message_create`) |

### MessagesController (`app/controllers/messages_controller.rb`)

| # | Site | Call | Owner | State |
|---|---|---|---|---|
| 11 | `:91` update | replace* `[@room, :messages]` → `[message, :presentation]` (messages/presentation) | WS8 | ported (`message_replace`) |
| 12 | `:92` update | replace* `[@room, :messages]` → `[message, :meta]` (messages/meta) | WS8 | API ready (`message_part_replace(.., "meta", html)`; needs `messages/_meta`) |
| 13 | `:97` update | replace* `[@room, :messages]` → `[message, :github_pr_cards]` (github/pull_requests/cards) | WS15 | API ready (`message_part_replace`) |
| 14 | `:98` update | … `[message, :twitter_cards]` (twitter/posts/cards) | WS15 | API ready |
| 15 | `:99` update | … `[message, :message_link_cards]` (messages/message_links/cards) | WS8 | API ready |
| 16 | `:100` update | … `[message, :fizzy_cards]` (fizzy/cards/cards) | WS15 | API ready |
| 17 | `:101` update | … `[message, :linkedin_cards]` (linkedin/posts/cards) | WS15 | API ready |
| 18 | `:102` update | … `[message, :link_embed_cards]` (link_embeds/cards) | WS15 | API ready |
| 19 | `:104` update | … `[message, :drive_attachments]` (messages/drive_attachments) | WS14 | API ready |
| 20 | `:236` reply tombstones | replace* `[reply.conversation, :messages]` → `reply` (messages/message) | WS8 | waiting on its domain |
| 21 | `:247` | cable → `user_<id>_unread_threads` `{threadId, roomId, refreshOnly: true}` | WS8 | waiting on its domain (`channel(&unread_threads::stream_name_for(id), ..)`) |

### Boosts

| # | Site | Call | Owner | State |
|---|---|---|---|---|
| 22 | `app/controllers/messages/boosts_controller.rb:54` | append* `[conversation, :messages]` → `boosts_message_<client_message_id>` (messages/boosts/boost) | WS8 | dead: our `create`/`destroy` call `broadcast_reactions` (#7). The Rust controller still does this upstream append (`boost_create`); WS8 should switch it to #7 |
| 23 | `app/controllers/messages/boosts_controller.rb:59` | remove `[conversation, :messages]` → `boost` | WS8 | dead, as #22 (Rust `boost_remove`) |

### Channel threads

| # | Site | Call | Owner | State |
|---|---|---|---|---|
| 24 | `app/controllers/channel_thread_messages_controller.rb:78` | replace* `[@thread, :messages]` → `[message, :presentation]` (messages/presentation) | WS8 | waiting on its domain (`turbo(Stream::thread_messages(id), Replace, message_dom_id(m, Some("presentation")), .., true)`) |
| 25 | `…:80` | … `[message, :meta]` (messages/meta) | WS8 | waiting on its domain |
| 26 | `…:86` | … `[message, :github_pr_cards]` (github/pull_requests/cards) | WS15 | waiting on its domain |
| 27 | `…:88` | … `[message, :twitter_cards]` (twitter/posts/cards) | WS15 | waiting on its domain |
| 28 | `…:90` | … `[message, :message_link_cards]` (messages/message_links/cards) | WS8 | waiting on its domain |
| 29 | `…:92` | … `[message, :fizzy_cards]` (fizzy/cards/cards) | WS15 | waiting on its domain |
| 30 | `…:94` | … `[message, :linkedin_cards]` (linkedin/posts/cards) | WS15 | waiting on its domain |
| 31 | `…:96` | … `[message, :link_embed_cards]` (link_embeds/cards) | WS15 | waiting on its domain |
| 32 | `…:99` | … `[message, :drive_attachments]` (messages/drive_attachments) | WS14 | waiting on its domain |
| 33 | `…:192` | replace* `[reply.conversation, :messages]` → `reply` (messages/message) | WS8 | waiting on its domain |
| 34 | `…:203` | cable → `user_<id>_unread_threads` `{threadId, roomId, refreshOnly: true}` | WS8 | waiting on its domain |
| 35 | `app/models/channel_thread.rb:882` | replace* `[message.room, :messages]` → `dom_id(message, :thread_indicator)` (messages/thread_indicator) | WS8 | waiting on its domain |
| 36 | `…:1012` | cable → `user_<id>_unreads` `{roomId}` | WS8 | waiting on its domain |
| 37 | `…:1330` | cable → `user_<id>_unread_threads` `{threadId, roomId}` | WS8 | waiting on its domain |

### Boards (`ChannelThread` on board rooms, `Rooms::BoardsController`)

| # | Site | Call | Owner | State |
|---|---|---|---|---|
| 38 | `app/models/channel_thread.rb:895` | replace `[room, :messages]` → `board_column_row_dom_id` (rooms/boards/row, `dom_suffix: :board_column_row`) | WS12 | waiting on its domain |
| 39 | `…:1002` | prepend `[room, :messages]` → `board_posts` (rooms/boards/row) | WS12 | waiting on its domain |
| 40 | `…:1024` | remove `[room, :messages]` → `board_column_row_dom_id` | WS12 | waiting on its domain |
| 41 | `…:1032` | replace `[room, :messages]` → `board_list_row_dom_id` (rooms/boards/row) | WS12 | waiting on its domain |
| 42 | `…:1038` | prepend `[room, :messages]` → `board_column_<work_status>` (rooms/boards/row, column suffix) | WS12 | waiting on its domain |
| 43 | `…:1044` | remove `[room, :messages]` → `board_list_row_dom_id` | WS12 | waiting on its domain |
| 44 | `…:1045` | remove `[room, :messages]` → `board_column_row_dom_id` | WS12 | waiting on its domain |
| 45 | `app/controllers/rooms/boards_controller.rb:71` | prepend `[user, :rooms]` → `board_rooms` (pre-rendered html per user) | WS12 | waiting on its domain |
| 46 | `…:77` | replace `[user, :rooms]` → `[room, :list]` (html) | WS12 | waiting on its domain |
| 47 | `…:80` | replace `[user, :rooms]` → `[room, :header]` (html) | WS12 | waiting on its domain |

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
| 61 | `…:48` | prepend `[user, :rooms]` → `shared_rooms` (users/sidebars/rooms/shared) | WS8/WS6 | API ready (non-empty real partial delivered; fork membership/unread locals and markup remain partial) |
| 62 | `…:59` | replace `[user, :rooms]` → `[room, :list]` (users/sidebars/rooms/direct), direct room's muted transition | WS8/WS6 | API ready (review fix supplies the non-empty real direct partial; complete fork row markup/muted class remains partial) |
| 63 | `…:62` | replace `[user, :rooms]` → `[room, :list]` (`row_partial_for(room)`, `unread:`), muted transition | WS8/WS6 | API ready (shared row delivered but membership/unread locals remain partial; stage/voice/board partials absent) |
| 64 | `app/controllers/rooms_controller.rb:87` join | prepend `[Current.user, :rooms]` → `shared_rooms` (users/sidebars/rooms/shared, `unread: false`) | WS8 | waiting on its domain (the join action isn't ported) |
| 65 | `app/controllers/rooms_controller.rb:214` destroy | remove `:rooms` → `[room, :list]` | WS8 | ported (`room_remove`) |
| 66 | `app/controllers/rooms/reads_controller.rb:9` | cable → `user_<id>_reads` `{room_id}` | WS8 | API ready (`broadcasts::read_room`; the controller isn't ported) |
| 67 | `app/controllers/rooms/reads_controller.rb:21` | cable → `user_<id>_unreads` `{roomId}` | WS8 | waiting on its domain (`channel`) |
| 68 | `app/models/rooms/direct.rb:202` | prepend `[member, :rooms]` → `direct_rooms` (users/sidebars/rooms/direct) | WS8 | waiting on its domain (`Partials::direct_room` renders it) |
| 69 | `app/models/rooms/direct.rb:205` | replace `[member, :rooms]` → `[room, :list]` (users/sidebars/rooms/direct) | WS8 | waiting on its domain |
| 70 | `app/models/rooms/direct.rb:209` | replace `[member, :rooms]` → `[room, :header]` (rooms/show/header_identity, `for_user:`) | WS8 | waiting on its domain |
| 71 | `app/models/message_pin.rb:63` | replace* `[room, :messages]` → `dom_id(message, :pin_badge)` (messages/pin_badge) | WS8 | waiting on its domain |
| 72 | `app/models/message_pin.rb:70` | replace* `[room, :messages]` → `dom_id(room, :pins_count)` (rooms/pins/count) | WS8 | waiting on its domain |
| 73 | `app/models/message_pin.rb:77` | replace* `[room, :messages]` → `dom_id(room, :pins_list)` (rooms/pins/list) | WS8 | waiting on its domain |
| 74 | `app/models/poll.rb:152` | replace* `[conversation, :messages]` → `dom_id(poll, :card)` (polls/poll); also from periodic `Poll.close_due!` | WS8 | waiting on its domain |

### Link cards and integrations

| # | Site | Call | Owner | State |
|---|---|---|---|---|
| 75 | `app/controllers/message_embed_suppressions_controller.rb:45` | replace* `[source.conversation, :messages]` → `[source, :linkedin_cards]` (linkedin/posts/cards) | WS15 | waiting on its domain |
| 76 | `…:47` | replace* … → `[source, :link_embed_cards]` (link_embeds/cards) | WS15 | waiting on its domain |
| 77 | `app/models/link_embed.rb:91` | replace* `[conversation, :messages]` → `dom_id(message, target)` (the card partial for `target`) | WS15 | waiting on its domain |
| 78 | `app/models/fizzy/card.rb:32` | replace* `[conversation, :messages]` → `dom_id(message, :fizzy_cards)` (fizzy/cards/cards) | WS15 | waiting on its domain |
| 79 | `app/models/github/pull_request.rb:116` | replace* `[conversation, :messages]` → `dom_id(message, :github_pr_cards)` (github/pull_requests/cards) | WS15 | waiting on its domain |
| 80 | `app/models/github/pull_request.rb:129` | replace* `[thread, :messages]` → `dom_id(thread, :github_pr_header)` (github/pull_requests/thread_header) | WS15 | waiting on its domain |
| 81 | `app/models/twitter/post.rb:79` | replace* `[conversation, :messages]` → `dom_id(message, :twitter_cards)` (twitter/posts/cards) | WS15 | waiting on its domain |
| 82 | `app/models/event/channel_timeline.rb:24` | replace* `[conversation, :messages]` → `dom_id(message, :event_cards)` (rooms/events/cards) | WS14 | waiting on its domain |

### Presence, status and activity

| # | Site | Call | Owner | State |
|---|---|---|---|---|
| 83 | `app/models/calendar/meeting_dispatcher.rb:54` (periodic) | update `[user, :status]` → `dom_id(user, :status_badge)` (users/statuses/badge) | WS14 | waiting on its domain (`update(Stream::user_status(id), ..)`) |
| 84 | `app/models/calendar/ooo_dispatcher.rb:66` (periodic) | update `[user, :ooo_notice]` → `dom_id(user, :ooo_notice)` (rooms/show/ooo_notice_line) | WS14 | waiting on its domain (`Stream::ooo_notice`) |
| 85 | `app/models/activity_item.rb:224` | cable → `user_<id>_activity` (activity payload) | WS12 | waiting on its domain (`channel(&activity::stream_name_for(id), ..)`) |

### Agents

| # | Site | Call | Owner | State |
|---|---|---|---|---|
| 86 | `app/models/agent.rb:298` | replace `agents:all` → `dom_id(agent, :status_badge)` (agents/status_badge) | WS11 | waiting on its domain (`Stream::named(agents::STREAM_NAME)`) |
| 87 | `app/models/agent.rb:304` | replace `agents:all` → `dom_id(agent, :directory_row)` (agents/directory/agent) | WS11 | waiting on its domain |
| 88 | `app/services/agents/direct_messages.rb:68` | prepend `[user, :rooms]` → `direct_rooms` (users/sidebars/rooms/direct) | WS11 | waiting on its domain |
| 89 | `app/services/agents/steps.rb:139` | replace `[conversation, :messages]` → `message` (messages/message) | WS11 | waiting on its domain |
| 90 | `app/services/agents/steps.rb:142` | replace `[thread, :messages]` → `dom_id(thread, :agent_steps)` (agent_steps/thread_steps) | WS11 | waiting on its domain |

### Huddles, voice and stage

| # | Site | Call | Owner | State |
|---|---|---|---|---|
| 91 | `app/models/huddle_grant.rb:267` | replace `[user, :rooms]` → `[room, :sidebar_voice_participants]` (html) | WS13 | waiting on its domain |
| 92 | `app/models/huddle_grant.rb:272` | replace `[room, :messages]` → `[room, :header_voice_participants]` (rooms/huddles/participants, `placement: :header`) | WS13 | waiting on its domain |
| 93 | `app/models/huddle_grant.rb:476` | cable → `user_<id>_activity` (call-ended payload) | WS13 | waiting on its domain |
| 94 | `app/models/huddle_grant.rb:537` | cable → `user_<id>_activity` `{activityItemId: 0, huddleInvitation: {eventType: "huddle_ended", ..}}` | WS13 | waiting on its domain |
| 95 | `app/models/huddle_grant.rb:562` | cable → `user_<id>_activity` `{.., huddleInvitation: {eventType: "huddle_started", .., silent}}` | WS13 | waiting on its domain |
| 96 | `app/models/huddle/join_notifier.rb:143` | cable → `user_<id>_huddle_notices` `{huddleJoinNotice: {eventType: "huddle_joined", ..}}` | WS13 | waiting on its domain (`huddle_notice::stream_name_for`) |
| 97 | `app/models/huddle/join_notifier.rb:158` | cable → `user_<id>_huddle_notices` `{huddleJoinNotice: {eventType: "huddle_left", ..}}` | WS13 | waiting on its domain |
| 98 | `app/models/huddle/join_notifier.rb:188` | cable → `user_<id>_huddle_notices` `{huddleJoinNotice: {eventType: "huddle_ended", roomId}}` | WS13 | waiting on its domain |
| 99 | `app/models/stream.rb:122` | append `[user, :rooms]` → `huddle_role_events` (rooms/stage/stream_event) | WS13 | waiting on its domain |
| 100 | `app/models/stream.rb:138` | replace `[stage_room, :messages]` → `[stage_room, :stage_live_badge]` (rooms/stage/live_badge); also the reconciler's `end_stale_live!` | WS13 | waiting on its domain |
| 101 | `app/models/stream.rb:144` | replace `[user, :rooms]` → `[stage_room, :sidebar_stage_live]` (rooms/stage/live_dot) | WS13 | waiting on its domain |
| 102 | `app/models/stream.rb:148` | replace `[user, :rooms]` → `[stage_room, :event_stage_live]` (rooms/events/venue_live_dot) | WS13 | waiting on its domain |
| 103 | `app/models/stream.rb:152` | replace `[user, :rooms]` → `[stage_room, :stage_panel]` (rooms/stage/panel_body) | WS13 | waiting on its domain |
| 104 | `app/controllers/rooms/call_moderation_controller.rb:109` | replace `[user, :rooms]` → `[room, :stage_roster]` (rooms/stage/roster, `viewer:`) | WS13 | waiting on its domain |
| 105 | `…:121` | append `[user, :rooms]` → `huddle_role_events` (rooms/stage/role_event) | WS13 | waiting on its domain |
| 106 | `app/controllers/rooms/stage/hands_controller.rb:72` | replace `[user, :rooms]` → `[room, :stage_roster]` (rooms/stage/roster) | WS13 | waiting on its domain |
| 107 | `app/controllers/rooms/stage/roles_controller.rb:71` | replace `[user, :rooms]` → `[room, :stage_roster]` (rooms/stage/roster) | WS13 | waiting on its domain |
| 108 | `…:79` | replace `[user, :rooms]` → `[room, :stage_panel]` (rooms/stage/panel_body) | WS13 | waiting on its domain |
| 109 | `…:96` | append `[user, :rooms]` → `huddle_role_events` (rooms/stage/role_event) | WS13 | waiting on its domain |
| 110 | `app/controllers/rooms/stages_controller.rb:116` | prepend `[user, :rooms]` → `stage_rooms` (html) | WS13 | waiting on its domain |
| 111 | `…:122` | replace `[user, :rooms]` → `[room, :list]` (html) | WS13 | waiting on its domain |
| 112 | `…:125` | replace `[user, :rooms]` → `[room, :header]` (html) | WS13 | waiting on its domain |
| 113 | `app/controllers/rooms/voices_controller.rb:71` | prepend `[user, :rooms]` → `voice_rooms` (html) | WS13 | waiting on its domain |
| 114 | `…:77` | replace `[user, :rooms]` → `[room, :list]` (html) | WS13 | waiting on its domain |
| 115 | `…:80` | replace `[user, :rooms]` → `[room, :header]` (html) | WS13 | waiting on its domain |

## Totals (implementation/API coverage, not full rendered HTML parity)

| State | Calls |
|---|---|
| ported | 15 (#1–3, 8, 10, 11, 48–51, 53, 54, 56, 57, 65) |
| API ready | 18 (#7, 12–19, 52, 55, 58–63, 66) |
| waiting on its domain | 80 |
| dead | 2 (#22, 23) |

Streams whose names end in `:messages` or `:threads` (`GUARDED_STREAM_SUFFIXES`) are served only
through `RoomMessagesChannel`, to current members of the room (or the thread's room); the stock
`Turbo::StreamsChannel` refuses them (`RoomStreamsAreAuthorized`). The other Turbo streams above
(`:rooms`, `[user, :rooms]`, `[user, :status]`, `[user, :ooo_notice]`, `agents:all`) stay
signature-only, and the `user_<id>_*` names belong to their channels.

## Source index

Checked by `python3 reference-tools/cable/broadcast_contract.py --check` from the repository root.
Every primitive has a full source location here; call arguments and partials are in the tables above.

| Source | Owner | State |
|---|---|---|
| `app/channels/presence_channel.rb:25` | WS7 | ported |
| `app/channels/typing_notifications_channel.rb:26` | WS7 | ported |
| `app/controllers/channel_thread_messages_controller.rb:192` | WS8 | waiting on its domain |
| `app/controllers/channel_thread_messages_controller.rb:203` | WS8 | waiting on its domain |
| `app/controllers/channel_thread_messages_controller.rb:78` | WS8 | waiting on its domain |
| `app/controllers/channel_thread_messages_controller.rb:80` | WS8 | waiting on its domain |
| `app/controllers/channel_thread_messages_controller.rb:86` | WS15 | waiting on its domain |
| `app/controllers/channel_thread_messages_controller.rb:88` | WS15 | waiting on its domain |
| `app/controllers/channel_thread_messages_controller.rb:90` | WS8 | waiting on its domain |
| `app/controllers/channel_thread_messages_controller.rb:92` | WS15 | waiting on its domain |
| `app/controllers/channel_thread_messages_controller.rb:94` | WS15 | waiting on its domain |
| `app/controllers/channel_thread_messages_controller.rb:96` | WS15 | waiting on its domain |
| `app/controllers/channel_thread_messages_controller.rb:99` | WS14 | waiting on its domain |
| `app/controllers/message_embed_suppressions_controller.rb:45` | WS15 | waiting on its domain |
| `app/controllers/message_embed_suppressions_controller.rb:47` | WS15 | waiting on its domain |
| `app/controllers/messages/boosts_controller.rb:54` | WS8 | dead |
| `app/controllers/messages/boosts_controller.rb:59` | WS8 | dead |
| `app/controllers/messages_controller.rb:236` | WS8 | waiting on its domain |
| `app/controllers/messages_controller.rb:247` | WS8 | waiting on its domain |
| `app/controllers/messages_controller.rb:91` | WS8 | ported |
| `app/controllers/messages_controller.rb:92` | WS8 | API ready |
| `app/controllers/messages_controller.rb:97` | WS15 | API ready |
| `app/controllers/messages_controller.rb:98` | WS15 | API ready |
| `app/controllers/messages_controller.rb:99` | WS8 | API ready |
| `app/controllers/messages_controller.rb:100` | WS15 | API ready |
| `app/controllers/messages_controller.rb:101` | WS15 | API ready |
| `app/controllers/messages_controller.rb:102` | WS15 | API ready |
| `app/controllers/messages_controller.rb:104` | WS14 | API ready |
| `app/controllers/rooms/boards_controller.rb:71` | WS12 | waiting on its domain |
| `app/controllers/rooms/boards_controller.rb:77` | WS12 | waiting on its domain |
| `app/controllers/rooms/boards_controller.rb:80` | WS12 | waiting on its domain |
| `app/controllers/rooms/call_moderation_controller.rb:109` | WS13 | waiting on its domain |
| `app/controllers/rooms/call_moderation_controller.rb:121` | WS13 | waiting on its domain |
| `app/controllers/rooms/closeds_controller.rb:77` | WS8 | ported |
| `app/controllers/rooms/closeds_controller.rb:83` | WS8 | ported |
| `app/controllers/rooms/closeds_controller.rb:86` | WS8 | API ready |
| `app/controllers/rooms/directs_controller.rb:79` | WS8 | ported |
| `app/controllers/rooms/involvements_controller.rb:39` | WS8 | ported |
| `app/controllers/rooms/involvements_controller.rb:42` | WS8/WS13 | API ready |
| `app/controllers/rooms/involvements_controller.rb:44` | WS8/WS13 | API ready |
| `app/controllers/rooms/involvements_controller.rb:46` | WS8/WS12 | API ready |
| `app/controllers/rooms/involvements_controller.rb:48` | WS8/WS6 | API ready |
| `app/controllers/rooms/involvements_controller.rb:59` | WS8/WS6 | API ready |
| `app/controllers/rooms/involvements_controller.rb:62` | WS8/WS6 | API ready |
| `app/controllers/rooms/opens_controller.rb:59` | WS8 | ported |
| `app/controllers/rooms/opens_controller.rb:63` | WS8 | ported |
| `app/controllers/rooms/opens_controller.rb:64` | WS8 | API ready |
| `app/controllers/rooms/reads_controller.rb:21` | WS8 | waiting on its domain |
| `app/controllers/rooms/reads_controller.rb:9` | WS8 | API ready |
| `app/controllers/rooms/stage/hands_controller.rb:72` | WS13 | waiting on its domain |
| `app/controllers/rooms/stage/roles_controller.rb:71` | WS13 | waiting on its domain |
| `app/controllers/rooms/stage/roles_controller.rb:79` | WS13 | waiting on its domain |
| `app/controllers/rooms/stage/roles_controller.rb:96` | WS13 | waiting on its domain |
| `app/controllers/rooms/stages_controller.rb:116` | WS13 | waiting on its domain |
| `app/controllers/rooms/stages_controller.rb:122` | WS13 | waiting on its domain |
| `app/controllers/rooms/stages_controller.rb:125` | WS13 | waiting on its domain |
| `app/controllers/rooms/voices_controller.rb:71` | WS13 | waiting on its domain |
| `app/controllers/rooms/voices_controller.rb:77` | WS13 | waiting on its domain |
| `app/controllers/rooms/voices_controller.rb:80` | WS13 | waiting on its domain |
| `app/controllers/rooms_controller.rb:214` | WS8 | ported |
| `app/controllers/rooms_controller.rb:87` | WS8 | waiting on its domain |
| `app/models/activity_item.rb:224` | WS12 | waiting on its domain |
| `app/models/agent.rb:298` | WS11 | waiting on its domain |
| `app/models/agent.rb:304` | WS11 | waiting on its domain |
| `app/models/calendar/meeting_dispatcher.rb:54` | WS14 | waiting on its domain |
| `app/models/calendar/ooo_dispatcher.rb:66` | WS14 | waiting on its domain |
| `app/models/channel_thread.rb:1002` | WS12 | waiting on its domain |
| `app/models/channel_thread.rb:1012` | WS8 | waiting on its domain |
| `app/models/channel_thread.rb:1024` | WS12 | waiting on its domain |
| `app/models/channel_thread.rb:1032` | WS12 | waiting on its domain |
| `app/models/channel_thread.rb:1038` | WS12 | waiting on its domain |
| `app/models/channel_thread.rb:1044` | WS12 | waiting on its domain |
| `app/models/channel_thread.rb:1045` | WS12 | waiting on its domain |
| `app/models/channel_thread.rb:1330` | WS8 | waiting on its domain |
| `app/models/channel_thread.rb:882` | WS8 | waiting on its domain |
| `app/models/channel_thread.rb:895` | WS12 | waiting on its domain |
| `app/models/event/channel_timeline.rb:24` | WS14 | waiting on its domain |
| `app/models/fizzy/card.rb:32` | WS15 | waiting on its domain |
| `app/models/github/pull_request.rb:116` | WS15 | waiting on its domain |
| `app/models/github/pull_request.rb:129` | WS15 | waiting on its domain |
| `app/models/huddle/join_notifier.rb:143` | WS13 | waiting on its domain |
| `app/models/huddle/join_notifier.rb:158` | WS13 | waiting on its domain |
| `app/models/huddle/join_notifier.rb:188` | WS13 | waiting on its domain |
| `app/models/huddle_grant.rb:267` | WS13 | waiting on its domain |
| `app/models/huddle_grant.rb:272` | WS13 | waiting on its domain |
| `app/models/huddle_grant.rb:476` | WS13 | waiting on its domain |
| `app/models/huddle_grant.rb:537` | WS13 | waiting on its domain |
| `app/models/huddle_grant.rb:562` | WS13 | waiting on its domain |
| `app/models/link_embed.rb:91` | WS15 | waiting on its domain |
| `app/models/membership.rb:265` | WS7 | ported |
| `app/models/membership.rb:266` | WS7 | ported |
| `app/models/message/broadcasts.rb:10` | WS11 | waiting on its domain |
| `app/models/message/broadcasts.rb:26` | WS11 | waiting on its domain |
| `app/models/message/broadcasts.rb:3` | WS8 | ported |
| `app/models/message/broadcasts.rb:33` | WS11 | waiting on its domain |
| `app/models/message/broadcasts.rb:42` | WS8 | API ready |
| `app/models/message/broadcasts.rb:48` | WS8 | ported |
| `app/models/message/broadcasts.rb:58` | WS8 | waiting on its domain |
| `app/models/message/broadcasts.rb:72` | WS8 | ported |
| `app/models/message_pin.rb:63` | WS8 | waiting on its domain |
| `app/models/message_pin.rb:70` | WS8 | waiting on its domain |
| `app/models/message_pin.rb:77` | WS8 | waiting on its domain |
| `app/models/poll.rb:152` | WS8 | waiting on its domain |
| `app/models/rooms/direct.rb:202` | WS8 | waiting on its domain |
| `app/models/rooms/direct.rb:205` | WS8 | waiting on its domain |
| `app/models/rooms/direct.rb:209` | WS8 | waiting on its domain |
| `app/models/stream.rb:122` | WS13 | waiting on its domain |
| `app/models/stream.rb:138` | WS13 | waiting on its domain |
| `app/models/stream.rb:144` | WS13 | waiting on its domain |
| `app/models/stream.rb:148` | WS13 | waiting on its domain |
| `app/models/stream.rb:152` | WS13 | waiting on its domain |
| `app/models/twitter/post.rb:79` | WS15 | waiting on its domain |
| `app/services/agents/direct_messages.rb:68` | WS11 | waiting on its domain |
| `app/services/agents/steps.rb:139` | WS11 | waiting on its domain |
| `app/services/agents/steps.rb:142` | WS11 | waiting on its domain |
