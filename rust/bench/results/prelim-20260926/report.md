```
date: 2026-09-26T19:53:03+02:00
host: 7.2.5-4-omarchy, AMD Ryzen 9 9955HX 16-Core Processor, 32 threads, 91GB
server cpus: 8-11 (nproc 4); loadgen cpus: 12-15
env: WEB_CONCURRENCY=3 JOB_CONCURRENCY=3 RAILS_MAX_THREADS=5 
rust extra env: RAILS_MAX_THREADS=128
reference image: campfire-reference:app sha256:70ab5e2abe726bfdfc177af4fa8465cd90ba3b3e3f532c8633f818cf63545adf 2026-09-26T14:42:50.327050882+02:00
rust image: campfire-rust:app sha256:e11e41a1baf9780913b89d850768b069ac166cd6c6295d485e0177b9cbd2e6a2 2026-09-26T19:51:41.042339493+02:00
rust HEAD: 2fee0df (dirty: 19 files)
```

Reps: reference 3, rust 3. Cells: median [min–max].

### Startup and memory

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
| cold start: docker run → /up 200 (ms) | 4,010 [2,518–4,169] | 399 [368–475] | 10.1× |
| idle memory.current (MB) | 319 [302–320] | 31.0 [31.0–31.0] | 10.3× |
| idle anon (MB) | 299 [283–300] | 29.0 [29.0–29.0] | 10.3× |
| peak memory.current under load (MB) | 1,456 [1,404–1,535] | 1,054 [993–1,134] | 1.4× |
| peak anon under load (MB) | 1,375 [1,363–1,484] | 994 [930–1,057] | 1.4× |

### HTTP (signed in as david; keep-alive; c = concurrent connections)

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
| room_show c=1 req/s | 51.4 [49.5–95.1] | 163 [154–197] | 3.2× |
| room_show c=1 p50 ms | 18.4 [10.3–18.9] | 5.85 [4.86–6.36] | 3.1× |
| room_show c=1 p99 ms | 28.6 [13.0–35.5] | 9.31 [7.80–9.56] | 3.1× |
| room_show c=16 req/s | 146 [132–217] | 638 [624–793] | 4.4× |
| room_show c=16 p50 ms | 96.3 [52.0–111.9] | 24.3 [19.6–24.7] | 4.0× |
| room_show c=16 p99 ms | 267 [266–300] | 46.0 [36.4–46.2] | 5.8× |
| room_show c=64 req/s | 135 [122–172] | 693 [676–799] | 5.1× |
| room_show c=64 p50 ms | 456 [348–524] | 90.1 [78.1–92.4] | 5.1× |
| room_show c=64 p99 ms | 727 [694–732] | 162 [144–165] | 4.5× |
| messages_page c=1 req/s | 119 [98–166] | 195 [188–257] | 1.6× |
| messages_page c=1 p50 ms | 7.44 [5.59–9.29] | 5.03 [3.67–5.08] | 1.5× |
| messages_page c=1 p99 ms | 25.1 [15.7–25.9] | 8.18 [6.21–8.73] | 3.1× |
| messages_page c=16 req/s | 312 [253–420] | 783 [756–920] | 2.5× |
| messages_page c=16 p50 ms | 46.7 [35.1–55.0] | 19.9 [16.8–20.6] | 2.3× |
| messages_page c=16 p99 ms | 151 [115–160] | 36.5 [31.8–37.8] | 4.1× |
| messages_page c=64 req/s | 259 [257–407] | 796 [738–948] | 3.1× |
| messages_page c=64 p50 ms | 237 [153–241] | 78.3 [65.9–84.3] | 3.0× |
| messages_page c=64 p99 ms | 363 [233–376] | 138 [116–154] | 2.6× |
| sidebar c=1 req/s | 123 [118–224] | 863 [659–1,036] | 7.0× |
| sidebar c=1 p50 ms | 7.53 [4.32–8.05] | 1.10 [0.93–1.40] | 6.8× |
| sidebar c=1 p99 ms | 15.3 [6.1–15.8] | 2.11 [1.59–3.46] | 7.2× |
| sidebar c=16 req/s | 345 [305–560] | 3,086 [2,859–4,004] | 8.9× |
| sidebar c=16 p50 ms | 45.2 [27.6–49.9] | 4.98 [3.82–5.36] | 9.1× |
| sidebar c=16 p99 ms | 99.1 [59.6–147.2] | 9.96 [7.88–11.14] | 9.9× |
| sidebar c=64 req/s | 329 [308–576] | 3,266 [3,044–4,148] | 9.9× |
| sidebar c=64 p50 ms | 188 [110–199] | 19.0 [15.0–20.5] | 9.9× |
| sidebar c=64 p99 ms | 304 [141–370] | 36.0 [27.7–38.5] | 8.4× |
| search c=1 req/s | 105 [92–178] | 366 [342–503] | 3.5× |
| search c=1 p50 ms | 9.18 [5.47–10.35] | 2.64 [1.88–2.76] | 3.5× |
| search c=1 p99 ms | 16.6 [7.4–18.6] | 4.50 [3.44–5.32] | 3.7× |
| search c=16 req/s | 280 [243–422] | 1,545 [1,347–1,856] | 5.5× |
| search c=16 p50 ms | 57.2 [39.2–63.3] | 10.0 [8.3–11.5] | 5.7× |
| search c=16 p99 ms | 110 [74–119] | 19.1 [16.1–21.3] | 5.8× |
| search c=64 req/s | 286 [238–361] | 1,613 [1,354–1,885] | 5.6× |
| search c=64 p50 ms | 211 [168–265] | 38.8 [33.1–46.1] | 5.4× |
| search c=64 p99 ms | 363 [332–365] | 66.6 [57.9–78.7] | 5.5× |
| avatar c=1 req/s | 14,904 [11,718–22,156] | 12,283 [10,290–19,894] | 0.8× |
| avatar c=1 p50 ms | 0.05 [0.04–0.07] | 0.07 [0.04–0.08] | 0.8× |
| avatar c=1 p99 ms | 0.20 [0.13–0.27] | 0.25 [0.13–0.32] | 0.8× |
| avatar c=16 req/s | 57,098 [51,846–86,260] | 55,276 [50,754–73,383] | 1.0× |
| avatar c=16 p50 ms | 0.19 [0.12–0.20] | 0.19 [0.14–0.21] | 1.0× |
| avatar c=16 p99 ms | 1.40 [0.92–1.54] | 1.53 [1.11–1.61] | 0.9× |
| avatar c=64 req/s | 46,667 [42,397–63,337] | 51,232 [38,690–60,442] | 1.1× |
| avatar c=64 p50 ms | 0.54 [0.36–0.63] | 0.47 [0.40–0.72] | 1.1× |
| avatar c=64 p99 ms | 7.66 [5.95–8.22] | 7.15 [6.15–8.75] | 1.1× |
| static_css c=1 req/s | 15,746 [13,814–16,147] | 14,904 [12,614–23,956] | 0.9× |
| static_css c=1 p50 ms | 0.05 [0.05–0.06] | 0.06 [0.04–0.07] | 0.9× |
| static_css c=1 p99 ms | 0.21 [0.20–0.26] | 0.22 [0.11–0.27] | 0.9× |
| static_css c=16 req/s | 88,800 [73,650–104,529] | 86,612 [74,391–105,193] | 1.0× |
| static_css c=16 p50 ms | 0.13 [0.11–0.15] | 0.13 [0.11–0.15] | 1.0× |
| static_css c=16 p99 ms | 0.93 [0.75–1.12] | 0.97 [0.76–1.12] | 1.0× |
| static_css c=64 req/s | 67,482 [59,432–94,984] | 63,736 [56,653–89,956] | 0.9× |
| static_css c=64 p50 ms | 0.48 [0.36–0.57] | 0.54 [0.41–0.64] | 0.9× |
| static_css c=64 p99 ms | 5.14 [3.78–5.59] | 5.33 [3.85–5.63] | 1.0× |
| up c=1 req/s | 1,121 [1,083–1,427] | 6,219 [4,182–8,039] | 5.5× |
| up c=1 p50 ms | 0.81 [0.65–0.84] | 0.14 [0.10–0.22] | 5.7× |
| up c=1 p99 ms | 2.29 [1.62–2.31] | 0.40 [0.30–0.71] | 5.7× |
| up c=16 req/s | 2,800 [2,590–3,309] | 16,813 [12,500–19,030] | 6.0× |
| up c=16 p50 ms | 5.34 [4.34–5.77] | 0.84 [0.77–1.14] | 6.4× |
| up c=16 p99 ms | 14.1 [13.3–14.5] | 2.46 [2.23–3.70] | 5.7× |
| up c=64 req/s | 2,838 [2,726–3,597] | 15,557 [12,501–19,513] | 5.5× |
| up c=64 p50 ms | 22.0 [17.4–22.6] | 3.46 [2.92–4.50] | 6.4× |
| up c=64 p99 ms | 41.6 [29.4–41.7] | 11.8 [9.2–15.2] | 3.5× |
| post_message c=1 req/s | 75.2 [71.6–123.7] | 626 [467–799] | 8.3× |
| post_message c=1 p50 ms | 12.2 [7.2–12.3] | 1.27 [0.96–1.73] | 9.6× |
| post_message c=1 p99 ms | 29.5 [24.7–49.3] | 13.6 [12.6–16.1] | 2.2× |
| post_message c=16 req/s | 166 [110–223] | 1,453 [1,314–1,659] | 8.8× |
| post_message c=16 p50 ms | 79.7 [70.0–100.7] | 7.82 [6.52–8.49] | 10.2× |
| post_message c=16 p99 ms | 287 [203–616] | 33.2 [29.2–36.4] | 8.6× |
| post_message c=64 req/s | 177 [112–238] | 1,539 [1,330–1,768] | 8.7× |
| post_message c=64 p50 ms | 356 [259–433] | 40.4 [35.3–46.5] | 8.8× |
| post_message c=64 p99 ms | 548 [388–1,733] | 74.1 [61.8–90.2] | 7.4× |

### HTTP errors / non-2xx-3xx (first rep, per app)

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
- reference: none
- rust: none

### Action Cable fan-out (one room; chatter.js subscriptions per client)

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
| 100 clients: subscribed | 100 [100–100] | 100 [100–100] | 1.0× |
| 100 clients: connect+subscribe all (s) | 0.33 [0.32–0.38] | 0.06 [0.06–0.06] | 5.5× |
| 100 clients: paced post→one client p50 ms | 17.4 [15.0–17.4] | 3.75 [3.06–4.70] | 4.6× |
| 100 clients: paced post→all clients p50 ms | 23.8 [20.6–24.8] | 4.27 [3.54–5.33] | 5.6× |
| 100 clients: paced post→all clients p99 ms | 96.8 [80.7–117.4] | 11.6 [5.7–19.5] | 8.4× |
| 100 clients: max sustained msgs/s (delivered to all) | 63.4 [55.2–80.4] | 482 [442–703] | 7.6× |
| 100 clients: deliveries/s (client×message) | 6,337 [5,518–8,042] | 48,216 [44,242–70,310] | 7.6× |
| 100 clients: saturated post→all p50 ms | 59.8 [47.2–64.4] | 6.59 [4.33–7.22] | 9.1× |
| 100 clients: saturated POST p50 ms | 53.2 [39.1–63.1] | 7.20 [4.82–7.82] | 7.4× |
| 500 clients: subscribed | 500 [500–500] | 500 [500–500] | 1.0× |
| 500 clients: connect+subscribe all (s) | 1.11 [0.89–1.16] | 1.09 [0.14–1.17] | 1.0× |
| 500 clients: paced post→one client p50 ms | 32.2 [26.7–38.0] | 7.78 [6.70–9.07] | 4.1× |
| 500 clients: paced post→all clients p50 ms | 65.1 [47.2–78.0] | 16.5 [13.8–18.6] | 3.9× |
| 500 clients: paced post→all clients p99 ms | 137 [98–143] | 26.1 [25.3–28.5] | 5.2× |
| 500 clients: max sustained msgs/s (delivered to all) | 18.2 [16.1–21.6] | 88.4 [78.1–122.1] | 4.9× |
| 500 clients: deliveries/s (client×message) | 9,107 [8,052–10,783] | 44,222 [39,064–61,068] | 4.9× |
| 500 clients: saturated post→all p50 ms | 204 [158–230] | 42.3 [29.5–46.1] | 4.8× |
| 500 clients: saturated POST p50 ms | 180 [162–193] | 43.5 [31.6–47.6] | 4.1× |
| 1000 clients: subscribed | 1,000 [1,000–1,000] | 1,000 [1,000–1,000] | 1.0× |
| 1000 clients: connect+subscribe all (s) | 1.89 [1.51–2.52] | 0.23 [0.16–1.25] | 8.2× |
| 1000 clients: paced post→one client p50 ms | 52.8 [41.8–60.5] | 13.5 [10.9–14.2] | 3.9× |
| 1000 clients: paced post→all clients p50 ms | 111 [87–127] | 31.5 [24.5–33.5] | 3.5× |
| 1000 clients: paced post→all clients p99 ms | 228 [183–286] | 49.0 [36.4–360.7] | 4.6× |
| 1000 clients: max sustained msgs/s (delivered to all) | 9.40 [9.10–11.70] | 45.7 [42.1–59.3] | 4.9× |
| 1000 clients: deliveries/s (client×message) | 9,443 [9,078–11,651] | 45,723 [42,143–59,299] | 4.8× |
| 1000 clients: saturated post→all p50 ms | 383 [334–390] | 76.4 [58.8–81.7] | 5.0× |
| 1000 clients: saturated POST p50 ms | 346 [233–394] | 84.2 [65.0–89.7] | 4.1× |

### Upload + thumbnail (black_hole.jpg, 505 KB)

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
| POST with attachment (ms) | 85.8 [85.8–88.1] | 39.4 [36.4–40.7] | 2.2× |
| then GET thumb → 200 (ms) | 0.50 [0.50–0.60] | 0.60 [0.50–0.70] | 0.8× |
| POST → thumbnail served (ms) | 86.4 [86.3–88.6] | 40.0 [36.9–41.5] | 2.2× |
