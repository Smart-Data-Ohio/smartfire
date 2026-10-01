# Room owner integration contract — native full pages and owner seams

Main is merged through `65ad0d39` (#167), including WS15e (#166), the shared
asset-golden helper (#168), board drift (#164/#165), WS17 (#170), attachments (#171),
and the held-listener deflake (#173). The latest merge commit is `2912fe66`.
Earlier merge commits brought in WS11 `18c9219c`, WS8bm `b14759da` and WS8bm2
`d24317e8`. WS9 authentication and request concerns remain the main implementations.
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

`controllers/rooms::render_show` and the full-page acceptance test both call the same
read-only `presenters::room_native::load(conn, app, room, user, message_id,
request_host, cache_base_url)` factory. It returns `NativePage { show, composer,
link_fetches, twitter_fetches, github_refreshes }`. The native owner inputs are:

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
   to `ShowView.shell.message_list = Some(list)`. The adapter calls the unchanged
   owner `presenter.room_message_list` seam and contributes the exact rooms/show prefix
   `"\n    \n"` for populated roots. The owner list seam is also invoked on an empty
   collection; the shell uses Rails' exact empty-room `"\n"` placeholder until the
   message owner supplies identical empty bytes. Standalone owner callers receive
   no extra bytes. Viewer dividers remain
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
Drive flow. Drive consent scopes select `None` or `Metadata`; public
Picker availability comes from WS13's configuration adapter: all of
`GOOGLE_CLIENT_ID`, `GOOGLE_PICKER_API_KEY` and `GOOGLE_CLOUD_PROJECT_NUMBER` must
be nonblank, and the viewer must be human. The same facts populate the layout
meta tags and the root/thread composer input. Eight complete root composer
captures match Rails, with public client-ID escaping checked through actual HTTP.
The public configuration adapter reads no account tokens or client secrets. The
existing linked-account scope predicate is factored into token-free
`Presenter::google_drive_consent`; both composer selection and layout preferences
now use it for the Rails `google-drive-previews` meta fact.

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

The shell now mounts the unchanged WS13 `66e4c66d` thread/poll templates through
`rooms::panels::{ThreadPanel,PollBuilder}` and WS8bm2 `1fdf42a6`'s actual
`pins::PanelPartial`. The thread factory receives the original room view plus
`Presenter::room_display_name(&room,None)` (neutral, including the viewer in an
unnamed DM). The shell contributes Rails' two-space content_for prefix and its trailing newline. PollBuilder
receives the same room ID and current request ViewContext; token generation stays
inside that rendering scope. Pins receives room ID, the full header STI param key
and `MessagePin::count_for_room` from the reader. The count/list/policy remain M2's.
The templates are copied without edits from the owning branches. Fourteen complete
thread/poll/pin captures match the pinned Rails corpus, separately from the real
HTTP viewer-token ownership test and original collapsed work-guide declaration.

Configured WS13 huddle header/sidebar adapters remain integration work. The real
`huddle::Config`, `HuddleGrant::participants_for`,
`controllers::rooms::call_navigation::model`, and stage/participant callback APIs
are absent from merged main. Their pinned inspection source is WS13 `498aa4e6`.
The existing readiness-only boolean is not a replacement for these APIs.
`ShellComponents.huddle_header` remains the stable trusted-HTML entry point;
`LayoutChrome.huddle_configured` is still false. All eight original huddle sidebar
controller declarations stay deferred to WS13; no local grant-policy copy is added.
The public Picker configuration seam is now connected; Google OAuth/token flow
and transport remain WS14's. No huddle policy, grant issuance or message/composer internals
are reproduced locally.

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
sets, GitHub refresh IDs and Twitter facts with their parent, so the caller receives every requested ID.
`PageResolver` delegates Twitter existence and signed image paths to main's resolver.
The existing preload pass also batches Fizzy cards, link references and Twitter posts
for selected messages and their quote/reply sources. Card/reference order comes from
the owners' read-only batch seams; frame/card rendering still uses their factories.
Empty provider facts are remembered, so preloaded quote rendering stays query-free
and four versus sixteen complete search-message renders use the same query count.
Root `render_show` returns pending link/Twitter IDs and GitHub refresh IDs from
`NativePage`, then calls main's `enqueue_render_fetches` and
`pull_requests::refresh_after_render` on the writer before returning the page.
A rejected enqueue clears the incomplete rendered fragment cache so a retry can
register those requests again. Actual provider jobs, claims, policy and broadcasts
remain in WS15e. WS11's suspension/fanout and WS17's status callbacks are retained;
main's linked-account disconnect hook runs in the existing user transaction.

## GitHub merge wiring boundary

The main WS15g factory `presenters::github::message_cards(conn, app, message)` and
`cache_stamp(conn, message)` supply existing `MessageComponents.github_cards_html`
and `github_cards_stamp`. There is no new card template or GitHub policy. Main's
factory fills the previously missing public card slot. `Presenter` children share
an `Rc<RefCell<BTreeSet<i64>>>` refresh set; selected message rendering drains it
once. The active M1 `messages::rendered::broadcast_edit_in` uses these same component
bytes and main's refresh-after-render writer seam.

M2 preload integration batches the selected/quote-source message IDs with persisted
GitHub references once, remembers empty facts for unlinked messages, and calls the
unchanged main factories only for linked messages. `GithubRendering` carries
`html`, `stamp`, and stale PR `refreshes`; registration occurs when a fragment is
actually rendered, rather than on preload. Per-linked-message domain reads remain
main's. M1 should reconcile these adapter fields/shared sets with its in-flight
message work; WS8br does not duplicate that work.

## Acceptance boundary

PR #175 review fixes add real thread HTTP provider calls. The content action passes
the authorized room and thread, the selected last/around-anchor message window,
viewer `UserView`, thread steps, room `updated_at` and `anchor: Option<i64>` to
`Conversation`. Its composer facts use the actual viewer and Drive flow with
`thread: Some(thread)`. Inside the request rendering scope it renders
`scheduled_messages::ComposerButton { ctx, room_id: composer.room_id,
thread_id: Some(thread.id) }` and supplies that Rust output as `scheduled_control`.
The content byte test now goes through the router with no supplied child HTML.

Standalone ordinary thread pages call main's `presenters::github::thread_header`
with the reader, verified rendering origin and thread. The provider resolves the
room/thread PR mapping and owns the public card/files, private or unknown lazy
frame, signed stream and write-actions mount. The adapter records the mapped PR
ID so main's `refresh_after_render` writer seam runs even without a starter.
The real HTTP comparisons include public, private, unknown and unmapped PRs;
queue-failure coverage checks a 200 response and atomic claim/job rollback.

Seven Rails HTML inputs formerly used by two comparison tests are removed: the
thread and root schedule controls, plus the shell's pins panel, thread panel,
pending-message template, composer and poll builder. Four additional complete
Rails HTTP captures cover empty channel, pair-DM, group-DM and open rooms. Their
Rust comparisons run the router and native providers. Only rendering token/nonce
entropy is fixed in a test task-local scope before the request; there is no HTML
input or response rewrite. Empty fixtures preserve the seed room timestamp on
both targets while removing messages through the respective domain callbacks.
The original shell seam/jump unit tests retain synthetic inputs only.

The four full-page fixtures come only from a Rails controller renderer at pinned
`d7c7de92`, with the approved `2e20b24c` application layout. They cover Designers
(David), the David/Kevin pair (David), the seeded group DM (Kevin), and empty HQ
(JZ). Real controller `show` assigns supply membership/unread/OOO/root selection;
Rust calls the same fact factory as its HTTP controller. Actual message list,
composer, pending template, thread/poll/pin panels, layout, shell, head, chrome and
meta tags render natively. No Rails component is substituted in the Rust path.
Global/per-form tokens (`GLOBAL`, `method:action`) and CSP `NONCE` are deterministic
renderer inputs on both targets. Real HTTP token ownership is tested separately.
Full-page comparison uses main's shared asset-golden helper: each live asset digest
must validate against compiled local assets; all surrounding bytes stay exact.

The separate strict native comparison checks all nine seeded complete message-list,
composer and pending-template regions without masks. The former 1089-byte GitHub
slot now comes from main's unchanged factory; there is no GitHub exception in the
Rust test. Configured huddle pages and original system-declaration coverage remain
open. Browser receipts and declaration mappings are separate inventories; no
screenshots, geometry-only comparisons or pixel work are included.
