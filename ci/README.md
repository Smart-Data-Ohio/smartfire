# Rust correctness gates

`Rust port` gates clippy, the production-input binary build, all ordinary workspace
nextest tests, and runnable doctests. It is an aggregator: it fails unless each of
these jobs succeeded (a failed, cancelled or skipped one fails it). Its summary
(`summarize-tests.py --expect`) also fails unless the shards' JUnit receipts hold exactly
the tests `cargo nextest list` selects for their filter, each passed once, with every
`#[ignore]` test reported; the same for the LLVM job's four; and libtest's doctest logs
account for every doctest they ran. Both gates run `check_gate_needs.py`, which fails if
any job in `rust.yml` is missing from their `needs`. It reports on every pull request:
`Rust changes` checks the PR's diff, and when it touches no Rust input (anything outside `docs/`,
`deploy/`, `frontend/`, other workflows and the root prose files) the jobs
below are skipped and `Rust port` passes only if every one of them was skipped. Pushes,
nightly and manual runs, empty diffs and unavailable history always run everything:

| Job | Runs |
| --- | --- |
| `Rust seeds` | `parity/bin/frozen-seeds check`: the committed seeds in `parity/seeds/frozen` match their manifest, the test keys and this checkout's schema migrations; its unit tests prove changed, missing, added or out-of-date seeds are rejected. |
| `Rust tests (K/12)` | `cargo nextest run --workspace --exclude html5ever --profile ci --no-tests fail --partition slice:K/12`: nextest's round-robin slice of every ordinary test but the four panic-recovery tests below (the campfire crates on Cranelift; the nightly run builds them with LLVM, `--config ci/llvm.toml`). Shard 1 also lists the tests the shards and the LLVM job must run, and runs `verify-ignored.sh`, against the harnesses it compiled. |
| `Rust tests (campfire panic recovery, LLVM)` | The four `CAMPFIRE_LLVM_ONLY_TESTS`, with the campfire crates on LLVM (`ci/llvm.toml`, the `ci-llvm` nextest profile): Cranelift can't unwind. Fails unless exactly those four ran and passed. |
| `Rust production toolchain check` | `cargo check --workspace` on the image's stable toolchain, which production builds with, from its own `stable` Cargo cache. |
| `Rust clippy, binaries and doctests` | ci unit tests, clippy, the production-input binary build, then the database and workspace doctests. |

Correctness builds run through `ci/exec.sh`, which selects the Dockerfile's stable release
(`RUSTUP_TOOLCHAIN`) and hides `.cargo/config.toml`, so they use the stable toolchain and LLVM
and restore their
own `correctness` Cargo cache (saved on main by browsers shard 2/4); the test shards' cache
holds nightly artifacts. Cargo caches hold only registry dependencies, so their keys are the
toolchains, build inputs and `Cargo.lock`, not the commit: a push to main saves one only when
no entry with that key exists. Every artifact upload overwrites its earlier attempt's, so a
failed job can be re-run on its own.

Test and correctness jobs restore the committed seeds (`frozen-seeds restore`, through the
setup action's `parity: seeds`); nothing in the workflow runs Ruby, Rails or a reference
image. Test failures are retained for the summary, then explicit gates fail the job.
No advisory correctness group remains.

The correctness jobs run on main, nightly, and manual workflows, through the same
setup action. `Rust correctness` is their aggregator: every job must succeed, each
must report exit 0 for the tested commit, the shards' JUnit receipts together must
contain exactly each suite's registered ignored tests (`correctness_gate.py`), and the
messaging behaviour shards' case receipts must cover all 139 named cases once
(`behavior-check.py --verify-receipts`). The slim pull-request gate runs only the
`Rust port` jobs. Branch protection is managed separately by the release lead.

`CORRECTNESS_SHARD=K/N` runs one deterministic slice of a suite: browsers split their
ignored tests by the recorded `seconds` in `ignored-tests.json`, messaging behaviour
splits whole case batches, and the Drive job runs its 45 declarations on four nextest
threads (each holds two: an app and a pinned Chromium container). The behaviour
shards use the two Rust hosts the `Rust messaging host (app|test)` jobs build once with
behavior-check.py's own commands, and load the prerequisite image the `app` job exports
instead of building it sixteen times.

| Job suffix | Execution |
| --- | --- |
| acme | Digest-pinned Pebble, then exactly 1 ignored TLS-ALPN certificate/cache test |
| browsers (4 shards) | Pinned Playwright image, gateway `ws` lockfile, and normal `campfire` binary (`WS11UI_BROWSER_BINARY`, compiled with the test harnesses while the prerequisite image builds), then exactly 6 WS11-UI, 7 WS12, 4 ledger, 1 WS13, and 1 gateway ignored tests, all on Rust from the frozen seeds; C221–C223 run the three inbox/filter/work sequences and reject their writer-defect controls |
| livekit | `web/bin/livekit-local setup/start` (checksum-pinned 1.13.7), polling/media transport regression tests, then exactly 1 ignored real-media test |
| drive | The pinned Chromium image, then exactly the 45 ignored Drive attachment, share, sudo and event-card declarations (`drive_browser_tests`, listed in `parity/system/drive-declarations.json`) |
| messaging behaviour (16 shards) | Python/Node harness regression tests (shard 1), then `python3 reference-tools/messaging/behavior-check.py --keep-going --shard K/16`: the 139 named cases on Rust, each from the frozen default seed and its recorded Rails fixture step (`test-support/behavior-fixtures`) |
| agents-ui | `python3 reference-tools/views/agents_ui/system_behavior.py --binary target/debug/campfire --scenario all` (pages, budget and work on Rust, against the recorded `test-support/agents-ui-fixtures`) |

No external harness in the requested messaging/WS11 scope lacks a scripted entry
point. The screen-matrix pixel harness is outside this correctness package.

The main Rust/Debian images, Playwright/Chromium image,
Node and Docker CLI images, and Pebble are digest-pinned. Nextest, libfaketime,
media sources, util-linux 2.42.4 (unshare), ChromeDriver (matching Chromium 153.0.8010.12), and LiveKit are
checksum-pinned. npm dependencies use committed lockfile integrity hashes.
The CI-only APT prerequisites and their complete runtime dependency closure,
including libraries already present in the toolchain, are pinned by exact version,
architecture, archive URL and SHA-256 in `apt-amd64.lock.json` (156 archives).
`install_apt.py` verifies every archive before invoking `dpkg`, checks the installed
versions and package audit, and has no online dependency resolver or fallback.
To update this lock, resolve the 20 `requested` packages with `--no-install-recommends`
against the CI toolchain, follow their installed Depends/Pre-Depends closure
(including installed virtual providers), download exact-version reinstalls of
the complete graph, and record the authenticated APT archive identities/hashes.
Build the prerequisite image to verify the complete graph.

This locks the CI-only additions. The Rust toolchain image layers still use
distribution repositories, and the BuildKit/frontend defaults are mutable. Those inherited inputs need a
separate reproducibility change before claiming every transitive build input is pinned.

Run from the repository root with Docker available:

```sh
python3 -m unittest discover -s ci -p 'test_*.py'
python3 -m unittest discover -s parity -p test_frozen_seeds.py
python3 parity/bin/frozen-seeds check
python3 parity/bin/frozen-seeds restore
docker build --target toolchain -t campfire-toolchain .
docker build --build-arg BASE_IMAGE=campfire-toolchain -f ci/Dockerfile -t campfire-correctness .
RUNNER_TEMP=/tmp/campfire-ci bash ci/verify-ignored.sh
RUNNER_TEMP=/tmp/campfire-ci bash ci/exec.sh bash ci/correctness.sh acme
# Repeat the last command for browsers, drive, livekit, messaging, and agents-ui.
# CI's slices: CORRECTNESS_SHARD=2/4 ... correctness.sh browsers (exec.sh passes it through).
```

The ignored runner uses `cargo nextest run --locked -p PACKAGE
--profile ci --build-jobs 4 -j 4 --no-fail-fast --run-ignored only --no-tests fail -E
"$(python3 ci/ignored_tests.py --filter SUITE)"`. Exact selectors come from
`ignored-tests.json`; the post-run check rejects missing, skipped, or failed
selected tests. `--success-output final` retains the successful nested browser
sequence and writer-control receipts. The first test shard additionally runs `nextest list --workspace
--exclude html5ever --run-ignored only --ignore-default-filter --message-format json`
against every compiled test binary. The package/binary/full-test-name set must
equal the correctness selectors plus the explicit `ignored-utilities.json` list
(66 correctness tests + 6 compiled utilities). This covers expanded conditional
attributes, procedural macros and `include!` without inferring their output from source.
Real compiler mutation probes exercise eight formatting/conditional/macro/include
forms. A lexical source guard also covers inactive `cfg_attr` branches and the
tools-only host source (7 source utilities), skips comments/literals, and fails
on an unclassifiable ignore. Exporters, recorders, measurements, and browser host helpers
remain ignored with `utility:` reasons. The two ignored kit documentation
examples remain documentation examples, not test attributes.

Native Selenium isolates Chromium/ChromeDriver with unprivileged user, network and PID
namespaces. The CI container permits these syscalls, and the pinned `iproute2`
prerequisite brings up its private loopback device. Generated browser-host test
executables use their own cache under `target/ws8bm-browser-host/`.
On Ubuntu runners, the messaging container uses a named AppArmor profile with an
explicit `userns` permission. The host's namespace restriction remains enabled.
`native-network-smoke.mjs` checks a real isolated driver endpoint and prints its
startup log on failure before the full 139-case suite starts.

Each correctness job uploads its full log, selected-test JUnit (where applicable),
and a JSON head/duration/exit receipt under `target/ci-receipts`. A red external
browser receipt stays red; the job supplies no correctness retries or exclusions.

The LiveKit gateway publishes its port marker by atomic rename after writing all
bytes. Rust waits on that marker before configuring its public endpoint. A real
filesystem regression pauses the writer with an empty file and asserts that the
ready marker remains absent until the complete port is published.
