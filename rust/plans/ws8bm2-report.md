# WS8bm2 message features C — coherent checkpoint; wider provider scope partial

Branch `rust/ws8bm2-message-features-c` starts at approved #197 head e3514fa1e2ee965fa5404e9bdd0c17e0d675cb21. Implementation commits a5e2ccbe9 and 18bd1c5f3 were pushed independently. The verification source is merge commit **0bd558eff241f985a56b5f5eba2cb505f85692ff**, with main **5f908337a953b8672472317b03bd0cd5de34194e** (including #197) as its second parent. Main merged automatically, keeping both sides; locked metadata exited 0 immediately afterward. The last checkpoint commit updates this report only; its pushed SHA is in the final reply. Rust-only work; no deployment or PR creation. Fresh-clone workspace: **4383 passed, 0 failed, 16 existing ignores** across 61 summaries. Strict clippy, release-input binary build and locked metadata all exited 0; 33/33 Rails fixtures replayed byte-for-byte. This checkpoint is ready for review; stopping after the final report push.

## Complete

- **P3 work inbox JSON:** `presenters/activity.rs` now distinguishes HTML and JSON source preloads. Work-event JSON skips actor and room rows it never serializes; ordinary Message/SavedItem JSON retains its room preload. `activity_items.rs` selects the correct loader, and standalone JSON `payload()` uses it too. The actual `/activity?type=threads` production-path regression includes writer-connection destination reads, compares the existing Rails JSON responses, and asserts zero unused actor/room association lookups. HTML/Turbo retain actor names, room names and owner source rendering.
- **P3 private/unknown PR pages:** `searches/preloads.rs` loads discussion mappings only for messages with at least one known-public card. Mixed pages still load the room-scoped mappings once. Claims, cache stamps, fragment-hit behavior, viewer zones and the owner's private projection are unchanged. Six real warm HTTP searches compare 60 complete Rails card containers; this uses production cache-key construction, not an isolated renderer.
- **Search headers:** 36 signed GETs cover missing/HTML/Turbo Accept, full/frame requests, search landing/no-match pages and shared profile controls, on bare Rails and `Rack::Deflater.new(Rails.application)`. All 18 production-stack Vary/header comparisons match. Empty-search form token presence matches; all 12 no-match search sections compare byte-for-byte. A real two-space Back-link indentation difference in `views/templates/searches/index.html` failed first and is fixed.
- **Older-window callbacks/jobs:** new pinned oracle covers public/private/unknown GitHub cards alongside generic and LinkedIn cards, including a suppressed message in each group, at 4 and 16 references. Forty newer roots put every provider reference outside the current room window. Thirty actual owner updates (PR title/state/review/check, generic metadata, LinkedIn metadata, negative fetch result retaining previous metadata, then empty metadata) produce 300 exact WebSocket frames. Six durable registered `Github::FetchPullRequestJob` executions use the real owner fetcher and HTTP/TLS client over the existing fixed-host test network and produce 60 further exact frames, with 24 authenticated outbound requests. The current-window HTTP exclusion is checked before and after every group.
- **Callback cost:** `channels/github_cards.rs` batches referencing messages, rooms, PRs, public discussion mappings and account facts, reusing the owner projection and detached renderer zone. `presenters/link_embeds.rs` uses the shared page preload and existing card-container renderer for a callback's reference set. `integrations/link_embed/store.rs` preloads visible sibling references and deduplicates fetch attempts by embed, inside the triggering transaction. Frame order, room stream, private projection, suppression and `maintain_scroll` remain unchanged.

## Header/body attribution

There is **no search-specific Vary defect** with identical requests and stacks. The bare Rails runner bypasses `config.ru`, so it lacks `Accept-Encoding`; the production Rack stack and Rust's `kit/deflater.rs` both add it. Exact examples: missing Accept gives bare Rails no Vary, production Rails/Rust `Accept-Encoding`; explicit HTML gives bare Rails `Accept`, production Rails/Rust `Accept,Accept-Encoding`. The same difference occurs on the profile control. Attribution: shared Rack/kit deflater layer (WS4), with oracle stack alignment in WS19; no shared production middleware was changed.

The first Rust probe accidentally used the Browser helper's default Accept for the missing-header case; this was corrected with an explicit empty Accept. Push metadata was aligned using both committed parity VAPID fixture keys; public-key-only boot leaves Rust's push transport disabled. Neither was a production fix.

Final raw bodies are retained under the fresh clone's `.scratch/ws8bm2-c/http-bodies/`, with Rails bodies in the assigned root's `.scratch/ws8bm2-c/final-rails-search_headers/db/`. Raw full-page diffs are `.scratch/ws8bm2-c/final-full-search-{0,6}.diff`. Diagnostic classification of per-request CSRF/nonce values and asset fingerprints leaves only two shared full-layout hunks: Rust has the approved `user-status:changed@window->member-panel#refreshPresence` action absent at d7c7de92 (approved #163 / WS8b-r2 drift), and the shared member-panel boundary has one fewer blank line (WS8b-m/layout owner). The corresponding frame responses have no remaining diagnostic hunks. Asset differences are the approved people/profile-card fingerprints; per-request token values are expected. Diagnostic classification is not a comparison mask in any passing regression: header and no-match-section assertions compare original bytes directly.

## Ownership, limits and remaining work

This is a coherent review checkpoint. **Unblocked provider follow-up remains; broader scope is partial.** The new older-window corpus covers root message PR/generic/LinkedIn updates and GitHub fetch jobs, but not every provider/thread/job permutation. Still to expand: mapped PR thread headers and reply/thread windows; older-window calendar-event, Fizzy and X updates/jobs; older-window generic/LinkedIn network fetch-job paths and stale-sibling job/rollback permutations beyond the inherited owner tests. These are named coverage gaps, not a claim that only owner-blocked work remains.

Date/coercion expansion stays **on hold for WS11-UI #196**. No time parser, multiparameter coercion or new date grammar was implemented; the existing pinned date/DST corpus still runs. There is no second `Date._parse` port.

The read-only `BoardSlaNudge` (WS12) and `AgentBudgetNotice` (WS11) activity seams remain until typed owner readers are exported. Real WS17 push, WS11 agent dispatch and WS13 huddle APIs remain wired. The new `TestApp::boot_with_github_reader` is a test-only injection of the owner's HTTP client; its default bootstrap keeps the previous client. `quote_integration_tests::insert_rows` shares existing fixture loading with the durable-job test. No provider transport or renderer implementation is replaced by a stand-in.

The four forbidden board files have no diff against the merged origin/main: `presenters/boards.rs`, `channel_thread/board.rs`, `channel_thread/work.rs`, and `board_posts.rs`. Their main-merge changes belong to their owners. #179 viewer-zone/cache behavior is unchanged. No timing threshold, retry, test concurrency or ignore was changed. No new browser harness was built.

Accepted browser inventory (not rerun in this slice): polls 4, pins/saved 7, slash 26, search/files 4, scheduling 4 = **45/45 on both apps** in the prior accepted checkpoint. The existing controller/deferred Rails-file inventories are retained in report history at 950e5de / main 056ab49ac. This slice adds four test functions and three Rails fixtures; it does not claim new ports of other deferred Rails files.

## Failing-first evidence

All failing-first commands used the existing four test threads and two Cargo build jobs, via the assigned-root CI adapter in `.scratch/ws8bm2-c/ci-env.sh`.

1. P3 regressions against e3514fa1 production, with only regression/vector registration added: `.scratch/ws8bm2-c/preload-before.log`, exit 101. Exact cards/JSON passed; unused-query assertions failed. Command: `CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh test --locked -p campfire -j2 controllers::message_features -- --test-threads=4 --nocapture`.

```text
WS8bm2 private provider page: 4 messages; Rust 49 reads / 1 discussion lookups; Rails 32 reads
WS8bm2 private provider page: 16 messages; Rust 49 reads / 1 discussion lookups; Rails 32 reads
WS8bm2 unknown provider page: 4 messages; Rust 49 reads / 1 discussion lookups; Rails 32 reads
WS8bm2 unknown provider page: 16 messages; Rust 49 reads / 1 discussion lookups; Rails 32 reads
WS8bm2 mixed provider page: 4 messages; Rust 49 reads / 1 discussion lookups; Rails 33 reads
WS8bm2 mixed provider page: 16 messages; Rust 49 reads / 1 discussion lookups; Rails 33 reads
WS8bm2 WS12 work inbox: 4 results; Rust 10 reads / 1 work reads; Rails 12 reads / 1 work reads
test result: FAILED. 173 passed; 2 failed; 0 ignored; 0 measured; 2108 filtered out; finished in 29.63s
```

2. Controlled search indentation regression: all corrected request headers and forgery settings retained, with only the old e3514fa1 two-space indentation restored. `.scratch/ws8bm2-c/headers-before-controlled.log`, exit 101. The failing left string has four spaces before `<a class="btn searches__back">`; Rails has six. Command: `CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh test --locked -p campfire -j2 controllers::message_features::search_header_tests -- --test-threads=4 --nocapture`.

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2283 filtered out; finished in 1.16s
```

3. Older-window regression with GitHub/embed callback production still at e3514fa1 (P3 changes already present but unrelated to these callback paths), plus new fixture/tests/test bootstrap: `.scratch/ws8bm2-c/older-before.log`, exit 101. All 360 frame bytes and six jobs already matched Rails; the 4/16 reader-plus-writer cost assertion failed for all fifteen mode/operation pairs. Command: `CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh test --locked -p campfire -j2 controllers::message_features::older_provider_tests -- --test-threads=4 --nocapture`.

```text
WS8bm2 older-provider false github: 4 messages; Rust 30 reads; Rails 8 reads; 4 exact frames
WS8bm2 older-provider false embed: 4 messages; Rust 135 reads; Rails 7 reads; 4 exact frames
WS8bm2 older-provider false linkedin: 4 messages; Rust 135 reads; Rails 7 reads; 4 exact frames
WS8bm2 older-provider false negative: 4 messages; Rust 135 reads; Rails 7 reads; 4 exact frames
WS8bm2 older-provider false empty: 4 messages; Rust 135 reads; Rails 7 reads; 4 exact frames
WS8bm2 older-provider false github: 16 messages; Rust 90 reads; Rails 20 reads; 16 exact frames
WS8bm2 older-provider false embed: 16 messages; Rust 531 reads; Rails 19 reads; 16 exact frames
WS8bm2 older-provider false linkedin: 16 messages; Rust 531 reads; Rails 19 reads; 16 exact frames
WS8bm2 older-provider false negative: 16 messages; Rust 531 reads; Rails 19 reads; 16 exact frames
WS8bm2 older-provider false empty: 16 messages; Rust 531 reads; Rails 19 reads; 16 exact frames
WS8bm2 older-provider true github: 4 messages; Rust 26 reads; Rails 7 reads; 4 exact frames
WS8bm2 older-provider true embed: 4 messages; Rust 131 reads; Rails 7 reads; 4 exact frames
WS8bm2 older-provider true github: 16 messages; Rust 74 reads; Rails 19 reads; 16 exact frames
WS8bm2 older-provider true embed: 16 messages; Rust 515 reads; Rails 19 reads; 16 exact frames
WS8bm2 older-provider null github: 4 messages; Rust 26 reads; Rails 7 reads; 4 exact frames
WS8bm2 older-provider null embed: 4 messages; Rust 131 reads; Rails 7 reads; 4 exact frames
WS8bm2 older-provider null github: 16 messages; Rust 74 reads; Rails 19 reads; 16 exact frames
WS8bm2 older-provider null embed: 16 messages; Rust 515 reads; Rails 19 reads; 16 exact frames
test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 2284 filtered out; finished in 6.81s
```

The intermediate ordinary-inbox format check caught an overbroad JSON-room optimization; it was narrowed in production without weakening the owner test. Final `controllers::presenters::activity` check:

```text
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 2280 filtered out; finished in 34.93s
```

Final focused feature namespace before merge (warning about the obsolete single-message callback adapter was then removed), exit 0:

```text
WS8bm2 older-provider jobs: 6/6 durable registered fetch jobs; 60/60 Rails WebSocket frames; 24/24 authenticated owner API reads; all roots outside current windows
WS8bm2 older-provider updates: 30/30 updates; 300/300 Rails WebSocket frames; constant 4/16 reader plus writer cost
WS8bm2 search header probe: 18/18 production-stack Vary/header comparisons; 12/12 exact no-match search sections; search form token presence matches; bare Rails has 18 Accept-Encoding differences, including shared profile controls
test result: ok. 178 passed; 0 failed; 0 ignored; 0 measured; 2108 filtered out; finished in 34.98s
```

## Fresh-clone commands and raw receipts

All gates run against committed merge 0bd558eff in a fresh local clone. Only the Cargo registry cache is copied; all three seeds are built independently. The existing normal native build cache is reused, not a scratch fixture directory. CI mode fails on missing seeds. The assigned-root adapter preserves the configured rustc throttle/slot file, uses two Cargo build jobs, four unchanged test threads, a four-CPU container cap and owned ports 52500–52599. No agents were spawned; the owned workspace/clippy/release commands ran sequentially. Independent Rails replay ran during the workspace gate.

Preparation commands, each exit 0:

```sh
git fetch origin
git switch -c rust/ws8bm2-message-features-c origin/rust/ws8bm2-message-features-b
git merge --no-ff origin/main -m 'Merge main after #197, preserving provider callback parity' -m 'Co-Authored-By: GPT-6.1 Sol <noreply@openai.com>'
source .scratch/ws8bm2-c/ci-env.sh
CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh metadata --locked --format-version 1 > .scratch/ws8bm2-c/merge-metadata.json 2> .scratch/ws8bm2-c/merge-metadata.err
git clone --no-hardlinks --branch rust/ws8bm2-message-features-c "$PWD" .scratch/ws8bm2-c/fresh
mkdir -p .scratch/ws8bm2-c/fresh/rust/.cargo-home
cp -a rust/.cargo-home/registry .scratch/ws8bm2-c/fresh/rust/.cargo-home/
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 bash .scratch/ws8bm2-c/fresh/rust/parity/bin/seed build default first_run agents_ui
```

Raw seed summary:

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
seed: building agents_ui
seed: agents_ui -> parity/.seed/agents_ui (6.1M)
```

From the fresh clone, with the same assigned-root adapter:

```sh
CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh test --locked --workspace --no-fail-fast -j2 -- --test-threads=4 --nocapture > "$WS8BM2_ROOT/.scratch/ws8bm2-c/fresh-workspace.log" 2>&1
CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh clippy --locked --workspace --all-targets -j2 -- -D warnings > "$WS8BM2_ROOT/.scratch/ws8bm2-c/fresh-clippy.log" 2>&1
CARGO_TARGET_DIR=/native-target bash rust/ci/with-release-inputs.sh bash ./ci/cargo.sh build --locked --workspace --bins > "$WS8BM2_ROOT/.scratch/ws8bm2-c/fresh-release-inputs.log" 2>&1
CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh metadata --locked --format-version 1 > "$WS8BM2_ROOT/.scratch/ws8bm2-c/fresh-metadata.json" 2> "$WS8BM2_ROOT/.scratch/ws8bm2-c/fresh-metadata.err"
```

Raw exit receipts:

```text
workspace exit 0
clippy exit 0
release-inputs exit 0
metadata exit 0
```

All 61 raw workspace summaries, including vendored html5ever and doctests; CI seeds required, no new ignores and no timing flakes/failure reruns:

```text
test result: ok. 2390 passed; 0 failed; 7 ignored; 0 measured; 0 filtered out; finished in 583.43s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.50s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.45s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 1256 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 152.35s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.65s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.97s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.97s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.18s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.46s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.54s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.13s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.77s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.63s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.92s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.87s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

```text
WS8bm2 fresh workspace aggregate: 4383 passed; 0 failed; 16 ignored; 61 summaries; exit 0
```

Relevant raw receipts from that same full workspace run (physical reads; Rails excludes cached/schema notifications):

```text
WS8bm2 older-provider false github: 4 messages; Rust 14 reads; Rails 8 reads; 4 exact frames
WS8bm2 older-provider false embed: 4 messages; Rust 37 reads; Rails 7 reads; 4 exact frames
WS8bm2 older-provider false linkedin: 4 messages; Rust 37 reads; Rails 7 reads; 4 exact frames
WS8bm2 older-provider false negative: 4 messages; Rust 37 reads; Rails 7 reads; 4 exact frames
WS8bm2 older-provider false empty: 4 messages; Rust 37 reads; Rails 7 reads; 4 exact frames
WS8bm2 older-provider false github: 16 messages; Rust 14 reads; Rails 20 reads; 16 exact frames
WS8bm2 older-provider false embed: 16 messages; Rust 37 reads; Rails 19 reads; 16 exact frames
WS8bm2 older-provider false linkedin: 16 messages; Rust 37 reads; Rails 19 reads; 16 exact frames
WS8bm2 older-provider false negative: 16 messages; Rust 37 reads; Rails 19 reads; 16 exact frames
WS8bm2 older-provider false empty: 16 messages; Rust 37 reads; Rails 19 reads; 16 exact frames
WS8bm2 older-provider true github: 4 messages; Rust 13 reads; Rails 7 reads; 4 exact frames
WS8bm2 older-provider true embed: 4 messages; Rust 36 reads; Rails 7 reads; 4 exact frames
WS8bm2 older-provider true linkedin: 4 messages; Rust 36 reads; Rails 7 reads; 4 exact frames
WS8bm2 older-provider true negative: 4 messages; Rust 36 reads; Rails 7 reads; 4 exact frames
WS8bm2 older-provider true empty: 4 messages; Rust 36 reads; Rails 7 reads; 4 exact frames
WS8bm2 private provider page: 4 messages; Rust 48 reads / 0 discussion lookups; Rails 32 reads
WS8bm2 private provider page: 16 messages; Rust 48 reads / 0 discussion lookups; Rails 32 reads
WS8bm2 unknown provider page: 4 messages; Rust 48 reads / 0 discussion lookups; Rails 32 reads
WS8bm2 unknown provider page: 16 messages; Rust 48 reads / 0 discussion lookups; Rails 32 reads
WS8bm2 mixed provider page: 4 messages; Rust 49 reads / 1 discussion lookups; Rails 33 reads
WS8bm2 mixed provider page: 16 messages; Rust 49 reads / 1 discussion lookups; Rails 33 reads
WS8bm2 private/unknown/mixed pages: 60/60 Rails card containers; six production warm HTTP searches
WS8bm2 older-provider true github: 16 messages; Rust 13 reads; Rails 19 reads; 16 exact frames
WS8bm2 older-provider true embed: 16 messages; Rust 36 reads; Rails 19 reads; 16 exact frames
WS8bm2 older-provider true linkedin: 16 messages; Rust 36 reads; Rails 19 reads; 16 exact frames
WS8bm2 older-provider true negative: 16 messages; Rust 36 reads; Rails 19 reads; 16 exact frames
WS8bm2 older-provider true empty: 16 messages; Rust 36 reads; Rails 19 reads; 16 exact frames
WS8bm2 older-provider null github: 4 messages; Rust 13 reads; Rails 7 reads; 4 exact frames
WS8bm2 older-provider null embed: 4 messages; Rust 36 reads; Rails 7 reads; 4 exact frames
WS8bm2 older-provider null linkedin: 4 messages; Rust 36 reads; Rails 7 reads; 4 exact frames
WS8bm2 older-provider null negative: 4 messages; Rust 36 reads; Rails 7 reads; 4 exact frames
WS8bm2 older-provider null empty: 4 messages; Rust 36 reads; Rails 7 reads; 4 exact frames
WS8bm2 older-provider jobs: 6/6 durable registered fetch jobs; 60/60 Rails WebSocket frames; 24/24 authenticated owner API reads; all roots outside current windows
WS8bm2 older-provider null github: 16 messages; Rust 13 reads; Rails 19 reads; 16 exact frames
WS8bm2 older-provider null embed: 16 messages; Rust 36 reads; Rails 19 reads; 16 exact frames
WS8bm2 older-provider null linkedin: 16 messages; Rust 36 reads; Rails 19 reads; 16 exact frames
WS8bm2 older-provider null negative: 16 messages; Rust 36 reads; Rails 19 reads; 16 exact frames
WS8bm2 older-provider null empty: 16 messages; Rust 36 reads; Rails 19 reads; 16 exact frames
WS8bm2 older-provider updates: 30/30 updates; 300/300 Rails WebSocket frames; constant 4/16 reader plus writer cost
WS8bm2 search header probe: 18/18 production-stack Vary/header comparisons; 12/12 exact no-match search sections; search form token presence matches; bare Rails has 18 Accept-Encoding differences, including shared profile controls
WS8bm2 WS12 work inbox: 4 results; Rust 8 reads / 1 work reads; Rails 12 reads / 1 work reads
WS8bm2 WS12 work inbox: 16 results; Rust 8 reads / 1 work reads; Rails 24 reads / 1 work reads
```

Strict all-target clippy, exit 0:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 56.06s
```

Release-input binary build, exit 0:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 01s
```

Locked metadata is valid JSON; both merge and fresh-clone commands exited 0 with empty stderr.

## Independent Rails replay

From the final clone, each of the three new fixtures and the two #197 fixtures is rerun from a separate storage copy of its newly built default seed; comparisons are exact and the shell uses `set -e`:

```sh
for name in private_provider_pages search_headers older_provider_callbacks ws12_consumers provider_batch; do
  storage="$PWD/../final-rails-$name"
  mkdir -p "$storage"
  cp -a rust/parity/.seed/default/. "$storage/"
  PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 bash rust/parity/bin/reference runner --storage "$storage" --time 2026-03-02T16:00:00Z --freeze "rust/reference-tools/messaging/$name.rb" "/rails/storage/db/$name.json"
  cmp "rust/vectors/messaging/$name.json" "$storage/db/$name.json"
done
python3 rust/reference-tools/messaging/verify_oracles.py
python3 rust/reference-tools/messaging/features-reference-check.py
```

All commands above exited 0. The five new/preceding fixtures' raw generator lines and success receipts:

```text
WS8bm2 provider Rails oracle: 11 GitHub containers; 7 embed/LinkedIn containers; 8 fixture tables
WS8bm2 private provider Rails: private 4=32 reads; private 16=32 reads; unknown 4=32 reads; unknown 16=32 reads; mixed 4=33 reads; mixed 16=33 reads; 60 card containers
WS8bm2 final oracle replay: private_provider_pages.json byte-identical
WS8bm2 Rails search header probe: 36 bare/wrapped full/frame requests; [nil, "Accept", "Accept-Encoding", "Accept,Accept-Encoding"]
WS8bm2 final oracle replay: search_headers.json byte-identical
WS8bm2 older-provider Rails oracle: 6 groups; 30 real updates; 6 real fetch jobs; 360 socket frames; all roots outside 40-message windows
WS8bm2 final oracle replay: older_provider_callbacks.json byte-identical
WS8bm2 WS12 consumer Rails: 15 workflow steps; work inbox 4 items=12 reads/1 work reads; 16 items=24 reads/1 work reads
WS8bm2 final oracle replay: ws12_consumers.json byte-identical
WS8bm2 provider Rails oracle: 11 GitHub containers; 7 embed/LinkedIn containers; 8 fixture tables
WS8bm2 populated provider Rails: 4 messages=35 reads; 16 messages=35 reads; 40 GitHub/event containers
WS8bm2 final oracle replay: provider_batch.json byte-identical
```

The 28 existing independent fixtures and source verification also passed:

```text
WS8bm2 oracle replay: features.json byte-identical
WS8bm2 oracle replay: saved.json byte-identical
WS8bm2 oracle replay: scheduled.json byte-identical
WS8bm2 oracle replay: search.json byte-identical
WS8bm2 oracle replay: preloads.json byte-identical
WS8bm2 oracle replay: slash.json byte-identical
WS8bm2 oracle replay: links_files.json byte-identical
WS8bm2 oracle replay: reminder_push.json byte-identical
WS8bm2 oracle replay: quote_integration.json byte-identical
WS8bm2 oracle replay: root_cache.json byte-identical
WS8bm2 oracle replay: panels.json byte-identical
WS8bm2 oracle replay: date_inputs.json byte-identical
WS8bm2 oracle replay: review_saved_race.json byte-identical
WS8bm2 oracle replay: review_dates.json byte-identical
WS8bm2 oracle replay: date_compact_widths.json byte-identical
WS8bm2 oracle replay: providers.json byte-identical
WS8bm2 oracle replay: provider_edits.json byte-identical
WS8bm2 oracle replay: event_cards.json byte-identical
WS8bm2 oracle replay: date_coercions.json byte-identical
WS8bm2 oracle replay: composer.json byte-identical
WS8bm2 oracle replay: composer_sti.json byte-identical
WS8bm2 oracle replay: twitter_preloads.json byte-identical
WS8bm2 oracle replay: twitter_cards.json byte-identical
WS8bm2 oracle replay: twitter_text.json byte-identical
WS8bm2 oracle replay: provider_callbacks.json byte-identical
WS8bm2 oracle replay: agent_command.json byte-identical
WS8bm2 oracle replay: user_coercions.json byte-identical
WS8bm2 oracle replay: date_years.json byte-identical
WS8bm2 oracle replay: 28/28 independently replayed fixtures byte-identical
WS8bm2 reference source check: 106 controller, model, helper and template files match d7c7de92
WS8bm2 reference check self-test: 2 injected source-byte/file-set differences rejected
```

## Cleanup and final state

The fresh clone produced exactly four generated JSON exports under `rust/target`; those files were inspected and removed by exact name, then the empty directory was removed. `test ! -e .scratch/ws8bm2-c/fresh/rust/target` passed. Raw cleanup receipt:

```text
WS8bm2 scratch cleanup: four generated JSON exports removed; fresh clone target absent
```

The release guard removed its temporary input directory. No owned test/reference container remains running. The fresh clone has no source edits; its untracked `.scratch/` contains output-only HTTP artifacts, never fixture inputs. Evidence, rebuilt seeds and the clone remain in the assigned `.scratch/ws8bm2-c/`; the normal native Cargo cache remains available. No extra scratch build target remains. The configured rustc throttle and Python model server were untouched; no stash was used.
