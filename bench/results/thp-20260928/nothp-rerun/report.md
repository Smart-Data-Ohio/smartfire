```
date: 2026-09-28T13:40:49+02:00
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
| cold start: docker run → /up 200 (ms) | 141 | – |
| idle memory.current (MB) | 13.0 | – |
| idle anon (MB) | 11.0 | – |
| peak memory.current under load (MB) | 205 | – |
| peak anon under load (MB) | 119 | – |

### HTTP (signed in as david; keep-alive; c = concurrent connections)

| Metric | Rust | Rust adv. |
|---|---|---|
| room_show c=1 req/s | 5,114 | – |
| room_show c=1 p50 ms | 0.19 | – |
| room_show c=1 p99 ms | 0.25 | – |
| room_show c=16 req/s | 20,293 | – |
| room_show c=16 p50 ms | 0.76 | – |
| room_show c=16 p99 ms | 1.49 | – |
| room_show c=64 req/s | 20,158 | – |
| room_show c=64 p50 ms | 3.16 | – |
| room_show c=64 p99 ms | 5.17 | – |
| messages_page c=1 req/s | 5,892 | – |
| messages_page c=1 p50 ms | 0.17 | – |
| messages_page c=1 p99 ms | 0.22 | – |
| messages_page c=16 req/s | 23,196 | – |
| messages_page c=16 p50 ms | 0.66 | – |
| messages_page c=16 p99 ms | 1.40 | – |
| messages_page c=64 req/s | 23,388 | – |
| messages_page c=64 p50 ms | 2.70 | – |
| messages_page c=64 p99 ms | 4.39 | – |
| sidebar c=1 req/s | 3,275 | – |
| sidebar c=1 p50 ms | 0.30 | – |
| sidebar c=1 p99 ms | 0.43 | – |
| sidebar c=16 req/s | 12,636 | – |
| sidebar c=16 p50 ms | 1.24 | – |
| sidebar c=16 p99 ms | 2.31 | – |
| sidebar c=64 req/s | 12,695 | – |
| sidebar c=64 p50 ms | 5.01 | – |
| sidebar c=64 p99 ms | 8.46 | – |
| search c=1 req/s | 6,436 | – |
| search c=1 p50 ms | 0.15 | – |
| search c=1 p99 ms | 0.21 | – |
| search c=16 req/s | 23,465 | – |
| search c=16 p50 ms | 0.64 | – |
| search c=16 p99 ms | 1.45 | – |
| search c=64 req/s | 24,184 | – |
| search c=64 p50 ms | 2.60 | – |
| search c=64 p99 ms | 4.22 | – |
| avatar c=1 req/s | 70,600 | – |
| avatar c=1 p50 ms | 0.01 | – |
| avatar c=1 p99 ms | 0.02 | – |
| avatar c=16 req/s | 428,391 | – |
| avatar c=16 p50 ms | 0.04 | – |
| avatar c=16 p99 ms | 0.07 | – |
| avatar c=64 req/s | 458,112 | – |
| avatar c=64 p50 ms | 0.13 | – |
| avatar c=64 p99 ms | 0.29 | – |
| static_css c=1 req/s | 75,866 | – |
| static_css c=1 p50 ms | 0.01 | – |
| static_css c=1 p99 ms | 0.02 | – |
| static_css c=16 req/s | 439,252 | – |
| static_css c=16 p50 ms | 0.03 | – |
| static_css c=16 p99 ms | 0.07 | – |
| static_css c=64 req/s | 464,876 | – |
| static_css c=64 p50 ms | 0.13 | – |
| static_css c=64 p99 ms | 0.28 | – |
| up c=1 req/s | 27,858 | – |
| up c=1 p50 ms | 0.03 | – |
| up c=1 p99 ms | 0.05 | – |
| up c=16 req/s | 134,961 | – |
| up c=16 p50 ms | 0.12 | – |
| up c=16 p99 ms | 0.20 | – |
| up c=64 req/s | 132,590 | – |
| up c=64 p50 ms | 0.47 | – |
| up c=64 p99 ms | 0.97 | – |
| post_message c=1 req/s | 2,211 | – |
| post_message c=1 p50 ms | 0.42 | – |
| post_message c=1 p99 ms | 1.71 | – |
| post_message c=16 req/s | 5,491 | – |
| post_message c=16 p50 ms | 2.69 | – |
| post_message c=16 p99 ms | 7.17 | – |
| post_message c=64 req/s | 5,475 | – |
| post_message c=64 p50 ms | 11.5 | – |
| post_message c=64 p99 ms | 16.8 | – |

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
