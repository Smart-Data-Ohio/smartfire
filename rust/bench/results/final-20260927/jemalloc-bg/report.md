```
date: 2026-09-27T07:04:13+02:00
host: 7.2.5-4-omarchy, AMD Ryzen 9 9955HX 16-Core Processor, 32 threads, 91GB
server cpus: 8-11 (nproc 4); loadgen cpus: 12-15; network: host
env: WEB_CONCURRENCY=3 JOB_CONCURRENCY=3 RAILS_MAX_THREADS=5 
rust extra env: _RJEM_MALLOC_CONF=background_thread:true
rust image: campfire-rust:head-5181aa4 sha256:033927228539c17717887043791c154d9996f7ca9eb2e3683c91f9925eb200d5 2026-09-27T04:05:37.180679451+02:00
rust HEAD: 5181aa4 (dirty: 0 files)
```

Reps: rust 3. Cells: median [min–max].

### Startup and memory

| Metric | Rust | Rust adv. |
|---|---|---|
| cold start: docker run → /up 200 (ms) | 231 [230–233] | – |
| idle memory.current (MB) | 50.0 [47.0–52.0] | – |
| idle anon (MB) | 46.0 [44.0–48.0] | – |
| peak memory.current under load (MB) | 1,829 [1,783–1,839] | – |
| peak anon under load (MB) | 1,527 [1,521–1,528] | – |

### HTTP (signed in as david; keep-alive; c = concurrent connections)

| Metric | Rust | Rust adv. |
|---|---|---|
| room_show c=1 req/s | 640 [618–653] | – |
| room_show c=1 p50 ms | 1.52 [1.51–1.57] | – |
| room_show c=1 p99 ms | 2.25 [2.16–2.31] | – |
| room_show c=16 req/s | 2,586 [2,580–2,594] | – |
| room_show c=16 p50 ms | 6.24 [6.22–6.24] | – |
| room_show c=16 p99 ms | 10.4 [10.4–10.4] | – |
| room_show c=64 req/s | 2,570 [2,560–2,570] | – |
| room_show c=64 p50 ms | 24.6 [24.5–24.7] | – |
| room_show c=64 p99 ms | 44.8 [43.2–45.0] | – |
| messages_page c=1 req/s | 1,021 [1,021–1,022] | – |
| messages_page c=1 p50 ms | 0.97 [0.96–0.97] | – |
| messages_page c=1 p99 ms | 1.11 [1.07–1.35] | – |
| messages_page c=16 req/s | 3,945 [3,937–3,967] | – |
| messages_page c=16 p50 ms | 4.05 [4.02–4.06] | – |
| messages_page c=16 p99 ms | 6.74 [6.73–6.74] | – |
| messages_page c=64 req/s | 3,935 [3,912–3,941] | – |
| messages_page c=64 p50 ms | 16.1 [16.1–16.2] | – |
| messages_page c=64 p99 ms | 27.4 [27.2–27.7] | – |
| sidebar c=1 req/s | 3,025 [2,964–3,031] | – |
| sidebar c=1 p50 ms | 0.32 [0.32–0.33] | – |
| sidebar c=1 p99 ms | 0.43 [0.41–0.45] | – |
| sidebar c=16 req/s | 11,080 [11,073–11,146] | – |
| sidebar c=16 p50 ms | 1.41 [1.41–1.41] | – |
| sidebar c=16 p99 ms | 2.58 [2.56–2.59] | – |
| sidebar c=64 req/s | 11,269 [11,224–11,293] | – |
| sidebar c=64 p50 ms | 5.63 [5.62–5.66] | – |
| sidebar c=64 p99 ms | 9.45 [9.35–9.63] | – |
| search c=1 req/s | 1,556 [1,551–1,564] | – |
| search c=1 p50 ms | 0.63 [0.63–0.63] | – |
| search c=1 p99 ms | 0.78 [0.74–0.81] | – |
| search c=16 req/s | 5,900 [5,895–5,957] | – |
| search c=16 p50 ms | 2.68 [2.66–2.68] | – |
| search c=16 p99 ms | 4.54 [4.50–4.56] | – |
| search c=64 req/s | 5,994 [5,910–6,000] | – |
| search c=64 p50 ms | 10.6 [10.6–10.7] | – |
| search c=64 p99 ms | 17.2 [17.2–17.6] | – |
| avatar c=1 req/s | 71,420 [70,436–71,903] | – |
| avatar c=1 p50 ms | 0.01 [0.01–0.01] | – |
| avatar c=1 p99 ms | 0.02 [0.02–0.02] | – |
| avatar c=16 req/s | 423,607 [412,784–429,539] | – |
| avatar c=16 p50 ms | 0.04 [0.04–0.04] | – |
| avatar c=16 p99 ms | 0.09 [0.09–0.09] | – |
| avatar c=64 req/s | 429,392 [421,481–440,061] | – |
| avatar c=64 p50 ms | 0.14 [0.14–0.14] | – |
| avatar c=64 p99 ms | 0.39 [0.38–0.39] | – |
| static_css c=1 req/s | 76,967 [76,152–77,345] | – |
| static_css c=1 p50 ms | 0.01 [0.01–0.01] | – |
| static_css c=1 p99 ms | 0.02 [0.02–0.02] | – |
| static_css c=16 req/s | 439,487 [435,264–452,479] | – |
| static_css c=16 p50 ms | 0.03 [0.03–0.03] | – |
| static_css c=16 p99 ms | 0.08 [0.08–0.08] | – |
| static_css c=64 req/s | 468,585 [458,555–475,590] | – |
| static_css c=64 p50 ms | 0.13 [0.13–0.13] | – |
| static_css c=64 p99 ms | 0.32 [0.32–0.32] | – |
| up c=1 req/s | 24,052 [23,966–24,244] | – |
| up c=1 p50 ms | 0.04 [0.04–0.04] | – |
| up c=1 p99 ms | 0.05 [0.05–0.05] | – |
| up c=16 req/s | 108,376 [108,077–109,674] | – |
| up c=16 p50 ms | 0.15 [0.15–0.15] | – |
| up c=16 p99 ms | 0.25 [0.25–0.26] | – |
| up c=64 req/s | 109,619 [105,613–109,770] | – |
| up c=64 p50 ms | 0.57 [0.57–0.59] | – |
| up c=64 p99 ms | 1.19 [1.18–1.23] | – |
| post_message c=1 req/s | 2,212 [2,192–2,216] | – |
| post_message c=1 p50 ms | 0.41 [0.41–0.42] | – |
| post_message c=1 p99 ms | 1.47 [1.45–1.50] | – |
| post_message c=16 req/s | 5,336 [5,252–5,352] | – |
| post_message c=16 p50 ms | 2.60 [2.58–2.63] | – |
| post_message c=16 p99 ms | 14.7 [14.5–14.7] | – |
| post_message c=64 req/s | 5,413 [5,302–5,452] | – |
| post_message c=64 p50 ms | 10.9 [10.8–11.1] | – |
| post_message c=64 p99 ms | 25.1 [24.7–25.1] | – |

### HTTP errors / non-2xx-3xx (first rep, per app)

| Metric | Rust | Rust adv. |
|---|---|---|
- rust: none

### Action Cable fan-out (one room; chatter.js subscriptions per client)

| Metric | Rust | Rust adv. |
|---|---|---|
| 100 clients: subscribed | 100 [100–100] | – |
| 100 clients: connect+subscribe all (s) | 0.06 [0.06–0.06] | – |
| 100 clients: paced post→one client p50 ms | 2.24 [2.18–2.29] | – |
| 100 clients: paced post→all clients p50 ms | 2.46 [2.39–2.51] | – |
| 100 clients: paced post→all clients p99 ms | 3.71 [2.90–5.58] | – |
| 100 clients: max sustained msgs/s (delivered to all) | 2,444 [2,423–2,448] | – |
| 100 clients: deliveries/s (client×message) | 244,377 [242,306–244,844] | – |
| 100 clients: saturated post→all p50 ms | 1.32 [1.32–1.33] | – |
| 100 clients: saturated POST p50 ms | 1.45 [1.45–1.46] | – |
| 500 clients: subscribed | 500 [500–500] | – |
| 500 clients: connect+subscribe all (s) | 0.13 [0.13–0.14] | – |
| 500 clients: paced post→one client p50 ms | 3.20 [3.18–3.28] | – |
| 500 clients: paced post→all clients p50 ms | 5.11 [4.95–5.12] | – |
| 500 clients: paced post→all clients p99 ms | 7.32 [6.64–8.06] | – |
| 500 clients: max sustained msgs/s (delivered to all) | 601 [600–610] | – |
| 500 clients: deliveries/s (client×message) | 300,400 [299,787–304,889] | – |
| 500 clients: saturated post→all p50 ms | 8.92 [8.89–8.94] | – |
| 500 clients: saturated POST p50 ms | 6.29 [6.26–6.30] | – |
| 1000 clients: subscribed | 1,000 [1,000–1,000] | – |
| 1000 clients: connect+subscribe all (s) | 0.14 [0.14–0.19] | – |
| 1000 clients: paced post→one client p50 ms | 5.05 [5.03–5.20] | – |
| 1000 clients: paced post→all clients p50 ms | 9.18 [8.97–9.41] | – |
| 1000 clients: paced post→all clients p99 ms | 13.8 [13.5–15.8] | – |
| 1000 clients: max sustained msgs/s (delivered to all) | 299 [289–308] | – |
| 1000 clients: deliveries/s (client×message) | 298,683 [288,587–308,265] | – |
| 1000 clients: saturated post→all p50 ms | 17.6 [16.9–17.8] | – |
| 1000 clients: saturated POST p50 ms | 13.3 [12.8–13.5] | – |

### Upload + thumbnail (black_hole.jpg, 505 KB)

| Metric | Rust | Rust adv. |
|---|---|---|
| POST with attachment (ms) | 27.5 [27.3–28.0] | – |
| then GET thumb → 200 (ms) | 0.20 [0.20–0.30] | – |
| POST → thumbnail served (ms) | 27.7 [27.5–28.2] | – |

### Memory during cable fan-out, by process (MB, peak within the phase)

App process: Rails' Puma master and workers (Action Cable runs in them), or Rust's one campfire
process (its front server included). Pss counts pages shared between forked workers once;
RssAnon counts them in every process.

| Metric | Rust | Rust adv. |
|---|---|---|
| 100 clients, all subscribed, idle: app process Pss | 1,408 [1,408–1,421] | – |
| 100 clients, all subscribed, idle: app process RssAnon | 1,373 [1,373–1,387] | – |
| 100 clients, all subscribed, idle: app + Redis + Thruster Pss | 1,408 [1,408–1,421] | – |
| 100 clients, all subscribed, idle: whole container Pss | 1,408 [1,408–1,421] | – |
| 100 clients, saturated fan-out: app process Pss | 1,381 [1,377–1,383] | – |
| 100 clients, saturated fan-out: app process RssAnon | 1,346 [1,342–1,348] | – |
| 100 clients, saturated fan-out: app + Redis + Thruster Pss | 1,381 [1,377–1,383] | – |
| 100 clients, saturated fan-out: whole container Pss | 1,381 [1,377–1,383] | – |
| 500 clients, all subscribed, idle: app process Pss | 1,368 [1,367–1,374] | – |
| 500 clients, all subscribed, idle: app process RssAnon | 1,332 [1,331–1,339] | – |
| 500 clients, all subscribed, idle: app + Redis + Thruster Pss | 1,368 [1,367–1,374] | – |
| 500 clients, all subscribed, idle: whole container Pss | 1,368 [1,367–1,374] | – |
| 500 clients, saturated fan-out: app process Pss | 1,374 [1,374–1,377] | – |
| 500 clients, saturated fan-out: app process RssAnon | 1,338 [1,338–1,342] | – |
| 500 clients, saturated fan-out: app + Redis + Thruster Pss | 1,374 [1,374–1,377] | – |
| 500 clients, saturated fan-out: whole container Pss | 1,374 [1,374–1,377] | – |
| 1000 clients, all subscribed, idle: app process Pss | 1,379 [1,377–1,382] | – |
| 1000 clients, all subscribed, idle: app process RssAnon | 1,343 [1,341–1,346] | – |
| 1000 clients, all subscribed, idle: app + Redis + Thruster Pss | 1,379 [1,377–1,382] | – |
| 1000 clients, all subscribed, idle: whole container Pss | 1,379 [1,377–1,382] | – |
| 1000 clients, saturated fan-out: app process Pss | 1,406 [1,404–1,410] | – |
| 1000 clients, saturated fan-out: app process RssAnon | 1,370 [1,368–1,374] | – |
| 1000 clients, saturated fan-out: app + Redis + Thruster Pss | 1,406 [1,404–1,410] | – |
| 1000 clients, saturated fan-out: whole container Pss | 1,406 [1,404–1,410] | – |
