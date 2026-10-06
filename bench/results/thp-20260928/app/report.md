```
date: 2026-09-28T13:33:01+02:00
host: 7.2.5-4-omarchy, AMD RYZEN AI MAX+ 395 w/ Radeon 8060S, 32 threads, 30GB
server cpus: 8-11 (nproc 4); loadgen cpus: 12-15; network: host
env: WEB_CONCURRENCY=3 JOB_CONCURRENCY=3 RAILS_MAX_THREADS=5 
rust extra env: 
rust image: campfire-rust:app sha256:7042897243ae9e9f6fc47c3e18d0ae00165de1ad8a3488369bd79d01a8d46ac7 2026-09-28T11:10:14.617808723+02:00
rust HEAD: 50687be (dirty: 0 files)
```

Reps: rust 1. Cells: median [min–max].

### Startup and memory

| Metric | Rust | Rust adv. |
|---|---|---|
| cold start: docker run → /up 200 (ms) | 139 | – |
| idle memory.current (MB) | 47.0 | – |
| idle anon (MB) | 45.0 | – |
| peak memory.current under load (MB) | 316 | – |
| peak anon under load (MB) | 174 | – |

### HTTP (signed in as david; keep-alive; c = concurrent connections)

| Metric | Rust | Rust adv. |
|---|---|---|
| room_show c=1 req/s | 5,034 | – |
| room_show c=1 p50 ms | 0.20 | – |
| room_show c=1 p99 ms | 0.26 | – |
| room_show c=16 req/s | 19,900 | – |
| room_show c=16 p50 ms | 0.78 | – |
| room_show c=16 p99 ms | 1.52 | – |
| room_show c=64 req/s | 19,398 | – |
| room_show c=64 p50 ms | 3.28 | – |
| room_show c=64 p99 ms | 5.43 | – |
| messages_page c=1 req/s | 5,202 | – |
| messages_page c=1 p50 ms | 0.19 | – |
| messages_page c=1 p99 ms | 0.27 | – |
| messages_page c=16 req/s | 22,117 | – |
| messages_page c=16 p50 ms | 0.69 | – |
| messages_page c=16 p99 ms | 1.50 | – |
| messages_page c=64 req/s | 23,189 | – |
| messages_page c=64 p50 ms | 2.71 | – |
| messages_page c=64 p99 ms | 4.55 | – |
| sidebar c=1 req/s | 3,223 | – |
| sidebar c=1 p50 ms | 0.30 | – |
| sidebar c=1 p99 ms | 0.42 | – |
| sidebar c=16 req/s | 12,276 | – |
| sidebar c=16 p50 ms | 1.27 | – |
| sidebar c=16 p99 ms | 2.37 | – |
| sidebar c=64 req/s | 12,579 | – |
| sidebar c=64 p50 ms | 5.07 | – |
| sidebar c=64 p99 ms | 8.54 | – |
| search c=1 req/s | 6,321 | – |
| search c=1 p50 ms | 0.15 | – |
| search c=1 p99 ms | 0.23 | – |
| search c=16 req/s | 22,308 | – |
| search c=16 p50 ms | 0.67 | – |
| search c=16 p99 ms | 1.56 | – |
| search c=64 req/s | 22,692 | – |
| search c=64 p50 ms | 2.76 | – |
| search c=64 p99 ms | 4.66 | – |
| avatar c=1 req/s | 68,114 | – |
| avatar c=1 p50 ms | 0.01 | – |
| avatar c=1 p99 ms | 0.02 | – |
| avatar c=16 req/s | 418,285 | – |
| avatar c=16 p50 ms | 0.04 | – |
| avatar c=16 p99 ms | 0.10 | – |
| avatar c=64 req/s | 380,817 | – |
| avatar c=64 p50 ms | 0.15 | – |
| avatar c=64 p99 ms | 0.51 | – |
| static_css c=1 req/s | 70,980 | – |
| static_css c=1 p50 ms | 0.01 | – |
| static_css c=1 p99 ms | 0.02 | – |
| static_css c=16 req/s | 435,629 | – |
| static_css c=16 p50 ms | 0.03 | – |
| static_css c=16 p99 ms | 0.06 | – |
| static_css c=64 req/s | 459,547 | – |
| static_css c=64 p50 ms | 0.13 | – |
| static_css c=64 p99 ms | 0.29 | – |
| up c=1 req/s | 28,041 | – |
| up c=1 p50 ms | 0.03 | – |
| up c=1 p99 ms | 0.05 | – |
| up c=16 req/s | 134,124 | – |
| up c=16 p50 ms | 0.12 | – |
| up c=16 p99 ms | 0.20 | – |
| up c=64 req/s | 133,564 | – |
| up c=64 p50 ms | 0.47 | – |
| up c=64 p99 ms | 0.97 | – |
| post_message c=1 req/s | 2,218 | – |
| post_message c=1 p50 ms | 0.42 | – |
| post_message c=1 p99 ms | 1.66 | – |
| post_message c=16 req/s | 5,475 | – |
| post_message c=16 p50 ms | 2.73 | – |
| post_message c=16 p99 ms | 6.78 | – |
| post_message c=64 req/s | 5,518 | – |
| post_message c=64 p50 ms | 11.4 | – |
| post_message c=64 p99 ms | 16.7 | – |

### HTTP errors / non-2xx-3xx (first rep, per app)

| Metric | Rust | Rust adv. |
|---|---|---|
- rust: none

### Action Cable fan-out (one room; chatter.js subscriptions per client)

| Metric | Rust | Rust adv. |
|---|---|---|

### Upload + thumbnail (black_hole.jpg, 505 KB)

| Metric | Rust | Rust adv. |
|---|---|---|
| POST with attachment (ms) | – | – |
| then GET thumb → 200 (ms) | – | – |
| POST → thumbnail served (ms) | – | – |

### Memory during cable fan-out, by process (MB, peak within the phase)

App process: Rails' Puma master and workers (Action Cable runs in them), or Rust's one campfire
process (its front server included). Pss counts pages shared between forked workers once;
RssAnon counts them in every process.

| Metric | Rust | Rust adv. |
|---|---|---|
