# Splitting the `campfire` crate

`crates/campfire` is one 100k-line crate (plus 130k lines of tests), so every edit recompiles all
of it, on one codegen pipeline. This plan splits it into a stack of crates that build in
parallel and rebuild only from the edited layer up. Production has served from this code since
2026-10-05: every step is a pure move, with no change to behaviour, HTML, SQL or headers.

Phase 1 (this PR) breaks the module cycles inside the one crate and adds a check that keeps them
broken. Phase 2+ extracts one crate per PR, lowest layer first.

## Crates

Lowest first. Each may depend only on the crates above it in this table (and the existing
workspace crates: `campfire_db`, `campfire_kit`, `campfire_views`, ...).

| # | Crate (layer in `layering_tests.rs`) | Modules | Lines (prod / unit tests) |
|---|---|---|---|
| 1 | `campfire_app` (`app`) | `app` (`App`, `AppState`, `AppCtx`), `config`, `errors`, `security`, `public_policy`, `picker_configuration`, `huddle_readiness`, `huddle`, `account_security`, `net`, `cable`, `queue`, `integrations`, `state`, `icons`, `ruby`, `test_support` | 23.3k / 10.3k |
| 2 | `campfire_web` (`web`) | `controllers::presenters`, `controllers::messages::rendered`, `concerns`, `authentication`, `active_storage`, `mail`, `messaging`, `rich_text` | 16.2k / 6.9k |
| 3 | `campfire_channels` (`channels`) | `channels`, `jobs` | 4.9k / 4.6k |
| 4 | `campfire_messages` (`message_features`, `messages`) | `controllers::message_features`, `controllers::messages` | 1.5k / 0.1k |
| 5 | `campfire_rooms` (`rooms`) | `controllers::rooms`, `controllers::room_categories` | 5.4k / 0.3k |
| 6 | `campfire_people` (`people`) | `controllers::{users, accounts, two_factor, qr_code, sessions, sudos}` | 7.0k / 0.9k |
| 7 | `campfire_controllers` (`controllers`) | every other `controllers::*` module | 12.1k / 0.9k |
| 8 | `campfire` (bin, `server`) | `main.rs`, `server` (boot, router, static files, error pages, `run`, `backup`), `admin`, `controllers` (the router root, `health`, `turbo_native`, `mailbox`) | 2.4k / 1.1k |
| 9 | `campfire_tests` (`harness`), test-only | the harness (`controllers::presenters::test_support`, `app::google_test_support`) and the 420 test files that boot the whole app or reach a higher layer | — / 130k |

`message_features` and `messages` stay separate layers in the check (the latter uses the
former), but are small enough to share a crate. Row 9 is the bulk of the test code: test
modules whose imports reach above their own layer (most call the seeded `Harness`, which boots
the full router). They can't stay as unit tests of a lower crate, so phase 2 moves them into
test crates that depend on `campfire` (see "Tests" below). The test modules that only use their
own layer stay as unit tests where they are: 376 of today's 3,149 tests. The other 2,773 move.

## The check

`crates/campfire/src/layering_tests.rs` (`cargo nextest run -p campfire -E 'test(/^layering_tests::/)'`)
walks the module tree from `main.rs` (including `#[path]` and inline modules), resolves every
`crate::`, `super::` and `self::` path (in `use` trees and in code) to the longest module it
names, and fails on any path to a higher layer. `#[cfg(test)] mod` modules are exempt, since
phase 2 relocates the ones that reach up; `#[cfg(test)]` items in ordinary modules are checked
like any other code, since they'd stay in their crate. A second test runs the walker on a
synthetic tree and requires it to report an upward `use` and an upward `super::` path. Adding
`use crate::controllers::rooms::pins;` to `cable.rs` and `use crate::admin;` to
`controllers/searches.rs` fails it with:

```
2 references to a higher layer (move the code down, or see plans/crate-split-plan.md):
cable.rs:8 (app) -> crate::controllers::rooms::pins (rooms)
controllers/searches.rs:4 (controllers) -> crate::admin (server)
```

The check sees module paths, not item resolution: a path through an imported name is covered by
the `use` that imported it, and a glob re-export would hide its sources (there are none across
layers). Phase 2 makes each boundary a crate boundary, where the compiler enforces it; the check
then shrinks to the layers still sharing a crate, and goes when the last one leaves.

## Cycle edges and how phase 1 broke them

Before phase 1 every layer reached every other: `app` booted the router, `jobs` rendered with
controller code, and controllers called each other's helpers. Each edge below went from the
first module up to the second; it's listed with the fix. "Moved" means the code moved unchanged
to the new module, which the old path re-exports (`pub(crate) use`), so callers' code didn't
change.

**`app` → everything (boot).** Moved `app.rs` from `Booted` onwards (`boot*`, `BootIntegrations`,
`open_database`, the router, static files, error pages, verifiers, `run`, `serve`, `backup`,
`copy_database`) into a new top-level `server` module; `app.rs` keeps `App`, `AppState` and
`AppCtx`. `main` calls `server::run()`, and tests that boot call `crate::server::boot*`.

**`app` → `controllers::accounts::bots::github_connections`** (JSON body parameters for the
router's recognizer). Moved `json_body_params`, `unused_json_param` and
`scoped_json_body_params` to `server::json_params` (they need `controllers::recognize`).

**`app::AppState` → `controllers::presenters::agent_payload`, `mail`, `concerns::{sudo,
two_factor}`** (the state types in `AppState`'s fields). Moved the state types down to `state::*`:
`state::mail::State` and `state::two_factor` are plain moves. `state::sudo` holds the
`GoogleSudo` trait and its state, `state::two_factor` the `GoogleReauthentication` trait, which
`integrations::google` implements as before. `state::agent_payload::State` keeps the installed
adapter as `Arc<dyn Any>`; the presenter's `StateExt` installs an `Arc<dyn MessagePayload>` and
downcasts it back (one `TypeId` comparison per agent payload render; not a hot path).

**`queue`/`jobs` → `channels` (event delivery) and `channels` → `jobs`.** Split `jobs.rs`: the
queue half (`Registry`, queue names, job structs, `request_for`, `discard_missing`,
`runner_config`, `Jobs`, the ad-hoc queue, peer callbacks, and `impl EventSink for Jobs`) moved
to `queue` in layer 1; `jobs` keeps the job bodies and `start`. `Jobs::emit` delivered
`Event::Broadcast`/`DisconnectUser` and board digests by calling `channels::sink::deliver` and
`channels::board_digests::deliver` directly; it now calls them through a `queue::Delivery` pair of
`fn` pointers that `jobs::start` installs once in a `OnceLock`, next to `set_cable`. Hot-path cost:
one atomic load and one indirect call per broadcast, in place of a direct call. Before `start`
there's no cable, so delivery was already a no-op then; the digest path still returns its "not
booted" error.

**`app`/`integrations` → `channels`** (cable types). Moved `Cable`, `CableUser`, `room_gid`,
`user_gid`, `read_rooms_stream_name` and `channels/broadcasts.rs` to `cable` (layer 1).
`channels` re-exports them.

**`integrations` → `jobs`/`controllers`** (job bodies and agent delivery). Moved
`integrations/jobs.rs` to `jobs/integrations.rs`, `integrations/agent_jobs{,/}` to
`jobs/agent_jobs{,/}` and `integrations/agent_payload_tests.rs` to `jobs/`. `web_push_pool` stays
in `integrations`.

**`integrations::net` used by every layer.** Moved to top-level `net` (layer 1);
`integrations` re-exports it.

**`mail`, `jobs` → `controllers::messages`** (saving a message and its attachments). Moved
`save_staged`, `canonicalize_body`, `canonical_body`, `process_attachment`,
`process_attachment_now` and `analyze_attachment` to `messaging::operations`.

**`rich_text`, `integrations::slack` → `controllers::autocompletable`** (icons). Moved `Brand`,
`ICON_CONFIG`, `BRANDS`, `client_icon_names`, `builtin_icon`, `icons`, `room_icon_resolves` to
`icons`.

**`config`, `integrations::github` → `concerns`** (`ruby_to_i`). Moved to `ruby`.

**`channels`, `concerns` → `controllers`** (`MatchedRoute`). Moved into `concerns`.

**`concerns`, `presenters::view_context` → `controllers::ledger_browser_tests`** (arrived with
#253: the test-only switch that turns off forgery protection). Moved `FORGERY_DISABLED` and
`forgery_disabled` to `test_support`.

**`channels`/`jobs`/presenters → controllers' presentation helpers.** Moved each to
`controllers::presenters`, re-exported at the old path:

| Was | Now |
|---|---|
| `controllers::searches::preloads` | `presenters::search_preloads` |
| `controllers::messages::freshness` | `presenters::message_freshness` |
| `controllers::messages::payload` (+ `rails-index-template-digest.txt`) | `presenters::message_payload` |
| `controllers::accounts::bots::input_casts` | `presenters::bot_input_casts` |
| `controllers::rooms::call_navigation` | `presenters::call_navigation` |
| `controllers::users::sidebars::composition` | `presenters::sidebar_composition` |
| `call_channels::{row, row_with_call_facts}`, `channels::huddle_effects::stage_model` | `presenters::calls` |
| `message_features::poll_view` | `presenters::message_parts` |
| `message_features::param_string`, `searches::display_query` | `presenters::params` |
| `rooms::pins::list` | `presenters::pins` |
| `rooms::shell::{NoticeFields, notice_from_fields, notice}` | `presenters::room_shell` |
| `test_support::{with_fixed_render_secrets, fixed_render_secrets}` (test-only) | `presenters::render_secrets` |

No traits or `dyn` were added besides `queue::Delivery` (fn pointers) and the agent payload
slot above.

## Visibility

Phase 1 widened only what the moves needed, all to `pub(crate)`: `queue::AdHocWork`,
`Jobs::{set_app, set_cable}`, `state::mail::Fanout`,
`presenters::room_shell::{NoticeFields, notice_from_fields}`, the `agent_jobs`,
`agent_streaming` and `next6_named` (test-only) modules, `agent_streaming::register`,
`webhook::reply`, `agent_jobs::post_with_network`, `bot_input_casts::json_token_string`, and the
`pub(super)` items of moved whole modules (`datetime`, `load`, `token_string`,
`fixed_render_secrets`, ...).

Phase 2 turns `pub(crate)` into `pub` exactly for the items another crate names, one boundary
per PR (the compiler lists them). Test-only items used across crates (`Partials::boost`,
`Jobs::perform_later` and the ad-hoc queue, `google::State::install*`, `render_secrets`,
`test_support`, the harness) go behind a `test-support` feature that the test crates enable,
so production builds don't compile them. Inherent impls must live with their type's crate;
the analyser found none that cross a layer (the four `impl Job for $name` macro arms are local).

## Builds: Cranelift and the panic tests

`.cargo/config.toml` gets one `[profile.dev.package.<crate>] codegen-backend = "cranelift"`
section per new crate (`campfire_app` ... `campfire_tests`), in the same PR that creates it.

The four panic-recovery tests keep their names (the test crates mirror the original module
paths; see below), so `CAMPFIRE_LLVM_ONLY_TESTS` doesn't change, but their package changes:

| Test | Crate after the split |
|---|---|
| `channels::message_features::tests::origin_is_present_for_commit_callbacks_and_restored_after_errors_and_panics` | `campfire_tests` |
| `jobs::tests::a_panicking_ad_hoc_job_is_logged_and_its_worker_carries_on` | `campfire_tests` |
| `test_support::server_startup_panics_reach_the_reporter_and_join_handle` | `campfire_app` |
| `controllers::agent_review_r5_tests::ws11_proxy_header_oracle_rejects_unapproved_changes` | `campfire_tests` |

A panic unwinds through every crate between the panic and its `catch_unwind`, so the LLVM step
needs LLVM for all the campfire crates, not just the test's own. `CAMPFIRE_LLVM` becomes
`--config rust/ci/llvm.toml`, a file setting `codegen-backend = "llvm"` for each of them, and
the job selects them as below.

## CI: one named test set

Since #254, `rust.yml` runs the whole workspace in 12 nextest slices (`--partition slice:N/12`),
every slice with seeds, so new crates are picked up without a CI edit. What still names the
`campfire` package is the LLVM-only selection (`package(campfire) and ($CAMPFIRE_LLVM_ONLY_TESTS)`,
excluded from the slices and run by the LLVM job) and the `browser`/`ws11ui` overrides in
`rust/.config/nextest.toml`. The first phase-2 PR defines the app's crates once, as the
packages built on `campfire_app`, in `rust/.config/nextest.toml`:

```toml
[profile.ci-llvm]          # the panic-recovery job
inherits = "ci"
default-filter = "rdeps(campfire_app)"
```

The LLVM job then runs `--profile ci-llvm -E "$CAMPFIRE_LLVM_ONLY_TESTS"`, the slices exclude
`rdeps(campfire_app) and ($CAMPFIRE_LLVM_ONLY_TESTS)`, and the overrides' `package(campfire)`
becomes `rdeps(campfire_app)` (nextest 0.9.146, pinned in `ci/Dockerfile`, has both
`default-filter` and `rdeps`). The job's exact-count check (four tests) is unchanged.
`ci/ignored-tests.json` and `ci/ignored-utilities.json` name a package and binary per test: the
ones that move get their new package in the PR that moves them. Phase 1 doesn't edit
`.github/workflows/` or `rust/ci/`.

## Tests

The test files that reach up (the `harness` layer, 420 files) move to `crates/campfire_tests`,
a library crate with only `#[cfg(test)]` code that depends on every campfire crate with the
`test-support` feature. Its `lib.rs` mirrors the original module tree (`mod controllers { mod
rooms { mod ws17_ooo_tests; } }` and so on, with `#[path]` to the moved files), so every test
keeps its current name, nextest selectors and ignored-test lists still match, and
`env!("CARGO_MANIFEST_DIR")`-relative paths are rewritten once (93 uses). Tests reach private
items through `super::`; those become `pub` + `#[doc(hidden)]` behind `test-support`, or the test
stays a unit test if that's the only upward edge. If one 130k-line test crate turns out to be
the critical path, it splits by area (rooms, people, agents, ...), each a separate crate that
compiles in parallel.

## Phase 2 sequence

One extraction per PR, lowest first; each PR moves files with `git mv`, turns the boundary's
`pub(crate)` into `pub`, adds the crate's Cranelift line, and leaves the check passing:

1. `campfire_app` (with `test_support` and its panic test), plus the `ci-llvm` nextest profile,
   `ci/llvm.toml` and the `rust.yml` switch.
2. `campfire_web`.
3. `campfire_channels`.
4. `campfire_messages`.
5. `campfire_rooms`.
6. `campfire_people`.
7. `campfire_controllers`.
8. `campfire_tests` (the harness, the 420 test files and three of the panic tests) and the
   trimmed `campfire` bin; the
   layering check is deleted here, since the compiler now enforces every boundary.

Each PR runs the full CI matrix and is merged before the next starts.

## Test names that changed in phase 1

Only where a whole file moved: `integrations::agent_jobs::*` → `jobs::agent_jobs::*` (93 tests),
`integrations::jobs::*` → `jobs::integrations::*` (4), `integrations::net::*` → `net::*` (39),
`controllers::messages::freshness::*` → `controllers::presenters::message_freshness::*` (1),
`controllers::messages::payload::*` → `controllers::presenters::message_payload::*` (1), and the
inline `integrations::agent_jobs::ws11_recovery_continues_after_one_durable_enqueue_failure` →
`jobs::agent_jobs::recovery_cases::...`. None are panic-recovery tests or in `rust/ci`'s lists.

## Open questions

- `campfire_tests` at 130k lines may be the slowest crate. Splitting it by area is cheap once
  it exists; measure first.
- Do the 420 harness test files keep their place in the source tree (via `#[path]`), next to
  the code they test, or move under `crates/campfire_tests/src/`? `#[path]` keeps diffs and
  blame quiet; moving them is clearer.
- `campfire_app` (23k lines) is the bottleneck every other crate waits on; `integrations` (most
  of it) could later become its own crate between `app` and `web` if measurements show that.
