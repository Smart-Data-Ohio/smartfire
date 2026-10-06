```
date: 2026-09-27T06:12:56+02:00
host: 7.2.5-4-omarchy, AMD Ryzen 9 9955HX 16-Core Processor, 32 threads, 91GB
server cpus: 8-11 (nproc 4); loadgen cpus: 12-15; network: bridge
env: WEB_CONCURRENCY=3 JOB_CONCURRENCY=3 RAILS_MAX_THREADS=5 
rust extra env: RAILS_MAX_THREADS=128
reference image: campfire-reference:app sha256:70ab5e2abe726bfdfc177af4fa8465cd90ba3b3e3f532c8633f818cf63545adf 2026-09-26T14:42:50.327050882+02:00
rust image: campfire-rust:head-5181aa4 sha256:033927228539c17717887043791c154d9996f7ca9eb2e3683c91f9925eb200d5 2026-09-27T04:05:37.180679451+02:00
rust HEAD: 5181aa4 (dirty: 0 files)
```

Reps: reference 3, rust 3. Cells: median [min–max].

### Startup and memory

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
| cold start: docker run → /up 200 (ms) | 2,469 [2,453–2,477] | 303 [293–313] | 8.1× |
| idle memory.current (MB) | 319 [309–320] | 60.0 [60.0–63.0] | 5.3× |
| idle anon (MB) | 296 [288–297] | 57.0 [57.0–59.0] | 5.2× |
| peak memory.current under load (MB) | 1,468 [1,447–1,568] | 1,858 [1,817–1,862] | 0.8× |
| peak anon under load (MB) | 1,414 [1,395–1,512] | 1,541 [1,539–1,542] | 0.9× |

### HTTP (signed in as david; keep-alive; c = concurrent connections)

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
| room_show c=1 req/s | 94.6 [89.2–96.1] | 637 [624–638] | 6.7× |
| room_show c=1 p50 ms | 10.2 [10.1–10.5] | 1.54 [1.54–1.57] | 6.6× |
| room_show c=1 p99 ms | 12.4 [12.1–27.7] | 2.22 [2.20–2.28] | 5.6× |
| room_show c=16 req/s | 235 [220–238] | 2,561 [2,553–2,570] | 10.9× |
| room_show c=16 p50 ms | 64.0 [57.8–69.3] | 6.29 [6.26–6.31] | 10.2× |
| room_show c=16 p99 ms | 178 [137–187] | 10.5 [10.4–10.6] | 16.9× |
| room_show c=64 req/s | 197 [197–223] | 2,541 [2,534–2,546] | 12.9× |
| room_show c=64 p50 ms | 332 [314–343] | 24.9 [24.9–25.0] | 13.3× |
| room_show c=64 p99 ms | 454 [413–461] | 43.9 [43.2–44.9] | 10.4× |
| messages_page c=1 req/s | 172 [172–178] | 1,009 [1,003–1,011] | 5.9× |
| messages_page c=1 p50 ms | 5.55 [5.44–5.59] | 0.98 [0.98–0.99] | 5.6× |
| messages_page c=1 p99 ms | 11.7 [7.5–11.8] | 1.11 [1.09–1.12] | 10.5× |
| messages_page c=16 req/s | 443 [439–453] | 3,926 [3,900–3,938] | 8.9× |
| messages_page c=16 p50 ms | 32.3 [32.0–35.0] | 4.07 [4.05–4.11] | 7.9× |
| messages_page c=16 p99 ms | 102 [101–104] | 6.75 [6.75–6.88] | 15.0× |
| messages_page c=64 req/s | 428 [419–433] | 3,877 [3,844–3,889] | 9.1× |
| messages_page c=64 p50 ms | 151 [150–152] | 16.4 [16.4–16.5] | 9.2× |
| messages_page c=64 p99 ms | 233 [227–237] | 27.5 [27.4–28.7] | 8.5× |
| sidebar c=1 req/s | 222 [221–222] | 2,890 [2,847–2,896] | 13.0× |
| sidebar c=1 p50 ms | 4.36 [4.35–4.39] | 0.34 [0.34–0.34] | 12.9× |
| sidebar c=1 p99 ms | 6.03 [6.00–6.08] | 0.44 [0.44–0.45] | 13.6× |
| sidebar c=16 req/s | 580 [574–588] | 10,821 [10,757–10,829] | 18.7× |
| sidebar c=16 p50 ms | 27.1 [26.1–27.6] | 1.45 [1.45–1.46] | 18.7× |
| sidebar c=16 p99 ms | 52.3 [50.7–52.9] | 2.62 [2.62–2.63] | 19.9× |
| sidebar c=64 req/s | 570 [531–578] | 10,792 [10,733–10,849] | 18.9× |
| sidebar c=64 p50 ms | 114 [111–117] | 5.88 [5.86–5.92] | 19.4× |
| sidebar c=64 p99 ms | 189 [143–202] | 9.88 [9.72–10.04] | 19.2× |
| search c=1 req/s | 176 [174–177] | 1,519 [1,489–1,548] | 8.6× |
| search c=1 p50 ms | 5.53 [5.52–5.61] | 0.65 [0.64–0.67] | 8.5× |
| search c=1 p99 ms | 7.37 [7.34–7.50] | 0.76 [0.75–0.76] | 9.7× |
| search c=16 req/s | 427 [416–430] | 5,831 [5,814–5,888] | 13.7× |
| search c=16 p50 ms | 37.3 [33.0–38.8] | 2.72 [2.69–2.73] | 13.7× |
| search c=16 p99 ms | 76.8 [64.4–80.0] | 4.54 [4.51–4.54] | 16.9× |
| search c=64 req/s | 413 [391–421] | 5,811 [5,754–5,868] | 14.1× |
| search c=64 p50 ms | 152 [150–157] | 10.9 [10.8–11.0] | 13.9× |
| search c=64 p99 ms | 189 [183–245] | 18.0 [17.6–18.3] | 10.5× |
| avatar c=1 req/s | 24,201 [24,106–24,220] | 47,378 [47,222–47,379] | 2.0× |
| avatar c=1 p50 ms | 0.04 [0.04–0.04] | 0.02 [0.02–0.02] | 1.9× |
| avatar c=1 p99 ms | 0.09 [0.09–0.10] | 0.03 [0.03–0.03] | 3.6× |
| avatar c=16 req/s | 86,267 [86,040–87,094] | 189,169 [180,229–191,004] | 2.2× |
| avatar c=16 p50 ms | 0.12 [0.12–0.12] | 0.09 [0.08–0.09] | 1.4× |
| avatar c=16 p99 ms | 0.92 [0.92–0.94] | 0.18 [0.18–0.19] | 5.0× |
| avatar c=64 req/s | 70,456 [70,322–70,798] | 191,998 [190,912–192,287] | 2.7× |
| avatar c=64 p50 ms | 0.34 [0.33–0.34] | 0.32 [0.32–0.33] | 1.0× |
| avatar c=64 p99 ms | 5.32 [5.24–5.33] | 0.81 [0.81–0.81] | 6.6× |
| static_css c=1 req/s | 30,046 [29,923–30,099] | 49,866 [49,770–49,928] | 1.7× |
| static_css c=1 p50 ms | 0.03 [0.03–0.03] | 0.02 [0.02–0.02] | 1.6× |
| static_css c=1 p99 ms | 0.07 [0.07–0.07] | 0.02 [0.02–0.03] | 2.8× |
| static_css c=16 req/s | 123,268 [122,816–123,829] | 177,806 [175,246–179,162] | 1.4× |
| static_css c=16 p50 ms | 0.10 [0.10–0.10] | 0.09 [0.09–0.09] | 1.1× |
| static_css c=16 p99 ms | 0.62 [0.61–0.63] | 0.17 [0.17–0.18] | 3.5× |
| static_css c=64 req/s | 99,362 [99,255–100,912] | 196,407 [195,963–196,432] | 2.0× |
| static_css c=64 p50 ms | 0.32 [0.32–0.32] | 0.33 [0.33–0.33] | 1.0× |
| static_css c=64 p99 ms | 3.61 [3.54–3.66] | 0.81 [0.81–0.82] | 4.4× |
| up c=1 req/s | 1,888 [1,881–1,898] | 20,304 [20,061–20,514] | 10.8× |
| up c=1 p50 ms | 0.50 [0.50–0.50] | 0.05 [0.05–0.05] | 10.5× |
| up c=1 p99 ms | 0.92 [0.90–0.96] | 0.06 [0.06–0.06] | 15.8× |
| up c=16 req/s | 4,223 [4,217–4,232] | 98,854 [98,744–99,721] | 23.4× |
| up c=16 p50 ms | 3.66 [3.65–3.67] | 0.16 [0.16–0.16] | 22.3× |
| up c=16 p99 ms | 7.55 [7.38–7.59] | 0.27 [0.27–0.27] | 27.9× |
| up c=64 req/s | 4,172 [4,148–4,182] | 98,930 [98,890–99,638] | 23.7× |
| up c=64 p50 ms | 15.2 [15.1–15.5] | 0.63 [0.62–0.63] | 24.2× |
| up c=64 p99 ms | 23.9 [23.2–25.1] | 1.30 [1.29–1.30] | 18.4× |
| post_message c=1 req/s | 143 [139–152] | 2,131 [2,120–2,138] | 14.9× |
| post_message c=1 p50 ms | 6.28 [6.09–6.29] | 0.43 [0.43–0.43] | 14.5× |
| post_message c=1 p99 ms | 22.2 [18.0–25.3] | 1.40 [1.36–1.42] | 15.9× |
| post_message c=16 req/s | 268 [263–277] | 5,195 [5,107–5,227] | 19.4× |
| post_message c=16 p50 ms | 53.2 [50.5–55.8] | 2.66 [2.66–2.70] | 20.0× |
| post_message c=16 p99 ms | 158 [156–163] | 14.8 [14.4–14.8] | 10.7× |
| post_message c=64 req/s | 273 [256–279] | 5,116 [5,092–5,191] | 18.8× |
| post_message c=64 p50 ms | 224 [207–227] | 11.5 [11.4–11.6] | 19.5× |
| post_message c=64 p99 ms | 345 [336–476] | 25.8 [24.9–26.0] | 13.4× |

### HTTP errors / non-2xx-3xx (first rep, per app)

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
- reference: none
- rust: none

### Action Cable fan-out (one room; chatter.js subscriptions per client)

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
| 100 clients: subscribed | 100 [100–100] | 100 [100–100] | 1.0× |
| 100 clients: connect+subscribe all (s) | 0.27 [0.27–0.27] | 0.06 [0.06–0.06] | 4.5× |
| 100 clients: paced post→one client p50 ms | 13.0 [12.9–13.3] | 2.44 [2.43–2.53] | 5.3× |
| 100 clients: paced post→all clients p50 ms | 18.0 [17.7–18.8] | 2.75 [2.73–2.88] | 6.5× |
| 100 clients: paced post→all clients p99 ms | 64.5 [57.6–65.4] | 4.84 [3.60–6.12] | 13.3× |
| 100 clients: max sustained msgs/s (delivered to all) | 93.1 [85.2–94.7] | 1,921 [1,896–1,938] | 20.6× |
| 100 clients: deliveries/s (client×message) | 9,308 [8,524–9,472] | 192,091 [189,642–193,845] | 20.6× |
| 100 clients: saturated post→all p50 ms | 44.0 [43.9–46.1] | 1.76 [1.76–1.78] | 25.0× |
| 100 clients: saturated POST p50 ms | 31.8 [28.3–36.3] | 1.85 [1.85–1.88] | 17.2× |
| 500 clients: subscribed | 500 [500–500] | 500 [500–500] | 1.0× |
| 500 clients: connect+subscribe all (s) | 0.71 [0.67–0.75] | 0.12 [0.09–0.12] | 5.9× |
| 500 clients: paced post→one client p50 ms | 24.7 [24.1–24.8] | 3.88 [3.56–3.92] | 6.4× |
| 500 clients: paced post→all clients p50 ms | 44.9 [43.9–44.9] | 6.07 [5.79–6.20] | 7.4× |
| 500 clients: paced post→all clients p99 ms | 115 [98–117] | 8.44 [8.10–8.46] | 13.6× |
| 500 clients: max sustained msgs/s (delivered to all) | 25.8 [25.6–26.5] | 512 [473–513] | 19.9× |
| 500 clients: deliveries/s (client×message) | 12,905 [12,796–13,255] | 256,217 [236,340–256,633] | 19.9× |
| 500 clients: saturated post→all p50 ms | 146 [125–175] | 9.18 [8.86–9.29] | 15.9× |
| 500 clients: saturated POST p50 ms | 102 [97–122] | 7.52 [7.51–8.16] | 13.6× |
| 1000 clients: subscribed | 1,000 [1,000–1,000] | 1,000 [1,000–1,000] | 1.0× |
| 1000 clients: connect+subscribe all (s) | 1.41 [1.29–1.84] | 1.18 [1.15–1.25] | 1.2× |
| 1000 clients: paced post→one client p50 ms | 37.4 [37.2–37.8] | 5.47 [5.45–5.82] | 6.8× |
| 1000 clients: paced post→all clients p50 ms | 77.0 [75.4–77.3] | 9.91 [9.79–10.10] | 7.8× |
| 1000 clients: paced post→all clients p99 ms | 163 [156–188] | 14.7 [13.0–14.8] | 11.1× |
| 1000 clients: max sustained msgs/s (delivered to all) | 14.2 [13.3–14.4] | 276 [246–277] | 19.5× |
| 1000 clients: deliveries/s (client×message) | 14,162 [13,270–14,407] | 276,163 [245,471–276,855] | 19.5× |
| 1000 clients: saturated post→all p50 ms | 255 [223–511] | 17.9 [17.5–18.1] | 14.2× |
| 1000 clients: saturated POST p50 ms | 154 [134–210] | 14.2 [14.2–15.8] | 10.8× |

### Upload + thumbnail (black_hole.jpg, 505 KB)

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
| POST with attachment (ms) | 55.1 [50.3–57.3] | 28.1 [27.5–28.2] | 2.0× |
| then GET thumb → 200 (ms) | 0.60 [0.50–0.70] | 0.40 [0.40–0.40] | 1.5× |
| POST → thumbnail served (ms) | 55.5 [51.0–57.9] | 28.5 [27.9–28.6] | 1.9× |

### Memory during cable fan-out, by process (MB, peak within the phase)

App process: Rails' Puma master and workers (Action Cable runs in them), or Rust's one campfire
process (its front server included). Pss counts pages shared between forked workers once;
RssAnon counts them in every process.

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
| 100 clients, all subscribed, idle: app process Pss | 556 [545–574] | 1,389 [1,388–1,395] | 0.4× |
| 100 clients, all subscribed, idle: app process RssAnon | 700 [693–721] | 1,355 [1,353–1,360] | 0.5× |
| 100 clients, all subscribed, idle: app + Redis + Thruster Pss | 596 [585–613] | 1,389 [1,388–1,395] | 0.4× |
| 100 clients, all subscribed, idle: whole container Pss | 879 [871–891] | 1,389 [1,388–1,395] | 0.6× |
| 100 clients, saturated fan-out: app process Pss | 703 [698–706] | 1,390 [1,386–1,394] | 0.5× |
| 100 clients, saturated fan-out: app process RssAnon | 848 [842–849] | 1,356 [1,352–1,360] | 0.6× |
| 100 clients, saturated fan-out: app + Redis + Thruster Pss | 754 [749–758] | 1,390 [1,386–1,394] | 0.5× |
| 100 clients, saturated fan-out: whole container Pss | 1,040 [1,028–1,043] | 1,390 [1,386–1,394] | 0.7× |
| 500 clients, all subscribed, idle: app process Pss | 647 [631–649] | 1,381 [1,380–1,383] | 0.5× |
| 500 clients, all subscribed, idle: app process RssAnon | 790 [774–793] | 1,346 [1,345–1,348] | 0.6× |
| 500 clients, all subscribed, idle: app + Redis + Thruster Pss | 717 [700–720] | 1,381 [1,380–1,383] | 0.5× |
| 500 clients, all subscribed, idle: whole container Pss | 1,000 [978–1,008] | 1,381 [1,380–1,383] | 0.7× |
| 500 clients, saturated fan-out: app process Pss | 831 [816–839] | 1,393 [1,393–1,397] | 0.6× |
| 500 clients, saturated fan-out: app process RssAnon | 974 [960–982] | 1,358 [1,358–1,362] | 0.7× |
| 500 clients, saturated fan-out: app + Redis + Thruster Pss | 912 [897–918] | 1,393 [1,393–1,397] | 0.7× |
| 500 clients, saturated fan-out: whole container Pss | 1,195 [1,184–1,198] | 1,393 [1,393–1,397] | 0.9× |
| 1000 clients, all subscribed, idle: app process Pss | 702 [701–717] | 1,392 [1,390–1,392] | 0.5× |
| 1000 clients, all subscribed, idle: app process RssAnon | 845 [844–858] | 1,357 [1,355–1,357] | 0.6× |
| 1000 clients, all subscribed, idle: app + Redis + Thruster Pss | 838 [836–849] | 1,392 [1,390–1,392] | 0.6× |
| 1000 clients, all subscribed, idle: whole container Pss | 1,126 [1,115–1,133] | 1,392 [1,390–1,392] | 0.8× |
| 1000 clients, saturated fan-out: app process Pss | 920 [914–1,083] | 1,424 [1,424–1,426] | 0.6× |
| 1000 clients, saturated fan-out: app process RssAnon | 1,064 [1,056–1,225] | 1,389 [1,389–1,391] | 0.8× |
| 1000 clients, saturated fan-out: app + Redis + Thruster Pss | 1,062 [1,052–1,222] | 1,424 [1,424–1,426] | 0.7× |
| 1000 clients, saturated fan-out: whole container Pss | 1,346 [1,332–1,497] | 1,424 [1,424–1,426] | 0.9× |
