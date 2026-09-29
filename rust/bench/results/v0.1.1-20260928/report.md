```
date: 2026-09-28T14:25:40+02:00
host: 7.2.5-4-omarchy, AMD RYZEN AI MAX+ 395 w/ Radeon 8060S, 32 threads, 30GB
server cpus: 8-11 (nproc 4); loadgen cpus: 12-15; network: host
env: WEB_CONCURRENCY=3 JOB_CONCURRENCY=3 RAILS_MAX_THREADS=5 
rust extra env: 
reference image: campfire-reference:bench-898653e sha256:d91fdb852402e2d3393ca262e50b657ea0cf861fc7d4150aaf3773c91d41c2fa 2026-09-27T22:01:29.746709947+02:00
rust image: campfire-rust:bench-v0.1.1 sha256:d91939c11e7a6466864cc38b7f131a05052c10c7f09badf8473b329f556151e5 2026-09-28T12:02:45.59036902Z
rust HEAD: 080f903 (dirty: 0 files)
```

Reps: reference 3, rust 3. Cells: median [min–max].

### Startup and memory

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
| cold start: docker run → /up 200 (ms) | 2,567 [2,559–2,593] | 149 [145–170] | 17.2× |
| idle memory.current (MB) | 309 [306–376] | 15.0 [13.0–49.0] | 20.6× |
| idle anon (MB) | 286 [283–289] | 11.0 [11.0–11.0] | 26.0× |
| peak memory.current under load (MB) | 3,663 [3,314–3,698] | 1,130 [1,097–1,163] | 3.2× |
| peak anon under load (MB) | 3,469 [3,163–3,509] | 430 [416–432] | 8.1× |

### HTTP (signed in as david; keep-alive; c = concurrent connections)

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
| room_show c=1 req/s | 92.5 [91.9–93.5] | 5,103 [5,063–5,145] | 55.2× |
| room_show c=1 p50 ms | 10.3 [10.1–10.4] | 0.19 [0.19–0.19] | 53.5× |
| room_show c=1 p99 ms | 13.3 [13.3–13.4] | 0.25 [0.25–0.26] | 52.3× |
| room_show c=16 req/s | 215 [211–215] | 20,479 [20,392–20,573] | 95.3× |
| room_show c=16 p50 ms | 69.2 [66.7–70.0] | 0.76 [0.75–0.76] | 91.6× |
| room_show c=16 p99 ms | 165 [164–211] | 1.49 [1.47–1.53] | 110.9× |
| room_show c=64 req/s | 182 [176–183] | 20,360 [19,968–20,682] | 111.6× |
| room_show c=64 p50 ms | 341 [338–421] | 3.12 [3.08–3.18] | 109.1× |
| room_show c=64 p99 ms | 515 [475–537] | 5.09 [5.06–5.29] | 101.1× |
| messages_page c=1 req/s | 173 [172–174] | 6,018 [5,926–6,020] | 34.8× |
| messages_page c=1 p50 ms | 5.60 [5.58–5.62] | 0.16 [0.16–0.17] | 34.4× |
| messages_page c=1 p99 ms | 7.51 [7.50–7.70] | 0.22 [0.21–0.23] | 34.2× |
| messages_page c=16 req/s | 406 [398–411] | 23,365 [23,284–23,381] | 57.5× |
| messages_page c=16 p50 ms | 38.7 [38.5–40.8] | 0.65 [0.65–0.65] | 59.8× |
| messages_page c=16 p99 ms | 80.3 [73.4–91.8] | 1.46 [1.41–1.48] | 55.2× |
| messages_page c=64 req/s | 377 [373–383] | 23,472 [23,354–23,568] | 62.2× |
| messages_page c=64 p50 ms | 162 [158–165] | 2.69 [2.66–2.70] | 60.3× |
| messages_page c=64 p99 ms | 247 [240–265] | 4.45 [4.33–4.48] | 55.6× |
| sidebar c=1 req/s | 215 [192–216] | 3,310 [3,288–3,310] | 15.4× |
| sidebar c=1 p50 ms | 4.50 [4.47–4.50] | 0.29 [0.29–0.29] | 15.3× |
| sidebar c=1 p99 ms | 6.59 [6.57–20.72] | 0.42 [0.41–0.42] | 15.7× |
| sidebar c=16 req/s | 528 [517–529] | 12,642 [12,455–12,715] | 23.9× |
| sidebar c=16 p50 ms | 29.9 [28.7–30.8] | 1.24 [1.23–1.26] | 24.2× |
| sidebar c=16 p99 ms | 62.2 [54.8–81.3] | 2.30 [2.27–2.32] | 27.1× |
| sidebar c=64 req/s | 525 [473–540] | 12,865 [12,859–12,953] | 24.5× |
| sidebar c=64 p50 ms | 117 [116–130] | 4.95 [4.92–4.96] | 23.6× |
| sidebar c=64 p99 ms | 215 [163–216] | 8.29 [8.26–8.37] | 25.9× |
| search c=1 req/s | 171 [167–172] | 6,426 [6,415–6,480] | 37.6× |
| search c=1 p50 ms | 5.67 [5.66–5.78] | 0.15 [0.15–0.15] | 37.3× |
| search c=1 p99 ms | 7.84 [7.82–7.91] | 0.21 [0.21–0.21] | 36.8× |
| search c=16 req/s | 385 [377–388] | 23,399 [23,333–23,733] | 60.8× |
| search c=16 p50 ms | 41.3 [40.0–42.4] | 0.64 [0.64–0.65] | 64.4× |
| search c=16 p99 ms | 70.3 [64.9–84.2] | 1.44 [1.44–1.50] | 48.9× |
| search c=64 req/s | 380 [348–391] | 24,125 [23,940–24,392] | 63.5× |
| search c=64 p50 ms | 165 [161–177] | 2.59 [2.58–2.61] | 63.8× |
| search c=64 p99 ms | 208 [197–272] | 4.42 [4.19–4.49] | 47.1× |
| avatar c=1 req/s | 28,728 [28,675–28,839] | 70,161 [68,874–71,306] | 2.4× |
| avatar c=1 p50 ms | 0.03 [0.03–0.03] | 0.01 [0.01–0.01] | 2.5× |
| avatar c=1 p99 ms | 0.09 [0.09–0.09] | 0.02 [0.02–0.02] | 5.2× |
| avatar c=16 req/s | 98,665 [96,883–98,783] | 419,534 [410,208–425,552] | 4.3× |
| avatar c=16 p50 ms | 0.10 [0.10–0.10] | 0.04 [0.04–0.04] | 2.9× |
| avatar c=16 p99 ms | 0.88 [0.87–0.90] | 0.08 [0.06–0.09] | 11.5× |
| avatar c=64 req/s | 77,676 [76,321–79,963] | 461,000 [460,444–464,487] | 5.9× |
| avatar c=64 p50 ms | 0.32 [0.31–0.35] | 0.13 [0.13–0.13] | 2.4× |
| avatar c=64 p99 ms | 4.93 [4.75–5.06] | 0.28 [0.28–0.28] | 17.7× |
| static_css c=1 req/s | 35,515 [35,479–35,566] | 76,319 [74,084–76,970] | 2.1× |
| static_css c=1 p50 ms | 0.03 [0.03–0.03] | 0.01 [0.01–0.01] | 2.2× |
| static_css c=1 p99 ms | 0.07 [0.07–0.07] | 0.02 [0.02–0.02] | 4.2× |
| static_css c=16 req/s | 134,307 [134,106–134,326] | 440,931 [414,951–448,975] | 3.3× |
| static_css c=16 p50 ms | 0.08 [0.08–0.08] | 0.03 [0.03–0.03] | 2.4× |
| static_css c=16 p99 ms | 0.65 [0.64–0.66] | 0.06 [0.06–0.08] | 10.2× |
| static_css c=64 req/s | 110,414 [108,527–112,390] | 473,418 [463,858–477,310] | 4.3× |
| static_css c=64 p50 ms | 0.28 [0.28–0.30] | 0.13 [0.13–0.13] | 2.2× |
| static_css c=64 p99 ms | 3.44 [3.27–3.49] | 0.28 [0.27–0.28] | 12.4× |
| up c=1 req/s | 1,816 [1,785–1,844] | 28,087 [27,992–28,295] | 15.5× |
| up c=1 p50 ms | 0.52 [0.51–0.53] | 0.03 [0.03–0.03] | 15.1× |
| up c=1 p99 ms | 1.07 [1.04–1.08] | 0.05 [0.04–0.05] | 23.3× |
| up c=16 req/s | 4,069 [4,041–4,108] | 136,228 [133,701–136,890] | 33.5× |
| up c=16 p50 ms | 3.81 [3.77–3.85] | 0.12 [0.12–0.12] | 32.3× |
| up c=16 p99 ms | 7.85 [7.82–7.88] | 0.20 [0.20–0.21] | 38.9× |
| up c=64 req/s | 4,026 [3,819–4,051] | 137,060 [136,758–139,060] | 34.0× |
| up c=64 p50 ms | 15.7 [15.6–16.4] | 0.46 [0.45–0.46] | 34.5× |
| up c=64 p99 ms | 24.4 [24.1–28.6] | 0.95 [0.93–0.95] | 25.8× |
| post_message c=1 req/s | 144 [139–146] | 2,244 [2,242–2,256] | 15.6× |
| post_message c=1 p50 ms | 6.41 [6.33–6.51] | 0.41 [0.41–0.41] | 15.6× |
| post_message c=1 p99 ms | 13.4 [13.3–23.1] | 1.67 [1.62–1.71] | 8.0× |
| post_message c=16 req/s | 274 [257–275] | 5,452 [5,438–5,456] | 19.9× |
| post_message c=16 p50 ms | 53.9 [52.6–55.3] | 2.71 [2.70–2.71] | 19.9× |
| post_message c=16 p99 ms | 142 [137–160] | 7.21 [7.15–7.22] | 19.7× |
| post_message c=64 req/s | 267 [260–271] | 5,442 [5,428–5,510] | 20.4× |
| post_message c=64 p50 ms | 238 [230–244] | 11.6 [11.5–11.6] | 20.6× |
| post_message c=64 p99 ms | 349 [342–384] | 16.7 [16.5–17.0] | 20.9× |

### HTTP errors / non-2xx-3xx (first rep, per app)

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
- reference: none
- rust: none

### Action Cable fan-out (one room; chatter.js subscriptions per client)

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
| 100 clients: subscribed | 100 [100–100] | 100 [100–100] | 1.0× |
| 100 clients: connect+subscribe all (s) | 0.32 [0.27–0.32] | 0.06 [0.06–0.06] | 5.3× |
| 100 clients: paced post→one client p50 ms | 14.9 [14.7–15.1] | 2.12 [2.04–2.15] | 7.0× |
| 100 clients: paced post→all clients p50 ms | 20.6 [20.2–20.8] | 2.33 [2.21–2.33] | 8.9× |
| 100 clients: paced post→all clients p99 ms | 64.0 [56.9–73.6] | 3.84 [3.21–14.10] | 16.7× |
| 100 clients: max sustained msgs/s (delivered to all) | 78.9 [77.4–79.1] | 3,123 [3,114–3,163] | 39.6× |
| 100 clients: deliveries/s (client×message) | 7,892 [7,741–7,910] | 312,272 [311,366–316,325] | 39.6× |
| 100 clients: saturated post→all p50 ms | 46.8 [46.4–46.9] | 1.18 [1.16–1.18] | 39.9× |
| 100 clients: saturated POST p50 ms | 38.1 [34.8–45.0] | 1.17 [1.14–1.17] | 32.7× |
| 1000 clients: subscribed | 1,000 [1,000–1,000] | 1,000 [1,000–1,000] | 1.0× |
| 1000 clients: connect+subscribe all (s) | 1.43 [1.38–1.44] | 1.11 [0.19–1.15] | 1.3× |
| 1000 clients: paced post→one client p50 ms | 54.7 [42.8–55.9] | 4.31 [4.04–4.37] | 12.7× |
| 1000 clients: paced post→all clients p50 ms | 107 [101–110] | 6.54 [6.37–6.66] | 16.4× |
| 1000 clients: paced post→all clients p99 ms | 218 [196–233] | 8.97 [8.58–9.61] | 24.4× |
| 1000 clients: max sustained msgs/s (delivered to all) | 10.8 [10.4–11.5] | 503 [501–515] | 46.6× |
| 1000 clients: deliveries/s (client×message) | 10,771 [10,404–11,516] | 503,302 [500,689–514,716] | 46.7× |
| 1000 clients: saturated post→all p50 ms | 2,214 [2,132–2,525] | 14.6 [14.5–14.8] | 152.1× |
| 1000 clients: saturated POST p50 ms | 150 [80–162] | 7.59 [7.44–7.63] | 19.7× |
| 5000 clients: subscribed | 5,000 [5,000–5,000] | 5,000 [5,000–5,000] | 1.0× |
| 5000 clients: connect+subscribe all (s) | 8.65 [6.88–8.75] | 1.51 [0.61–1.70] | 5.7× |
| 5000 clients: paced post→one client p50 ms | 206 [184–432] | 12.1 [11.8–12.2] | 17.1× |
| 5000 clients: paced post→all clients p50 ms | 573 [557–1,837] | 21.2 [20.1–22.2] | 27.1× |
| 5000 clients: paced post→all clients p99 ms | 1,287 [1,146–3,004] | 33.4 [30.8–38.0] | 38.5× |
| 5000 clients: max sustained msgs/s (delivered to all) | 2.40 [2.20–2.60] | 119 [118–119] | 49.7× |
| 5000 clients: deliveries/s (client×message) | 11,930 [11,225–13,171] | 595,886 [587,390–597,150] | 49.9× |
| 5000 clients: saturated post→all p50 ms | 2,265 [1,497–5,689] | 117 [117–118] | 19.3× |
| 5000 clients: saturated POST p50 ms | 980 [580–1,001] | 18.5 [18.5–18.7] | 52.9× |
| 10000 clients: subscribed | 10,000 [10,000–10,000] | 10,000 [10,000–10,000] | 1.0× |
| 10000 clients: connect+subscribe all (s) | 29.1 [29.0–29.2] | 2.40 [1.40–2.54] | 12.1× |
| 10000 clients: paced post→one client p50 ms | 567 [560–583] | 21.4 [21.3–22.0] | 26.5× |
| 10000 clients: paced post→all clients p50 ms | 1,171 [1,136–1,175] | 39.8 [37.9–42.0] | 29.4× |
| 10000 clients: paced post→all clients p99 ms | 1,519 [1,504–1,674] | 61.0 [58.1–64.4] | 24.9× |
| 10000 clients: max sustained msgs/s (delivered to all) | 1.00 [0.90–1.00] | 63.9 [63.2–64.7] | 63.9× |
| 10000 clients: deliveries/s (client×message) | 9,585 [9,129–10,142] | 638,688 [632,150–646,601] | 66.6× |
| 10000 clients: saturated post→all p50 ms | 3,361 [3,095–4,272] | 261 [253–275] | 12.9× |
| 10000 clients: saturated POST p50 ms | 3,717 [2,007–3,946] | 25.4 [24.0–25.6] | 146.3× |

### Upload + thumbnail (black_hole.jpg, 505 KB)

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
| POST with attachment (ms) | 132 [110–137] | 29.2 [28.4–29.9] | 4.5× |
| then GET thumb → 200 (ms) | 0.40 [0.40–0.40] | 0.30 [0.30–0.30] | 1.3× |
| POST → thumbnail served (ms) | 132 [111–138] | 29.4 [28.6–30.8] | 4.5× |

### Memory during cable fan-out, by process (MB, peak within the phase)

App process: Rails' Puma master and workers (Action Cable runs in them), or Rust's one campfire
process (its front server included). Pss counts pages shared between forked workers once;
RssAnon counts them in every process.

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
| 100 clients, all subscribed, idle: app process Pss | 555 [550–571] | 149 [138–152] | 3.7× |
| 100 clients, all subscribed, idle: app process RssAnon | 702 [697–718] | 125 [113–128] | 5.6× |
| 100 clients, all subscribed, idle: app + Redis + Thruster Pss | 595 [589–611] | 149 [138–152] | 4.0× |
| 100 clients, all subscribed, idle: whole container Pss | 889 [872–909] | 149 [138–152] | 6.0× |
| 100 clients, saturated fan-out: app process Pss | 660 [654–665] | 158 [146–160] | 4.2× |
| 100 clients, saturated fan-out: app process RssAnon | 806 [801–812] | 134 [121–136] | 6.0× |
| 100 clients, saturated fan-out: app + Redis + Thruster Pss | 711 [705–716] | 158 [146–160] | 4.5× |
| 100 clients, saturated fan-out: whole container Pss | 1,006 [988–1,017] | 158 [146–160] | 6.4× |
| 1000 clients, all subscribed, idle: app process Pss | 645 [638–658] | 169 [158–169] | 3.8× |
| 1000 clients, all subscribed, idle: app process RssAnon | 790 [781–802] | 143 [131–144] | 5.5× |
| 1000 clients, all subscribed, idle: app + Redis + Thruster Pss | 744 [737–757] | 169 [158–169] | 4.4× |
| 1000 clients, all subscribed, idle: whole container Pss | 1,046 [1,015–1,057] | 169 [158–169] | 6.2× |
| 1000 clients, saturated fan-out: app process Pss | 922 [922–997] | 168 [156–169] | 5.5× |
| 1000 clients, saturated fan-out: app process RssAnon | 1,066 [1,065–1,141] | 142 [130–143] | 7.5× |
| 1000 clients, saturated fan-out: app + Redis + Thruster Pss | 1,030 [1,030–1,105] | 168 [156–169] | 6.1× |
| 1000 clients, saturated fan-out: whole container Pss | 1,330 [1,309–1,406] | 168 [156–169] | 7.9× |
| 5000 clients, all subscribed, idle: app process Pss | 989 [985–989] | 222 [219–231] | 4.5× |
| 5000 clients, all subscribed, idle: app process RssAnon | 1,130 [1,128–1,132] | 195 [193–205] | 5.8× |
| 5000 clients, all subscribed, idle: app + Redis + Thruster Pss | 1,362 [1,361–1,369] | 222 [219–231] | 6.1× |
| 5000 clients, all subscribed, idle: whole container Pss | 1,659 [1,639–1,668] | 222 [219–231] | 7.5× |
| 5000 clients, saturated fan-out: app process Pss | 1,855 [1,820–1,963] | 219 [217–230] | 8.5× |
| 5000 clients, saturated fan-out: app process RssAnon | 1,998 [1,960–2,105] | 194 [191–204] | 10.3× |
| 5000 clients, saturated fan-out: app + Redis + Thruster Pss | 2,272 [2,228–2,375] | 219 [217–230] | 10.4× |
| 5000 clients, saturated fan-out: whole container Pss | 2,567 [2,502–2,670] | 219 [217–230] | 11.7× |
| 10000 clients, all subscribed, idle: app process Pss | 1,507 [1,493–1,510] | 313 [292–322] | 4.8× |
| 10000 clients, all subscribed, idle: app process RssAnon | 1,647 [1,630–1,653] | 286 [267–296] | 5.8× |
| 10000 clients, all subscribed, idle: app + Redis + Thruster Pss | 2,472 [2,320–2,512] | 313 [292–322] | 7.9× |
| 10000 clients, all subscribed, idle: whole container Pss | 2,766 [2,614–2,784] | 313 [292–322] | 8.8× |
| 10000 clients, saturated fan-out: app process Pss | 2,191 [2,050–2,260] | 310 [296–315] | 7.1× |
| 10000 clients, saturated fan-out: app process RssAnon | 2,328 [2,193–2,400] | 283 [272–289] | 8.2× |
| 10000 clients, saturated fan-out: app + Redis + Thruster Pss | 3,248 [2,921–3,269] | 310 [296–315] | 10.5× |
| 10000 clients, saturated fan-out: whole container Pss | 3,519 [3,213–3,562] | 310 [296–315] | 11.4× |
