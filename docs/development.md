## Development

Smartfire is a Rust workspace at the repository root. Run commands from there unless they say
otherwise. [`docs/rust-port.md`](rust-port.md) and [`AGENTS.md`](../AGENTS.md)
describe the layout and working rules, [`ci/README.md`](../ci/README.md) the CI jobs, and
[`ops/README.md`](../ops/README.md) the image, migrations and release contract.

### Prerequisites

- **Rust via rustup.** [`rust-toolchain.toml`](../rust-toolchain.toml) pins a nightly
  with the Cranelift, clippy and rustfmt components; rustup installs it the first time you run
  `cargo` in the repository. Use rustup's `cargo`: a stable toolchain (or anything that sets
  `RUSTUP_TOOLCHAIN`) rejects the Cranelift settings in
  [`.cargo/config.toml`](../.cargo/config.toml).
- **[cargo-nextest](https://nexte.st)** for the test suite.
- **libvips** (the app links it) and **ffmpeg/ffprobe** (file analysis and video previews).
  Storage tests compare media bytes only when your local libvips and ffmpeg match the image's
  builds; otherwise those checks skip.
- **Python 3** for the seed and CI helper scripts.
- **Docker** to build the image and to run the correctness suites the way CI does.
- **Node.js** for the huddle authorization gateway, the vendored JavaScript builders in
  `web/script/`, and the browser and messaging harnesses.

Dev, test and CI builds compile the `campfire` crate with Cranelift for speed; every other crate,
and every release build, uses LLVM. Cranelift can't unwind, so the few panic-recovery tests need
campfire rebuilt with LLVM (see [Running tests](#running-tests)).

### Running the server

```sh
SECRET_KEY_BASE_DUMMY=1 DISABLE_SSL=1 HTTP_PORT=3000 TARGET_PORT=3001 \
  CAMPFIRE_STORAGE_PATH="$HOME/.local/share/smartfire-dev" \
  cargo run -p campfire -- server
```

Then open http://localhost:3000. On first run you'll be guided through creating your admin
account, and you can sign in with that account from then on.

- `SECRET_KEY_BASE_DUMMY=1` uses a throwaway key; set `SECRET_KEY_BASE` instead to keep sessions
  across restarts.
- `DISABLE_SSL` turns off the HTTPS redirect. `HTTP_PORT` is the front server's port (default 80);
  `TARGET_PORT` is the app's own loopback listener (default 3000).
- `CAMPFIRE_STORAGE_PATH` holds the database (`db/production.sqlite3`), uploads (`files/`) and
  backups (`backups/`). It defaults to `storage/` under the working directory. On an empty
  database the server loads the compiled schema; on an existing one it refuses to start unless the
  database's migrations match the build (see [Migrations](#migrations)).
- There is no Redis or separate worker: background jobs, caching and Action Cable run in the one
  process.

Other settings (Google, GitHub, mail, LiveKit, public policy pages) are environment variables
read by [`crates/app/src/config.rs`](../crates/app/src/config.rs); the
[self-hosting guide](self-hosting.md) lists the ones operators set.

### Web Push notifications

Browser push notifications need a VAPID key pair in `VAPID_PRIVATE_KEY` and `VAPID_PUBLIC_KEY`.
When either is missing or they don't form a matching P-256 pair, the server logs that Web Push is
off and carries on. Generate a pair as described under "Secrets" in the
[self-hosting guide](self-hosting.md).

### Frontend assets

The stylesheets, JavaScript (Stimulus controllers, Turbo, the import map), vendored JavaScript and
`public/` files live in [`web/`](../web). The `campfire_assets` crate digests them at
build time, so a change shows up after the next `cargo run`. Templates are Askama files under
`crates/views/templates/`. The checked-in bundles in `web/vendor/javascript/` are built by
the Node projects in `web/script/` (`livekit-client`, `code-highlighter`); rebuild them only
when changing their pinned packages (see [`web/script/livekit-client/README.md`](../web/script/livekit-client/README.md)).
For local huddles, see [huddles](huddles.md).

### Running tests

The app's integration tests read committed SQLite seeds. Restore them first:

```sh
python3 parity/bin/frozen-seeds restore   # copies the seeds to parity/.seed/
```

`frozen-seeds` finds the seeds relative to its own location, so it works from any directory;
only the `campfire` binary passed to `migrate` is relative to your working directory.

Without them, seed-dependent tests skip locally with a message (and fail when `CI` is set), so
say whether the seeds were restored when you report results. Then:

```sh
cargo nextest run --workspace --exclude html5ever \
  -E "not (rdeps(campfire_app) and ($CAMPFIRE_LLVM_ONLY_TESTS))"
cargo nextest run -p campfire -p campfire_app --config ci/llvm.toml --profile ci-llvm \
  -E "$CAMPFIRE_LLVM_ONLY_TESTS"
cargo test --workspace --exclude html5ever --doc
```

`CAMPFIRE_LLVM_ONLY_TESTS` is the nextest filter for the four panic-recovery tests, defined in
[`.github/workflows/rust.yml`](../.github/workflows/rust.yml); copy it from there. Plain
`cargo test --workspace` stops at the first of those tests under Cranelift.

#### Correctness suites

Beyond the ordinary tests, the correctness suites drive Rust in real browsers and against recorded
fixtures: ACME (Pebble), the UI browser suites, Google Drive browser declarations, project-local
LiveKit, the 139 messaging behaviour cases and the agents UI. They are `#[ignore]` tests and
Python/Node harnesses run by [`ci/correctness.sh`](../ci/correctness.sh) inside the CI
container. They run on pushes to `main`, nightly and on manual dispatch, not on pull requests. To
run one locally (from the repository root, with Docker):

```sh
python3 parity/bin/frozen-seeds restore
docker build --target toolchain -t campfire-toolchain .
docker build --build-arg BASE_IMAGE=campfire-toolchain -f ci/Dockerfile -t campfire-correctness .
RUNNER_TEMP=/tmp/campfire-ci bash ci/exec.sh bash ci/correctness.sh acme
```

Replace `acme` with `browsers`, `drive`, `livekit`, `messaging` or `agents-ui`.
[`ci/README.md`](../ci/README.md) has the sharding variables and the full job list.

### Lint

CI runs clippy with warnings denied:

```sh
cargo clippy --locked --workspace --exclude html5ever --all-targets -- -D warnings
```

`html5ever` is a vendored copy kept identical to upstream apart from one backported fix.

### Migrations

Schema changes are SQL files in `crates/db/migrations/<VERSION>_<name>.sql`, with a 14-digit
UTC timestamp version. They are compiled into the binary and applied only by
`campfire db-migrate DATABASE`, which the release script runs while writes are frozen; the server
never migrates on boot. After adding one:

```sh
CAMPFIRE_SCHEMA_DUMP=write cargo test -p campfire_db --lib schema::tests::schema_files
cargo build -p campfire
python3 parity/bin/frozen-seeds migrate target/debug/campfire
```

The first command regenerates the schema files the server boots from; the last migrates the
committed test seeds. To migrate a local development database, stop the server and run
`cargo run -p campfire -- db-migrate PATH/TO/db/production.sqlite3`. Migrations must be additive
for a normal release; see [`ops/README.md`](../ops/README.md#writing-a-migration).

### Continuous integration

- **Rust** ([`rust.yml`](../.github/workflows/rust.yml)): on every pull request, the required
  `Rust port` check aggregates the frozen-seed check, twelve nextest shards, clippy with the binary
  build and doctests, the LLVM panic-recovery tests, and a stable-toolchain check. A pull request
  that touches only `docs/`, `deploy/`, `frontend/`, other workflows or the root prose files skips
  the build and passes. Pushes to `main`, the nightly schedule and manual runs also run the correctness suites
  behind `Rust correctness`.
- **Frontend** ([`frontend.yml`](../.github/workflows/frontend.yml)): the `Frontend` check runs
  Biome, the anti-slop Oxlint rules, `tsc`, Vitest and `vite build` for the SPA in `frontend/`,
  and puts the entry chunks' gzip sizes in the job summary, then runs `cargo test -p campfire_spa`
  with that build embedded (`SPA_DIST`). It reports on every pull request; one that touches
  neither `frontend/`, `crates/spa/` nor the workflow passes without installing anything. See
  [`frontend/README.md`](../frontend/README.md).
- **Repository checks** ([`repo.yml`](../.github/workflows/repo.yml)): `GitHub Actions audit`
  (actionlint, zizmor and the Google deployment configuration tests),
  `Huddle authorization gateway` (the Node gateway's tests) and `Dependency audit`
  (cargo-audit over `Cargo.lock` and npm audit of the gateway).
- **CodeQL** ([`codeql.yml`](../.github/workflows/codeql.yml)): code scanning for Rust,
  JavaScript, Python, C and the workflows.

To run the full Rust matrix, correctness suites included, on a branch before merging:

```sh
gh workflow run rust.yml --ref YOUR-BRANCH -f scope=full
```

`-f ref=COMMIT` tests a specific branch, tag or commit instead of the workflow ref, and
`-f scope=messaging-and-browsers` runs only those correctness suites.

### Building the image

```sh
docker build -t smartfire .
```

The image is built from the [`Dockerfile`](../Dockerfile) with the repository root as the context. See
the [self-hosting guide](self-hosting.md) to run it and [deploy/README.md](../deploy/README.md) for
production releases.

### Contributing

You are welcome - and encouraged - to modify Smartfire to your liking.
If you'd like to contribute your changes back, please read our [contributing guide](../CONTRIBUTING.md) first.
