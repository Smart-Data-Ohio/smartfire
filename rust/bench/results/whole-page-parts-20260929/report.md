```
date: 2026-09-29T10:57:38+0200
host: 7.2.5-4-omarchy, 32 threads
server cpus: 8-11; loadgen cpus: 12-15
image: campfire-rust:app sha256:7042897243ae9e9f6fc47c3e18d0ae00165de1ad8a3488369bd79d01a8d46ac7
native-main: /home/dhh/Work/basecamp/once-campfire-rust/target/prof-main/release/campfire preload=
native-parts: /home/dhh/Work/basecamp/once-campfire-rust/target/prof-parts/release/campfire preload=
env: WEB_CONCURRENCY=3 RAILS_MAX_THREADS=5 JOB_CONCURRENCY=3
routes: sidebar,room_show,messages_page,search,post_message; concs: 1,16; secs: 5; cable: ; app env:
```

Reps per configuration: native-main 3, native-parts 3. Cells: median [min–max]; (×) is the gain over native-main (>1 is better).
Host load (1-min loadavg at start of each run): native-main 14.34/17.28/16.53, native-parts 17.78/17.01/13.91

### HTTP sidebar

| Metric | native-main | native-parts |
|---|---|---|
| c=1 req/s | 2,342 [2,319–2,469] | 4,279 [4,273–4,721] (1.83×) |
| c=1 p50 ms | 0.41 [0.39–0.42] | 0.22 [0.20–0.22] (1.92×) |
| c=1 p99 ms | 0.63 [0.60–0.68] | 0.40 [0.34–0.42] (1.59×) |
| c=16 req/s | 10,683 [10,631–11,132] | 19,400 [19,019–19,717] (1.82×) |
| c=16 p50 ms | 1.46 [1.40–1.48] | 0.79 [0.77–0.79] (1.85×) |
| c=16 p99 ms | 2.78 [2.70–2.83] | 1.70 [1.65–1.80] (1.64×) |
| c=1 campfire CPU ms/req | 0.41 [0.39–0.41] | 0.22 [0.20–0.22] (1.87×) |
| c=1 thrust CPU ms/req | – | – |
| c=1 docker-proxy CPU ms/req | – | – |
| c=1 loadgen cores busy | 0.040 [0.040–0.040] | 0.070 [0.060–0.070] |
| c=1 campfire cores busy | 0.96 [0.95–0.96] | 0.94 [0.93–0.95] |
| c=16 campfire CPU ms/req | 0.35 [0.34–0.36] | 0.19 [0.19–0.19] (1.87×) |
| c=16 thrust CPU ms/req | – | – |
| c=16 docker-proxy CPU ms/req | – | – |
| c=16 loadgen cores busy | 0.14 [0.14–0.15] | 0.25 [0.25–0.26] |
| c=16 campfire cores busy | 3.78 [3.77–3.84] | 3.71 [3.61–3.77] |
| avg response bytes | 5,910 [5,910–5,910] | 5,910 [5,910–5,910] |

### HTTP room_show

| Metric | native-main | native-parts |
|---|---|---|
| c=1 req/s | 3,610 [3,325–4,181] | 3,774 [3,569–3,840] (1.05×) |
| c=1 p50 ms | 0.27 [0.23–0.29] | 0.25 [0.24–0.26] (1.06×) |
| c=1 p99 ms | 0.46 [0.37–0.56] | 0.48 [0.40–0.54] (0.96×) |
| c=16 req/s | 16,710 [15,646–17,734] | 16,927 [16,624–17,302] (1.01×) |
| c=16 p50 ms | 0.92 [0.86–0.96] | 0.90 [0.88–0.91] (1.02×) |
| c=16 p99 ms | 1.90 [1.75–2.12] | 1.90 [1.86–1.95] (1.00×) |
| c=1 campfire CPU ms/req | 0.26 [0.23–0.28] | 0.25 [0.25–0.26] (1.05×) |
| c=1 thrust CPU ms/req | – | – |
| c=1 docker-proxy CPU ms/req | – | – |
| c=1 loadgen cores busy | 0.060 [0.050–0.060] | 0.060 [0.050–0.060] |
| c=1 campfire cores busy | 0.95 [0.94–0.96] | 0.95 [0.94–0.96] |
| c=16 campfire CPU ms/req | 0.22 [0.21–0.23] | 0.22 [0.21–0.22] (1.02×) |
| c=16 thrust CPU ms/req | – | – |
| c=16 docker-proxy CPU ms/req | – | – |
| c=16 loadgen cores busy | 0.25 [0.24–0.26] | 0.24 [0.23–0.25] |
| c=16 campfire cores busy | 3.72 [3.65–3.72] | 3.69 [3.69–3.69] |
| avg response bytes | 24,231 [24,231–24,231] | 24,231 [24,231–24,231] |

### HTTP messages_page

| Metric | native-main | native-parts |
|---|---|---|
| c=1 req/s | 4,276 [3,590–4,941] | 4,422 [3,909–4,464] (1.03×) |
| c=1 p50 ms | 0.22 [0.19–0.25] | 0.21 [0.21–0.24] (1.03×) |
| c=1 p99 ms | 0.41 [0.34–0.72] | 0.39 [0.38–0.48] (1.05×) |
| c=16 req/s | 19,651 [18,622–20,181] | 19,149 [19,054–19,983] (0.97×) |
| c=16 p50 ms | 0.77 [0.75–0.80] | 0.78 [0.75–0.79] (0.98×) |
| c=16 p99 ms | 1.75 [1.67–1.93] | 1.81 [1.73–1.81] (0.97×) |
| c=1 campfire CPU ms/req | 0.22 [0.20–0.26] | 0.22 [0.22–0.24] (1.03×) |
| c=1 thrust CPU ms/req | – | – |
| c=1 docker-proxy CPU ms/req | – | – |
| c=1 loadgen cores busy | 0.060 [0.050–0.070] | 0.060 [0.060–0.070] |
| c=1 campfire cores busy | 0.96 [0.92–0.97] | 0.96 [0.95–0.96] |
| c=16 campfire CPU ms/req | 0.19 [0.18–0.19] | 0.19 [0.18–0.19] (0.98×) |
| c=16 thrust CPU ms/req | – | – |
| c=16 docker-proxy CPU ms/req | – | – |
| c=16 loadgen cores busy | 0.27 [0.27–0.30] | 0.27 [0.27–0.30] |
| c=16 campfire cores busy | 3.64 [3.61–3.68] | 3.65 [3.63–3.66] |
| avg response bytes | 16,158 [16,158–16,158] | 16,158 [16,158–16,158] |

### HTTP search

| Metric | native-main | native-parts |
|---|---|---|
| c=1 req/s | 4,340 [4,296–4,823] | 4,480 [4,443–4,733] (1.03×) |
| c=1 p50 ms | 0.20 [0.19–0.22] | 0.20 [0.20–0.21] (0.97×) |
| c=1 p99 ms | 0.42 [0.35–0.55] | 0.43 [0.37–0.47] (0.97×) |
| c=16 req/s | 19,754 [19,380–19,997] | 19,783 [19,281–20,234] (1.00×) |
| c=16 p50 ms | 0.75 [0.75–0.77] | 0.76 [0.74–0.78] (0.99×) |
| c=16 p99 ms | 1.77 [1.68–1.79] | 1.73 [1.70–1.78] (1.02×) |
| c=1 campfire CPU ms/req | 0.22 [0.20–0.22] | 0.21 [0.21–0.22] (1.02×) |
| c=1 thrust CPU ms/req | – | – |
| c=1 docker-proxy CPU ms/req | – | – |
| c=1 loadgen cores busy | 0.060 [0.060–0.070] | 0.060 [0.060–0.060] |
| c=1 campfire cores busy | 0.96 [0.94–0.98] | 0.96 [0.96–0.98] |
| c=16 campfire CPU ms/req | 0.18 [0.18–0.18] | 0.18 [0.18–0.18] (0.98×) |
| c=16 thrust CPU ms/req | – | – |
| c=16 docker-proxy CPU ms/req | – | – |
| c=16 loadgen cores busy | 0.26 [0.25–0.26] | 0.26 [0.25–0.26] |
| c=16 campfire cores busy | 3.54 [3.51–3.58] | 3.56 [3.55–3.62] |
| avg response bytes | 9,766 [9,766–9,766] | 9,766 [9,766–9,766] |

### HTTP post_message

| Metric | native-main | native-parts |
|---|---|---|
| c=1 req/s | 1,589 [1,453–1,799] | 1,651 [1,470–1,714] (1.04×) |
| c=1 p50 ms | 0.55 [0.50–0.61] | 0.55 [0.51–0.61] (0.98×) |
| c=1 p99 ms | 2.17 [1.88–2.32] | 1.93 [1.92–2.18] (1.13×) |
| c=16 req/s | 4,803 [4,463–4,849] | 4,650 [4,524–4,840] (0.97×) |
| c=16 p50 ms | 3.03 [2.94–3.18] | 3.12 [3.01–3.13] (0.97×) |
| c=16 p99 ms | 9.41 [9.03–10.8] | 9.37 [9.20–10.2] (1.01×) |
| c=1 campfire CPU ms/req | 0.64 [0.59–0.70] | 0.64 [0.60–0.70] (0.99×) |
| c=1 thrust CPU ms/req | – | – |
| c=1 docker-proxy CPU ms/req | – | – |
| c=1 loadgen cores busy | 0.040 [0.030–0.050] | 0.050 [0.040–0.050] |
| c=1 campfire cores busy | 1.02 [1.02–1.07] | 1.03 [1.02–1.06] |
| c=16 campfire CPU ms/req | 0.59 [0.57–0.61] | 0.60 [0.60–0.62] (0.97×) |
| c=16 thrust CPU ms/req | – | – |
| c=16 docker-proxy CPU ms/req | – | – |
| c=16 loadgen cores busy | 0.090 [0.080–0.10] | 0.11 [0.11–0.12] |
| c=16 campfire cores busy | 2.74 [2.72–2.82] | 2.87 [2.73–2.88] |
| avg response bytes | 1,992 [1,992–1,992] | 1,992 [1,992–1,992] |
