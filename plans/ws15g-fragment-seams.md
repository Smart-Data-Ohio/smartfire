# GitHub fragments for the owning pages

Reference: `d7c7de92`. The functions below render the owned GitHub sections only. `vectors/github_seed_fragments.json` records Rails' exact fragment bytes on the default seed; its four template hashes are checked by `reference-tools/github/seed-fragments.sh`. Forms use ordinary request-local CSRF helpers, never a shared fragment cache.

## WS8b-m: `channel_threads#show`

Wire `controllers::presenters::github::thread_header(conn, ctx, &thread)` into the thread page above the timeline, after the controller authorizes room membership and verifies the thread belongs to that room. It returns an empty string for a non-PR thread, and loads the actual PR mapping, public changed files and discussion link for a PR thread. Private/unknown PRs render only their per-viewer lazy card frame. Treat the returned string as already rendered HTML.

The header contains the lazy write-action frame, `github_write_actions_channel_thread_<id>`, whose source is `/rooms/<room_id>/github/pull_request_write_actions/<pr_id>`. That route is already registered and renders `campfire_views::github::write_actions::WriteActions::render()`, using the viewing human's linked account and request-local form tokens. Do not preload its human forms into the shared header.

Collect the mapped PR id while preparing the page, then call `integrations::github::pull_requests::refresh_after_render(&app.db, ids).await` after releasing the read closure. This shared helper checks staleness and atomically claims/enqueues a fetch, with the same graceful queue-failure policy as room renders. Rails' `github_pr_thread_pull_request` requests this refresh even for a private header.

Default seed: room `654632876`, thread `8`, PR `1`. Both the header (including the loading write frame) and the loaded unlinked write frame match Rails byte for byte. The loaded route is also compared through the real HTTP caller. **Cutover reconciliation (2026-10-04): the owning thread page is implemented on main.** `rust/crates/campfire/src/controllers/channel_threads/github_tests.rs:7` runs `github_thread_show_matches_complete_rails_public_private_and_unknown_bodies` through actual thread GETs, comparing complete public/private/unknown bodies, the write-actions frame and nonmember 404. That test passed in [CI run 37200618245](https://github.com/Smart-Data-Ohio/smartfire/actions/runs/37200618245); see [WS15g-019/046/051/052](ledger-ws14-ws15.json). The earlier 501/WS8b-m handoff was a historical receipt, superseded by the merged page implementation. Other thread variants remain individually listed in [the current open assertions](ledger-ws14-ws15-remaining.md).

## WS8b-r2: `users/profiles/show`

Load `controllers::presenters::github::connection(&app, user.id).await` outside a database read closure. It projects only display fields, evaluates `usable?` with the real encrypted account, and reloads a newly recorded disconnection reason. Pass that projection to `campfire_views::github::connections::profile(&data)`.

The existing profile controller and `ProfileShow::github_panel` already call these functions and the template renders the section after the profile form. Keep that wiring when merging the profile workstream. The separate manual `github_login` editing field/policy remains a deferred profile concern.

## WS11-ui: `accounts/bots/edit`

Load the same account projection for `bot.id`; call `campfire_views::github::connections::bot(&data, bot.id, viewer.is_administrator())`. Only administrators see connect/reconnect/disconnect controls. Nonadministrators may see a usable login, but never a write form or a token.

The existing bot controller, `BotsEdit::github_panel` and template are already wired, before the delete/key controls. Keep those fields and calls when merging WS11-ui. The surrounding bot edit-page authorization and complete-page parity belong to WS11-ui.

## Verification scope

The committed oracle covers PAT/App/unlinked/rejected/blank-reason/escaped values, both App configuration states and both administrator states (24 cases, each containing both connection fragments). Rust verifies exact bytes with fixed form tokens following the established Rails form-token oracle convention; no HTML masks are applied. It separately exercises real encrypted-account projection and the two current seed HTTP pages. Complete thread/profile/bot pages are not claimed byte-identical by these fragment contracts.
