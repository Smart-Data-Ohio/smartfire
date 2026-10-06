# Final benchmark: Rails vs Rust (2026-09-27)

This compares the production images of the Rails reference and the Rust port. Both ran on the same
seed, the same CPUs and the same network path, at the same process model. The Rust image was built
from a clean `git archive` of HEAD `5181aa4`, the commit that passed the final lean parity gate
(Rust vs Rails 970/970, 1 flaky pixel cell).

The Rust port wins every measurement except one. After the HTTP suite, the Rust process holds
more memory than Rails does, because of its fragment cache (see Surprises).

- **Pages:** 9–19× Rails' throughput at c=16, and 6–13× lower median latency at c=1.
- **Writes:** message POSTs run at 19× the throughput, with a 12× lower p99.
- **Action Cable:** 22–26× the delivery throughput and 7–9× lower delivery latency.
- **Startup:** cold start is 10.6× faster.
- **Memory:** at idle Rust uses 6.3× less. With 1,000 cable clients on a fresh process, the app
  process uses 2.6–3.0× less, and the container 3.7× less.

## Headline (main run: 5 reps per app, interleaved)

Each cell is the median across reps, with [min–max] across reps. "Rust adv." is how many times
better Rust is: higher throughput, or lower latency, time or memory. "Rust, no gzip" is the same
Rust image serving `Accept-Encoding: identity`, which shows the app's own cost apart from
compression. Rails compresses in every column; there is no Rails no-gzip run.

| Measurement | Rails | Rust | Rust adv. | Rust, no gzip |
|---|---|---|---|---|
| Cold start, `docker run` → `/up` 200 (ms) | 2,457 [2,432–2,473] | 232 [222–234] | **10.6×** |  |
| Idle memory, container `memory.current` (MB) | 309 [305–314] | 49.0 [47.0–50.0] | **6.3×** |  |
| Room page, c=16 (req/s) | 234 [232–248] | 2,577 [2,564–2,598] | **11.0×** | 8,973 [8,927–9,020] |
| Room page, c=1 p50 (ms) | 10.2 [9.8–10.5] | 1.55 [1.53–1.58] | **6.6×** | 0.47 [0.45–0.47] |
| Room page, c=64 p99 (ms) | 391 [348–420] | 44.1 [42.9–44.8] | **8.9×** | 12.5 [12.3–12.7] |
| Messages page (`?before=`), c=16 (req/s) | 444 [443–456] | 3,939 [3,921–3,969] | **8.9×** | 16,500 [16,358–16,690] |
| Messages page (`?before=`), c=1 p50 (ms) | 5.51 [5.42–5.58] | 0.98 [0.97–0.99] | **5.6×** | 0.24 [0.22–0.25] |
| Messages page (`?before=`), c=64 p99 (ms) | 226 [200–244] | 27.6 [26.7–28.1] | **8.2×** | 6.49 [6.41–6.67] |
| Sidebar, c=16 (req/s) | 581 [569–603] | 11,066 [11,008–11,292] | **19.0×** | 19,881 [19,609–20,024] |
| Sidebar, c=1 p50 (ms) | 4.36 [4.29–4.39] | 0.33 [0.32–0.36] | **13.2×** | 0.17 [0.17–0.18] |
| Sidebar, c=64 p99 (ms) | 183 [150–190] | 9.48 [9.32–9.63] | **19.3×** | 5.03 [4.97–5.35] |
| Search, c=16 (req/s) | 420 [414–427] | 5,933 [5,865–5,998] | **14.1×** | 15,254 [14,894–15,339] |
| Search, c=1 p50 (ms) | 5.53 [5.49–5.64] | 0.63 [0.63–0.66] | **8.8×** | 0.24 [0.24–0.25] |
| Search, c=64 p99 (ms) | 228 [177–243] | 17.4 [17.1–17.5] | **13.2×** | 6.32 [6.19–6.79] |
| Message POST, c=16 (req/s) | 273 [262–282] | 5,272 [5,260–5,304] | **19.3×** | 5,515 [5,484–5,620] |
| Message POST, c=1 p50 (ms) | 6.13 [6.04–6.16] | 0.41 [0.41–0.42] | **14.9×** | 0.35 [0.34–0.35] |
| Message POST, c=64 p99 (ms) | 370 [332–456] | 24.8 [24.4–90.2] | **14.9×** | 24.5 [24.2–24.9] |
| Avatar, c=16 (req/s) | 98,244 [97,362–99,830] | 413,217 [393,490–426,580] | **4.2×** | 416,941 [405,720–436,162] |
| Static CSS, c=16 (req/s) | 135,193 [134,176–136,287] | 426,374 [417,727–432,479] | **3.2×** | 424,559 [415,862–439,195] |
| `/up`, c=16 (req/s) | 4,289 [4,247–4,329] | 107,715 [106,810–110,551] | **25.1×** | 163,589 [160,950–165,620] |
| Message POST, c=1 p99 (ms) | 18.4 [18.1–23.4] | 1.51 [1.39–1.56] | **12.1×** | 1.52 [1.43–1.60] |
| Cable 100 clients: deliveries/s | 9,286 [8,644–9,648] | 244,719 [242,799–247,746] | **26.4×** |  |
| Cable 100 clients: paced post → all clients p50 (ms) | 17.3 [16.9–18.5] | 2.46 [2.44–2.51] | **7.0×** |  |
| Cable 500 clients: deliveries/s | 13,523 [13,106–13,727] | 301,899 [292,379–313,220] | **22.3×** |  |
| Cable 500 clients: paced post → all clients p50 (ms) | 43.8 [43.1–44.4] | 5.11 [4.77–5.17] | **8.6×** |  |
| Cable 1000 clients: deliveries/s | 13,646 [12,712–14,955] | 304,197 [297,988–310,255] | **22.3×** |  |
| Cable 1000 clients: paced post → all clients p50 (ms) | 78.8 [75.7–85.1] | 8.97 [8.43–9.02] | **8.8×** |  |
| Cable 1000 clients: paced post → all clients p99 (ms) | 159 [105–162] | 12.3 [11.4–14.6] | **12.9×** |  |
| Cable 1000 clients, subscribed and idle: app process Pss (MB) † | 459 [455–484] | 154 [151–154] | **3.0×** |  |
| Cable 1000 clients, saturated: app process Pss (MB) † | 836 [751–879] | 319 [309–331] | **2.6×** |  |
| Cable 1000 clients, saturated: app process RssAnon (MB) † | 985 [897–1,030] | 287 [277–299] | **3.4×** |  |
| Cable 1000 clients, saturated: whole container Pss (MB) † | 1,188 [1,095–1,255] | 319 [309–331] | **3.7×** |  |
| Upload 505 KB JPEG → thumbnail served (ms) | 55.4 [50.1–55.5] | 27.8 [27.6–28.7] | **2.0×** |  |

† From a fresh process running only the 1,000-client cable suite (`cable-only/`, 5 reps per app,
interleaved). "App process" is Rails' Puma master and workers, which also run Action Cable, or Rust's
single `campfire` process, which includes its front server. Pss counts pages that forked Puma workers
share only once, which flatters Rails; RssAnon counts them in every process. "Whole container" adds
Rails' Thruster, Redis and Resque, which Rust has no counterpart for. Per-phase peaks for 100, 500 and
1,000 clients after the full HTTP suite are in `main/report.md`; see the second surprise below
before reading them.

## What the numbers say

- **gzip now dominates Rust's page cost.** Without it, the room page serves 3.5× more requests,
  the messages page 4.2×, search 2.6× and the sidebar 1.8×. That puts compression at 60–76% of the
  CPU of the big pages. Rust's gzip is level 6 on zlib-rs, which parity with Rails' `Rack::Deflater`
  requires. The app work alone takes 0.47 ms at the median for a 464 KB room page, against Rails'
  10.2 ms including gzip.
- **POSTs are bound by SQLite, not compression.** Turning off gzip adds only 5–16%. The c=1 p99 is
  1.5 ms (it was 13.6 ms in the preliminary run), now that WAL checkpoints run off the writer
  thread.
- **Cheap routes:** `/up` runs at 25× Rails, and avatars and static CSS at 3–5×. Rails serves avatars
  and CSS from Thruster's cache, so this compares two front servers more than two apps. Both ratios
  are above 1 for the first time; in the preliminary run, the Rust app still sat behind Thruster.
- **Cable at 1,000 clients:** Rust delivers 304k frames/s against Rails' 13.6k. Rust is flat from
  500 to 1,000 clients at about 300k deliveries/s, which is the load generator's ceiling (about
  2.5 of its 4 cores), not the app's.
- **Upload to thumbnail** is only 2.0×. Most of the time is libvips and ffmpeg work that both apps
  do in C.

## Surprises

1. **A parity bug the gate couldn't see, found by the no-gzip column** (fixed in fa1deb9 before the
   gate and before these runs). In the first smoke run, Rust served avatars without gzip at a
   tenth of the gzip rate. The front server's response cache stores a body when it sees the end of
   the stream. But hyper stops polling a streamed body once its `Content-Length` bytes are written,
   so `send_file` responses sent without gzip were never cached. They answered `X-Cache: miss`
   where Thruster answers `hit`, and every request went back to the app. Browsers always send
   gzip, so no parity cell could reach this path, and the header-shape sweep ignores `x-cache`. A
   regression test now covers it (`crates/kit/tests/front.rs`).
2. **After the full HTTP suite, the Rust process holds about 1.37 GB** (Rails' Puma holds about
   0.7 GB). So the per-phase cable memory in `main/report.md` shows Rust at 0.4–1.0× of Rails.
   This is not a leak and not the allocator:
   - With jemalloc's `background_thread:true`, it's unchanged at 1.37 GB (`jemalloc-bg/`, 3 reps,
     same throughput).
   - It is the in-process fragment store, which stands in for Rails' `redis_cache_store`. Its limit
     is 50,000 entries (`campfire_views::fragment_cache::DEFAULT_CAPACITY`), with no bound in
     bytes.
   - The closed-loop POST suite creates about 100,000 messages on Rust against about 5,800 on
     Rails, because Rust is 19× faster. Every one of them gets rendered and cached.
   - Measured directly on a fresh process: 60,000 POSTs take RssAnon from 47 MB to 1.29 GB, while
     60,000 room-page GETs add 36 MB. Growth stops at about 1.30 GB, from 60,000 up to 160,000
     posts, so each message costs about 25 KB of cache.
   - Rails keeps the same kind of data in Redis (26 MB for its 5,800 messages). The reference's
     `config/redis.conf` sets no `maxmemory`, so Redis would keep growing where this store stops.

   Bounding the store in bytes instead of entries would keep this to whatever budget is chosen.
   The same effect shows in the "peak memory.current under load" row of `main/report.md` (Rust
   1,867 MB vs Rails 1,468 MB).
3. **Rails is about 1.5× faster than in the preliminary report on the same configuration.** For
   example, room page c=16 went from 146 to 235 req/s, and cable at 1,000 clients from 9.4k to 14.2k
   deliveries/s. The preliminary run shared the host with parity captures (load average 8–40).
   Here every app's run started at a 1-minute load average below 1.5. The ratios still grew mostly
   through Rust's own work since then. In the same configuration, the room page went from 4.4× to
   10.9× Rails, and cable deliveries at 1,000 clients from 4.8× to 19.5×.

## The preliminary configuration, rerun

`prelim-config/` (3 reps per app, interleaved) uses the preliminary run's setup. The container
sits on Docker's bridge with a published port, so traffic goes through docker-proxy. There is no DNS
resolver, and Rust gets `RAILS_MAX_THREADS=128`. The images are today's (`5181aa4`). "Prelim" is
`bench/results/prelim-20260926` (image from 2fee0df plus a dirty tree, on a busy host).

| Measurement | Rails, prelim | Rust, prelim | adv. | Rails, now | Rust, now | adv. |
|---|---|---|---|---|---|---|
| Cold start (ms) | 4,010 | 399 | 10.1× | 2,469 | 303 | **8.1×** |
| Idle memory.current (MB) | 319 | 31.0 | 10.3× | 319 | 60.0 | **5.3×** |
| Peak memory.current over the rep (MB) | 1,456 | 1,054 | 1.4× | 1,468 | 1,858 | **0.8×** |
| Room page c=16 (req/s) | 146 | 638 | 4.4× | 235 | 2,561 | **10.9×** |
| Messages page c=16 (req/s) | 312 | 783 | 2.5× | 443 | 3,926 | **8.9×** |
| Sidebar c=16 (req/s) | 345 | 3,086 | 8.9× | 580 | 10,821 | **18.7×** |
| Search c=16 (req/s) | 280 | 1,545 | 5.5× | 427 | 5,831 | **13.7×** |
| Avatar c=16 (req/s) | 57,098 | 55,276 | 1.0× | 86,267 | 189,169 | **2.2×** |
| Static CSS c=16 (req/s) | 88,800 | 86,612 | 1.0× | 123,268 | 177,806 | **1.4×** |
| `/up` c=16 (req/s) | 2,800 | 16,813 | 6.0× | 4,223 | 98,854 | **23.4×** |
| Message POST c=16 (req/s) | 166 | 1,453 | 8.8× | 268 | 5,195 | **19.4×** |
| Message POST c=1 p99 (ms) | 29.5 | 13.6 | 2.2× | 22.2 | 1.40 | **15.9×** |
| Cable 1000: deliveries/s | 9,443 | 45,723 | 4.8× | 14,162 | 276,163 | **19.5×** |
| Cable 1000: post → all p50 (ms) | 111 | 31.5 | 3.5× | 77.0 | 9.91 | **7.8×** |
| Upload → thumbnail (ms) | 86.4 | 40.0 | 2.2× | 55.5 | 28.5 | **1.9×** |

docker-proxy costs Rust more than Rails because Rust serves so many more requests per second. For
example, `/up` runs at 99k through the proxy against 108k direct, and avatars at 189k against 413k.
That's why bench/run now uses `--network host`. Rust's idle memory (60 vs 31 MB) and peak memory
went up since the preliminary run: jemalloc's arenas account for the first (786a74d), and the
fragment store above for the second.

## Environment and method

```
date: 2026-09-27T04:46:24+02:00
host: 7.2.5-4-omarchy, AMD Ryzen 9 9955HX 16-Core Processor, 32 threads, 91GB
server cpus: 8-11 (nproc 4); loadgen cpus: 12-15; network: host
env: WEB_CONCURRENCY=3 JOB_CONCURRENCY=3 RAILS_MAX_THREADS=5 
rust extra env: 
reference image: campfire-reference:app sha256:70ab5e2abe726bfdfc177af4fa8465cd90ba3b3e3f532c8633f818cf63545adf 2026-09-26T14:42:50.327050882+02:00
rust image: campfire-rust:head-5181aa4 sha256:033927228539c17717887043791c154d9996f7ca9eb2e3683c91f9925eb200d5 2026-09-27T04:05:37.180679451+02:00
rust-identity image: campfire-rust:head-5181aa4 sha256:033927228539c17717887043791c154d9996f7ca9eb2e3683c91f9925eb200d5 2026-09-27T04:05:37.180679451+02:00
rust HEAD: 5181aa4 (dirty: 0 files)
```

- **Host:** AMD Ryzen 9 9955HX (16 cores, 32 threads), 91 GB, kernel 7.2.5, btrfs.
- **Isolation:** no other agents or containers ran. Before each app's run, `bench/run` waited for
  the 1-minute load average to drop below 1.5. Across all 34 runs it was 1.27–1.49 at start, and
  the longest wait was 100 s. Each run's start and end load is in `*/uptime.log` and the logs.
- **CPU pinning:** the server was pinned to CPUs 8–11, the load generator (`bench/loadgen`) to
  CPUs 12–15, and the memory sampler to CPU 0.
- **Network:** `--network host` for the main run, with no docker-proxy. Each app's front server
  (Thruster, or the Rust binary's own) listens on the port.
- **Process model:** the reference's defaults for 4 CPUs (`WEB_CONCURRENCY=3`,
  `JOB_CONCURRENCY=3`, `RAILS_MAX_THREADS=5`). Rust uses the same variables, in one process.
- **Images:** the unmodified production images, `campfire-reference:app` and
  `campfire-rust:head-5181aa4`. The Rust image was built by `/Dockerfile` from a clean
  `git archive` of 5181aa4 plus the pinned `reference/` submodule. The cargo stage was built
  uncached, so the cached target directory couldn't stand in for the archived sources.
- **Order:** 5 reps, with apps interleaved within each rep (reference, rust, rust-identity) and the
  order reversed on even reps. Every run gets a fresh copy of the default parity seed and a fresh
  container.
- **Per run:**
  - cold start, then idle memory after 10 s;
  - the HTTP suite: 8 routes × c = 1, 16 and 64 × 8 s, signed in as David, keep-alive;
  - the cable fan-out: 100, 500 and 1,000 clients with a paced phase and a 15 s saturated phase;
  - five 505 KB uploads.
- **Memory sampling:** container memory every 200 ms, and each process's Pss and RssAnon by role
  every 250 ms during the fan-out (`bench/lib/procmem.py`).
- **Errors:** no errors and no status ≥ 400 in any run.

## Files

| Directory | What | Reps |
|---|---|---|
| `main/` | Every measurement, plus Rust with `Accept-Encoding: identity` (full tables in `main/report.md`) | 5 per app |
| `cable-only/` | A fresh process, 1,000-client fan-out only (`SUITES=cable`) | 5 per app |
| `jemalloc-bg/` | Rust, full suite, `_RJEM_MALLOC_CONF=background_thread:true` | 3 |
| `prelim-config/` | The preliminary run's setup with today's images | 3 per app |

Reproduce:

```sh
export RUST_IMAGE=campfire-rust:head-5181aa4 REFERENCE_IMAGE=campfire-reference:app
bench/run --apps reference,rust,rust-identity --reps 5 --out bench/results/<dir>/main
SUITES=cable CABLE_CLIENTS=1000 bench/run --apps reference,rust --reps 5 --out bench/results/<dir>/cable-only
RUST_EXTRA_ENV=_RJEM_MALLOC_CONF=background_thread:true bench/run --apps rust --reps 3 --out bench/results/<dir>/jemalloc-bg
NETWORK=bridge RUST_EXTRA_ENV=RAILS_MAX_THREADS=128 bench/run --apps reference,rust --reps 3 --out bench/results/<dir>/prelim-config
```
