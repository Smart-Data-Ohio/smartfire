# Smartfire

Smartfire is this Rust app. It started as 37signals' port of stock ONCE Campfire
([basecamp/once-campfire-rust](https://github.com/basecamp/once-campfire-rust), MIT, imported here
with its history by `git subtree`; we don't track it), which we extended until it matched
Smartfire's original Rails app (a fork of [basecamp/once-campfire](https://github.com/basecamp/once-campfire),
MIT). Production has run on it since 2026-10-05, and the Rails app has been removed from the
repository; it survives in the git history and as the recorded vectors, fixtures and seeds the
tests pin. See `README.md` and `docs/rust-port.md` for where things stand.

- **Behavior is defined by this code and its tests.** The golden vectors, frozen seeds and
  recorded fixtures pin what Rails did; change them only together with the behavior they pin.
  Comments that cite `app/...` or other Rails paths are provenance: they name the Rails file the
  code was ported from (find it in the git history before the Rails removal).
- **Drop-in compatible with existing data:** the same SQLite database and schema, storage layout,
  signed/encrypted cookies (so people stay signed in across releases) and environment variables.
- Port-owned frontend changes go in `crates/assets/overrides/`, which shadows the copied Rails
  assets by logical path (`crates/assets/OVERRIDES.md`).

## Frontend and fixture inputs

The app's static inputs live at the repository root:

- `web/`, laid out like the Rails app it came from: `app/assets`, `app/javascript`,
  `vendor/javascript`, `public/`, `config/importmap.rb`, `config/initializers/assets.rb`, the JS
  builders that vendor bundles into `vendor/javascript` (`script/livekit-client`,
  `script/code-highlighter`), the LiveKit gateway (`script/livekit-gateway`) and
  `bin/livekit-local`. `crates/assets/build.rs` reads them, and the `Dockerfile` copies them.
- `fixtures/`: the Rails app's `test/fixtures`, which the tests load.
- `test-support/`: data the tests read that reference tools once held (attachment analyzer inputs,
  agents UI cast inputs, the post-pin status files, Node test adapters).

In the tests, `campfire_db::fixtures::reference_root()` is `web`, `reference_dir()` is
`fixtures`, and `reference_path("public/500.html")` / `reference_path("test/fixtures/files/...")`
maps a Rails-relative path to its copy here.

## Layout

| Path | Package | What |
|---|---|---|
| `crates/rails_compat` | `rails_compat` | Rails signing/encryption/serialization contracts, verified by `vectors/` |
| `crates/kit` | `campfire_kit` | Axum adapter, `Ctx`, params, cookies, session, forgery protection, flash, formats, responses, gzip, and the front server (TLS, ACME, HTTP/2, response cache) |
| `crates/routes` | `campfire_routes` | Path helpers mirroring `config/routes.rb` |
| `crates/db` | `campfire_db` | rusqlite over the existing schema, models, queries, fixtures loader |
| `crates/richtext` | `campfire_richtext` | Action Text content pipeline: sanitize, attachments, autolink, plain text |
| `crates/storage` | `campfire_storage` | Active Storage-compatible blobs, disk service, variants (libvips), previews (ffmpeg) |
| `crates/cable` | `campfire_cable` | Action Cable protocol server, its WebSocket implementation, and in-process pub/sub |
| `crates/assets` | `campfire_assets` | Propshaft-compatible digesting, importmap, vendored JS/CSS, port-owned overrides |
| `crates/views` | `campfire_views` | Askama templates (at the ERB file's relative path) and view helpers |
| `crates/campfire` | `campfire` (bin) | Controllers, router wiring, channels, jobs, integrations |
| `parity/` | — | Frozen test seeds (`parity/seeds`), the pinned Playwright image the browser suites run in, template coverage |
| `reference-tools/` | — | The browser and behaviour harnesses the correctness suites run against Rust |
| `bench/` | — | Load generator, benchmark scripts and recorded results (upstream's, against stock Campfire) |
| `plans/` | — | Upstream's conversion plan and reports, kept for their reasoning |
| `ci/` | — | CI scripts, the correctness suites and their toolchain image (`ci/README.md`) |
| `ops/` | — | The image's backup and restore hooks, and the release tests (`ops/README.md`) |
| `deploy/` | — | Production deploy, GCP and backup scripts (`deploy/README.md`) |
| `docs/` | — | User, operator and developer documentation |

CI for this tree is `.github/workflows/rust.yml`. It runs on every pull request, building unless
only `docs/`, `deploy/`, other workflows or the root prose files changed, and never runs Ruby,
Rails or a reference image. Every ordinary nextest group and runnable doctest is required; tests run as twelve nextest partitions beside the frozen-seed check, clippy/doctest,
LLVM panic-recovery and stable-toolchain jobs, and the `Rust port` job fails unless all of
them succeeded.
`ci/cargo.sh` uses the Dockerfile's pinned toolchain/media and mold, and the CI-only
`ci/cargo-config.toml` (optimized dependencies, unoptimized workspace crates); local builds
retain their normal linker and profile. `.github/actions/rust-setup` restores the committed
seeds and the toolchain image.

Dev, test and CI builds use the nightly in `rust-toolchain.toml`, and `.cargo/config.toml` builds
the `campfire` crate with the Cranelift backend (everything else, and every release build, uses
LLVM; the production image stays on the Dockerfile's stable toolchain, and so do the CI
correctness suites: `ci/exec.sh` selects that stable release and hides `.cargo/config.toml`). Cranelift can't unwind:
tests of panic recovery need `--config 'profile.dev.package.campfire.codegen-backend="llvm"'`.
Measurements and rejected options: `plans/build-speed-report.md`.

Separate required correctness jobs (some sharded, behind the `Rust correctness` gate) run
Pebble ACME, WS11/WS12/WS13/ledger browsers and the gateway Node suite, the Drive browser
declarations, project-local LiveKit, messaging behaviour and the WS11 agent UI, all on Rust
from the frozen seeds and recorded fixtures. `ci/ignored-tests.json` supplies exact nextest ignored-only selectors;
`ci/ignored_tests.py` rejects any ignored test without a CI owner or a `utility:` reason,
and verifies that every selected test appears as passed in its JUnit receipt.
`ci/verify-ignored.sh` also reconciles compiler/nextest-discovered ignores with the
correctness selectors and explicit compiled utility list; source checks cover
inactive conditional attributes and the tools-only host. See
`ci/README.md` for commands, pins, and job names. All builds/tests use at most four slots.

Schema changes are SQL migrations in `crates/db/migrations/<VERSION>_<name>.sql` (a 14-digit UTC
timestamp after the last Rails migration, 20261003180000), compiled into the binary and applied only
by `campfire db-migrate DATABASE`, which the release script runs; boot never migrates. After adding
one, regenerate `crates/db/src/schema.sql`, `schema_migrations.txt` and `schema_sequences.txt` with
`CAMPFIRE_SCHEMA_DUMP=write cargo test -p campfire_db --lib schema::tests::schema_files` (the test
fails while they're stale), and migrate the committed test seeds with
`python3 parity/bin/frozen-seeds migrate target/debug/campfire`. `crates/db/baseline/` is the frozen
Rails-era schema those start from, generated from the Rails app before it was removed. The Ruby
differential and rollback comparisons are retired; their recorded results
(`crates/db/src/tests/message_save_touches.json`, the differential test's expected rows) are frozen.

## Working rules

- Work from the repository root, with rustup's `cargo` (`~/.cargo/bin`): `rust-toolchain.toml` selects the
  nightly. Stable cargo, including `mise exec rust@1.98.1` (it sets `RUSTUP_TOOLCHAIN`), rejects
  `.cargo/config.toml`'s Cranelift settings.
- `cargo nextest run --workspace --exclude html5ever -E "not (package(campfire) and ($CAMPFIRE_LLVM_ONLY_TESTS))"`
  runs everything but four panic-recovery tests, which need campfire on LLVM:
  `cargo nextest run -p campfire --config 'profile.dev.package.campfire.codegen-backend="llvm"' -E "package(campfire) and ($CAMPFIRE_LLVM_ONLY_TESTS)"`
  (`CAMPFIRE_LLVM_ONLY_TESTS` is in `.github/workflows/rust.yml`). Plain `cargo test --workspace`
  exits 101 at the first of them under Cranelift. The app's integration tests need
  the committed seeds (`python3 parity/bin/frozen-seeds restore` copies `default`, `first_run`,
  `agents_ui` and `ledger_originals` to `parity/.seed`). Missing seeds fail whenever `CI` is set;
  locally they skip with a message, so say whether the seeds were restored when reporting
  results. The `Rust seeds` job checks them against their manifest and this build's migrations;
  after adding a migration, run `python3 parity/bin/frozen-seeds migrate target/debug/campfire` from
  the repository root (the binary path is relative to the working directory; the script finds the seeds itself). See
  `parity/seeds/README.md`. Storage vectors compare media bytes only when the
  local libvips/ffmpeg match the ones that produced `vectors/storage.json`.
- `cargo clippy --workspace --exclude html5ever --all-targets` should stay clean. (`html5ever` is a
  vendored copy with one backported fix, kept identical to upstream otherwise.)
- Put shared dependency versions in the root `[workspace.dependencies]`, and reference them with
  `foo.workspace = true`.
- Write code that reads like the surrounding code: small, clearly named functions, and comments
  only where the behavior is non-obvious. Keep existing `app/...` provenance comments.
- Tests live beside the code. Golden-vector tests read `vectors/*.json`, recorded from the Rails app
  (and upstream's from stock Campfire); they're frozen now that Rails is gone, so change them
  only with the behavior they pin.
- Never `git stash` (other agents share this repository).
