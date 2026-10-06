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
| Tests that compare with Rails source the port doesn't copy (`db/schema.rb`, `db/migrate`, `app/models`, `app/views`, `config/icons.yml`) | `campfire_db::fixtures::rails_root()`: `CAMPFIRE_REFERENCE`, else `rust/..` |
| `Dockerfile` | copies `web/`'s inputs from its own context (`docker build .` from `rust/`) |
| `parity/bin/*`, `reference-tools/*`, `crates/assets/script/revendor` (they run Rails) | `REFERENCE_ROOT=${CAMPFIRE_REFERENCE:-rust/..}` |
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
| `reference-tools/` | — | Ruby scripts run inside the reference app to produce `vectors/` |
| `bench/` | — | Load generator, benchmark scripts and recorded results (upstream's, against stock Campfire) |
| `plans/` | — | Upstream's conversion plan and reports, kept for their reasoning |

CI for this tree is `.github/workflows/rust.yml` at the repository root. It runs on
Rust and Rails comparison inputs. Every ordinary nextest group and runnable doctest is
required; the app and workspace steps report all failures before explicit outcome gates.
`ci/cargo.sh` uses the Dockerfile's pinned toolchain/media and mold; local builds retain
their normal linker. Shared pinned Rails seed build/restore/validation lives in
`.github/actions/rust-setup`.

Dev, test and CI builds use the nightly in `rust-toolchain.toml`, and `.cargo/config.toml` builds
the `campfire` crate with the Cranelift backend (everything else, and every release build, uses
LLVM; the production image stays on the Dockerfile's stable toolchain). Cranelift can't unwind:
tests of panic recovery need `--config 'profile.dev.package.campfire.codegen-backend="llvm"'`.
Measurements and rejected options: `plans/build-speed-report.md`.

Separate required correctness jobs run Rails differential/rollback, Pebble ACME,
WS12/WS13 browsers and the gateway Node suite, project-local LiveKit, paired messaging,
and WS11 agent UI. `ci/ignored-tests.json` supplies exact nextest ignored-only selectors;
`ci/ignored_tests.py` rejects any ignored test without a CI owner or a `utility:` reason,
and verifies that every selected test appears as passed in its JUnit receipt.
`ci/verify-ignored.sh` also reconciles compiler/nextest-discovered ignores with the
correctness selectors and explicit compiled utility list; source checks cover
inactive conditional attributes and the tools-only host. See
`ci/README.md` for commands, pins, and job names. All builds/tests use at most four slots.

`crates/db/src/schema.sql` (with `schema_migrations.txt`, `schema_sha1.txt` and
`schema_sequences.txt`) is generated from the Rails app by `reference-tools/db/regenerate-schema.sh`;
rerun it after a Rails migration (`--check` verifies). `reference-tools/db/check-migration-replay.sh`
checks that replaying every migration from empty (how production databases were built) gives the
same schema. `reference-tools/db/differential.sh` compares fixtures and a scenario with Ruby's,
regenerates the reference's `Message` save-timestamp table (`crates/db/src/tests/message_save_touches.json`)
and checks that Rails reads, and validates, every row the Rust crate wrote.

## Working rules

- Work from `rust/`, with rustup's `cargo` (`~/.cargo/bin`): `rust-toolchain.toml` selects the
  nightly. Stable cargo, including `mise exec rust@1.98.1` (it sets `RUSTUP_TOOLCHAIN`), rejects
  `.cargo/config.toml`'s Cranelift settings.
- `cargo test --workspace --exclude html5ever` runs everything. The app's integration tests need
  the `default`, `first_run` and `agents_ui` seeds (`parity/bin/seed build default first_run agents_ui`, which runs the
  reference). Missing seeds fail whenever `CI` is set; locally they skip with a message, so say
  which seeds were built when reporting results. CI archives `parity/reference.sha`, caches
  the reference image and seeds by their exact inputs, and validates even restored seeds with
  Rails before testing. See `parity/seeds/README.md`. Storage vectors compare media bytes only when the
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
- Tests live beside the code. Golden-vector tests read `vectors/*.json`; regenerate them from the
  reference app (`reference-tools/`) rather than by hand. Upstream generated the current ones from
  stock Campfire.
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
