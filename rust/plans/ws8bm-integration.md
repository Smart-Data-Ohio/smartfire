# Merged message list and composer contract

Pinned Rails: `d7c7de92`, plus the approved #163 application layout/assets drift.
The real WS8b-r room shell, M2 schedule/poll/message-link providers, WS15g GitHub
renderer and WS14e event callbacks are now merged. Public presenter and view inputs
remain stable.

## Room list

`Presenter::messages(&records)` and `campfire_views::messages::Index { ctx, messages }`
remain the scrolling-page entry point. The room-owned adapter is:

```rust
presenter.room_message_list(&messages, divider.message_id, divider.count)?
```

`controllers::presenters::room_native::load` calls that adapter inside the app's
fragment-cache context and supplies its bytes to `ShowView.shell.message_list`.
`room_shell::find_messages` owns room/anchor selection; `room_shell::unread_divider`
owns membership cursors, counts and scroll/jump facts. The rooms controller retains
its read side effects. Keep `cache_base_url` at the verified request origin.

The adapter includes the populated list's invitation-expression boundary (`\n    \n`)
and divider indentation. Mount it verbatim. Empty rooms retain the shell's single
newline before the invitation. The divider stays outside shared message fragments;
missing/off-page divider IDs produce an ordinary list. `room_native::message_list`
is the only shell wrapper. Do not duplicate its boundary. `PendingTemplate` already
ends with its source newline and must also be mounted verbatim.

`message_collection_cache_key` retains Rails' exact collection-helper composition.
The stored fragment uses `message_fragment_cache_key`: that collection key followed
by the exact `MessagesController#index` page validator for the message and its reply
source, the individual rendered-record Rails key array (including all user roles,
cards/references, resolved icons, body/attachment identities, poll options/votes and its closed
boolean), projected rendered room labels, then the verified origin. Search room icons
have their own resolved-value suffix; unrelated room touches and unused icons do not
change existing fragment keys. The collection prefix carries the pinned Rails
template digest. Rails' initial room list renders uncached; these complete dependencies
preserve its freshness when Rust reuses a fragment. There is no nested HTML boost cache.
See `ws8bma-review-fixes.md` for the exact Rails input/key/touch/digest source ledger.
Use this stored key for cache witnesses/readbacks. The review oracle records both
Rails compositions and actual room bytes after same-second source edits and renames.

`room-list.rb` covers nine actual Rails room requests, selected IDs, unread facts and
complete container bytes. The merged `native_component_capture_matches_rails_root_selection`
compares all nine list/composer/template components. `rooms::full_page_tests` compares
complete pages; request tokens retain their real session semantics. Provider functions
supply populated GitHub, X, message-link, Fizzy, LinkedIn and link-embed bodies.
No copied provider markup is substituted.

## Composer and thread pane

Keep `campfire_views::messages::composer::Composer { ctx, facts, scheduled_control }`
for the inline pane. `FooterComposer` shares those fields and reproduces the room's
captured `content_for :footer` whitespace. `room_native::components` mounts the inline
composer with the caller's two-space footer prefix. The old foundation Lexxy template
remains available to its existing callers.

The shell supplies:

- The request `ViewContext`: verified `base_url`, current viewer, asset resolver,
  signed-stream signer and CSRF tokens through `Layout::render`.
- `Facts.room_id`, `room_kind` and viewer-specific `room_name`; `thread: None` for
  the room or `Some(Thread { id, name })` for the pane.
- Complete built-in command names followed by ordered room agent-command names,
  using `Presenter::composer_facts(room, viewer, thread, drive)`.
- `DriveFlow::Share` when enhanced Picker is configured for a human; otherwise
  `Metadata` for the stored drive.file grant or `None`, using
  `Presenter::composer_drive_flow(viewer, picker_available)`.
- The real M2 `scheduled_messages::ComposerButton { ctx, room_id, thread_id }` as
  trusted `helpers::Html`. Room rendering supplies `None`; the clearly named
  `channel_threads::render_thread_schedule_control` supplies `Some(thread_id)`.
- `PendingTemplate { ctx, user: presenter.user_view(viewer.id)? }` for optimistic
  messages. It is request-owned and stays outside the shared collection cache.

`Conversation` additionally takes `thread_id`, parent room `updated_at`, scoped anchor,
selected message items, viewer `UserView` and ordered thread agent-step facts. It signs
only the thread messages stream and emits one live region. Reads do not join the viewer.
`conversation_and_room_composer_match_rails_bytes_through_http` now exercises the real
merged schedule child, complete fixed-token pane bodies, scoped windows and headers.

## Standalone thread and providers

`GET /rooms/:room_id/threads/:id` renders ordinary reachable threads with the latest
40 replies and Application/FrameLayout. Its Rails standalone show has no composer;
that lives in `/content`. Fixed-token goldens cover ordinary, empty, stale, closed,
locked, deleted-starter and frame cases. Real HTTP retains valid request tokens.

`channel_threads::render_thread_pull_request_header` calls
`Presenter::github_thread_header`, scopes the mapping by room and thread and delegates
to WS15g's private-safe renderer. It uses the verified origin and retains the lazy
write frame. Stale refresh intent is enqueued after releasing the reader. Root/thread
edit broadcasts also use the real GitHub/X renderer through WS7's publisher and guard.

The merged room has exactly one message container with `role="log"`,
`aria-live="polite"` and `aria-relevant="additions"`. Its current header token submits
all six cached GitHub/reaction/legacy/poll forms. Missing/foreign tokens return 422
with unchanged message, boost, thread and vote counts. Cached fragments contain no
viewer token; a cache witness regression verifies the room mounts the shared value
rather than silently rebuilding it. The separate real thread-header regression remains.
WS14e's callback is exercised by the enabled real legacy PATCH event-reference probe.

## Remaining seams and historical aids

WS12 activity, work and board show/pane seams remain authorized, flagged 501 responses.
Ten named work-controller declarations are owner-blocked. JSON reads keep their
existing behavior. No work/board control policy is implemented here.

`owner-integration-check.py`, `owner-schedule-integration.patch` and
`owner-current-integration.patch` are historical aids for earlier unpublished owner
combinations. Do not apply them over this merge or use them as current acceptance.
Current native controller tests and regenerated pinned Rails goldens are the gate.
Behaviour browser cases have a separate named ledger; no pixel phase or blanket system
sign-off is claimed.
