# Overnight report

## Morning summary

**The port is done and matches Rails.**

| Gate (final HEAD, all 5 seeds) | Result |
|---|---|
| Ruby vs Ruby, lean | 970/970, 0 flaky |
| Rust vs Rails, lean | 970/970, 1 flaky (a one-gray-level antialiasing arc), 0 allowlisted |
| Rust vs Rails, full matrix | All 5,018 cells compared. It found 2 Rust-side bugs, only visible on Firefox and WebKit, and both are fixed (c2b253c, f2a1a8f). Every cell compared on the fixed builds passes. The last part, laptop and tablet on the default seed (3881a02), passed 2,174/2,174 with 0 flaky. |
| Rust vs Rails, lean, after those fixes (f2a1a8f) | 969/970. The one failure was the sidebar-toggle arc, which Ruby vs Ruby shows too. |
| Rust vs Rails, lean, with the arc fixed in the harness (2a6df22) | 970/970, 0 flaky. |

- **What the gates check:**
  - server output for every state: HTML, live DOM, accessibility tree, every subresource and the
    cable frames;
  - pixels on Chromium at desktop and phone in light and dark, plus a smoke set on Firefox and WebKit.
- **Other checks:**
  - the header-shape sweep shows 0 differences;
  - all workspace tests pass;
  - Rails boots on a database the Rust app has written.
- **Thruster is gone.** `campfire server` does the TLS/ACME, HTTP/2, caching and compression itself.
  Thruster's certificate storage is compatible both ways.
- **Performance (final benchmark):** 9–19× Rails throughput on pages, 19× on message POSTs and
  22–26× on cable fan-out. It cold-starts 10.6× faster and uses 6.3× less memory at idle. The
  optimization passes roughly tripled the Rust app's own throughput: gzip on zlib-rs, fragment-first
  rendering, WAL checkpoints off the writer, cached statements, cable frames shared across
  subscribers, and LTO plus jemalloc.
- **Still open or worth a look:**
  - The fragment cache is now bounded by bytes, like Rails' `MemoryStore`: 32 MB by default, set
    with `CAMPFIRE_FRAGMENT_CACHE_MB` (1d6ac20). After 60k message POSTs the process holds 129 MB,
    down from 1.28 GB, with no throughput change. The reference's Redis cache has no bound at all.
  - Harness: HTTP-01 ACME is unit-tested only. The sidebar-toggle arc flake is root-caused and
    fixed (2a6df22): it was stale Chromium raster, set by a race in the page, not either server.
  - Reference bugs the port reproduces faithfully, worth fixing upstream in Campfire:
    - Edge user agents get a 500 on profile and room pages, because `install-edge.svg` is missing.
    - `/searches?q=NOT` raises.
    - new-ping autocomplete never shows suggestions.
    - a mention of a deleted user blanks the whole message.
    - editing a message with a missing attachment raises.
    - `directs#show` returns 500.
  - There's one incident to note under Incidents below.
- **Repo:** github.com/basecamp/once-campfire-rust (private). Start with `plans/rust-conversion.md`,
  then `AGENTS.md`.

The running log follows.

## Log

- **Thruster replaced** (6b0d797). `campfire server` now does TLS with automatic ACME (tested
  against Pebble), HTTP/2, Thruster's response cache and zstd/gzip, and reads every Thruster env
  var. Certificates are stored in `/rails/storage/thruster`, so existing installs keep theirs.
  Thruster 0.1.23 serves a certificate the new binary wrote. HTTP-01 is only unit-tested.
- **Header-shape parity** (a0f5914). `reference-tools/http_shape/sweep.py` finds 0 differences
  across about 70 requests (pages, streams, JSON, assets, avatars, blobs, 304s, HEAD, errors).
- **Performance attribution** (96d10ea, `plans/perf-attribution.md`). gzip is 62% of room-show
  CPU. Cached message fragments were still fully rebuilt before the cache lookup. Cable zero-filled
  a 128 KiB buffer on every read. The POST p99 stall is SQLite's WAL checkpoint fsyncing on the
  writer thread.

## Incidents

- While fetching the mimalloc source, the profiling agent sent the user's email address in the
  User-Agent header of one crates.io API request. It didn't happen again, and nothing else left
  the machine.

## Log, continued

- **Lean gate committed** (4797f84). Ruby vs Ruby: 963 of 970 cells identical; crowd, custom_styles
  and restricted are fully clean. What remains is a random join code on first run (4 cells) plus 3
  rare pixel flakes.
- **Harness determinism fixes that landed with it:**
  - one Action Cable command worker in the reference;
  - frozen clocks for every server;
  - a capture container with no network of its own;
  - fixed-tick readiness;
  - scripts delivered in request order;
  - pinned Chromium font and Skia flags.
- **Reference bug found:** new-ping autocomplete never shows suggestions, because a plain fetch
  requests JSON and gets HTML back.
- **Rust vs Rails, first lean run:**

  | Seed | Pass | Of |
  |---|---|---|
  | crowd | 25 | 25 |
  | custom_styles | 33 | 33 |
  | restricted | 8 | 8 |
  | first_run | 12 | 16 (the join-code cells) |
  | default | 822 | 888 (54 fail, 12 error) |

- **Coordinator decisions:**
  - Mask the random join code, since seeding Ruby's RNG can't make Rust match.
  - A pixel-only difference whose server output is identical gets up to 2 re-captures; if one
    matches, the cell passes but is flagged flaky. Server-output differences are never retried.
- **Cable optimization pass** (4e77f67, 95f2d93, 95e0a3e, a6616a2, a119dab). Measured at 1,000
  clients, protocol output byte-identical:

  | | Before | After |
  |---|---|---|
  | Server CPU per delivery | 62.6 µs | 13.4 µs (4.7×) |
  | Sustained messages per second | 57 | 198 (3.5×, now bound by the load generator) |
  | Delivery latency p50 / p99 | 22.8 / 46.0 ms | 11.0 / 17.3 ms |
  | Memory under saturation | 208 MB | 112 MB |

  How: a 4 KiB read buffer read in its own task, each frame encoded once and shared across
  subscribers, batched writes, and connections reading straight from the hub's ring buffers.
- **Rust vs Rails app fixes** (3b70734, 69bda70, eda40f2, 03218c3). A rerun of every affected
  state passed 114 of 114 cells.
  - Broadcast URLs drop the request's port, as Rails' `ApplicationController.renderer` does.
  - A panicking action now returns the normal 500 page.
  - Empty autocomplete keeps ERB's trailing newline, which gives it Rack's ETag.
  - The front server writes `Date` from the real clock. A frozen `Date` made Chrome treat
    preloaded assets as stale and stall the sign-in pages.
  - What remains is the composer focus-ring timing flake, which the harness agent is fixing.
- **PARITY MILESTONE** (9ae1c15, candidate built from a clean archive of e89cc43):

  | Gate | Result |
  |---|---|
  | Ruby vs Ruby, lean, all seeds | 970/970 pass, 1 flaky |
  | Rust vs Rails, lean, all seeds | 970/970 pass, 2 flaky, 0 allowlisted |

  The flaky cells are all the same issue: one gray level on the phone sidebar toggle's arc after
  the sign-up redirect. Their server output is identical. How the gate got here:
  - The random join code is masked on the first-run page (typed text placeholder, plus pixel masks
    over the invite field and QR code).
  - The composer focus race was root-caused and fixed: Playwright's per-document clock offset
    decided whether Lexxy's rAF mount ran before the composer's zero-delay focus.
  - `reference`/`candidate` `down --all` now only stop instances the caller owns.
- **HTTP optimization pass** (399e035, b9d269e, 2741830, 12197b5, 4dd3ff3, e89cc43, b93306d,
  786a74d). Each commit passed the full test suite and the header-shape sweep; the later commits
  also passed the HTTP-heavy lean parity subset.

  | Change | Effect |
  |---|---|
  | gzip on zlib-rs | room show 662 → 955 req/s |
  | Look up the fragment before building the view | room show 989 → 1,664; messages page 1,116 → 2,234 |
  | WAL checkpoints on their own thread | POST p99 at c=1: 12.5 → 1.7 ms |
  | Prepared statements cached everywhere | POST 4,257 → 4,693 |
  | Fragments shared, not copied; sidebar LIMIT inlined | messages page 2,233 → 2,666 |
  | Stylesheet tags built once per process | 6–10% less CPU per request |
  | Fat LTO + codegen-units=1 + jemalloc | 5–14% faster per route; idle RSS 11 → 45 MB; builds about 2× slower |

  Measured natively on 4 pinned cores; see the commits for conditions. What's left on hot pages is
  gzip level 6 (55–70% of CPU) and Rack's SHA-256 ETag, both required for parity with Rails.
- **Final lean gate** (candidate `campfire-candidate:head-5181aa4`, built from a clean `git archive`
  of HEAD plus the pinned `reference` submodule, with the cargo build stage uncached so the cached
  target dir couldn't stand in for the sources):

  | Gate | Result |
  |---|---|
  | Ruby vs Ruby, lean, all seeds (out/lean-final-self) | 970/970 pass, 0 flaky |
  | Rust vs Rails, lean, all seeds (out/rust-lean-final) | 970/970 pass, 1 flaky, 0 allowlisted |

  The flaky cell is the known sidebar-toggle arc (`auth/join/completed @ chromium-phone-light`).
  Before the gate ran, the benchmark's no-gzip runs turned up one real difference, and it was fixed
  first (fa1deb9). The front server's response cache never stored a streamed body of declared
  length: hyper stops polling after the last `Content-Length` byte, so the end of stream never
  arrived. As a result, avatars (`send_file`) requested with `Accept-Encoding: identity` always
  answered `X-Cache: miss`, where Thruster answers `hit`, and always went back to the app.
  Browsers always send gzip, so no parity cell could see it, and the header-shape sweep ignores
  `x-cache`.
- **Final benchmark** (`bench/results/final-20260927/report.md`). Production images, `5181aa4`
  against the reference, host networking, a quiet host, 5 interleaved reps. Rust vs Rails:

  | Measurement | Rust vs Rails |
  |---|---|
  | Pages, throughput at c=16 | 9–19× |
  | Message POSTs, throughput | 19× |
  | Cable deliveries/s | 22–26× |
  | Cold start | 10.6× faster |
  | Idle memory | 6.3× less |
  | App process at 1,000 cable clients | 2.6–3.0× less |

  Without gzip, Rust's pages serve 1.8–4.2× more requests again.

  One surprise: after the HTTP suite the Rust process holds 1.37 GB. That is the fragment store
  filled to its 50,000-entry cap by the ~100k messages the POST suite creates (Rails creates ~5.8k
  in the same time). It needs a byte bound.
- **Fragment store bounded by bytes** (1d6ac20). This fixes the 1.37 GB surprise in the final
  benchmark. The reference keeps fragments in Redis, and its `config/redis.conf` sets no
  `maxmemory`, so there is no bound to copy. The store now works like Rails' `MemoryStore`:
  - Each entry counts its key, its payload and 240 bytes.
  - Going over the limit evicts least recently used entries down to three quarters of it.
  - The default limit is 32 MB, and `CAMPFIRE_FRAGMENT_CACHE_MB` sets it.
  - Fragments are shrunk to fit before they're stored. Askama's size hint had left most of each
    fragment's allocation as spare capacity, which the `Arc` kept alive.

  Measured on native builds pinned to cores 8–15, after 60k POSTs (RssAnon):

  | Build | RssAnon |
  |---|---|
  | Before | 1,280 MB |
  | After | 129 MB |
  | After, with the store off | 92 MB |

  Room show and POSTs, 3 interleaved reps, didn't change beyond noise: room show c=16 at
  2,212 → 2,216 req/s, and POST c=16 at 4,532 → 4,479 req/s. The workspace tests pass, and the
  header sweep shows 0 differences.

## Final full-matrix parity

Rust vs Rails with `--matrix full`: 3 engines × 4 viewports × 2 schemes, plus the breakpoint sweep
on every engine. Candidates were built from a clean `git archive` of each commit plus the pinned
`reference` submodule. The cargo target cache mount was removed and the build stage was run with
`--no-cache-filter build`, so every build compiled for real (2–3 minutes, `campfire` included).
Captures ran at 8 workers or fewer. Every container was pinned with `docker update --cpuset-cpus
0-7,16-23`, away from the benchmark cores 8–15 and their SMT siblings 24–31. Nothing was masked or
allowlisted. Output is in `parity/out/rust-full-final/` and `parity/out/rust-lean-fixed/`.

**All 5,018 cells were compared.** The first pass covered 3,133 and was stopped partway through the
default seed's laptop cells. A second pass ran the default seed's laptop and tablet cells in full,
on a candidate built the same way from a clean archive of 3881a02 (`campfire-rust:head-3881a02`,
`campfire` compiled from scratch in 1m54s), at 6 workers. No benchmark was running, so its
containers weren't cpuset-pinned. Output is in `parity/out/rust-full-lt/`.
That pass is 2,174 cells: 1,092 laptop and 1,068 tablet page cells, plus the 14 fragment cells,
which don't depend on the viewport and were already covered with desktop. (The 1,106 and 1,082 in
the earlier count each included those 14.)

Results by run. P = pass, F = fail, Fl = flaky (passed on a pixel-only retry), E = error:

| Run | Build | Cells | P | F | Fl | E | chromium | firefox | webkit |
|---|---|---|---|---|---|---|---|---|---|
| default, desktop + phone | beb4114 | 2,222 | 2,212 | 10 | 0 | 0 | 750/750 | 734/736 | 728/736 |
| default, breakpoint sweep | f2a1a8f | 154 | 154 | 0 | 0 | 0 | 66/66 | 44/44 | 44/44 |
| default, laptop (stopped at 275 of 1,106) | f2a1a8f | 275 | 275 | 0 | 1 | 0 | ~94 | ~91 | ~91 |
| **default, laptop + tablet, complete** | 3881a02 | 2,174 | 2,174 | 0 | 0 | 0 | 734/734 | 720/720 | 720/720 |
| first_run | beb4114 | 96 | 96 | 0 | 0 | 0 | 32/32 | 32/32 | 32/32 |
| restricted | beb4114 | 48 | 48 | 0 | 0 | 0 | 16/16 | 16/16 | 16/16 |
| crowd | beb4114 | 145 | 145 | 0 | 0 | 0 | 49/49 | 48/48 | 48/48 |
| custom_styles | beb4114 | 193 | 193 | 0 | 0 | 0 | 65/65 | 64/64 | 64/64 |
| rerun: both `auth/incompatible_browser` states, full matrix | f2a1a8f | 48 | 48 | 0 | 0 | 0 | 16/16 | 16/16 | 16/16 |

- The flaky laptop cell in the stopped run is `search/submitted @ webkit-laptop-light`: pixels
  only, and it matched on a retry. The complete laptop + tablet pass had no flaky cells. Its
  chromium count includes the 14 fragment cells.
- The laptop run was stopped mid-run, so it wrote no report. Its counts come from its log, which
  prints every failing and flaky cell, and its per-engine split from the capture files.

**Genuine Rust-side differences: 2, both fixed.** Both only show on engines the lean matrix doesn't
run for these states. The complete laptop + tablet pass found no others.

1. **The blocked-browser page answered 406 for non-HTML formats** (c2b253c). States:
   `auth/incompatible_browser` (webkit, 4 cells) and `auth/incompatible_browser/apple_messages`
   (webkit, 4 cells). Failing layer: network.
   - WebKit fetches `/webmanifest.json` with the blocked user agent.
   - `allow_browser`'s block is an explicit `render template:`, so Rails answers 200 `text/html`
     with that page for any format: `.json`, `.js`, `.xml`, `Accept: application/json`.
   - The port ran the implicit render's template lookup and answered `406 application/json`.
2. **A Live controller's rendered page got an ETag** (f2a1a8f). State:
   `auth/incompatible_browser/apple_messages @ firefox-phone-{light,dark}`. Failing layer: network.
   - Firefox on phones fetches `/account/logo` with the blocked user agent.
   - `Accounts::LogosController` includes `ActiveStorage::Streaming`, so its body is a
     `Live::Buffer`. `Rack::ETag` can't digest that, and Rails sends `Cache-Control: no-cache` with
     no ETag.
   - The port digested the body.
   - The fix is in `campfire_kit` (`rack_etag` skips Live responses). Every other 200 from a Live
     controller already carried `stale?` validators, so nothing else changes.

Both fixes have tests that fail without them, and the campfire and kit test suites pass. Checked by
hand, the fixed build answers every probed case with the same status and headers as Rails. The
full-matrix rerun of both states passes 48 of 48.

**Lean gate on the fixed build** (candidate `head-f2a1a8f`, which also includes the fragment-store
bound 1d6ac20):

| Seed | Cells | P | F | Fl |
|---|---|---|---|---|
| default | 888 | 887 | 1 | 0 |
| first_run | 16 | 16 | 0 | 0 |
| custom_styles | 33 | 33 | 0 | 0 |
| crowd | 25 | 25 | 0 | 0 |
| restricted | 8 | 8 | 0 | 0 |
| **all** | **970** | **969** | **1** | **0** |

The one lean failure was the sidebar-toggle arc, `auth/join/completed @ chromium-phone-light`.
All five text layers were identical, and the pixels differed only along the top of the phone
sidebar toggle's arc (an 83×18 box).

**The arc flake is root-caused and fixed in the harness (2a6df22).** It wasn't the worker or the
browser process. Within one browser process, captures alternated between the two variants. The
cause was stale raster in Chromium's layer tiles, and a race in the page decided which variant you
got:

- After the redirect, the room's sidebar turbo-frame loads once or twice.
  `rooms_list_controller.js` reloads the frame when `UnreadRoomsChannel` confirms, and whether that
  becomes a second load depends on when the confirmation lands relative to the first one.
- The DOM ends up identical either way. The network layer already collapses the repeated request.
- The pixels followed the number of loads in 20 of 20 probe captures. With two loads the toggle
  is rastered again. With one, it keeps tiles rastered earlier, a gray level off.
- Which way the race goes depends on how fast each server answers, so within one run each side
  tended to keep its variant, even across retries on fresh servers.

The fix: before the screenshot, Chromium captures promote the root to its own layer and back,
giving each change 150ms to be drawn (`rasterAfresh` in `parity/capture/capture.ts`). That throws
every tile away, so the screenshot is always a fresh raster of the final page. The page and its
outputs are untouched, and nothing is masked or allowlisted.

Checking the fix:

- 42 of 42 probe captures (light and dark, one load or two) were identical.
- Without the 150ms waits, the revert sometimes landed in the same frame, and `errors/500` came out
  two ways. That was caught in a lean run and fixed. With the waits, `errors/500` was 20 of 20
  identical.
- Rust vs Rails, `auth/join/completed` at phone, 6 runs: all 12 cells identical on both sides.

**Lean gate on 2a6df22** (candidate `head-3881a02`; the app code is unchanged since f2a1a8f).
Output is in `parity/out/rust-lean-afresh/`.

| Seed | Cells | P | F | Fl |
|---|---|---|---|---|
| default | 888 | 888 | 0 | 0 |
| first_run | 16 | 16 | 0 | 0 |
| custom_styles | 33 | 33 | 0 | 0 |
| crowd | 25 | 25 | 0 | 0 |
| restricted | 8 | 8 | 0 | 0 |
| **all** | **970** | **970** | **0** | **0** |

All instances are torn down, and no parity containers are left running.
