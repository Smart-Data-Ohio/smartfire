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
