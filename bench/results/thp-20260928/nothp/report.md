```
date: 2026-09-28T13:36:41+02:00
host: 7.2.5-4-omarchy, AMD RYZEN AI MAX+ 395 w/ Radeon 8060S, 32 threads, 30GB
server cpus: 8-11 (nproc 4); loadgen cpus: 12-15; network: host
env: WEB_CONCURRENCY=3 JOB_CONCURRENCY=3 RAILS_MAX_THREADS=5 
rust extra env: 
rust image: campfire-rust:nothp sha256:89c128841812c9e1c5f1e4f8690dd15f422aa72827e7ff7b2875216518610e52 2026-09-28T13:18:12.610782156+02:00
rust HEAD: 50687be (dirty: 0 files)
```

Reps: rust 1. Cells: median [min–max].

### Startup and memory

| Metric | Rust | Rust adv. |
|---|---|---|
| cold start: docker run → /up 200 (ms) | 146 | – |
| idle memory.current (MB) | 14.0 | – |
| idle anon (MB) | 11.0 | – |
| peak memory.current under load (MB) | 199 | – |
| peak anon under load (MB) | 114 | – |

### HTTP (signed in as david; keep-alive; c = concurrent connections)

| Metric | Rust | Rust adv. |
|---|---|---|
| room_show c=1 req/s | 4,908 | – |
| room_show c=1 p50 ms | 0.20 | – |
| room_show c=1 p99 ms | 0.28 | – |
| room_show c=16 req/s | 19,882 | – |
| room_show c=16 p50 ms | 0.78 | – |
| room_show c=16 p99 ms | 1.51 | – |
| room_show c=64 req/s | 19,610 | – |
| room_show c=64 p50 ms | 3.25 | – |
| room_show c=64 p99 ms | 5.37 | – |
| messages_page c=1 req/s | 5,861 | – |
| messages_page c=1 p50 ms | 0.17 | – |
| messages_page c=1 p99 ms | 0.23 | – |
| messages_page c=16 req/s | 22,572 | – |
| messages_page c=16 p50 ms | 0.67 | – |
| messages_page c=16 p99 ms | 1.52 | – |
| messages_page c=64 req/s | 23,363 | – |
| messages_page c=64 p50 ms | 2.67 | – |
| messages_page c=64 p99 ms | 4.61 | – |
| sidebar c=1 req/s | 3,182 | – |
| sidebar c=1 p50 ms | 0.30 | – |
| sidebar c=1 p99 ms | 0.44 | – |
| sidebar c=16 req/s | 12,158 | – |
| sidebar c=16 p50 ms | 1.29 | – |
| sidebar c=16 p99 ms | 2.37 | – |
| sidebar c=64 req/s | 12,383 | – |
| sidebar c=64 p50 ms | 5.15 | – |
| sidebar c=64 p99 ms | 8.70 | – |
| search c=1 req/s | 6,370 | – |
| search c=1 p50 ms | 0.15 | – |
| search c=1 p99 ms | 0.22 | – |
| search c=16 req/s | 23,227 | – |
| search c=16 p50 ms | 0.65 | – |
| search c=16 p99 ms | 1.52 | – |
| search c=64 req/s | 23,946 | – |
| search c=64 p50 ms | 2.61 | – |
| search c=64 p99 ms | 4.46 | – |
| avatar c=1 req/s | 68,755 | – |
| avatar c=1 p50 ms | 0.01 | – |
| avatar c=1 p99 ms | 0.02 | – |
| avatar c=16 req/s | 413,408 | – |
| avatar c=16 p50 ms | 0.04 | – |
| avatar c=16 p99 ms | 0.08 | – |
| avatar c=64 req/s | 459,982 | – |
| avatar c=64 p50 ms | 0.13 | – |
| avatar c=64 p99 ms | 0.28 | – |
| static_css c=1 req/s | 74,789 | – |
| static_css c=1 p50 ms | 0.01 | – |
| static_css c=1 p99 ms | 0.02 | – |
| static_css c=16 req/s | 438,451 | – |
| static_css c=16 p50 ms | 0.03 | – |
| static_css c=16 p99 ms | 0.07 | – |
| static_css c=64 req/s | 474,015 | – |
| static_css c=64 p50 ms | 0.13 | – |
| static_css c=64 p99 ms | 0.27 | – |
| up c=1 req/s | 27,411 | – |
| up c=1 p50 ms | 0.04 | – |
| up c=1 p99 ms | 0.05 | – |
| up c=16 req/s | 134,055 | – |
| up c=16 p50 ms | 0.12 | – |
| up c=16 p99 ms | 0.21 | – |
| up c=64 req/s | 134,325 | – |
| up c=64 p50 ms | 0.46 | – |
| up c=64 p99 ms | 0.97 | – |
| post_message c=1 req/s | 2,192 | – |
| post_message c=1 p50 ms | 0.42 | – |
| post_message c=1 p99 ms | 1.74 | – |
| post_message c=16 req/s | 5,434 | – |
| post_message c=16 p50 ms | 2.73 | – |
| post_message c=16 p99 ms | 6.97 | – |
| post_message c=64 req/s | 5,191 | – |
| post_message c=64 p50 ms | 11.6 | – |
| post_message c=64 p99 ms | 17.2 | – |

### HTTP errors / non-2xx-3xx (first rep, per app)

| Metric | Rust | Rust adv. |
|---|---|---|
- rust: post_message c=64: {'200': 41551} errors=1805

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
