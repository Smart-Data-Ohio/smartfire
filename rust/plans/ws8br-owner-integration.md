# Room owner integration contract — native adapters mounted, byte acceptance partial

Main `4278cb1e` is already merged. This continuation merges the actual WS17
`41dbe4bd`, WS11 `18c9219c`, WS8bm `b14759da` and WS8bm2 `d24317e8` code with
merge commits. WS9 authentication and request concerns remain the main implementations.
WS8br2 is not merged or implemented here. Its presenter entry points and shell fields
remain stable.

## Members JSON

`rooms/members#index` authenticates through WS9, returns empty 401 for an unsigned
JSON request, denies bot credentials, and finds an alive room through the viewer's
membership. It reads active room users in SQLite `LOWER(name), id` order, then invokes
WS17 `UserStatusSettings::for_ids` and `WorkspacePresenceLease::presence_by_user_id`.
Status text and effective presence come from those owner objects. WS11 `Agent::for_user`
and `working_presence_text` supply agent state: a checked-in, unsuspended agent is
online; its working text, nonblank note, or humanized status supplies the label.
There is no locally implemented presence TTL or agent policy.

Stars are read live, scoped to the viewer. Avatar URLs use the fresh signed-avatar
helper and verified request origin. The JSON field order is exactly `id`, `name`,
`avatar_url`, `bot`, `online`, `presence`, `status`, `starred`. Headers include Rails'
no-store/no-cache policy and its observed Rack-generated ETag (SHA256 of the actual
JSON, first 32 hex characters). The six complete HTTP goldens include headers and
bytes for three rooms and two viewers; no membership or presence write occurs.

## Exact room shell inputs

`controllers/rooms::render_show` supplies the native owner entry points with:

1. A reader connection, actual merged app state, request host, verified request origin
   as `cache_base_url`, and the app's shared fragment store.
2. Root `Message` records from `room_shell::find_messages`: last 40, or up to 40 before
   a same-room root anchor plus the anchor plus 40 after (81 total). Foreign, missing,
   and thread anchors fall back to the last page. Their original IDs, timestamps,
   client IDs, creators, content and owner feature records are not replaced.
3. The current membership's `last_read_message_id` and `unread_at` produce
   `divider.message_id: Option<i64>` and `divider.count: i64`. The exact call is
   `room_native::message_list(&presenter, &messages, divider.message_id, divider.count)` under
   `fragment_cache::with(&app.fragment_cache, ...)`. Its returned string is assigned
   unchanged to `ShowView.shell.message_list = Some(list)`. The adapter calls the unchanged
   owner `presenter.room_message_list` seam and contributes the exact rooms/show prefix
   `"\n    \n"`; standalone owner list callers receive no extra bytes. Viewer dividers remain
   outside the shared per-message cache.
4. `ShowView.room`: ID, owner `RoomKind` (Open/Closed/Direct; voice/stage/board currently
   use Closed until their screen-owner integration), persisted name, viewer display name, resolved
   header identity and involvement. `ShowView.user`: ID, name, title and fresh signed
   avatar URL. The same selected `MessageItem`s identify cached fragments. Other
   inputs are room `updated_at`, the original-room/unpaged invitation predicate,
   account join code, signed `[room_gid, "messages"]` stream name, divider scroll/jump
   facts, and WS17's real per-viewer OOO notice members.
5. Request `ViewContext`: viewer/admin/bot/preferences, account, assets, verified URL,
   referrer/last-room, time zone, flash and chrome. The layout lends actual request
   CSRF and CSP values. Broadcast contexts remain detached.

`presenter.composer_facts(&room, &viewer, None, drive_flow)` supplies WS8bm's root
composer with room ID/kind, viewer-relative domain display name, `thread: None`, the
static slash-command registry followed by room-scoped agent slash commands, and the
Drive flow. Drive consent scopes currently select `None` or `Metadata`; the unresolved
WS14 Picker availability input is explicitly `false`, so configured `Share` acceptance
is still pending.

Inside the request rendering scope, `room_native::components` renders:

- WS8bm2 `scheduled_messages::ComposerButton { ctx, room_id, thread_id: None }`, passed
  as the trusted `scheduled_control` argument to WS8bm `composer::Composer`.
- WS8bm `channel_threads::PendingTemplate { ctx, user: &show.user }`.

The root footer adapter removes the owner composer's one initial source newline and
contributes two spaces, reproducing Rails capture/content_for. The pending-template
adapter preserves Rails' final source newline. The adapted bytes become
`shell.composer = Some(...)` and
`shell.message_template = Some(...)`. The actual HTTP page mounts all three native
components; it does not inject Rails message/composer fragments. Request tests check
selected roots, around-anchor roots, schedule controls, real viewer token ownership,
and different unread boundaries on warm shared fragments.

`ShellComponents` also retains `thread_panel`, `pins_panel`, `poll_builder`,
`huddle_header`, and `ooo_notices` slots. A full owner panel entry point is not present
for all of these. In particular, WS8bm2 currently supplies pin count/list factories,
not a complete pins-panel factory; WS8bm supplies no root thread-panel factory.
WS13 huddle and WS14 configured Picker facts also remain integration work. Do not
reimplement those partials or provider policies in this shell.

## Pin refresh

Root creation/update windows and pin-change timestamps are selected by the refresh
controller. A quiet refresh returns 204 before format negotiation. Only when pins
changed, it calls WS8bm2 `controllers::rooms::pins::list(conn, app, room)` and stores
that actual `pins::List` in `RefreshView.pins`.

`RefreshShow` renders the owner `CountPartial` and `ListPartial` with its current
`ViewContext`, after message append/replace streams. Targets are
`pins_count_<room_param_key>_<id>` and `pins_list_<room_param_key>_<id>`; full STI keys
are preserved. Ordering, excerpts, pin/unpin policy and durable events stay in WS8bm2.
The empty refresh is byte-identical to Rails. Populated owner rendering is compared
with fixed request secrets; the separate real HTTP response's Unpin token is verified
for its viewer and rejected for another viewer. No response bytes are normalized.

## Merged provider inputs

`Presenter::renderable_message` calls main's `link_embeds::components(self, message)`
for persisted Twitter, Fizzy, generic-link and LinkedIn facts, and retains WS8bm2's
`quote_references` alongside them. Its preloaded child presenters share pending fetch
sets and Twitter facts with their parent, so the caller receives every requested ID.
`PageResolver` delegates Twitter existence and signed image paths to main's resolver.
The existing preload pass also batches Fizzy cards, link references and Twitter posts
for selected messages and their quote/reply sources. Card/reference order comes from
the owners' read-only batch seams; frame/card rendering still uses their factories.
Empty provider facts are remembered, so preloaded quote rendering stays query-free
and four versus sixteen complete search-message renders use the same query count.
Root `render_show` returns pending link and Twitter IDs from its reader closure and
calls main's `enqueue_render_fetches` on the writer before returning the page.
A rejected enqueue clears the incomplete rendered fragment cache so a retry can
register those requests again. Actual provider jobs, claims, policy and broadcasts
remain in WS15e. WS11's suspension/fanout and WS17's status callbacks are retained;
main's linked-account disconnect hook runs in the existing user transaction.

## Acceptance boundary

The strict native comparison checks three seeded room root collections and all nine
complete list/composer/pending-template regions. It deliberately exits nonzero on
any difference, with no masks, allowlists, ignored app cases or substituted fragments.
All six composer/template regions and both DM list regions now match exactly.
The remaining populated Designers list differs only in the empty GitHub PR card slot. WS15g's
unmerged adapter exposes `github::message_cards(conn, app, message)` and
`github::cache_stamp(conn, message)`, carried by `MessageComponents.github_cards_html`
and `github_cards_stamp`. Those fields/factories are not on main yet; reconcile them
with the merged provider and quote components rather than replacing either.
Fizzy, generic link embed and LinkedIn now call the WS15e factories merged on main;
their complete container bytes are also asserted through the real room-page HTTP path. `ws8br-native-residual.json` retains
each exact Rails/Rust region; the strict checker still exits 1. The owner message
list and composer internals are unchanged. The app suite proves native
mounting and controller behavior, not byte-identical full-page acceptance.

Browser/system acceptance is inventoried for the end-to-end phase. Previously shipped
DM and inbound-email probes remain tracked, but are not rerun or claimed in this
continuation. The controller source inventory distinguishes named native passes from
actual pinned Rails Minitest runs and from unexecuted browser declarations.
