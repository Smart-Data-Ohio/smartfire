# Rust correctness gates

`Rust port` gates clippy, the production-input binary build, all ordinary workspace
nextest tests, and runnable doctests. App/workspace failures are retained for the
summary, then explicit gates fail the job. No advisory correctness group remains.

The `Rust correctness (database|acme|browsers|livekit|messaging|agents-ui)` jobs share
the same seed/reference/toolchain setup through `.github/actions/rust-setup`.
They run on pull requests and pushes to main touching Rust or Rails comparison
inputs. Branch protection is managed separately by the release lead.

| Job suffix | Execution |
| --- | --- |
| database | `reference-tools/db/differential.sh --prepare-only`, `reference-tools/auth/rollback.sh --prepare-only`, then exactly 3 ignored Ruby DB/rollback comparisons |
| acme | Digest-pinned Pebble, then exactly 1 ignored TLS-ALPN certificate/cache test |
| browsers | Pinned Playwright image and gateway `ws` lockfile, then exactly 7 WS12, 1 WS13, and 1 gateway ignored tests |
| livekit | `bin/livekit-local setup/start` (checksum-pinned 1.13.7), polling/media transport regression tests, then exactly 1 ignored real-media test |
| messaging | `python3 reference-tools/messaging/behavior-check.py --keep-going` (all paired Rails/Rust cases, including native Selenium cases and persisted-row assertions) |
| agents-ui | `python3 reference-tools/views/agents_ui/system_behavior.py --binary target/debug/campfire --scenario all` (pages, budget, work against Rails and Rust) |

No external harness in the requested messaging/WS11 scope lacks a scripted entry
point. The screen-matrix pixel harness is outside this correctness package.

The main Rust/Debian images, archived Rails Ruby base, Playwright/Chromium image,
Node and Docker CLI images, and Pebble are digest-pinned. Nextest, libfaketime,
media sources, ChromeDriver (matching Chromium 153.0.8010.12), and LiveKit are
checksum-pinned. npm dependencies use committed lockfile integrity hashes.
Locally built layers inherit those pins and the committed build inputs.

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
RUNNER_TEMP=/tmp/campfire-ci bash rust/ci/exec.sh bash rust/ci/correctness.sh database
# Repeat the last command for acme, browsers, livekit, messaging, and agents-ui.
```

The ignored runner uses `cargo nextest run --locked -p PACKAGE
--profile ci --build-jobs 4 -j 4 --no-fail-fast --run-ignored only --no-tests fail -E
"$(python3 ci/ignored_tests.py --filter SUITE)"`. Exact selectors come from
`ignored-tests.json`; the post-run check rejects missing, skipped, or failed
selected tests. Exporters, recorders, measurements, and browser host helpers
remain ignored with `utility:` reasons. The two ignored kit documentation
examples remain documentation examples, not test attributes.

Each correctness job uploads its full log, selected-test JUnit (where applicable),
and a JSON head/duration/exit receipt under `target/ci-receipts`. A red external
browser receipt stays red; the job supplies no correctness retries or exclusions.
