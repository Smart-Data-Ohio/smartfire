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
- **Docker** to build the image and use the CI toolchain wrapper.
- **Node.js and pnpm** for the React UI and its browser checks, and Node.js for the huddle gateway.

Local dev and test builds use Cranelift for the application crates named in `.cargo/config.toml`.
CI tests and release builds use LLVM. Cranelift cannot unwind, so panic-recovery tests need
`--config ci/llvm.toml` (see [Running tests](#running-tests)).

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

The React UI lives in [`frontend/`](../frontend/README.md). Run `pnpm install --frozen-lockfile`
and `pnpm check` there to validate and build `dist/`, then rebuild the Rust binary to embed it.
The image performs this build in a Node stage; its runtime has no Node. The SPA and JSON sync
are always served. Retained auth and public HTML live in `crates/retained_pages/templates/`,
and their auth assets, shared media and public bytes live in `crates/static_assets/`.
Historical digested asset URLs retain their bytes. For local huddles, see [huddles](huddles.md).

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
cargo nextest run --config ci/llvm.toml --workspace --exclude html5ever -j 4
cargo test --workspace --exclude html5ever --doc -j 4
```

The LLVM override lets the ordinary panic-recovery tests unwind. Plain
`cargo test --workspace` under Cranelift still fails at those tests.
Every retained test runs on pull requests that change its inputs; the separate
correctness suites have been removed. See [`ci/README.md`](../ci/README.md).

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

The six required checks report on every PR and main push. Irrelevant jobs skip
expensive steps internally so their check names still appear.

- `Rust port` aggregates two nextest shards, clippy/binaries/doctests and the frozen-seed check.
  Main pushes run all Rust checks. PRs select them by Rust, Cargo, CI, asset and fixture inputs.
- `Frontend` runs lint, typecheck, Vitest, a build and `crates/spa` tests with that dist.
  Three separate e2e shards run the Vite mock server and production-preview tests.
- `GitHub Actions audit` runs actionlint and zizmor for `.github` changes.
- `Huddle authorization gateway` tests gateway and media inputs.
- `Ops scripts` tests release, deployment and backup scripts and their workflows.
- `Dependency audit` runs for manifest and lockfile changes and weekly for new advisories.

CodeQL runs weekly or manually. There is no scheduled Rust test tier.
[`ci/README.md`](../ci/README.md) describes the scopes and retained tests.

To check a branch manually:

```sh
gh workflow run rust.yml --ref YOUR-BRANCH
```

`-f ref=COMMIT` tests a specific revision instead of the workflow ref.

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
