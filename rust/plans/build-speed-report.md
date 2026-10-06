# Build speed: Cranelift for dev/test, measurements and options

Branch `rust/fast-builds`, based on main `6db097e0`. Owner's decision: Cranelift for dev, test and
CI; LLVM for release, permanently.

## What changed

| File | Change |
|---|---|
| `rust-toolchain.toml` (new) | Pins `nightly-2026-08-21` with `rustc-codegen-cranelift-preview`, clippy, rustfmt. |
| `.cargo/config.toml` (new) | `[unstable] codegen-backend = true`; `[profile.dev.package.campfire] codegen-backend = "cranelift"`; `[profile.release] codegen-backend = "llvm"`. |
| `Cargo.toml` | `[profile.dev.package.libsqlite3-sys] opt-level = 1`. |
| `clippy.toml` (new) | `msrv = "1.98.1"`, the production toolchain, so `incompatible_msrv` flags std APIs newer than stable. |
| `Dockerfile` | The toolchain stage (CI) installs the nightly from `rust-toolchain.toml`. The production build stage copies neither file and stays on stable `RUST_VERSION` and LLVM. |
| `.dockerignore` | Leaves `/.cargo` out of the build context. |
| Two `#[allow]`s | Nightly clippy's `single_element_loop` (agent_delivery.rs) and deprecated `fetch_update` (google_api_tests.rs; `try_update` isn't on stable 1.98 yet). |
| 167 tool scripts and docs | Dropped `mise exec rust@1.98.1 --` from their cargo calls. That form sets `RUSTUP_TOOLCHAIN=1.98.1`, which overrides `rust-toolchain.toml`, and stable cargo then rejects `.cargo/config.toml` with "feature `codegen-backend` is required". Recorded commands in `plans/` and the `*.json`/`*.txt` evidence files are left as they were. |

Rustup and cargo find `rust-toolchain.toml` and `.cargo/config.toml` from the current directory, not from `--manifest-path`. Scripts that run `cargo --manifest-path rust/Cargo.toml` from the repository root therefore keep building with the default stable toolchain and LLVM. That works, but it doesn't share artifacts with builds run from `rust/`.

How release stays on LLVM:
- `[profile.release] codegen-backend = "llvm"` is set explicitly.
- The production image never sees `.cargo/config.toml`. Stable cargo would reject it anyway: "config profile `release` is not valid".
- The production image builds on stable, where Cranelift isn't available.

I checked a local `cargo build --release` with this config: all 378 units ran with `-Z codegen-backend=llvm`, and the binary's `.comment` section has no Cranelift string.

Only the `campfire` crate uses Cranelift. Dependencies and the other workspace crates (campfire_db, campfire_kit, …) stay on LLVM, for two reasons:
- **Speed.** Cranelift builds them no faster. They are small, or they are dependencies that get built once.
- **Correctness.**
  - Cranelift doesn't support zlib-rs's `llvm.x86.pclmulqdq.256`, so the campfire_kit tests abort at runtime.
  - rustup's Cranelift can't unwind (see "Cranelift and panics").

## Method

The machine is shared with other workers, so the load average varied from 5 to 34 across runs.
- Everything was built with `-j12` against a dedicated target dir, with `RUSTC_WRAPPER` cleared.
- Builds were timed with `$EPOCHREALTIME`. Per-unit times come from `cargo --timings`, and the phase split from `-Ztime-passes`.
- "Clean" means `cargo test -p campfire --no-run` from an empty target dir.
- "Incremental" means the test build after a one-line string edit:
  - app edit: `crates/campfire/src/app/qr_code.rs`;
  - db edit: `crates/db/src/models/agent_delivery.rs`, which also rebuilds campfire_db and relinks.
- The final comparison alternated baseline and final configs (A, F, A, F) so that both saw similar load.

## Results

### Before / after (campfire test build)

| | Baseline (stable 1.99, LLVM) | Final (nightly, campfire on Cranelift) | Change |
|---|---|---|---|
| Clean test build, quiet machine (load ~5) | 249s | 175–178s | −29% |
| Clean test build, alternating series | 289s, 343s | 228s, 203s | −27%/−34% (avg −30%) |
| … of which the `campfire` test crate | 207–289s | 133–180s | −36–44% |
| Incremental, app leaf edit | 82–132s (median ~110s under load, 82–92s quiet) | 32–41s | −60–70% |
| Incremental, db edit (campfire_db + campfire) | 62–96s | 24–30s | −60–70% |
| Link of the campfire test binary | 32.8s (2.4GB binary) | 3–5s (1.8GB) | −90% |
| Release build (LLVM, fat LTO, cu=1) | 411s | 450s at load ~12, same settings | unchanged (noise) |
| Release binary | 72.7MB | 72.7MB | identical settings |
| campfire_db test suite | 67s wall / 301s user CPU | 55s / 204s | −18% wall, −32% CPU (SQLite -O1) |
| App test sample (a few hundred campfire tests) | 52s user CPU | 46s user CPU | −12% |
| Target dir after the series | 14G | 11G | −3G |

The test sample's results matched LLVM, apart from the panic tests below.

### CI (PR #256 run 37387060037, cold cache, vs main run 37379298592)

| Step | main | this PR |
|---|---|---|
| Image + toolchain | cached | 5:36 (installs the nightly once; cached afterwards) |
| Clippy | 1:57 | 2:10 |
| Binaries | 1:38 | 1:41 |
| Enumerate ignored tests (builds the test binaries) | 5:49 | 3:36 |
| campfire_db tests | 2:47 | 1:39 |
| Seed-dependent campfire tests | 51:49 | 27:42 |
| Other workspace tests | 1:52 | 1:07 |

Campfire suite on CI: 3077 run, 3073 passed, 78 skipped, and 4 failed. The 4 failures are exactly the panic-recovery tests listed below. campfire_db: 1453 passed. Other workspace: 757 passed.

Where the seed-dependent step's time goes depends on the other worker's sharding rewrite. These numbers only show that the Cranelift binary runs the suite at least as fast as LLVM's.

### Where the time went (time-passes, the campfire crate, non-incremental)

| Phase | LLVM (stable) | Cranelift |
|---|---|---|
| Frontend (expand, typeck, borrowck) | ~50s (typeck 18.8s, borrowck 27s) | ~45s |
| Monomorphization collector | 57s | 30s |
| Codegen to IR + backend passes | 80s + 75s | ~36s |
| Link | 32.8s | 4s |
| Total | 238s | 119s |

Incremental (app edit, Cranelift):
- expand 3.3s, mono 4.1s, partition 4.3s, codegen ~22s, persist 2.1s, link 4s.
- The dep graph has 17.8M nodes and 170M edges. The fixed cost of checking it is why an incremental rebuild of the 200k-line crate can't go much below ~30s. That cost is the motivation for the crate-split proposal.

Nightly with LLVM alone was slower than stable: 298s clean, 97–105s incremental. The whole gain comes from Cranelift.

## Cranelift and panics (unresolved: owner's choice)

rustup's `rustc-codegen-cranelift-preview` doesn't implement unwinding. Inside Cranelift-compiled code:
- `catch_unwind` does not catch. The process exits with 101.
- Destructors don't run while a panic propagates.

`catch_unwind` is generic, so it is instantiated in the calling crate. That means the libraries built with LLVM don't help when the caller is campfire.

I confirmed this with a scratch test: a panicking test that holds a `static Mutex` hangs the next test. A plain `cargo test` run can therefore hang after one failure. nextest, which CI uses, runs each test in its own process and is not affected.

These 4 tests exercise panic recovery and fail under Cranelift:
- `channels::message_features::tests::origin_is_present_for_commit_callbacks_and_restored_after_errors_and_panics`
- `controllers::agent_review_r5_tests::ws11_proxy_header_oracle_rejects_unapproved_changes`
- `jobs::tests::a_panicking_ad_hoc_job_is_logged_and_its_worker_carries_on`
- `test_support::server_startup_panics_reach_the_reporter_and_join_handle`

They pass on LLVM:

```
cargo test -p campfire --config 'profile.dev.package.campfire.codegen-backend="llvm"' <filter>
```

Note: `--config profile.dev.codegen-backend="llvm"` is **not** enough, because the per-package override wins. I verified this through `.comment`.

Production is unaffected: release is LLVM with unwinding.

## Required CI changes (not made here: `.github/workflows/` and `rust/ci/` belong to `ci/fast-rust`)

1. **Panic-recovery tests.** Without one of the following, the seed-dependent gate fails on the 4 tests above. Options:
   - (a) Add the 4 tests to `ci/ignored-tests.json`, or a nextest filter, for the Cranelift run, and run them in a small extra step built with `--config 'profile.dev.package.campfire.codegen-backend="llvm"'`. That step costs one LLVM build of the campfire test crate, ~2–3 min on CI.
   - (b) Have `ci/cargo.sh` pass that `--config` for the whole campfire test run. This loses the CI speedup but keeps the local one.

   I'd pick (a).
2. **Cache keys** must hash `rust/rust-toolchain.toml` and `rust/.cargo/config.toml`, as well as `rust/Cargo.toml`. Changing the toolchain or the backend invalidates every artifact. Today's key (`rust.yml`, `hashFiles('rust/Dockerfile', 'rust/Cargo.toml', 'rust/ci/cargo.sh')`) changes for this PR only because the Dockerfile changed. A later nightly bump in `rust-toolchain.toml` alone would restore a stale cache.
3. **Toolchain.** Nothing more is needed. `rust/Dockerfile`'s toolchain stage already installs the pinned nightly and components from `rust-toolchain.toml`. Run 37387060037 built the image and ran clippy, the binaries and all test steps with it. To bump the nightly, edit `rust-toolchain.toml` only.
4. **Optional:** `RUSTFLAGS`/profile-rustflags `-Zthreads=4` for the campfire crate on CI. In a CI-like run (`CARGO_INCREMENTAL=0`, `-j4`) the crate took 90.4s instead of 123.6s (−27%), with peak RSS 12.1GB instead of 10.1GB. Enable it only if the runner has the memory headroom.
5. **Linker:** CI keeps mold.
6. **Debuginfo:** CI keeps its `CARGO_PROFILE_*_DEBUG=line-tables-only`.
7. **Recommended: a stable check.** From the repository root, so that neither `rust-toolchain.toml` nor `.cargo/config.toml` applies, run `RUSTUP_TOOLCHAIN=1.98.1 cargo check --locked --manifest-path rust/Cargo.toml --workspace --exclude html5ever`. This catches code that compiles only on the nightly before it reaches the production image. It took 51s locally from empty, at `-j12`. The toolchain image would need 1.98.1 as well. It already has it: it's the base image's toolchain.

## Options measured and rejected

| Option | Result | Why rejected |
|---|---|---|
| Cranelift for the whole workspace (all deps too) | Clean 215s (vs 175–178s mixed); incremental 32–35s | Slower clean build. zlib-rs SIMD intrinsic aborts campfire_kit tests. Panic handling broken in every crate (campfire_db aborts with "failed to initiate panic"). |
| `-Zthreads=8` / `=4` parallel frontend, local default | Non-incremental campfire crate 119s → 77s / 75s; incremental 33–37s (no gain); peak RSS +2–4.6GB | Locally only ~10s off a clean build and nothing off incremental, which is the common loop, at +2GB or more per build on a shared machine. Recommended for CI only (item 4 above). |
| wild 0.10.0 linker | Relink 1.6–3.4s (median 1.9s) | Same as mold (1.7–5.8s, median 1.8s) and rust-lld (2.9–5.2s). With Cranelift's smaller binary, link is ~4s, and no linker change shows in wall time. Requiring a new tool for every developer isn't worth ~1s. |
| mold locally | as above | Same reason. rust-lld is the default on x86_64 Linux since 1.90. CI already uses mold. |
| `debug = "line-tables-only"` locally | Clean 207s, incremental 34–44s (within noise); target 11G → 8.4G | No measurable speed gain. Developers keep full debuginfo. CI already sets it. |
| `split-debuginfo = "unpacked"` | Not separately kept | Cranelift's link is already ~4s. The 33s LLVM link was dominated by the ~1GB `.debug_str`, which Cranelift's smaller binary mostly avoids. Nothing left to win. |
| campfire `codegen-units = 1024` | Clean 212s, incremental 35–46s | Worse on both. |
| `opt-level = 1` for all dependencies | Clean 257s (+80s); test libs 94–99s vs 120–135s | The +80s on every clean build (and CI cache miss) costs more than it saves. Only SQLite is worth it (kept). |
| Disabling incremental locally | — | Incremental is what makes the 32–41s rebuild possible. CI already runs with `CARGO_INCREMENTAL=0`. |
| Cranelift for release | — | Dropped by owner decision: release is LLVM permanently. |
| LLVM release tuning (thin LTO / codegen-units > 1) | Not measured | The owner allowed it only if cheap. A fair comparison needs runtime-throughput benchmarking (bench/run with Docker images and seeds) against fat LTO / cu=1, and it isn't cheap. No change recommended. |

## Crate-split proposal (not implemented)

The `campfire` crate is ~200k lines, of which ~100k are `*test*.rs` files. Its incremental floor (~30s on Cranelift) comes from the size of its dependency graph. Smaller crates rebuild much faster:

| Crate | Size | Incremental test rebuild after a leaf edit |
|---|---|---|
| campfire_kit | 13k lines | 1.3–1.9s |
| campfire_db | 81k lines | 4.3–8.5s (Cranelift), 5.4–6.4s (LLVM) |
| campfire | 200k lines | ~37s |

Module sizes inside campfire:
- controllers 118k lines, integrations 42k, app 18.6k, channels 9.8k, jobs 4.6k, concerns 3.6k;
- the rest are small.

The modules are cyclic today, and each cycle has to be broken first:
- controllers → app (`AppCtx`, `App`, `AppState`);
- integrations, channels, app and jobs → `controllers::presenters` (`Presenter`, `page`, `presenters::test_support`);
- controllers → integrations (`net::Network`, github, twitter);
- jobs → `channels::tests::support`.

Proposed layering, lowest first:
1. **`campfire_app`**: App/AppCtx/AppState, config, errors, security, authentication, account_security, public_policy, concerns, messaging, rich_text, mail, active_storage, picker_configuration, huddle_readiness. Router assembly moves out to the bin.
2. **`campfire_presenters`**: presenters plus `messages::rendered`.
3. **`campfire_integrations`**, possibly split per provider.
4. **`campfire_channels`**: channels, huddle and jobs.
5. **Controller crates**: rooms, message_features, users+accounts, messages, and one for the rest (github/slack/agents/…).
6. **`campfire`** (bin): main, router, cross-cutting app tests.
7. **`campfire_test_support`**: `presenters::test_support`, `test_support.rs`, `integrations::test_support` and `channels::tests::support`, behind a `test-support` feature.

Estimated effect:
- An edit in a leaf controller rebuilds one ~20–30k-line crate plus relinks. That's ~3–6s instead of ~37s, by analogy with campfire_db's 81k lines at 4–8s.
- Clean critical path ~200s → ~150s, because the controller crates compile in parallel.

Costs:
- More test binaries and more disk.
- Edits to the core crates still cascade to everything above them.
- `pub(crate)` items that cross the new boundaries become `pub`.
- The cycle-breaking is the bulk of the work.

## Disk and machine notes

- Measurement target dirs and scratch crates lived under `~/.cache/fast-builds` (outside the repo) and were removed afterwards.
- With the final config, a full local test target is ~11G (baseline 14G).
- The nightly toolchain with the Cranelift component adds ~0.7G under `~/.rustup`.
- Toolchain skew: tests and CI build on the nightly, while production builds on stable 1.98.1. clippy's `msrv` flags std APIs newer than 1.98.1. It does not catch language features stabilised after 1.98.1, and PR CI doesn't build the production image (`publish-rust-image.yml` builds it after merge). See CI change 7.
