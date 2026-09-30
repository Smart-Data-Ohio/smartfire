# Room HTTP owner integration — partial, merge instructions

WS8br's shell/presenters remain additive. This branch does not merge other workers or
copy their partials/domain policies. The lead merges the branches. The inspected refs
are WS8bm `68f6615b`, WS8bm2 `f168c348`, WS17 `1021be6a`, WS11 `8f338ac6`.

## Members JSON

`rooms/members#index` still needs the owner facts and route adapter. Never substitute
all-offline, empty status or absent-agent defaults. After owner merges, the adapter:

1. Authenticates JSON requests with empty 401 (HTML uses the existing session redirect),
   denies bots through `Before`, and resolves an alive membership-scoped room with 404.
2. Loads only active room members, ordered by SQLite `LOWER(users.name), users.id`.
3. Calls WS17 `UserStatusSettings::for_ids(conn, &ids)` and
   `WorkspacePresenceLease::presence_by_user_id(conn, &ids, now)` on the same reader.
   Calls `effective_presence(lease_state)` and `status_text_display(now)` on those settings.
4. Calls WS11 `Agent::for_user(conn, id)` and `working_presence_text(now)` for associated
   bots. Online is last-seen present and suspended absent, with status precedence
   working-presence text, nonblank status note, humanized status.
5. Reads `user_stars` scoped to the current viewer live on each request. Uses the shared
   fresh avatar URL helper with the verified request origin. Returns only `id`, `name`,
   `avatar_url`, `bot`, `online`, `presence`, `status`, `starred`. Uses Rails' no-store
   headers; no shared fragment cache/ETag and no membership or lease writes.

These owner APIs are absent from WS8br's base. The picker in this branch reads only
association existence and persisted star flags; it does not implement their policies.

## Exactly what the shell currently supplies

`controllers/rooms::render_show` supplies WS8bm's presenter with:

- The reader connection, app state, request host and `cache_base_url` from the verified
  request origin; `app.fragment_cache` scopes message presentation.
- Root room `Message` records selected by `room_shell::find_messages(conn, room.id,
  message_id)`: up to 40 last-page rows, or up to 40 before + anchor + 40 after
  (81 rows) when the anchor is a root in the same room. Foreign/missing/thread anchors
  fall back to the last page.
- `divider.message_id: Option<i64>` and `divider.count: i64`, derived from the current
  user's membership and its `last_read_message_id`/`unread_at` cursors. Out-of-page
  dividers give a jump URL; in-page count above five enables scroll. These facts do not
  belong in shared message fragment keys.
- `ShowView.room` (ID, STI-derived kind, persisted name, viewer display name, resolved
  header identity, involvement), `ShowView.user` (ID, name, title, fresh signed avatar
  URL), selected `MessageItem`s, room updated-at timestamp, original-room invitation
  predicate, account join code and the SHA1-keyed signed room/messages stream name.
- Request `ViewContext`: current viewer/admin/bot/preferences, account identity, assets,
  URL/referrer/last-room, time zone, flash, chrome; request-bound CSRF and CSP nonce are
  lent by the normal page renderer. Broadcasts use detached contexts instead.

The stable list call on WS8bm is:

```rust
let message_list = campfire_views::fragment_cache::with(&app.fragment_cache, || {
    presenter.room_message_list(&messages, divider.message_id, divider.count)
})?;
// Assign Some(message_list) to ShowView.shell.message_list.
```

`ShellComponents` accepts trusted owner-rendered strings for `message_list`, `composer`,
`message_template`, `thread_panel`, `pins_panel`, `poll_builder`, `huddle_header` and
`ooo_notices`. The first three use `Option<String>` to distinguish supplied empty output
from the current fallback. `rooms::room_message_list(ctx, show)` returns supplied bytes
verbatim, or the authorized zero-byte empty-room placeholder. Current HTTP still uses
`ShellComponents::default()`: this is not full native list/composer acceptance.

The current composer fallback is passed `room = &show.room` and request `ViewContext`
through `rooms/show/_composer`; it has no explicit `user` local. The current viewer is
in the context; `ShowView.user` is supplied separately to the client message template.
The owner has not supplied a completed controller-level
composer factory. The inspected WS8bm report explicitly defers composer parity. The
lead must reconcile this factory and its request tokens, plus WS13 huddle/voice facts,
WS17 OOO notices and WS8bm2 poll/pin mounting. Complete region goldens lending Rails
owner fragments prove the surrounding shell only, not these implementations.

## Refresh and pins

The current refresh controller selects root `page_created_since` records, then
`page_updated_since` excluding the new IDs. It returns 204 before format negotiation
when both are empty and `room.pins_changed_at <= since` (or nil). Otherwise it presents
message items under the app cache. `RefreshView` currently lacks pin output: a pin-only
refresh does not yet carry the required count/list streams.

After WS8bm2 merges, call its `controllers::rooms::pins::list(conn, app, room)` seam.
Render its `pins::CountPartial` and `pins::ListPartial`, and wrap those trusted bytes in
replace streams targeting `pins_count_<room_param_key>_<id>` and
`pins_list_<room_param_key>_<id>`. Preserve STI param keys for voice/stage/board rooms.
Do not query pin rows/order/excerpts here or reimplement pin/unpin/note/broadcast policy.
Do not put request CSRF values in detached pin broadcasts. WS8bm2 owns those partials.

Remaining acceptance: real configured members JSON/no-store/authorization; populated
refresh plus pin-only/count/list streams; merged full room HTTP/layout/list/composer
bytes, browser behavior and pixels, including unread/around and each viewer. The new
14-file Rails reference receipt proves the oracle runs, not these deferred Rust cases.
