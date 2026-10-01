# Message presentation integration

`Presenter::messages(&records)` and `campfire_views::messages::Index { ctx, messages }`
remain the scrolling-page entry point. No presenter signature, `MessageItem` variant or
`Index` field was changed. Set `cache_base_url` to the verified request origin and render
under the app's fragment-cache context.

For the message-owned slot inside `rooms/show`, the additive entry point is:

```rust
presenter.room_message_list(&messages, divider.message_id, divider.count)?
```

WS8b-r's `controllers::rooms::render_show` supplies this string as
`ShellComponents::message_list`. Its `find_messages` owns room/anchor selection and its
`unread_divider` owns membership cursors, counts, scroll/jump facts and read side effects.
This method supplies the complete contents of Rails's messages container, including
the empty invitation-expression line before the unread branch, its indentation and
divider bytes. Mount the returned string verbatim; do not add that line again. It uses
record IDs to find the divider position; it does not parse cached HTML. The per-viewer
divider sits outside the shared message fragments. Missing or off-page divider IDs
render the ordinary list without a marker.

`room-list.rb` drives nine actual RoomsController requests, records their selected IDs
and unread facts, and captures the actual rendered container in pinned
`rooms/show.html.erb`, including the invitation boundary. The Rust test checks root-only anchor selection, WS8a's around
windows and this adapter's bytes against those results. Full HTTP room-shell integration
requires the lead's owner merge and populated-card providers; it is not claimed by that adapter test.

`UnreadDivider` and `RoomIndex` are additive views. `uncached_message` remains the
individual/broadcast entry point. WS8b-m2 can continue extending message facts without
changing the existing callers.

The message-owned Markdown composer is the additive, stable
`campfire_views::messages::composer::Composer { ctx, facts, scheduled_control }`.
Keep `Composer` for the inline pane. For the room's captured footer use the additive
`messages::composer::FooterComposer { ctx, facts, scheduled_control }`, which shares
the same markup and fields but reproduces Rails's `content_for :footer` whitespace.
The old Lexxy `rooms/show/_composer.html` remains available to its existing callers.

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
  feature or copy its template. The content controller's explicitly flagged
  `render_thread_schedule_control(ctx, room_id, thread_id)` currently returns an empty child;
  wire the provider at this call site after merging M2 to complete full HTTP pane bytes.

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

The lead's concrete schedule integration is in
`reference-tools/messaging/owner-schedule-integration.patch`: it wires the named thread
call site to M2's actual `ComposerButton`, switches WS8b-r's room footer to `FooterComposer`,
and makes the complete pane comparison use the real child provider. Apply it after the
owner merge. It is a patch because the feature module is absent from the WS8bm baseline;
the worker branch still has an explicitly empty schedule call site.

`owner-integration-check.py`, `owner-schedule-integration.patch` and
`owner-current-integration.patch` are historical integration aids for the earlier
published shell revisions `27990da2851f4c056db71c6b430c894307bc6bfe` and
`6dc741c9bd42922914d619f3d87889c63e5b839d`. Refresh their merge handling before using
them with the current main/worker revisions; the old automated checker is not current
acceptance evidence. The patch records the real schedule child, M2's batched pre-cache
fetch intent, sanitized preloaded filenames, old lifecycle payload decoding and verbatim
list/pending-template mounts. Preserve both sides' agent replay/budget policy and
root-versus-thread attachment commit boundaries when resolving overlaps.

The earlier isolated owner merge had eight of nine exact room-component comparisons;
its only difference was Designers' missing 1,089-byte GitHub card. Main #167 is now merged
via `e32d20ab`, and `4955cfb0` verifies the owned card using WS15g's real renderer.
`complete_github_containers_match_rails_in_room_lists_on_cold_and_warm_caches` compares
every complete GitHub container in the three seed room lists, including empty containers,
against the actual pinned Rails room responses on cold and warm fragment caches.
Removing the real provider reproduces the precise 1,089-byte difference. No shell
template change or copied provider markup is needed for WS8b-r to consume the card.
This closes the owned GitHub slot; the complete current owner merge still needs a new
nine-component check. The unmerged M2 message-link placeholder is a separate 157-byte
full-list difference on this worker branch, so these card comparisons do not claim a
whole-list or whole-page pass. The schedule call site remains explicitly empty here
until the lead merges M2's actual provider.

## Standalone thread show and PR integration

`GET /rooms/:room_id/threads/:id` routes to `channel_threads::show` and returns 200 for
ordinary reachable threads (the latest 40 replies), using `layouts::Application` or
`layouts::FrameLayout`. The pinned standalone Rails show has no composer or schedule
child; those belong to `/content` and `Conversation`, as documented above. Full fixed-token
template/layout goldens cover ordinary, empty, stale, closed, locked, deleted-starter and
Turbo-frame cases. The full layout comes from the explicitly approved Rails #163
revision `2e20b24c`; every other oracle field is cross-checked unchanged against
`d7c7de92`. `check-goldens.py` regenerates both references and rejects content drift.
Live HTTP retains its real session tokens; its owned body and headers are checked
separately. `Layout::load` now supplies ordered brand/alias/custom icon names and the
current viewer's latest ten searches through the existing domain provider. Real thread
GETs compare the complete icon meta value and token-free recent-search child against
Rails for two viewers, before and after custom icons. The Clear form keeps its real
request token. Remaining Google/presence/shell inputs require merged owner acceptance;
these scoped comparisons do not claim full live-page parity.

The clearly named WS15g call site is
`controllers/channel_threads.rs::render_thread_pull_request_header`. Its output becomes
`campfire_views::channel_threads::Show.pull_request_header`, directly after the thread header
and before its starter, with Rails's surrounding whitespace. It now calls
`Presenter::github_thread_header`, which scopes the mapping by both thread and parent
room, delegates to WS15g's private-safe `presenters::github::thread_header` and uses the
verified request origin. The controller authorizes room/thread access before rendering.
Stale PR refresh intent is collected during the read and enqueued after the reader is
released, alongside existing message refreshes. WS15g's real lazy write frame is retained.
Actual thread GETs compare complete owned bodies for public, private and unknown PRs;
a signed-in non-member gets 404 without PR content. Card omission is not accepted as
populated-provider parity.

The shared root/thread rendered edit path also takes WS15g's stale refresh intent from
the presenter and enqueues after releasing its reader. This matters even when the human
edit submits only ignored fields and saves unchanged legacy text: Rails still renders the
card and requests a refresh. The existing twelve concurrent message/room-refresh caller
checks are preserved; three actual Rails edits cover a bodyless legacy root and unchanged
rich-text root/thread bodies, with complete responses/row/claim/job comparisons. Durable
queue rejection preserves the successful edit response and rolls back the refresh claim.

Work and board HTML show and pane content routes return an authorized 501 pending WS12;
their JSON show read details retain the existing API. No work/board shell or control policy
is implemented here.
