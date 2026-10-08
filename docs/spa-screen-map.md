# SPA screen map: classic GET route audit

This audit sorts every classic GET route into one of three buckets: it has a row in `crates/spa/src/screens.rs` (the pairs the coexistence redirects and the service worker's push-click mapping use), it stays classic for a stated reason, or it is a **gap** with neither. Each gap needs a screen or a decision before the default flips to the SPA and before the `/app` prefix goes. Update this file when a gap gets one.

Audited against main at `b2a465c8c` (#329 added the people rows; the route table is unchanged since `5eb8e0956`, where the audit was taken).

The classic route declarations are `crates/routes/routes.json` (`table`), compiled into `campfire_routes::TABLE` by `crates/routes/build.rs`. `crates/campfire/src/controllers.rs:210` constructs its ordered dispatch table; `dispatch` at line 628 installs `MatchedRoute.endpoint`, and `recognize` at line 643 treats HEAD as GET and selects the first matching declaration. The audit enumerates all **195 GET declarations** once, in declaration order. Optional `(.:format)` suffixes are suppressed below.

There are **83 page / explicitly retained OAuth routes**: **31 routes with screen-map rows**, **12 stays-classic routes**, and **40 gaps**. `/users/:id` is split: human profiles map to `/app/people/:id`, while bot/agent profiles are an additional **one stays-classic decision** pending PR #320. The QR transfer image is an explicitly retained classic auth endpoint in the image group, not a page. The remaining **112 non-page declarations** appear in **13 grouped rows**. Thus the two tables contain **96 data rows**, covering all 195 declarations. Gaps are recorded for a later decision; this work does not assign new SPA URLs to them.

A direct HTML document counts as a page even if Turbo can also request its content as a frame. The grouped frame row contains endpoints whose purpose is a fragment/pagination read (for example message history, sidebar, rich cards, and Slack status), not the page that embeds it. JSON variants of an otherwise real HTML page do not remove its page row. OAuth starts/callbacks are retained as individual rows because the requested coexistence decisions expressly name them, although some return redirects rather than a document.

## Page routes

| Route | Endpoint | SPA screen, stays classic + reason, or gap |
|---|---|---|
| `/` | `welcome#show` | `/app/` (ported) |
| `/first_run` | `first_runs#show` | **gap** — first-run setup has no SPA row or recorded stays-classic decision |
| `/session/transfers/:id` | `sessions/transfers#show` | **stays classic** — session transfer consumes a signed authentication link; explicitly stays classic |
| `/session/new` | `sessions#new` | **stays classic** — password sign-in; explicitly stays classic |
| `/session/google/callback` | `sessions/google#callback` | **stays classic** — Google sign-in callback; explicitly stays classic |
| `/sudo/new` | `sudos#new` | **stays classic** — sudo reauthentication; explicitly stays classic |
| `/two_factor_setup` | `two_factor/setups#show` | **stays classic** — two-factor enrollment; explicitly stays classic |
| `/two_factor_challenge` | `two_factor/challenges#show` | **stays classic** — two-factor sign-in challenge; explicitly stays classic |
| `/account/bots/:bot_id/credentials` | `accounts/bots/credentials#index` | `/app/admin/bots/:bot_id/credentials` (ported) |
| `/account/bots/:bot_id/grants` | `accounts/bots/grants#index` | `/app/admin/bots/:bot_id/grants` (ported) |
| `/account/bots` | `accounts/bots#index` | `/app/admin/bots` (ported) |
| `/account/bots/new` | `accounts/bots#new` | `/app/admin/bots/new` (ported) |
| `/account/bots/:id/edit` | `accounts/bots#edit` | `/app/admin/bots/:id` (ported) |
| `/account/icons` | `accounts/icons#index` | `/app/admin/icons` (ported) |
| `/account/slack_import/runs/:id/plan` | `accounts/slack_import_runs#plan` | `/app/admin/slack/runs/:id/plan` (ported) |
| `/account/slack_import/runs` | `accounts/slack_import_runs#index` | `/app/admin/slack/runs` (ported) |
| `/account/slack_import/runs/:id` | `accounts/slack_import_runs#show` | `/app/admin/slack/runs/:id` (ported) |
| `/account/slack_import` | `accounts/slack_imports#show` | `/app/admin/slack` (ported) |
| `/account/custom_styles/edit` | `accounts/custom_styles#edit` | `/app/admin/styles` (ported) |
| `/account/audit_log` | `accounts/audit_logs#show` | `/app/admin/audit-log` (ported) |
| `/account/integrations_health` | `accounts/integrations_health#show` | `/app/admin/integrations` (ported) |
| `/account/edit` | `accounts#edit` | `/app/admin` (ported); `/app/admin/people` (ported) |
| `/join/:join_code` | `users#new` | **stays classic** — join-code enrollment; explicitly stays classic |
| `/users/:user_id/profile` | `users/profiles#show` | `/app/settings` (ported); `/app/settings/notifications` (ported); `/app/settings/appearance` (ported); `/app/settings/calls` (ported); `/app/settings/integrations` (ported); `/app/settings/rooms` (ported); `/app/settings/security` (ported); only the `me` spelling maps; numeric user ids keep existing authorization/classic behavior |
| `/users/:user_id/sessions` | `users/sessions#index` | `/app/settings/sessions` (ported); only the `me` spelling maps; numeric user ids keep existing authorization/classic behavior |
| `/users/:user_id/status/edit` | `users/statuses#edit` | `/app/settings/status` (ported); only the `me` spelling maps; numeric user ids keep existing authorization/classic behavior |
| `/users/:user_id/push_subscriptions` | `users/push_subscriptions#index` | `/app/settings/devices` (ported); only the `me` spelling maps; numeric user ids keep existing authorization/classic behavior |
| `/users` | `users#index` | `/app/people` (ported, #329) |
| `/users/:id` | `users#show` | `/app/people/:id` (ported, #329); bot/agent profiles stay classic pending PR #320 (/app/agents/:id) |
| `/agents` | `agents/directory#index` | **gap** — agent directory has no SPA route or row |
| `/agents/:id/events` | `agents/events#ledger` | **gap** — human agent event ledger has no SPA route or row |
| `/agents/:id/approvals` | `agents/approvals#for_agent` | **gap** — human agent approvals history has no SPA route or row |
| `/rooms/:room_id/messages/:message_id/fizzy_cards/new` | `rooms/fizzy/message_cards#new` | **gap** — Fizzy create-card form has no SPA page route or row |
| `/rooms/:room_id/messages/:id/edit` | `messages#edit` | **gap** — SPA has inline editing, but no classic-edit page route/row |
| `/rooms/:room_id/messages/:id` | `messages#show` | **gap** — individual message page has no SPA row (different from the /rooms/:room_id/@:message_id permalink) |
| `/rooms/:room_id/threads/:thread_id/messages/:message_id/fizzy_cards/new` | `rooms/fizzy/message_cards#new` | **gap** — Fizzy create-card form has no SPA page route or row |
| `/rooms/:room_id/threads` | `channel_threads#index` | **gap** — standalone thread list has no SPA page route or row; the SPA room has a threads pane |
| `/rooms/:room_id/threads/new` | `channel_threads#new` | **gap** — board-post creation form has no SPA page route or row; SPA /r/:roomId/t/new is a conversation reply draft, not this board-only endpoint |
| `/rooms/:room_id/threads/:id` | `channel_threads#show` | `/app/r/:room_id/t/:id` (ported) |
| `/rooms/:room_id/files` | `rooms/files#index` | **gap** — classic file listing has no SPA page route or row; SPA files pane exists inside a room |
| `/rooms/:room_id/pins` | `rooms/pins#index` | **gap** — classic pinned-message listing has no SPA page route or row; SPA pins pane exists inside a room |
| `/rooms/:room_id/events/:event_id/attendance` | `rooms/events/attendances#show` | **gap** — attendance form has no SPA page route or row (can render a full document on direct GET) |
| `/rooms/:room_id/events` | `rooms/events#index` | **gap** — classic calendar listing has no SPA page route or row; SPA events pane exists inside a room |
| `/rooms/:room_id/events/new` | `rooms/events#new` | **gap** — calendar event creation form has no SPA page route or row |
| `/rooms/:room_id/events/:id/edit` | `rooms/events#edit` | **gap** — calendar event editing form has no SPA page route or row |
| `/rooms/:room_id/events/:id` | `rooms/events#show` | **gap** — calendar event details page has no SPA page route or row; SPA event card/dialog is not a routable screen |
| `/rooms/:room_id/involvement` | `rooms/involvements#show` | **gap** — notification-involvement form has no SPA page route or row |
| `/rooms/:room_id/@:message_id` | `rooms#show` | `/app/r/:room_id/m/:message_id` (ported); board-room navigation retains its existing SPA-to-classic fallback |
| `/rooms/:id` | `rooms#show` | `/app/r/:id` (ported); board-room navigation retains its existing SPA-to-classic fallback |
| `/rooms/opens/new` | `rooms/opens#new` | **gap** — room creation/editing form has no SPA page route or screen-map row |
| `/rooms/opens/:id/edit` | `rooms/opens#edit` | **gap** — room creation/editing form has no SPA page route or screen-map row |
| `/rooms/closeds/new` | `rooms/closeds#new` | **gap** — room creation/editing form has no SPA page route or screen-map row |
| `/rooms/closeds/:id/edit` | `rooms/closeds#edit` | **gap** — room creation/editing form has no SPA page route or screen-map row |
| `/rooms/directs/new` | `rooms/directs#new` | **gap** — room creation/editing form has no SPA page route or screen-map row |
| `/rooms/directs/:id/edit` | `rooms/directs#edit` | **gap** — room creation/editing form has no SPA page route or screen-map row |
| `/rooms/voices/new` | `rooms/voices#new` | **gap** — room creation/editing form has no SPA page route or screen-map row |
| `/rooms/voices/:id/edit` | `rooms/voices#edit` | **gap** — room creation/editing form has no SPA page route or screen-map row |
| `/rooms/stages/new` | `rooms/stages#new` | **gap** — room creation/editing form has no SPA page route or screen-map row |
| `/rooms/stages/:id/edit` | `rooms/stages#edit` | **gap** — room creation/editing form has no SPA page route or screen-map row |
| `/rooms/boards/new` | `rooms/boards#new` | **gap** — room creation/editing form has no SPA page route or screen-map row |
| `/rooms/boards/:id/edit` | `rooms/boards#edit` | **gap** — room creation/editing form has no SPA page route or screen-map row |
| `/rooms/boards/:board_id/automations` | `rooms/boards/automations#show` | **gap** — board automation editor has no SPA route or row |
| `/messages/:message_id/boosts` | `messages/boosts#index` | **gap** — reactions list has no SPA page route or row |
| `/messages/:message_id/boosts/new` | `messages/boosts#new` | **gap** — reaction form has no SPA page route or row |
| `/messages/:id/edit` | `messages#edit` | **gap** — SPA has inline editing, but no classic-edit page route/row |
| `/messages/:id` | `messages#show` | **gap** — individual message page has no SPA row (different from the /rooms/:room_id/@:message_id permalink) |
| `/saved` | `saved_items#index` | `/app/saved` (ported) |
| `/scheduled_messages` | `scheduled_messages#index` | `/app/scheduled` (ported) |
| `/searches` | `searches#index` | `/app/search` (ported) |
| `/activity` | `activity_items#index` | `/app/activity` (ported) |
| `/work` | `work_threads#index` | `/app/work` (unported) |
| `/threads/:thread_id/work/links` | `threads/work/links#index` | **gap** — work-link editor/list has no SPA route or row |
| `/threads/:thread_id/work/handoff/new` | `threads/work/handoffs#new` | **gap** — work-handoff form has no SPA route or row |
| `/github/app/connect` | `github/app_connections#connect` | **stays classic** — GitHub OAuth start (plain link/redirect); explicitly stays classic |
| `/github/app/callback` | `github/app_connections#callback` | **stays classic** — GitHub OAuth callback; explicitly stays classic |
| `/slack/oauth/start` | `slack/oauth#start` | **stays classic** — Slack OAuth start remains a plain classic link; explicitly stays classic |
| `/slack/oauth/callback` | `slack/oauth#callback` | **stays classic** — Slack OAuth callback; explicitly stays classic |
| `/slack/imports` | `slack/imports#index` | `/app/settings/slack` (ported) |
| `/slack/imports/:id` | `slack/imports#show` | `/app/settings/slack/:id` (ported) |
| `/google/callback` | `google/connections#callback` | **stays classic** — Google integration OAuth callback; explicitly stays classic |
| `/about` | `public_pages#about` | **gap** — public About page has no SPA route, row, or recorded stays-classic decision |
| `/privacy` | `public_pages#privacy` | **gap** — public Privacy page has no SPA route, row, or recorded stays-classic decision |
| `/terms` | `public_pages#terms` | **gap** — public Terms page has no SPA route, row, or recorded stays-classic decision |

## Not a page (complete grouped declarations)

Each group lists every declaration it covers, paired with its endpoint. These are excluded from page-port decisions, not omitted from the audit.

| Route kind | Endpoint / complete route list | Why it is not a page |
|---|---|---|
| JSON-only API and polling responses (37) | `/internal/huddle/grants/:id` — `internal/huddle#show`<br>`/users/huddle_presence` — `users/huddle_presence#show`<br>`/users/presence` — `users/presences#show`<br>`/autocompletable/users` — `autocompletable/users#index`<br>`/autocompletable/icons` — `autocompletable/icons#index`<br>`/autocompletable/slash_commands` — `autocompletable/slash_commands#index`<br>`/agents/me` — `agents#me`<br>`/agents/events` — `agents/events#index`<br>`/agents/approvals` — `agents/approvals#index`<br>`/agents/approvals/:id` — `agents/approvals#show`<br>`/agents/context` — `agents/contexts#show`<br>`/agents/work` — `agents/work#index`<br>`/agents/work/:id` — `agents/work#show`<br>`/rooms/:room_id/agents/polls/:id` — `agents/polls#show`<br>`/rooms/:room_id/agents/posts` — `agents/posts#index`<br>`/agents/fizzy/boards` — `agents/fizzy/boards#index`<br>`/agents/fizzy/boards/:id` — `agents/fizzy/boards#show`<br>`/agents/fizzy/cards/search` — `agents/fizzy/cards#search`<br>`/agents/fizzy/cards/:account_id/:number` — `agents/fizzy/cards#show`<br>`/rooms/:room_id/messages/:id/actions` — `messages#actions`<br>`/rooms/:room_id/messages/:id/forward_source` — `message_forward_sources#forward_source`<br>`/rooms/:room_id/messages/:message_id/forwards/destinations` — `message_forwards#destinations`<br>`/rooms/:room_id/threads/:thread_id/messages/:id/actions` — `channel_thread_messages#actions`<br>`/rooms/:room_id/threads/:thread_id/messages/:id/forward_source` — `message_forward_sources#forward_source`<br>`/rooms/:room_id/threads/:thread_id/messages/:message_id/forwards/destinations` — `message_forwards#destinations`<br>`/rooms/:room_id/:bot_key/messages` — `messages/by_bots#index`<br>`/rooms/:room_id/members` — `rooms/members#index`<br>`/rooms/:room_id/drive_recipients` — `rooms/drive_recipients#index`<br>`/rooms/:room_id/huddle/participants` — `rooms/huddles#participants`<br>`/rooms/:room_id/huddle` — `rooms/huddles#show`<br>`/messages/:id/forward_source` — `message_forward_sources#forward_source`<br>`/messages/:message_id/forwards/destinations` — `message_forwards#destinations`<br>`/switcher` — `switchers#show`<br>`/room_categories` — `room_categories#index`<br>`/activity/unread_count` — `activity_items#unread_count`<br>`/google/drive/files` — `google/drive_files#index`<br>`/google/drive/files/:id` — `google/drive_files#show` | Controllers return machine-readable data; no HTML document screen. |
| Unavailable declarations (404/500) (23) | `/first_run/new` — `first_runs#new`<br>`/first_run/edit` — `first_runs#edit`<br>`/session/edit` — `sessions#edit`<br>`/session` — `sessions#show`<br>`/account/users/new` — `accounts/users#new`<br>`/account/users/:id/edit` — `accounts/users#edit`<br>`/account/users/:id` — `accounts/users#show`<br>`/account/bots/:id` — `accounts/bots#show`<br>`/account/new` — `accounts#new`<br>`/account` — `accounts#show`<br>`/users/:user_id/profile/new` — `users/profiles#new`<br>`/users/:user_id/profile/edit` — `users/profiles#edit`<br>`/users/:user_id/push_subscriptions/new` — `users/push_subscriptions#new`<br>`/users/:user_id/push_subscriptions/:id/edit` — `users/push_subscriptions#edit`<br>`/users/:user_id/push_subscriptions/:id` — `users/push_subscriptions#show`<br>`/rooms/:room_id/messages/new` — `messages#new`<br>`/rooms/:room_id/settings` — `rooms/settings#show`<br>`/rooms/new` — `rooms#new`<br>`/rooms/:id/edit` — `rooms#edit`<br>`/rooms/directs/:id` — `rooms/directs#show`<br>`/messages/:message_id/boosts/:id/edit` — `messages/boosts#edit`<br>`/messages/:message_id/boosts/:id` — `messages/boosts#show`<br>`/messages/new` — `messages#new` | Pinned route-table action_not_found / missing_controller responses, plus the pinned directs#show nil-room failure; no working page to map. |
| Turbo-stream-only refresh / pagination responses (2) | `/account/users` — `accounts/users#index`<br>`/rooms/:room_id/refresh` — `rooms/refreshes#show` | Turbo Stream payloads, not navigable document pages. |
| HTML frame / fragment / pagination responses (12) | `/account/slack_import/runs/:id/status` — `accounts/slack_import_runs#status`<br>`/users/:id/card` — `users/cards#show`<br>`/users/:user_id/sidebar` — `users/sidebars#show`<br>`/rooms/:room_id/messages` — `messages#index`<br>`/rooms/:room_id/threads/:id/content` — `channel_threads#content`<br>`/rooms/:room_id/threads/:thread_id/messages` — `channel_thread_messages#index`<br>`/rooms/:room_id/message_links/:id` — `rooms/message_links#show`<br>`/rooms/:room_id/github/pull_request_write_actions/:id` — `github/pull_request_write_actions#show`<br>`/rooms/:room_id/github/pull_requests/:id/card` — `rooms/github/pull_request_cards#show`<br>`/rooms/:room_id/fizzy/cards/:id/card` — `rooms/fizzy/cards#show`<br>`/messages` — `messages#index`<br>`/slack/imports/:id/status` — `slack/imports#status` | Embedded HTML reads: profile card, sidebar, history/content pagination, rich cards, write actions, and Slack status polling. |
| Image responses (QR transfer image stays classic) (5) | `/account/logo` — `accounts/logos#show`<br>`/qr_code/:id` — `qr_code#show`<br>`/users/:user_id/avatar` — `users/avatars#show`<br>`/icons/:name` — `workspace_icons#show`<br>`/embeds/image/:signed` — `embeds/images#show` | Logo/avatar/icon/unfurl image bytes; /qr_code/:id is the explicitly retained classic transfer QR SVG endpoint. |
| Protocol method refusal (MCP GET: 405) (1) | `/agents/mcp` — `agents/mcp#method_not_allowed` | MCP uses POST; GET returns method-not-allowed. |
| Navigation aliases / redirects (no page body) (13) | `/rooms/:room_id/threads/:thread_id/messages/:id` — `channel_thread_messages#show`<br>`/rooms` — `rooms#index`<br>`/rooms/opens` — `rooms/opens#index`<br>`/rooms/opens/:id` — `rooms/opens#show`<br>`/rooms/closeds` — `rooms/closeds#index`<br>`/rooms/closeds/:id` — `rooms/closeds#show`<br>`/rooms/directs` — `rooms/directs#index`<br>`/rooms/voices` — `rooms/voices#index`<br>`/rooms/voices/:id` — `rooms/voices#show`<br>`/rooms/stages` — `rooms/stages#index`<br>`/rooms/stages/:id` — `rooms/stages#show`<br>`/rooms/boards` — `rooms/boards#index`<br>`/rooms/boards/:id` — `rooms/boards#show` | Existing controller redirect reaches /rooms/:id (or a room with thread query); the room screen row applies there. No new shortcut behavior is added. |
| Poll reads (JSON / Turbo Stream; HTML redirects to the room) (1) | `/rooms/:room_id/polls/:id` — `rooms/polls#show` | Poll data/update transport; HTML reaches the room instead of rendering a poll page. |
| Classic worker and manifest (2) | `/webmanifest` — `pwa#manifest`<br>`/service-worker` — `pwa#service_worker` | Installed-app metadata and JavaScript, not documents. |
| Health probe (1) | `/up` — `rails/health#show` | Health response only. |
| Turbo Native navigation protocol responses (3) | `/recede_historical_location` — `turbo/native/navigation#recede`<br>`/resume_historical_location` — `turbo/native/navigation#resume`<br>`/refresh_historical_location` — `turbo/native/navigation#refresh` | Native navigation bridge protocol bodies, not product screens. |
| Disabled mailbox ingress / conductor responses (404/403) (5) | `/rails/action_mailbox/mandrill/inbound_emails` — `action_mailbox/ingresses/mandrill/inbound_emails#health_check`<br>`/rails/conductor/action_mailbox/inbound_emails` — `rails/conductor/action_mailbox/inbound_emails#index`<br>`/rails/conductor/action_mailbox/inbound_emails/new` — `rails/conductor/action_mailbox/inbound_emails#new`<br>`/rails/conductor/action_mailbox/inbound_emails/:id` — `rails/conductor/action_mailbox/inbound_emails#show`<br>`/rails/conductor/action_mailbox/inbound_emails/sources/new` — `rails/conductor/action_mailbox/inbound_emails/sources#new` | Campfire disables these framework endpoints; controller emits head 404/403. |
| Storage files and file redirects (7) | `/rails/active_storage/blobs/redirect/:signed_id/*filename` — `active_storage/blobs/redirect#show`<br>`/rails/active_storage/blobs/proxy/:signed_id/*filename` — `active_storage/blobs/proxy#show`<br>`/rails/active_storage/blobs/:signed_id/*filename` — `active_storage/blobs/redirect#show`<br>`/rails/active_storage/representations/redirect/:signed_blob_id/:variation_key/*filename` — `active_storage/representations/redirect#show`<br>`/rails/active_storage/representations/proxy/:signed_blob_id/:variation_key/*filename` — `active_storage/representations/proxy#show`<br>`/rails/active_storage/representations/:signed_blob_id/:variation_key/*filename` — `active_storage/representations/redirect#show`<br>`/rails/active_storage/disk/:encoded_key/*filename` — `active_storage/disk#show` | Active Storage blob/representation/disk delivery. |

## Server routes outside the classic table

`crates/campfire/src/server.rs:260` mounts `/cable` (WebSocket transport), conditional `/api/v1/*` JSON routes, conditional `/app` and `/app/*` (SPA shell and embedded files), and static public files including `/assets/*`, before the classic dispatcher. It adds no separate production classic GET page. Its explicit CSP/calendar/GitHub webhooks are POST; the special `/agents/mcp` binding delegates to the already audited endpoint; its disk fallback delegates to the storage controller. `/test_session` is a test-only browser fixture, not a production page. Consequently there are no extra production classic page routes missing from the 195-declaration inventory.

## Screen-map findings and changes

- All currently built classic-backed SPA routes in `frontend/src/router.tsx` already have `screens.rs` rows. The two intentional SPA-only exceptions are the design gallery and `/app/r/:roomId/t/new?parent=...`; that draft is not the classic board-only `channel_threads#new` form. A new reverse-direction router test now checks this completeness in addition to the existing screen-to-router check.
- No screen-map row was added and no existing row or ported flag changed. Existing `/work` remains unported; conditional board-room fallback remains in the room route.
- `/users` and `/users/:id` map to `/app/people` and `/app/people/:id` since #329, so `/users/:id` push clicks open the person page. Bot/agent profiles retain the requested classic decision until PR #320 lands.
- Slack issues are **already ported here**: the classic administrator run page `/account/slack_import/runs/:id?page=N` is the `accounts/slack_import_runs#show` row, routed to `/app/admin/slack/runs/:id?page=N`. `frontend/src/features/slack/slack-run.tsx:326` implements issues with “Older issues” pagination, and the run router validates `page`. There is no separate classic issue-page GET declaration. The administrator and personal `/status` endpoints remain classic polling fragments and are grouped above; they do not need screen rows. No Slack behavior was changed by this audit.
- `classicToSpaUrl(path, origin)` reads the same generated screen map through `spaUrlFor`; it maps only ported same-origin paths and appends the original parsed query and fragment without URLSearchParams rewriting. It returns null for unported/unknown/foreign/malformed paths, so the worker can keep them as given. Existing regular classic-link mapping still removes the `classic` parameter as before.

## Gaps (undecided)

- `/first_run` — `first_runs#show`
- `/agents` — `agents/directory#index`
- `/agents/:id/events` — `agents/events#ledger`
- `/agents/:id/approvals` — `agents/approvals#for_agent`
- `/rooms/:room_id/messages/:message_id/fizzy_cards/new` — `rooms/fizzy/message_cards#new`
- `/rooms/:room_id/messages/:id/edit` — `messages#edit`
- `/rooms/:room_id/messages/:id` — `messages#show`
- `/rooms/:room_id/threads/:thread_id/messages/:message_id/fizzy_cards/new` — `rooms/fizzy/message_cards#new`
- `/rooms/:room_id/threads` — `channel_threads#index`
- `/rooms/:room_id/threads/new` — `channel_threads#new`
- `/rooms/:room_id/files` — `rooms/files#index`
- `/rooms/:room_id/pins` — `rooms/pins#index`
- `/rooms/:room_id/events/:event_id/attendance` — `rooms/events/attendances#show`
- `/rooms/:room_id/events` — `rooms/events#index`
- `/rooms/:room_id/events/new` — `rooms/events#new`
- `/rooms/:room_id/events/:id/edit` — `rooms/events#edit`
- `/rooms/:room_id/events/:id` — `rooms/events#show`
- `/rooms/:room_id/involvement` — `rooms/involvements#show`
- `/rooms/opens/new` — `rooms/opens#new`
- `/rooms/opens/:id/edit` — `rooms/opens#edit`
- `/rooms/closeds/new` — `rooms/closeds#new`
- `/rooms/closeds/:id/edit` — `rooms/closeds#edit`
- `/rooms/directs/new` — `rooms/directs#new`
- `/rooms/directs/:id/edit` — `rooms/directs#edit`
- `/rooms/voices/new` — `rooms/voices#new`
- `/rooms/voices/:id/edit` — `rooms/voices#edit`
- `/rooms/stages/new` — `rooms/stages#new`
- `/rooms/stages/:id/edit` — `rooms/stages#edit`
- `/rooms/boards/new` — `rooms/boards#new`
- `/rooms/boards/:id/edit` — `rooms/boards#edit`
- `/rooms/boards/:board_id/automations` — `rooms/boards/automations#show`
- `/messages/:message_id/boosts` — `messages/boosts#index`
- `/messages/:message_id/boosts/new` — `messages/boosts#new`
- `/messages/:id/edit` — `messages#edit`
- `/messages/:id` — `messages#show`
- `/threads/:thread_id/work/links` — `threads/work/links#index`
- `/threads/:thread_id/work/handoff/new` — `threads/work/handoffs#new`
- `/about` — `public_pages#about`
- `/privacy` — `public_pages#privacy`
- `/terms` — `public_pages#terms`
