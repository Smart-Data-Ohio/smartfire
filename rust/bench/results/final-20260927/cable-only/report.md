```
date: 2026-09-27T06:52:08+02:00
host: 7.2.5-4-omarchy, AMD Ryzen 9 9955HX 16-Core Processor, 32 threads, 91GB
server cpus: 8-11 (nproc 4); loadgen cpus: 12-15; network: host
env: WEB_CONCURRENCY=3 JOB_CONCURRENCY=3 RAILS_MAX_THREADS=5 
rust extra env: 
reference image: campfire-reference:app sha256:70ab5e2abe726bfdfc177af4fa8465cd90ba3b3e3f532c8633f818cf63545adf 2026-09-26T14:42:50.327050882+02:00
rust image: campfire-rust:head-5181aa4 sha256:033927228539c17717887043791c154d9996f7ca9eb2e3683c91f9925eb200d5 2026-09-27T04:05:37.180679451+02:00
rust HEAD: 5181aa4 (dirty: 0 files)
```

Reps: reference 5, rust 5. Cells: median [min–max].

### Startup and memory

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
| cold start: docker run → /up 200 (ms) | 2,449 [2,411–2,467] | 232 [225–861] | 10.6× |
| idle memory.current (MB) | 309 [302–317] | 47.0 [47.0–50.0] | 6.6× |
| idle anon (MB) | 288 [280–295] | 44.0 [44.0–46.0] | 6.5× |
| peak memory.current under load (MB) | 1,176 [1,081–1,240] | 379 [359–392] | 3.1× |
| peak anon under load (MB) | 1,136 [1,041–1,200] | 289 [277–301] | 3.9× |

### HTTP (signed in as david; keep-alive; c = concurrent connections)

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|

### HTTP errors / non-2xx-3xx (first rep, per app)

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
- reference: none
- rust: none

### Action Cable fan-out (one room; chatter.js subscriptions per client)

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
| 1000 clients: subscribed | 1,000 [1,000–1,000] | 1,000 [1,000–1,000] | 1.0× |
| 1000 clients: connect+subscribe all (s) | 1.51 [1.47–1.56] | 0.13 [0.11–1.09] | 11.6× |
| 1000 clients: paced post→one client p50 ms | 38.7 [36.0–47.7] | 4.83 [4.63–4.98] | 8.0× |
| 1000 clients: paced post→all clients p50 ms | 78.6 [77.3–97.4] | 9.29 [8.32–9.94] | 8.5× |
| 1000 clients: paced post→all clients p99 ms | 162 [151–193] | 12.1 [11.9–12.8] | 13.4× |
| 1000 clients: max sustained msgs/s (delivered to all) | 13.9 [11.4–14.4] | 303 [286–309] | 21.8× |
| 1000 clients: deliveries/s (client×message) | 13,864 [11,395–14,436] | 302,722 [285,633–308,707] | 21.8× |
| 1000 clients: saturated post→all p50 ms | 221 [213–1,126] | 17.2 [16.9–18.0] | 12.9× |
| 1000 clients: saturated POST p50 ms | 230 [182–268] | 13.0 [12.8–13.6] | 17.7× |

### Upload + thumbnail (black_hole.jpg, 505 KB)

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
| POST with attachment (ms) | – | – | – |
| then GET thumb → 200 (ms) | – | – | – |
| POST → thumbnail served (ms) | – | – | – |

### Memory during cable fan-out, by process (MB, peak within the phase)

App process: Rails' Puma master and workers (Action Cable runs in them), or Rust's one campfire
process (its front server included). Pss counts pages shared between forked workers once;
RssAnon counts them in every process.

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
| 1000 clients, all subscribed, idle: app process Pss | 459 [455–484] | 154 [151–154] | 3.0× |
| 1000 clients, all subscribed, idle: app process RssAnon | 622 [621–646] | 122 [120–123] | 5.1× |
| 1000 clients, all subscribed, idle: app + Redis + Thruster Pss | 539 [535–564] | 154 [151–154] | 3.5× |
| 1000 clients, all subscribed, idle: whole container Pss | 713 [695–727] | 154 [151–154] | 4.6× |
| 1000 clients, saturated fan-out: app process Pss | 836 [751–879] | 319 [309–331] | 2.6× |
| 1000 clients, saturated fan-out: app process RssAnon | 985 [897–1,030] | 287 [277–299] | 3.4× |
| 1000 clients, saturated fan-out: app + Redis + Thruster Pss | 925 [841–970] | 319 [309–331] | 2.9× |
| 1000 clients, saturated fan-out: whole container Pss | 1,188 [1,095–1,255] | 319 [309–331] | 3.7× |
