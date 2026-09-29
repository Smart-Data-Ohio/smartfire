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

Reps: reference 5, rust 5, rust-identity 5. Cells: median [min–max].

### Startup and memory

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
| cold start: docker run → /up 200 (ms) | 2,457 [2,432–2,473] | 232 [222–234] | 10.6× |
| idle memory.current (MB) | 309 [305–314] | 49.0 [47.0–50.0] | 6.3× |
| idle anon (MB) | 287 [283–291] | 46.0 [44.0–46.0] | 6.2× |
| peak memory.current under load (MB) | 1,468 [1,383–1,552] | 1,867 [1,838–1,876] | 0.8× |
| peak anon under load (MB) | 1,417 [1,326–1,500] | 1,514 [1,510–1,520] | 0.9× |

### HTTP (signed in as david; keep-alive; c = concurrent connections)

| Metric | Rails | Rust | Rust adv. | Rust, no gzip | vs Rust gzip |
|---|---|---|---|---|---|
| room_show c=1 req/s | 94.5 [87.4–95.8] | 636 [622–639] | 6.7× | 2,090 [2,081–2,172] | 3.29× |
| room_show c=1 p50 ms | 10.2 [9.8–10.5] | 1.55 [1.53–1.58] | 6.6× | 0.47 [0.45–0.47] | 3.32× |
| room_show c=1 p99 ms | 12.4 [11.8–31.4] | 2.24 [2.22–2.25] | 5.5× | 0.61 [0.55–0.63] | 3.70× |
| room_show c=16 req/s | 234 [232–248] | 2,577 [2,564–2,598] | 11.0× | 8,973 [8,927–9,020] | 3.48× |
| room_show c=16 p50 ms | 62.3 [60.7–64.8] | 6.25 [6.21–6.26] | 10.0× | 1.76 [1.75–1.77] | 3.55× |
| room_show c=16 p99 ms | 170 [156–174] | 10.4 [10.3–10.6] | 16.4× | 3.12 [3.07–3.17] | 3.34× |
| room_show c=64 req/s | 206 [197–233] | 2,564 [2,547–2,578] | 12.4× | 8,621 [8,566–8,652] | 3.36× |
| room_show c=64 p50 ms | 311 [270–317] | 24.7 [24.5–24.8] | 12.6× | 7.35 [7.33–7.40] | 3.36× |
| room_show c=64 p99 ms | 391 [348–420] | 44.1 [42.9–44.8] | 8.9× | 12.5 [12.3–12.7] | 3.53× |
| messages_page c=1 req/s | 176 [169–177] | 1,014 [1,003–1,018] | 5.8× | 3,938 [3,866–4,336] | 3.88× |
| messages_page c=1 p50 ms | 5.51 [5.42–5.58] | 0.98 [0.97–0.99] | 5.6× | 0.24 [0.22–0.25] | 3.98× |
| messages_page c=1 p99 ms | 7.34 [7.20–15.27] | 1.15 [1.11–1.39] | 6.4× | 0.32 [0.31–0.35] | 3.56× |
| messages_page c=16 req/s | 444 [443–456] | 3,939 [3,921–3,969] | 8.9× | 16,500 [16,358–16,690] | 4.19× |
| messages_page c=16 p50 ms | 33.4 [28.3–35.6] | 4.05 [4.03–4.07] | 8.2× | 0.92 [0.91–0.93] | 4.41× |
| messages_page c=16 p99 ms | 90.3 [74.2–104.3] | 6.73 [6.66–6.80] | 13.4× | 1.83 [1.79–1.90] | 3.67× |
| messages_page c=64 req/s | 431 [420–442] | 3,925 [3,889–3,938] | 9.1× | 15,530 [15,350–15,573] | 3.96× |
| messages_page c=64 p50 ms | 151 [144–156] | 16.2 [16.1–16.3] | 9.3× | 4.08 [4.03–4.13] | 3.96× |
| messages_page c=64 p99 ms | 226 [200–244] | 27.6 [26.7–28.1] | 8.2× | 6.49 [6.41–6.67] | 4.25× |
| sidebar c=1 req/s | 222 [222–223] | 2,980 [2,764–3,038] | 13.4× | 5,554 [5,453–5,620] | 1.86× |
| sidebar c=1 p50 ms | 4.36 [4.29–4.39] | 0.33 [0.32–0.36] | 13.2× | 0.17 [0.17–0.18] | 1.88× |
| sidebar c=1 p99 ms | 6.08 [6.01–6.19] | 0.43 [0.42–0.44] | 14.3× | 0.24 [0.24–0.24] | 1.77× |
| sidebar c=16 req/s | 581 [569–603] | 11,066 [11,008–11,292] | 19.0× | 19,881 [19,609–20,024] | 1.80× |
| sidebar c=16 p50 ms | 26.7 [23.9–27.4] | 1.42 [1.39–1.42] | 18.9× | 0.76 [0.76–0.77] | 1.86× |
| sidebar c=16 p99 ms | 52.9 [47.8–58.3] | 2.58 [2.53–2.62] | 20.5× | 1.59 [1.57–1.69] | 1.62× |
| sidebar c=64 req/s | 541 [527–593] | 11,265 [11,249–11,397] | 20.8× | 20,833 [20,576–20,908] | 1.85× |
| sidebar c=64 p50 ms | 114 [104–116] | 5.63 [5.57–5.64] | 20.3× | 3.02 [2.97–3.07] | 1.87× |
| sidebar c=64 p99 ms | 183 [150–190] | 9.48 [9.32–9.63] | 19.3× | 5.03 [4.97–5.35] | 1.89× |
| search c=1 req/s | 176 [173–177] | 1,559 [1,517–1,569] | 8.9× | 4,019 [3,919–4,088] | 2.58× |
| search c=1 p50 ms | 5.53 [5.49–5.64] | 0.63 [0.63–0.66] | 8.8× | 0.24 [0.24–0.25] | 2.60× |
| search c=1 p99 ms | 7.35 [7.24–7.38] | 0.74 [0.73–0.78] | 9.9× | 0.32 [0.30–0.33] | 2.36× |
| search c=16 req/s | 420 [414–427] | 5,933 [5,865–5,998] | 14.1× | 15,254 [14,894–15,339] | 2.57× |
| search c=16 p50 ms | 38.5 [36.2–41.8] | 2.67 [2.63–2.69] | 14.4× | 1.01 [1.00–1.02] | 2.65× |
| search c=16 p99 ms | 71.5 [60.8–79.2] | 4.51 [4.47–4.58] | 15.8× | 1.95 [1.94–2.12] | 2.32× |
| search c=64 req/s | 391 [387–418] | 5,971 [5,894–6,026] | 15.3× | 15,398 [15,195–15,642] | 2.58× |
| search c=64 p50 ms | 154 [152–161] | 10.6 [10.6–10.8] | 14.5× | 4.12 [4.07–4.13] | 2.58× |
| search c=64 p99 ms | 228 [177–243] | 17.4 [17.1–17.5] | 13.2× | 6.32 [6.19–6.79] | 2.75× |
| avatar c=1 req/s | 29,983 [29,894–30,417] | 71,071 [70,380–71,463] | 2.4× | 71,539 [70,525–71,775] | 1.01× |
| avatar c=1 p50 ms | 0.03 [0.03–0.03] | 0.01 [0.01–0.01] | 2.4× | 0.01 [0.01–0.01] | 1.00× |
| avatar c=1 p99 ms | 0.09 [0.09–0.09] | 0.02 [0.02–0.02] | 5.3× | 0.02 [0.02–0.02] | 0.94× |
| avatar c=16 req/s | 98,244 [97,362–99,830] | 413,217 [393,490–426,580] | 4.2× | 416,941 [405,720–436,162] | 1.01× |
| avatar c=16 p50 ms | 0.10 [0.10–0.10] | 0.04 [0.04–0.04] | 2.9× | 0.04 [0.04–0.04] | 0.97× |
| avatar c=16 p99 ms | 0.88 [0.88–0.90] | 0.09 [0.09–0.10] | 9.5× | 0.09 [0.09–0.09] | 1.01× |
| avatar c=64 req/s | 78,988 [78,544–79,496] | 429,852 [425,342–435,304] | 5.4× | 437,124 [408,459–441,521] | 1.02× |
| avatar c=64 p50 ms | 0.33 [0.32–0.33] | 0.14 [0.14–0.14] | 2.4× | 0.14 [0.14–0.14] | 1.01× |
| avatar c=64 p99 ms | 4.85 [4.79–4.87] | 0.39 [0.37–0.40] | 12.6× | 0.38 [0.37–0.45] | 1.03× |
| static_css c=1 req/s | 37,429 [37,286–38,051] | 76,446 [75,726–78,362] | 2.0× | 75,394 [74,468–76,742] | 0.99× |
| static_css c=1 p50 ms | 0.03 [0.02–0.03] | 0.01 [0.01–0.01] | 2.1× | 0.01 [0.01–0.01] | 1.00× |
| static_css c=1 p99 ms | 0.06 [0.06–0.06] | 0.02 [0.02–0.02] | 3.9× | 0.02 [0.02–0.02] | 1.00× |
| static_css c=16 req/s | 135,193 [134,176–136,287] | 426,374 [417,727–432,479] | 3.2× | 424,559 [415,862–439,195] | 1.00× |
| static_css c=16 p50 ms | 0.08 [0.08–0.08] | 0.03 [0.03–0.03] | 2.4× | 0.03 [0.03–0.04] | 1.00× |
| static_css c=16 p99 ms | 0.65 [0.63–0.66] | 0.08 [0.08–0.09] | 7.8× | 0.08 [0.08–0.08] | 1.02× |
| static_css c=64 req/s | 111,078 [110,376–114,298] | 459,212 [441,194–465,595] | 4.1× | 452,850 [442,689–468,730] | 0.99× |
| static_css c=64 p50 ms | 0.29 [0.28–0.30] | 0.13 [0.13–0.13] | 2.2× | 0.13 [0.13–0.13] | 0.99× |
| static_css c=64 p99 ms | 3.39 [3.27–3.43] | 0.33 [0.32–0.34] | 10.4× | 0.33 [0.31–0.34] | 0.98× |
| up c=1 req/s | 1,927 [1,924–1,935] | 23,891 [23,640–23,956] | 12.4× | 36,386 [36,050–36,551] | 1.52× |
| up c=1 p50 ms | 0.49 [0.49–0.49] | 0.04 [0.04–0.04] | 12.0× | 0.03 [0.03–0.03] | 1.58× |
| up c=1 p99 ms | 0.93 [0.86–0.98] | 0.05 [0.05–0.05] | 19.0× | 0.03 [0.03–0.03] | 1.48× |
| up c=16 req/s | 4,289 [4,247–4,329] | 107,715 [106,810–110,551] | 25.1× | 163,589 [160,950–165,620] | 1.52× |
| up c=16 p50 ms | 3.63 [3.59–3.66] | 0.15 [0.15–0.15] | 24.2× | 0.10 [0.10–0.10] | 1.53× |
| up c=16 p99 ms | 7.33 [7.21–7.41] | 0.26 [0.25–0.26] | 28.5× | 0.17 [0.17–0.17] | 1.54× |
| up c=64 req/s | 4,231 [4,192–4,285] | 109,222 [107,001–111,856] | 25.8× | 165,218 [162,440–166,270] | 1.51× |
| up c=64 p50 ms | 15.1 [14.7–15.2] | 0.57 [0.55–0.58] | 26.4× | 0.38 [0.37–0.39] | 1.52× |
| up c=64 p99 ms | 23.1 [22.4–23.4] | 1.18 [1.16–1.21] | 19.5× | 0.78 [0.78–0.79] | 1.52× |
| post_message c=1 req/s | 151 [147–154] | 2,202 [2,183–2,213] | 14.6× | 2,559 [2,539–2,574] | 1.16× |
| post_message c=1 p50 ms | 6.13 [6.04–6.16] | 0.41 [0.41–0.42] | 14.9× | 0.35 [0.34–0.35] | 1.18× |
| post_message c=1 p99 ms | 18.4 [18.1–23.4] | 1.51 [1.39–1.56] | 12.1× | 1.52 [1.43–1.60] | 0.99× |
| post_message c=16 req/s | 273 [262–282] | 5,272 [5,260–5,304] | 19.3× | 5,515 [5,484–5,620] | 1.05× |
| post_message c=16 p50 ms | 53.6 [45.2–55.9] | 2.62 [2.60–2.63] | 20.5× | 2.46 [2.42–2.48] | 1.07× |
| post_message c=16 p99 ms | 159 [149–167] | 14.6 [14.6–14.8] | 10.9× | 14.5 [14.4–14.7] | 1.00× |
| post_message c=64 req/s | 272 [264–279] | 5,355 [4,188–5,498] | 19.7× | 5,691 [5,620–5,845] | 1.06× |
| post_message c=64 p50 ms | 226 [219–243] | 11.0 [10.7–11.2] | 20.6× | 10.3 [10.0–10.4] | 1.07× |
| post_message c=64 p99 ms | 370 [332–456] | 24.8 [24.4–90.2] | 14.9× | 24.5 [24.2–24.9] | 1.01× |

### HTTP errors / non-2xx-3xx (first rep, per app)

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
- reference: none
- rust: none
- rust-identity: none

### Action Cable fan-out (one room; chatter.js subscriptions per client)

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
| 100 clients: subscribed | 100 [100–100] | 100 [100–100] | 1.0× |
| 100 clients: connect+subscribe all (s) | 0.22 [0.22–0.27] | 0.06 [0.06–0.11] | 3.7× |
| 100 clients: paced post→one client p50 ms | 13.1 [12.6–13.5] | 2.21 [2.19–2.29] | 5.9× |
| 100 clients: paced post→all clients p50 ms | 17.3 [16.9–18.5] | 2.46 [2.44–2.51] | 7.0× |
| 100 clients: paced post→all clients p99 ms | 62.4 [27.1–64.2] | 5.86 [2.81–6.27] | 10.7× |
| 100 clients: max sustained msgs/s (delivered to all) | 92.9 [86.4–96.5] | 2,447 [2,428–2,478] | 26.3× |
| 100 clients: deliveries/s (client×message) | 9,286 [8,644–9,648] | 244,719 [242,799–247,746] | 26.4× |
| 100 clients: saturated post→all p50 ms | 40.7 [39.1–41.0] | 1.32 [1.31–1.33] | 30.8× |
| 100 clients: saturated POST p50 ms | 34.0 [29.6–39.0] | 1.45 [1.44–1.47] | 23.4× |
| 500 clients: subscribed | 500 [500–500] | 500 [500–500] | 1.0× |
| 500 clients: connect+subscribe all (s) | 0.69 [0.68–0.75] | 0.13 [0.11–0.14] | 5.3× |
| 500 clients: paced post→one client p50 ms | 23.2 [23.0–24.7] | 3.25 [3.19–3.34] | 7.1× |
| 500 clients: paced post→all clients p50 ms | 43.8 [43.1–44.4] | 5.11 [4.77–5.17] | 8.6× |
| 500 clients: paced post→all clients p99 ms | 112 [66–127] | 8.53 [6.71–21.52] | 13.1× |
| 500 clients: max sustained msgs/s (delivered to all) | 27.0 [26.2–27.5] | 604 [585–626] | 22.4× |
| 500 clients: deliveries/s (client×message) | 13,523 [13,106–13,727] | 301,899 [292,379–313,220] | 22.3× |
| 500 clients: saturated post→all p50 ms | 124 [114–129] | 8.75 [8.70–9.03] | 14.2× |
| 500 clients: saturated POST p50 ms | 111 [107–142] | 6.25 [6.14–6.32] | 17.7× |
| 1000 clients: subscribed | 1,000 [1,000–1,000] | 1,000 [1,000–1,000] | 1.0× |
| 1000 clients: connect+subscribe all (s) | 1.32 [1.32–1.39] | 0.13 [0.12–1.15] | 10.2× |
| 1000 clients: paced post→one client p50 ms | 38.0 [37.2–40.9] | 4.86 [4.75–4.91] | 7.8× |
| 1000 clients: paced post→all clients p50 ms | 78.8 [75.7–85.1] | 8.97 [8.43–9.02] | 8.8× |
| 1000 clients: paced post→all clients p99 ms | 159 [105–162] | 12.3 [11.4–14.6] | 12.9× |
| 1000 clients: max sustained msgs/s (delivered to all) | 13.6 [12.7–15.0] | 304 [298–310] | 22.4× |
| 1000 clients: deliveries/s (client×message) | 13,646 [12,712–14,955] | 304,197 [297,988–310,255] | 22.3× |
| 1000 clients: saturated post→all p50 ms | 266 [214–2,179] | 16.7 [16.3–17.1] | 15.9× |
| 1000 clients: saturated POST p50 ms | 184 [120–245] | 13.0 [12.7–13.3] | 14.2× |

### Upload + thumbnail (black_hole.jpg, 505 KB)

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
| POST with attachment (ms) | 55.0 [49.7–55.2] | 27.5 [27.4–28.4] | 2.0× |
| then GET thumb → 200 (ms) | 0.40 [0.40–0.50] | 0.30 [0.20–0.30] | 1.3× |
| POST → thumbnail served (ms) | 55.4 [50.1–55.5] | 27.8 [27.6–28.7] | 2.0× |

### Memory during cable fan-out, by process (MB, peak within the phase)

App process: Rails' Puma master and workers (Action Cable runs in them), or Rust's one campfire
process (its front server included). Pss counts pages shared between forked workers once;
RssAnon counts them in every process.

| Metric | Rails | Rust | Rust adv. |
|---|---|---|---|
| 100 clients, all subscribed, idle: app process Pss | 560 [545–579] | 1,365 [1,356–1,369] | 0.4× |
| 100 clients, all subscribed, idle: app process RssAnon | 703 [693–723] | 1,330 [1,322–1,335] | 0.5× |
| 100 clients, all subscribed, idle: app + Redis + Thruster Pss | 600 [585–619] | 1,365 [1,356–1,369] | 0.4× |
| 100 clients, all subscribed, idle: whole container Pss | 883 [870–898] | 1,365 [1,356–1,369] | 0.6× |
| 100 clients, saturated fan-out: app process Pss | 699 [670–714] | 1,368 [1,363–1,371] | 0.5× |
| 100 clients, saturated fan-out: app process RssAnon | 842 [813–859] | 1,334 [1,329–1,336] | 0.6× |
| 100 clients, saturated fan-out: app + Redis + Thruster Pss | 751 [722–766] | 1,368 [1,363–1,371] | 0.5× |
| 100 clients, saturated fan-out: whole container Pss | 1,028 [1,004–1,066] | 1,368 [1,363–1,371] | 0.8× |
| 500 clients, all subscribed, idle: app process Pss | 635 [632–652] | 1,353 [1,352–1,357] | 0.5× |
| 500 clients, all subscribed, idle: app process RssAnon | 778 [774–796] | 1,318 [1,316–1,322] | 0.6× |
| 500 clients, all subscribed, idle: app + Redis + Thruster Pss | 704 [701–722] | 1,353 [1,352–1,357] | 0.5× |
| 500 clients, all subscribed, idle: whole container Pss | 997 [980–1,021] | 1,353 [1,352–1,357] | 0.7× |
| 500 clients, saturated fan-out: app process Pss | 829 [782–839] | 1,368 [1,366–1,371] | 0.6× |
| 500 clients, saturated fan-out: app process RssAnon | 973 [923–981] | 1,333 [1,331–1,335] | 0.7× |
| 500 clients, saturated fan-out: app + Redis + Thruster Pss | 911 [865–919] | 1,368 [1,366–1,371] | 0.7× |
| 500 clients, saturated fan-out: whole container Pss | 1,193 [1,161–1,210] | 1,368 [1,366–1,371] | 0.9× |
| 1000 clients, all subscribed, idle: app process Pss | 708 [681–719] | 1,372 [1,369–1,375] | 0.5× |
| 1000 clients, all subscribed, idle: app process RssAnon | 850 [821–861] | 1,336 [1,334–1,339] | 0.6× |
| 1000 clients, all subscribed, idle: app + Redis + Thruster Pss | 823 [798–849] | 1,372 [1,369–1,375] | 0.6× |
| 1000 clients, all subscribed, idle: whole container Pss | 1,110 [1,099–1,129] | 1,372 [1,369–1,375] | 0.8× |
| 1000 clients, saturated fan-out: app process Pss | 981 [907–1,039] | 1,408 [1,403–1,409] | 0.7× |
| 1000 clients, saturated fan-out: app process RssAnon | 1,124 [1,047–1,179] | 1,372 [1,368–1,373] | 0.8× |
| 1000 clients, saturated fan-out: app + Redis + Thruster Pss | 1,092 [1,051–1,176] | 1,408 [1,403–1,409] | 0.8× |
| 1000 clients, saturated fan-out: whole container Pss | 1,388 [1,325–1,471] | 1,408 [1,403–1,409] | 1.0× |
