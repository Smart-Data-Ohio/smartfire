```
date: 2026-09-27T00:23:16+0200
host: 7.2.5-4-omarchy, 32 threads
server cpus: 4-7; loadgen cpus: 0-3
image: campfire-rust:perf-a654b08 sha256:82a2c82b7f7ff1d371c701e3d9f81b894f76c1120231f421a817d183748a54c3
native-base: ./target/perf-base/release/campfire preload=
native-expws: ./target/perf-exp-ws/release/campfire preload=
native-expzlib: ./target/perf-exp-zlib/release/campfire preload=
env: WEB_CONCURRENCY=3 RAILS_MAX_THREADS=5 JOB_CONCURRENCY=3
routes: room_show,messages_page,sidebar; concs: 1,16; secs: 5; cable: 1000; app env:
```

Reps per configuration: native-base 3, native-expws 3, native-expzlib 3. Cells: median [min–max]; (×) is the gain over native-base (>1 is better).
Host load (1-min loadavg at start of each run): native-base 34.17/33.16/29.06, native-expws 42.01/30.78/29.90, native-expzlib 39.81/30.66/33.89

### HTTP room_show

| Metric | native-base | native-expws | native-expzlib |
|---|---|---|---|
| c=1 req/s | 133 [125–196] | 146 [127–151] (1.09×) | 205 [196–223] (1.54×) |
| c=1 p50 ms | 7.38 [4.94–7.74] | 6.74 [6.52–7.66] (1.09×) | 4.37 [4.22–4.90] (1.69×) |
| c=1 p99 ms | 11.2 [7.08–13.3] | 10.4 [9.58–12.4] (1.08×) | 8.94 [7.81–11.0] (1.26×) |
| c=16 req/s | 604 [553–757] | 616 [604–637] (1.02×) | 883 [812–962] (1.46×) |
| c=16 p50 ms | 25.8 [20.7–28.3] | 25.6 [24.4–26.0] (1.01×) | 17.7 [16.2–18.8] (1.46×) |
| c=16 p99 ms | 46.2 [37.2–49.1] | 45.2 [43.0–45.2] (1.02×) | 30.8 [28.8–40.9] (1.50×) |
| c=1 campfire CPU ms/req | 7.01 [4.96–7.26] | 6.47 [6.26–7.17] (1.08×) | 4.44 [4.27–4.74] (1.58×) |
| c=1 thrust CPU ms/req | – | – | – |
| c=1 docker-proxy CPU ms/req | – | – | – |
| c=1 loadgen cores busy | 0.020 [0.020–0.020] | 0.020 [0.020–0.030] | 0.030 [0.020–0.030] |
| c=1 campfire cores busy | 0.93 [0.91–0.97] | 0.94 [0.91–0.95] | 0.93 [0.91–0.95] |
| c=16 campfire CPU ms/req | 6.21 [4.96–6.52] | 6.10 [5.79–6.23] (1.02×) | 4.13 [3.79–4.29] (1.50×) |
| c=16 thrust CPU ms/req | – | – | – |
| c=16 docker-proxy CPU ms/req | – | – | – |
| c=16 loadgen cores busy | 0.050 [0.040–0.060] | 0.050 [0.050–0.060] | 0.060 [0.060–0.070] |
| c=16 campfire cores busy | 3.75 [3.60–3.75] | 3.75 [3.69–3.76] | 3.64 [3.48–3.65] |
| avg response bytes | 44,268 [44,265–44,272] | 44,269 [44,248–44,283] | 45,620 [45,538–45,730] |

### HTTP messages_page

| Metric | native-base | native-expws | native-expzlib |
|---|---|---|---|
| c=1 req/s | 174 [127–254] | 162 [160–170] (0.93×) | 279 [161–315] (1.60×) |
| c=1 p50 ms | 5.46 [3.78–7.58] | 5.94 [5.74–6.08] (0.92×) | 3.24 [2.88–6.00] (1.69×) |
| c=1 p99 ms | 10.9 [6.72–15.3] | 9.84 [9.05–11.2] (1.11×) | 8.94 [6.59–12.0] (1.22×) |
| c=16 req/s | 802 [689–984] | 742 [741–752] (0.93×) | 1,058 [1,019–1,337] (1.32×) |
| c=16 p50 ms | 19.4 [15.8–22.8] | 21.0 [20.9–21.2] (0.93×) | 14.6 [11.6–15.3] (1.33×) |
| c=16 p99 ms | 36.1 [28.1–39.7] | 36.1 [35.5–39.7] (1.00×) | 27.1 [20.2–27.2] (1.33×) |
| c=1 campfire CPU ms/req | 5.22 [3.80–6.45] | 5.59 [5.44–5.69] (0.93×) | 3.34 [2.99–4.96] (1.56×) |
| c=1 thrust CPU ms/req | – | – | – |
| c=1 docker-proxy CPU ms/req | – | – | – |
| c=1 loadgen cores busy | 0.020 [0.020–0.020] | 0.020 [0.020–0.020] | 0.020 [0.020–0.030] |
| c=1 campfire cores busy | 0.91 [0.82–0.97] | 0.91 [0.90–0.92] | 0.93 [0.80–0.94] |
| c=16 campfire CPU ms/req | 4.57 [3.71–5.11] | 4.86 [4.82–4.91] (0.94×) | 3.35 [2.67–3.48] (1.36×) |
| c=16 thrust CPU ms/req | – | – | – |
| c=16 docker-proxy CPU ms/req | – | – | – |
| c=16 loadgen cores busy | 0.060 [0.050–0.060] | 0.060 [0.060–0.060] | 0.070 [0.050–0.070] |
| c=16 campfire cores busy | 3.65 [3.52–3.67] | 3.60 [3.58–3.68] | 3.55 [3.54–3.56] |
| avg response bytes | 34,650 [34,643–34,653] | 34,651 [34,621–34,687] | 36,012 [35,962–36,078] |

### HTTP sidebar

| Metric | native-base | native-expws | native-expzlib |
|---|---|---|---|
| c=1 req/s | 656 [395–1,154] | 728 [592–772] (1.11×) | 930 [789–1,147] (1.42×) |
| c=1 p50 ms | 1.32 [0.72–2.22] | 1.23 [1.16–1.46] (1.07×) | 0.95 [0.69–1.10] (1.38×) |
| c=1 p99 ms | 4.13 [2.50–6.09] | 3.55 [3.50–4.36] (1.16×) | 3.05 [2.98–3.61] (1.36×) |
| c=16 req/s | 3,740 [2,850–4,834] | 3,916 [3,798–4,018] (1.05×) | 4,244 [4,241–4,350] (1.13×) |
| c=16 p50 ms | 4.13 [3.12–5.04] | 3.90 [3.77–4.03] (1.06×) | 3.58 [3.54–3.61] (1.15×) |
| c=16 p99 ms | 8.09 [6.96–15.0] | 8.23 [8.23–8.28] (0.98×) | 7.36 [6.93–7.73] (1.10×) |
| c=1 campfire CPU ms/req | 1.24 [0.79–1.67] | 1.16 [1.10–1.33] (1.07×) | 0.93 [0.76–1.05] (1.34×) |
| c=1 thrust CPU ms/req | – | – | – |
| c=1 docker-proxy CPU ms/req | – | – | – |
| c=1 loadgen cores busy | 0.050 [0.040–0.050] | 0.050 [0.040–0.050] | 0.050 [0.040–0.050] |
| c=1 campfire cores busy | 0.82 [0.66–0.92] | 0.84 [0.79–0.85] | 0.86 [0.83–0.87] |
| c=16 campfire CPU ms/req | 0.96 [0.74–1.08] | 0.89 [0.89–0.93] (1.07×) | 0.83 [0.81–0.83] (1.16×) |
| c=16 thrust CPU ms/req | – | – | – |
| c=16 docker-proxy CPU ms/req | – | – | – |
| c=16 loadgen cores busy | 0.14 [0.12–0.15] | 0.14 [0.13–0.14] | 0.15 [0.14–0.16] |
| c=16 campfire cores busy | 3.57 [3.07–3.60] | 3.55 [3.51–3.59] | 3.52 [3.50–3.52] |
| avg response bytes | 6,277 [6,277–6,277] | 6,277 [6,277–6,277] | 6,291 [6,291–6,291] |

### Action Cable, 1000 clients

| Metric | native-base | native-expws | native-expzlib |
|---|---|---|---|
| subscribed | 1,000 [1,000–1,000] | 1,000 [1,000–1,000] | 1,000 [1,000–1,000] |
| paced post→all p50 ms | 29.1 [23.2–32.8] | 15.2 [12.2–28.3] (1.92×) | 25.8 [22.2–32.4] (1.13×) |
| paced post→all p99 ms | 46.2 [42.0–64.7] | 34.8 [24.7–53.3] (1.33×) | 49.3 [37.5–50.1] (0.94×) |
| paced POST p50 ms | 35.7 [28.9–41.0] | 14.9 [13.1–24.4] (2.40×) | 32.9 [30.5–40.9] (1.09×) |
| max sustained msgs/s (to all) | 41.8 [40.1–44.8] | 115 [99.3–120] (2.74×) | 43.8 [43.5–53.6] (1.05×) |
| deliveries/s | 41,753 [40,145–44,768] | 114,627 [99,327–120,383] (2.75×) | 43,809 [43,462–53,648] (1.05×) |
| saturated post→all p50 ms | 71.0 [52.6–75.0] | 33.3 [31.7–37.6] (2.13×) | 61.1 [58.9–76.9] (1.16×) |
| campfire CPU µs/delivery | 65.5 [53.8–77.1] | 23.4 [22.0–25.4] (2.80×) | 62.3 [60.2–82.5] (1.05×) |
| thrust CPU µs/delivery | – | – | – |
| docker-proxy CPU µs/delivery | – | – | – |
| loadgen CPU µs/delivery | 39.3 [34.0–47.3] | 28.4 [26.7–32.2] | 38.7 [37.7–49.9] |
| RssAnon peak MB: before | 30.5 [29.6–31.3] | 30.8 [29.9–31.0] | 31.7 [30.9–33.1] |
| RssAnon peak MB: connected | 179 [179–180] | 59.0 [58.5–59.0] | 180 [179–181] |
| RssAnon peak MB: paced | 201 [200–209] | 80.2 [79.5–80.3] | 201 [200–204] |
| RssAnon peak MB: saturated | 213 [209–227] | 101 [97.5–103] | 213 [209–229] |
| RssAnon peak MB: saturated_posted | 210 [209–212] | 101 [97.6–103] | 213 [209–215] |
| RssAnon peak MB: saturated_drained | 210 [209–212] | 101 [97.6–103] | 213 [209–215] |
| thrust RssAnon peak MB (saturated) | 0.000 [0.000–0.000] | 0.000 [0.000–0.000] | 0.000 [0.000–0.000] |
| container memory.current peak MB (saturated) | 0.000 [0.000–0.000] | 0.000 [0.000–0.000] | 0.000 [0.000–0.000] |
| peak threads | 49.0 [49.0–49.0] | 48.0 [41.0–63.0] | 48.0 [45.0–61.0] |
