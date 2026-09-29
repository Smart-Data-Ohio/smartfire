The Rust image benchmarked here is `main` at `898653e` (tagged `campfire-rust:bench-898653e`,
built before this run); `rust HEAD` below is the checkout `bench/run` ran from, not the image.

```
date: 2026-09-27T23:02:08+02:00
host: 7.2.5-4-omarchy, AMD RYZEN AI MAX+ 395 w/ Radeon 8060S, 32 threads, 30GB
server cpus: 8-11 (nproc 4); loadgen cpus: 12-15; network: host
env: WEB_CONCURRENCY=3 JOB_CONCURRENCY=3 RAILS_MAX_THREADS=5 
rust extra env: 
reference image: campfire-reference:bench-898653e sha256:d91fdb852402e2d3393ca262e50b657ea0cf861fc7d4150aaf3773c91d41c2fa 2026-09-27T22:01:29.746709947+02:00
rust image: campfire-rust:bench-898653e sha256:ee687a85dcc32ab3c384a1979af2b3ba68ca05fc28f71f3337c600f82afb4d8c 2026-09-27T22:02:28.171466671+02:00
rust HEAD: 2947c64 (dirty: 0 files)
```

Reps: reference 3, rust 3. Cells: median [min–max].

### Startup and memory

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
| cold start: docker run → /up 200 (ms) | 2,536 [2,529–2,599] | 135 [135–139] | 18.8× |
| idle memory.current (MB) | 304 [302–316] | 47.0 [47.0–47.0] | 6.5× |
| idle anon (MB) | 282 [281–294] | 44.0 [44.0–44.0] | 6.4× |
| peak memory.current under load (MB) | 3,447 [3,240–3,460] | 1,148 [1,094–1,206] | 3.0× |
| peak anon under load (MB) | 3,292 [3,088–3,305] | 899 [838–915] | 3.7× |

### HTTP (signed in as david; keep-alive; c = concurrent connections)

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
| room_show c=1 req/s | 92.9 [92.8–93.3] | 1,510 [1,502–1,528] | 16.3× |
| room_show c=1 p50 ms | 10.5 [10.4–10.5] | 0.65 [0.65–0.66] | 16.0× |
| room_show c=1 p99 ms | 13.0 [12.8–13.4] | 0.78 [0.78–0.80] | 16.7× |
| room_show c=16 req/s | 221 [221–228] | 6,002 [5,961–6,021] | 27.2× |
| room_show c=16 p50 ms | 69.6 [69.3–70.0] | 2.68 [2.66–2.69] | 26.0× |
| room_show c=16 p99 ms | 173 [146–202] | 4.55 [4.54–4.55] | 38.0× |
| room_show c=64 req/s | 185 [184–189] | 5,944 [5,932–6,002] | 32.1× |
| room_show c=64 p50 ms | 340 [336–344] | 10.6 [10.6–10.7] | 32.0× |
| room_show c=64 p99 ms | 453 [425–509] | 18.7 [18.6–18.9] | 24.3× |
| messages_page c=1 req/s | 174 [155–175] | 4,523 [4,471–4,561] | 26.1× |
| messages_page c=1 p50 ms | 5.59 [5.56–5.63] | 0.22 [0.21–0.22] | 25.7× |
| messages_page c=1 p99 ms | 7.47 [7.41–19.57] | 0.29 [0.29–0.30] | 25.4× |
| messages_page c=16 req/s | 396 [388–408] | 17,540 [17,460–17,548] | 44.3× |
| messages_page c=16 p50 ms | 37.8 [33.2–40.0] | 0.88 [0.88–0.88] | 43.0× |
| messages_page c=16 p99 ms | 94.3 [89.0–107.1] | 1.68 [1.68–1.72] | 56.0× |
| messages_page c=64 req/s | 382 [378–384] | 17,326 [17,084–17,582] | 45.4× |
| messages_page c=64 p50 ms | 160 [159–163] | 3.66 [3.62–3.70] | 43.7× |
| messages_page c=64 p99 ms | 257 [238–269] | 5.78 [5.69–5.97] | 44.4× |
| sidebar c=1 req/s | 216 [215–217] | 3,044 [2,874–3,045] | 14.1× |
| sidebar c=1 p50 ms | 4.46 [4.43–4.50] | 0.32 [0.32–0.34] | 13.9× |
| sidebar c=1 p99 ms | 6.53 [6.39–6.54] | 0.45 [0.43–0.45] | 14.7× |
| sidebar c=16 req/s | 528 [526–533] | 11,550 [11,436–11,608] | 21.9× |
| sidebar c=16 p50 ms | 31.5 [30.8–32.4] | 1.35 [1.34–1.36] | 23.3× |
| sidebar c=16 p99 ms | 58.1 [58.1–58.9] | 2.50 [2.48–2.52] | 23.3× |
| sidebar c=64 req/s | 489 [484–538] | 11,860 [11,748–11,912] | 24.3× |
| sidebar c=64 p50 ms | 129 [118–138] | 5.36 [5.33–5.41] | 24.0× |
| sidebar c=64 p99 ms | 225 [141–244] | 8.80 [8.79–8.92] | 25.5× |
| search c=1 req/s | 172 [170–173] | 2,398 [2,389–2,412] | 13.9× |
| search c=1 p50 ms | 5.63 [5.62–5.70] | 0.41 [0.41–0.41] | 13.8× |
| search c=1 p99 ms | 7.74 [7.73–7.95] | 0.56 [0.55–0.56] | 13.8× |
| search c=16 req/s | 388 [381–390] | 8,970 [8,894–9,024] | 23.1× |
| search c=16 p50 ms | 38.8 [34.3–44.1] | 1.75 [1.74–1.76] | 22.2× |
| search c=16 p99 ms | 86.8 [81.5–93.9] | 3.07 [3.06–3.12] | 28.3× |
| search c=64 req/s | 357 [355–382] | 9,027 [8,954–9,108] | 25.3× |
| search c=64 p50 ms | 174 [167–176] | 7.06 [7.00–7.11] | 24.6× |
| search c=64 p99 ms | 263 [212–271] | 11.1 [11.0–11.2] | 23.8× |
| avatar c=1 req/s | 28,899 [28,663–29,468] | 69,218 [68,824–70,047] | 2.4× |
| avatar c=1 p50 ms | 0.03 [0.03–0.03] | 0.01 [0.01–0.01] | 2.5× |
| avatar c=1 p99 ms | 0.09 [0.09–0.09] | 0.02 [0.02–0.02] | 5.2× |
| avatar c=16 req/s | 98,475 [97,605–99,308] | 413,950 [412,218–420,140] | 4.2× |
| avatar c=16 p50 ms | 0.10 [0.10–0.10] | 0.04 [0.04–0.04] | 2.8× |
| avatar c=16 p99 ms | 0.88 [0.88–0.89] | 0.07 [0.06–0.07] | 13.1× |
| avatar c=64 req/s | 78,766 [78,450–79,408] | 441,911 [439,584–444,943] | 5.6× |
| avatar c=64 p50 ms | 0.33 [0.31–0.33] | 0.14 [0.14–0.14] | 2.4× |
| avatar c=64 p99 ms | 4.86 [4.81–4.89] | 0.29 [0.29–0.29] | 16.7× |
| static_css c=1 req/s | 35,582 [35,576–35,615] | 75,194 [74,110–75,342] | 2.1× |
| static_css c=1 p50 ms | 0.03 [0.03–0.03] | 0.01 [0.01–0.01] | 2.2× |
| static_css c=1 p99 ms | 0.07 [0.07–0.07] | 0.02 [0.02–0.02] | 4.1× |
| static_css c=16 req/s | 137,312 [136,450–141,061] | 434,265 [428,719–435,875] | 3.2× |
| static_css c=16 p50 ms | 0.08 [0.08–0.08] | 0.04 [0.03–0.04] | 2.3× |
| static_css c=16 p99 ms | 0.62 [0.59–0.63] | 0.07 [0.06–0.07] | 9.4× |
| static_css c=64 req/s | 111,021 [110,969–111,113] | 462,314 [459,128–466,583] | 4.2× |
| static_css c=64 p50 ms | 0.28 [0.28–0.29] | 0.13 [0.13–0.13] | 2.2× |
| static_css c=64 p99 ms | 3.40 [3.39–3.42] | 0.28 [0.28–0.29] | 12.0× |
| up c=1 req/s | 1,844 [1,834–1,856] | 23,466 [23,351–23,595] | 12.7× |
| up c=1 p50 ms | 0.51 [0.51–0.52] | 0.04 [0.04–0.04] | 12.4× |
| up c=1 p99 ms | 1.06 [1.06–1.10] | 0.06 [0.05–0.06] | 18.9× |
| up c=16 req/s | 4,088 [4,065–4,109] | 111,291 [110,907–111,342] | 27.2× |
| up c=16 p50 ms | 3.77 [3.77–3.83] | 0.14 [0.14–0.14] | 26.0× |
| up c=16 p99 ms | 7.84 [7.78–7.85] | 0.25 [0.25–0.25] | 31.5× |
| up c=64 req/s | 4,032 [4,018–4,068] | 112,065 [112,002–112,831] | 27.8× |
| up c=64 p50 ms | 15.7 [15.6–15.8] | 0.56 [0.55–0.56] | 28.3× |
| up c=64 p99 ms | 24.5 [24.2–24.6] | 1.15 [1.15–1.15] | 21.3× |
| post_message c=1 req/s | 141 [127–146] | 2,158 [2,155–2,178] | 15.3× |
| post_message c=1 p50 ms | 6.46 [6.40–6.55] | 0.43 [0.42–0.43] | 15.1× |
| post_message c=1 p99 ms | 17.7 [13.4–33.3] | 1.77 [1.75–1.80] | 10.0× |
| post_message c=16 req/s | 273 [257–273] | 5,269 [5,246–5,376] | 19.3× |
| post_message c=16 p50 ms | 53.4 [52.6–59.9] | 2.80 [2.74–2.80] | 19.1× |
| post_message c=16 p99 ms | 152 [144–172] | 7.40 [7.38–7.42] | 20.5× |
| post_message c=64 req/s | 267 [264–268] | 5,301 [5,296–5,348] | 19.8× |
| post_message c=64 p50 ms | 230 [229–238] | 11.8 [11.7–11.8] | 19.5× |
| post_message c=64 p99 ms | 381 [364–389] | 19.4 [19.1–19.5] | 19.6× |

### HTTP errors / non-2xx-3xx (first rep, per app)

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
- reference: none
- rust: none

### Action Cable fan-out (one room; chatter.js subscriptions per client)

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
| 100 clients: subscribed | 100 [100–100] | 100 [100–100] | 1.0× |
| 100 clients: connect+subscribe all (s) | 0.28 [0.27–0.32] | 0.06 [0.06–0.06] | 4.7× |
| 100 clients: paced post→one client p50 ms | 14.7 [14.5–14.8] | 2.20 [2.14–2.21] | 6.7× |
| 100 clients: paced post→all clients p50 ms | 19.9 [19.8–20.1] | 2.40 [2.38–2.42] | 8.3× |
| 100 clients: paced post→all clients p99 ms | 73.2 [73.0–73.2] | 4.28 [2.92–4.70] | 17.1× |
| 100 clients: max sustained msgs/s (delivered to all) | 79.3 [79.1–80.7] | 2,449 [2,441–2,461] | 30.9× |
| 100 clients: deliveries/s (client×message) | 7,934 [7,914–8,068] | 244,900 [244,112–246,075] | 30.9× |
| 100 clients: saturated post→all p50 ms | 45.3 [43.9–46.4] | 1.36 [1.36–1.37] | 33.2× |
| 100 clients: saturated POST p50 ms | 43.2 [42.8–44.3] | 1.50 [1.49–1.50] | 28.8× |
| 1000 clients: subscribed | 1,000 [1,000–1,000] | 1,000 [1,000–1,000] | 1.0× |
| 1000 clients: connect+subscribe all (s) | 1.42 [1.40–1.44] | 0.16 [0.14–1.18] | 8.9× |
| 1000 clients: paced post→one client p50 ms | 46.9 [46.5–47.2] | 4.53 [4.51–4.86] | 10.3× |
| 1000 clients: paced post→all clients p50 ms | 101 [95–102] | 7.12 [6.87–7.28] | 14.2× |
| 1000 clients: paced post→all clients p99 ms | 189 [189–198] | 10.2 [9.4–11.7] | 18.5× |
| 1000 clients: max sustained msgs/s (delivered to all) | 11.1 [11.0–11.4] | 373 [369–374] | 33.6× |
| 1000 clients: deliveries/s (client×message) | 11,127 [11,047–11,383] | 373,319 [369,411–374,046] | 33.6× |
| 1000 clients: saturated post→all p50 ms | 2,146 [1,929–2,464] | 12.6 [12.5–12.9] | 171.0× |
| 1000 clients: saturated POST p50 ms | 139 [80–203] | 10.5 [10.4–10.6] | 13.3× |
| 5000 clients: subscribed | 5,000 [5,000–5,000] | 5,000 [5,000–5,000] | 1.0× |
| 5000 clients: connect+subscribe all (s) | 8.40 [8.15–8.54] | 1.55 [1.43–1.62] | 5.4× |
| 5000 clients: paced post→one client p50 ms | 253 [218–364] | 13.0 [12.6–13.2] | 19.4× |
| 5000 clients: paced post→all clients p50 ms | 741 [550–3,344] | 23.1 [22.6–23.7] | 32.1× |
| 5000 clients: paced post→all clients p99 ms | 932 [906–4,731] | 31.9 [31.4–35.9] | 29.2× |
| 5000 clients: max sustained msgs/s (delivered to all) | 2.20 [2.10–2.30] | 75.0 [74.7–75.7] | 34.1× |
| 5000 clients: deliveries/s (client×message) | 11,239 [10,624–11,446] | 375,151 [373,647–378,688] | 33.4× |
| 5000 clients: saturated post→all p50 ms | 4,653 [2,529–4,960] | 54.4 [54.3–54.9] | 85.5× |
| 5000 clients: saturated POST p50 ms | 561 [454–902] | 53.6 [53.5–54.0] | 10.5× |
| 10000 clients: subscribed | 10,000 [9,867–10,000] | 10,000 [10,000–10,000] | 1.0× |
| 10000 clients: connect+subscribe all (s) | 29.2 [16.9–29.3] | 2.26 [1.97–2.28] | 12.9× |
| 10000 clients: paced post→one client p50 ms | 616 [549–690] | 23.1 [23.1–23.2] | 26.6× |
| 10000 clients: paced post→all clients p50 ms | 1,293 [1,186–1,426] | 42.3 [41.4–42.9] | 30.6× |
| 10000 clients: paced post→all clients p99 ms | 1,744 [1,401–3,056] | 58.9 [52.7–64.0] | 29.6× |
| 10000 clients: max sustained msgs/s (delivered to all) | 0.90 [0.90–0.90] | 38.0 [37.8–38.6] | 42.2× |
| 10000 clients: deliveries/s (client×message) | 8,949 [8,653–9,229] | 379,608 [378,434–385,901] | 42.4× |
| 10000 clients: saturated post→all p50 ms | 4,895 [4,280–5,685] | 107 [106–111] | 45.6× |
| 10000 clients: saturated POST p50 ms | 3,498 [2,574–4,329] | 109 [109–110] | 32.2× |

### Upload + thumbnail (black_hole.jpg, 505 KB)

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
| POST with attachment (ms) | 104 [68–174] | 28.7 [27.9–28.8] | 3.6× |
| then GET thumb → 200 (ms) | 0.40 [0.40–0.40] | 0.30 [0.30–0.30] | 1.3× |
| POST → thumbnail served (ms) | 104 [68–175] | 29.0 [28.1–29.5] | 3.6× |

### Memory during cable fan-out, by process (MB, peak within the phase)

App process: Rails' Puma master and workers (Action Cable runs in them), or Rust's one campfire
process (its front server included). Pss counts pages shared between forked workers once;
RssAnon counts them in every process.

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
| 100 clients, all subscribed, idle: app process Pss | 553 [550–560] | 216 [213–217] | 2.6× |
| 100 clients, all subscribed, idle: app process RssAnon | 699 [696–708] | 181 [178–183] | 3.9× |
| 100 clients, all subscribed, idle: app + Redis + Thruster Pss | 593 [590–601] | 216 [213–217] | 2.7× |
| 100 clients, all subscribed, idle: whole container Pss | 886 [870–889] | 216 [213–217] | 4.1× |
| 100 clients, saturated fan-out: app process Pss | 663 [655–671] | 216 [216–221] | 3.1× |
| 100 clients, saturated fan-out: app process RssAnon | 807 [800–818] | 182 [181–187] | 4.4× |
| 100 clients, saturated fan-out: app + Redis + Thruster Pss | 714 [706–722] | 216 [216–221] | 3.3× |
| 100 clients, saturated fan-out: whole container Pss | 1,007 [993–1,009] | 216 [216–221] | 4.7× |
| 1000 clients, all subscribed, idle: app process Pss | 642 [642–645] | 234 [230–237] | 2.7× |
| 1000 clients, all subscribed, idle: app process RssAnon | 785 [783–791] | 198 [194–202] | 4.0× |
| 1000 clients, all subscribed, idle: app + Redis + Thruster Pss | 742 [740–744] | 234 [230–237] | 3.2× |
| 1000 clients, all subscribed, idle: whole container Pss | 1,031 [1,019–1,043] | 234 [230–237] | 4.4× |
| 1000 clients, saturated fan-out: app process Pss | 930 [914–971] | 272 [268–275] | 3.4× |
| 1000 clients, saturated fan-out: app process RssAnon | 1,072 [1,059–1,113] | 236 [232–239] | 4.5× |
| 1000 clients, saturated fan-out: app + Redis + Thruster Pss | 1,038 [1,021–1,080] | 272 [268–275] | 3.8× |
| 1000 clients, saturated fan-out: whole container Pss | 1,316 [1,308–1,380] | 272 [268–275] | 4.8× |
| 5000 clients, all subscribed, idle: app process Pss | 986 [985–1,015] | 382 [380–385] | 2.6× |
| 5000 clients, all subscribed, idle: app process RssAnon | 1,130 [1,125–1,156] | 347 [345–349] | 3.3× |
| 5000 clients, all subscribed, idle: app + Redis + Thruster Pss | 1,362 [1,361–1,393] | 382 [380–385] | 3.6× |
| 5000 clients, all subscribed, idle: whole container Pss | 1,648 [1,637–1,692] | 382 [380–385] | 4.3× |
| 5000 clients, saturated fan-out: app process Pss | 1,855 [1,678–1,863] | 552 [550–555] | 3.4× |
| 5000 clients, saturated fan-out: app process RssAnon | 1,994 [1,821–2,004] | 516 [514–519] | 3.9× |
| 5000 clients, saturated fan-out: app + Redis + Thruster Pss | 2,266 [2,090–2,278] | 552 [550–555] | 4.1× |
| 5000 clients, saturated fan-out: whole container Pss | 2,539 [2,372–2,573] | 552 [550–555] | 4.6× |
| 10000 clients, all subscribed, idle: app process Pss | 1,479 [1,474–1,502] | 582 [577–594] | 2.5× |
| 10000 clients, all subscribed, idle: app process RssAnon | 1,616 [1,614–1,645] | 546 [541–559] | 3.0× |
| 10000 clients, all subscribed, idle: app + Redis + Thruster Pss | 2,399 [2,373–2,409] | 582 [577–594] | 4.1× |
| 10000 clients, all subscribed, idle: whole container Pss | 2,680 [2,653–2,694] | 582 [577–594] | 4.6× |
| 10000 clients, saturated fan-out: app process Pss | 2,083 [1,932–2,089] | 876 [816–952] | 2.4× |
| 10000 clients, saturated fan-out: app process RssAnon | 2,219 [2,074–2,229] | 840 [780–916] | 2.6× |
| 10000 clients, saturated fan-out: app + Redis + Thruster Pss | 3,057 [2,856–3,059] | 876 [816–952] | 3.5× |
| 10000 clients, saturated fan-out: whole container Pss | 3,328 [3,136–3,352] | 876 [816–952] | 3.8× |
