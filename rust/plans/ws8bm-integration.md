# Message presentation integration

`Presenter::messages(&records)` and `campfire_views::messages::Index { ctx, messages }`
remain the scrolling-page entry point. No presenter signature, `MessageItem` variant or
`Index` field was changed. Set `cache_base_url` to the verified request origin and render
under the app's fragment-cache context.

For the message-owned slot inside `rooms/show`, the additive entry point is:

```rust
presenter.room_message_list(&messages, divider.message_id, divider.count)?
```

WS8b-r's `room_shell::render_show` can supply this string as
`ShellComponents::message_list`. Its `find_messages` owns room/anchor selection and its
`unread_divider` owns membership cursors, counts, scroll/jump facts and read side effects.
This method supplies the exact Rails list-slot indentation and divider bytes. It uses
record IDs to find the divider position; it does not parse cached HTML. The per-viewer
divider sits outside the shared message fragments. Missing or off-page divider IDs
render the ordinary list without a marker.

`room-list.rb` drives nine actual RoomsController requests, records their selected IDs
and unread facts, and renders the list slot read directly from the pinned
`rooms/show.html.erb`. The Rust test checks root-only anchor selection, WS8a's around
windows and this adapter's bytes against those results. Full HTTP room-shell integration
requires the lead's merge of WS8b-r and this hook; it is not claimed by that adapter test.

`UnreadDivider` and `RoomIndex` are additive views. `uncached_message` remains the
individual/broadcast entry point. WS8b-m2 can continue extending message facts without
changing the existing callers.

The message-owned Markdown composer is the additive, stable
`campfire_views::messages::composer::Composer { ctx, facts, scheduled_control }`.
WS8b-r can render it into the room shell's footer/composer slot. It does not call or
replace the old Lexxy `rooms/show/_composer.html` template on this branch.

The merged shell must supply:

- Its normal request `ViewContext`, with verified `base_url`, current viewer, asset resolver,
  signed-stream signer, and request CSRF tokens lent through `Layout::render`.
- `Facts.room_id`, `room_kind`, and the viewer's `room_name` (direct-room display name);
  `thread: None` for the room composer, or `Some(Thread { id, name })` for a pane.
- The complete built-in command names followed by ordered room agent-command names.
  `Presenter::composer_facts(room, viewer, thread, drive)` supplies these read-only facts;
  WS8b-m2 retains the slash/autocomplete endpoints and command execution.
- `DriveFlow::Share` when the Google owner resolves enhanced Picker availability;
  otherwise `Metadata` for a stored drive.file grant or `None`. The additive
  `Presenter::composer_drive_flow(viewer, share_picker_available)` reads only that grant.
- WS8b-m2's rendered `scheduled_messages::ComposerButton { ctx, room_id, thread_id }`
  as trusted `scheduled_control: helpers::Html`. This branch does not implement that
  feature or copy its template. The content controller currently passes an empty child;
  the lead must wire this provider after merging M2 to complete full HTTP pane bytes.

For the optimistic client message template, mount the additive
`channel_threads::PendingTemplate { ctx, user: presenter.user_view(viewer.id)? }`.
Its bytes match the pinned Markdown `messages/_template`; the previous foundation
Lexxy template remains available to its existing callers.

`channel_threads::Conversation` also requires `thread_id`, the parent room's
`updated_at`, the scoped optional anchor ID, selected message items, viewer `UserView`,
ordered thread agent-step facts, composer facts and the schedule child. It signs the
thread-only messages stream and emits a single live region. Reads do not join the viewer.
The committed differential compares this full view with a fixed-token Rails renderer,
with the actual Rails schedule child supplied as the explicit feature input; the HTTP
check verifies windows, headers and live CSRF validity separately. No response mask is used.

## Standalone thread show and PR integration

`GET /rooms/:room_id/threads/:id` routes to `channel_threads::show` and returns 200 for
ordinary reachable threads (the latest 40 replies), using `layouts::Application` or
`layouts::FrameLayout`. The pinned standalone Rails show has no composer or schedule
child; those belong to `/content` and `Conversation`, as documented above. Full fixed-token
template/layout goldens cover ordinary, empty, stale, closed, locked, deleted-starter and
Turbo-frame cases. Live HTTP retains its real session tokens; its owned body and headers
are checked separately. Chrome is an explicit owner input in full-layout goldens; current
`Layout::load` still needs the owner's icons/Google/presence facts for full live-page parity.

The clearly named WS15g call site is
`controllers/channel_threads.rs::render_thread_pull_request_header`. Its output becomes
`campfire_views::channel_threads::Show.pull_request_header`, directly after the thread header
and before its starter, with Rails's surrounding whitespace. Main at the inspected baseline
does not contain WS15g's provider. The call site is explicitly flagged and currently empty;
after the lead's owner merge, resolve `github_pr_thread_pull_request(thread)` there and render
WS15g's `campfire_views::github::thread_header(ctx, room_id, thread_id, card)` with the proper
request context. Do not treat an empty PR slot as acceptance of a populated PR thread.

Work and board HTML show routes return an authorized 501 pending WS12; their JSON read
details retain the existing API. No work/board shell or control policy is implemented here.
