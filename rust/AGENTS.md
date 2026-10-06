# Smartfire in Rust (`rust/`)

A Rust port of Smartfire, the Rails app at the root of this repository. It starts from 37signals'
port of stock ONCE Campfire ([basecamp/once-campfire-rust](https://github.com/basecamp/once-campfire-rust),
MIT, imported here with its history by `git subtree`; we don't track it). That port was pixel and
behavior identical to stock Campfire; Smartfire adds a great deal on top, so most of our screens
and routes aren't ported yet. See `README.md` for where things stand.

The goal is a drop-in replacement for the Rails app, indistinguishable from it before cutover:

- **The Rails app at the repository root is the reference.** It's the oracle for everything: when
  in doubt about behavior, read the Ruby. Rails feature work is frozen (fixes only), so it's a fixed
  target. Don't edit the Rails app from Rust work.
- **Exact pixel and behavior parity with the Rails app before cutover.** Any visual or behavioral
  difference is a bug until then, even ones that look like improvements. The upstream port's
  deliberate divergences from Rails (README, "Known differences") are inherited, not endorsed:
  each needs a decision before cutover.
- **Drop-in compatible:** the same SQLite database and schema, storage layout, signed/encrypted
  cookies (so people stay signed in across the switch) and environment variables.
- Port-owned frontend changes would go in `crates/assets/overrides/`, which shadows the
  reference's assets by logical path. There are none before cutover (`crates/assets/OVERRIDES.md`).

## Where the reference lives

The port owns copies of the reference's static inputs, in `rust/`:

- `web/`, laid out like the Rails app: `app/assets`, `app/javascript`, `vendor/javascript`,
  `public/`, `config/importmap.rb`, `config/initializers/assets.rb`, the JS builders that vendor
  bundles into `vendor/javascript` (`script/livekit-client`, `script/code-highlighter`), the LiveKit
  gateway (`script/livekit-gateway`) and `bin/livekit-local`.
- `fixtures/`: the reference's `test/fixtures`.
- `test-support/`: data the tests read that reference tools once held (attachment analyzer inputs,
  agents UI cast inputs, the post-pin status files, Node test adapters).

These are canonical. Until the Rails app is removed it reads them through relative symlinks at the
old paths (`app/assets -> ../rust/web/app/assets`, `test/fixtures -> ../rust/fixtures`, ...); the
root `.dockerignore` keeps `rust/web/` in the Rails image's context so they resolve there too.
Edit the files under `rust/`.

`CAMPFIRE_REFERENCE` names a reference Rails app's root, as an absolute path (Cargo runs build
scripts and tests from each crate's directory, so a relative one resolves differently there than in
the shell tools). Tests read it at compile time, so changing it rebuilds them. Unset, Rust reads its
own copies:

| Consumer | How it finds its inputs |
|---|---|
| `crates/assets/build.rs` | `CAMPFIRE_REFERENCE`, else `rust/web`; reads `app/assets`, `app/javascript`, `vendor/javascript`, `public/`, `config/importmap.rb`, `config/initializers/assets.rb` |
| Tests | `campfire_db::fixtures`: `reference_root()` is `rust/web`, `reference_dir()` is `rust/fixtures`, and `reference_path("public/500.html")` / `reference_path("test/fixtures/files/...")` maps a Rails path to the port's copy (all `CAMPFIRE_REFERENCE` instead, when set at compile time) |
| `Dockerfile` | copies `web/`'s inputs from its own context (`docker build .` from `rust/`) |
| `parity/bin/reference`, `parity/bin/candidate`, `crates/assets/script/revendor` (they run Rails; nothing in CI does) | `REFERENCE_ROOT=${CAMPFIRE_REFERENCE:-rust/..}` |
| `parity/capture` (TypeScript) | `REFERENCE_DIR`; `reference/...` paths in `screens.yml` resolve against it (`repoPath`) |

`reference/` in comments, docs and `screens.yml` means the reference app's root. There's no
submodule and no `rust/reference` symlink: a `rust/reference -> ..` link would make a loop
(`rust/reference/rust/...`) for anything that walks directories.

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
| `parity/` | — | Playwright parity harness, screen inventory, reference Docker setup |
| `reference-tools/` | — | The browser and behaviour harnesses the correctness suites run against Rust |
| `bench/` | — | Load generator, benchmark scripts and recorded results (upstream's, against stock Campfire) |
| `plans/` | — | Upstream's conversion plan and reports, kept for their reasoning |

CI for this tree is `.github/workflows/rust.yml` at the repository root. It runs on
every pull request, building only when `rust/`, the workflow or its setup action changed, and
never runs Ruby, Rails or a reference image. Every ordinary nextest group and runnable doctest is
required; tests run as twelve nextest partitions beside the frozen-seed check, clippy/doctest,
LLVM panic-recovery and stable-toolchain jobs, and the `Rust port` job fails unless all of
them succeeded.
`ci/cargo.sh` uses the Dockerfile's pinned toolchain/media and mold, and the CI-only
`ci/cargo-config.toml` (optimized dependencies, unoptimized workspace crates); local builds
retain their normal linker and profile. `.github/actions/rust-setup` restores the committed
seeds and the toolchain image.

Dev, test and CI builds use the nightly in `rust-toolchain.toml`, and `.cargo/config.toml` builds
the `campfire` crate with the Cranelift backend (everything else, and every release build, uses
LLVM; the production image stays on the Dockerfile's stable toolchain, and so do the CI
correctness suites, which build from the repository root). Cranelift can't unwind:
tests of panic recovery need `--config 'profile.dev.package.campfire.codegen-backend="llvm"'`.
Measurements and rejected options: `plans/build-speed-report.md`.

Separate required correctness jobs (some sharded, behind the `Rust correctness` gate) run
Pebble ACME, WS11/WS12/WS13/ledger browsers and the gateway Node suite, the Drive browser
declarations, project-local LiveKit, messaging behaviour and the WS11 agent UI, all on Rust
from the frozen seeds and recorded Rails fixtures. `ci/ignored-tests.json` supplies exact nextest ignored-only selectors;
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

- Work from `rust/`, with rustup's `cargo` (`~/.cargo/bin`): `rust-toolchain.toml` selects the
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
  after adding a migration, run `frozen-seeds migrate target/debug/campfire`. See
  `parity/seeds/README.md`. Storage vectors compare media bytes only when the
  local libvips/ffmpeg match the ones that produced `vectors/storage.json`.
- `cargo clippy --workspace --exclude html5ever --all-targets` should stay clean. (`html5ever` is a
  vendored copy with one backported fix, kept identical to upstream otherwise.)
- Put shared dependency versions in the root `[workspace.dependencies]`, and reference them with
  `foo.workspace = true`.
- When matching existing behavior, read the reference's source. When it depends on Rails or gem
  internals, read the gem source inside the reference image
  (`docker run --rm campfire-reference bundle show <gem>`), not docs or memory.
- Write code that reads like the surrounding code: small, clearly named functions, and comments
  only where the behavior is non-obvious. Cite the reference file (`app/...` in the Rails app) when
  matching Rails.
- Tests live beside the code. Golden-vector tests read `vectors/*.json`, recorded from the Rails app
  (and upstream's from stock Campfire); they're frozen now that Rails is gone, so change them
  only with the behavior they pin.
- Never `git stash` (other agents share this repository), and don't edit files outside `rust/`
  for port work unless the task says so.

## Reference container

`parity/bin/reference build` builds `campfire-reference:app` from the Rails app's own `Dockerfile`
at the repository root, then the parity image `campfire-reference` on top of it
(`parity/docker/Dockerfile`: libfaketime, the test fixtures, a one-worker resque pool). It runs in
production mode with a fixed `SECRET_KEY_BASE` (see `parity/.env.reference`) so that golden
vectors, seeds and screenshots are reproducible. Those keys are for tests only.
`parity/bin/candidate build` builds the Rust image (`Dockerfile`) and `campfire-candidate` on top of it. Without Docker, `PARITY_RUNTIME=native` runs
the reference with the host's Ruby (not canonical: media bytes differ), and captures fall back to
the pinned Playwright image under bubblewrap.
