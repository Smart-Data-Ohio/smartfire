# Rust correctness gates

`Rust port` gates clippy, the production-input binary build, all ordinary workspace
nextest tests, and runnable doctests. App/workspace failures are retained for the
summary, then explicit gates fail the job. No advisory correctness group remains.

The `Rust correctness (database|acme|browsers|livekit|messaging|agents-ui)` jobs share
the same seed/reference/toolchain setup through `.github/actions/rust-setup`.
The slim pull-request gate runs only `Rust port`; correctness jobs run on main,
nightly, and manual workflows. Branch protection is managed separately by the release lead.

| Job suffix | Execution |
| --- | --- |
| database | `reference-tools/db/differential.sh --prepare-only`, `reference-tools/auth/rollback.sh --prepare-only`, then exactly 3 ignored Ruby DB/rollback comparisons |
| acme | Digest-pinned Pebble, then exactly 1 ignored TLS-ALPN certificate/cache test |
| browsers | Pinned Playwright image and gateway `ws` lockfile, then exactly 7 WS12, 1 WS13, and 1 gateway ignored tests; C221–C223 also run the three paired inbox/filter/work sequences and reject their writer-defect controls |
| livekit | `bin/livekit-local setup/start` (checksum-pinned 1.13.7), polling/media transport regression tests, then exactly 1 ignored real-media test |
| messaging | Python/Node harness regression tests, `python3 reference-tools/messaging/behavior-check.py --keep-going` (paired Rails/Rust cases), then 53 registered original WS14/WS15 browser declarations against Rust via the pinned native Selenium image |
| agents-ui | `python3 reference-tools/views/agents_ui/system_behavior.py --binary target/debug/campfire --scenario all` (pages, budget, work against Rails and Rust) |

No external harness in the requested messaging/WS11 scope lacks a scripted entry
point. The screen-matrix pixel harness is outside this correctness package.

The main Rust/Debian images, archived Rails Ruby base, Playwright/Chromium image,
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

This locks the CI-only additions. The earlier Rust/Rails/parity image layers still
use distribution repositories, the BuildKit/frontend defaults are mutable, and
the Rails gem lockfile has no archive checksums. Those inherited inputs need a
separate reproducibility change before claiming every transitive build input is pinned.

Run from the repository root with Docker available:

```sh
python3 -m unittest discover -s rust/ci -p 'test_*.py'
python3 -m unittest discover -s rust/parity -p test_ci_seed.py
rust/parity/bin/ci-seed prepare
rust/parity/bin/ci-seed image
rust/parity/bin/ci-seed build
rust/parity/bin/ci-seed validate
docker build --target toolchain -f rust/Dockerfile -t campfire-toolchain rust
docker build --build-arg BASE_IMAGE=campfire-toolchain -f rust/ci/Dockerfile -t campfire-correctness rust
RUNNER_TEMP=/tmp/campfire-ci bash rust/ci/verify-ignored.sh
RUNNER_TEMP=/tmp/campfire-ci bash rust/ci/exec.sh bash rust/ci/correctness.sh database
# Repeat the last command for acme, browsers, livekit, messaging, and agents-ui.
```

The ignored runner uses `cargo nextest run --locked -p PACKAGE
--profile ci --build-jobs 4 -j 4 --no-fail-fast --run-ignored only --no-tests fail -E
"$(python3 ci/ignored_tests.py --filter SUITE)"`. Exact selectors come from
`ignored-tests.json`; the post-run check rejects missing, skipped, or failed
selected tests. `--success-output final` retains the successful nested browser
sequence and writer-control receipts. The ordinary job additionally runs `nextest list --workspace
--exclude html5ever --run-ignored only --ignore-default-filter --message-format json`
against every compiled test binary. The package/binary/full-test-name set must
equal the correctness selectors plus the explicit `ignored-utilities.json` list
(67 correctness tests + 6 compiled utilities). This covers expanded conditional
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
startup log on failure before the full 145-case paired suite starts.

Each correctness job uploads its full log, selected-test JUnit (where applicable),
and a JSON head/duration/exit receipt under `target/ci-receipts`. A red external
browser receipt stays red; the job supplies no correctness retries or exclusions.

The LiveKit gateway publishes its port marker by atomic rename after writing all
bytes. Rust waits on that marker before configuring its public endpoint. A real
filesystem regression pauses the writer with an empty file and asserts that the
ready marker remains absent until the complete port is published.
