```
date: 2026-09-27T00:46:23+0200
host: 7.2.5-4-omarchy, 32 threads
server cpus: 8-11; loadgen cpus: 12-15
image: campfire-rust:perf-a654b08 sha256:82a2c82b7f7ff1d371c701e3d9f81b894f76c1120231f421a817d183748a54c3
native-base: ./target/perf-base/release/campfire preload=
native-lto: ./target/perf-lto/release/campfire preload=
native-tuned: ./target/perf-tuned/release/campfire preload=
native-base-je: ./target/perf-base/release/campfire preload=/usr/lib/libjemalloc.so
native-tuned-je: ./target/perf-tuned/release/campfire preload=/usr/lib/libjemalloc.so
env: WEB_CONCURRENCY=3 RAILS_MAX_THREADS=5 JOB_CONCURRENCY=3
routes: room_show,room_show_identity,sidebar; concs: 16; secs: 4; cable: ; app env:
```

Reps per configuration: native-base 5, native-base-je 5, native-lto 5, native-tuned 5, native-tuned-je 5. Cells: median [min–max]; (×) is the gain over native-base (>1 is better).
Host load (1-min loadavg at start of each run): native-base 9.93/14.36/10.37/27.27/36.95, native-base-je 10.71/13.49/23.88/32.26/27.40, native-lto 11.27/13.64/12.60/32.86/31.40, native-tuned 10.52/14.66/20.72/31.37/28.36, native-tuned-je 14.78/12.21/26.00/40.97/24.33

### HTTP room_show

| Metric | native-base | native-base-je | native-lto | native-tuned | native-tuned-je |
|---|---|---|---|---|---|
| c=16 req/s | 795 [682–948] | 757 [604–954] (0.95×) | 653 [528–945] (0.82×) | 724 [607–914] (0.91×) | 672 [543–1,006] (0.85×) |
| c=16 p50 ms | 19.8 [16.7–23.0] | 20.7 [16.5–25.7] (0.95×) | 24.2 [16.7–29.4] (0.82×) | 21.7 [17.3–25.9] (0.91×) | 22.6 [15.7–26.4] (0.87×) |
| c=16 p99 ms | 34.0 [28.2–39.6] | 36.2 [28.4–49.4] (0.94×) | 40.6 [28.7–53.8] (0.84×) | 37.8 [29.6–54.4] (0.90×) | 39.8 [27.1–100] (0.85×) |
| c=16 campfire CPU ms/req | 4.62 [3.99–5.48] | 5.02 [3.98–6.01] (0.92×) | 5.72 [3.97–6.49] (0.81×) | 5.18 [4.08–6.09] (0.89×) | 5.48 [3.80–6.14] (0.84×) |
| c=16 thrust CPU ms/req | – | – | – | – | – |
| c=16 docker-proxy CPU ms/req | – | – | – | – | – |
| c=16 loadgen cores busy | 0.030 [0.020–0.050] | 0.040 [0.020–0.060] | 0.050 [0.020–0.060] | 0.040 [0.020–0.060] | 0.040 [0.020–0.060] |
| c=16 campfire cores busy | 3.74 [3.67–3.78] | 3.78 [3.62–3.80] | 3.74 [3.43–3.78] | 3.73 [3.62–3.76] | 3.76 [2.98–3.82] |
| avg response bytes | 44,271 [44,259–44,277] | 44,253 [44,250–44,271] | 44,256 [44,240–44,283] | 44,272 [44,244–44,279] | 44,272 [44,250–44,287] |

### HTTP room_show_identity

| Metric | native-base | native-base-je | native-lto | native-tuned | native-tuned-je |
|---|---|---|---|---|---|
| c=16 req/s | 1,607 [1,575–1,962] | 1,679 [1,303–2,259] (1.05×) | 1,676 [1,332–2,086] (1.04×) | 1,676 [885–2,210] (1.04×) | 2,040 [1,190–2,460] (1.27×) |
| c=16 p50 ms | 9.31 [7.95–9.72] | 9.03 [6.93–11.8] (1.03×) | 9.24 [7.50–11.6] (1.01×) | 9.21 [7.10–14.8] (1.01×) | 7.54 [6.34–12.7] (1.24×) |
| c=16 p99 ms | 17.9 [13.8–20.8] | 22.2 [11.9–34.0] (0.80×) | 18.8 [12.9–22.1] (0.95×) | 16.8 [12.0–51.9] (1.07×) | 15.0 [11.1–27.2] (1.19×) |
| c=16 campfire CPU ms/req | 2.15 [1.77–2.20] | 2.03 [1.55–2.59] (1.06×) | 2.04 [1.66–2.49] (1.06×) | 2.05 [1.58–2.88] (1.05×) | 1.68 [1.43–2.71] (1.28×) |
| c=16 thrust CPU ms/req | – | – | – | – | – |
| c=16 docker-proxy CPU ms/req | – | – | – | – | – |
| c=16 loadgen cores busy | 0.18 [0.16–0.21] | 0.20 [0.17–0.24] | 0.21 [0.16–0.22] | 0.20 [0.16–0.24] | 0.21 [0.19–0.25] |
| c=16 campfire cores busy | 3.46 [3.42–3.47] | 3.37 [3.30–3.50] | 3.41 [3.32–3.45] | 3.42 [2.54–3.48] | 3.42 [3.22–3.51] |
| avg response bytes | 463,818 [463,818–463,818] | 463,818 [463,818–463,818] | 463,818 [463,818–463,818] | 463,818 [463,818–463,818] | 463,818 [463,818–463,818] |

### HTTP sidebar

| Metric | native-base | native-base-je | native-lto | native-tuned | native-tuned-je |
|---|---|---|---|---|---|
| c=16 req/s | 5,298 [4,522–5,643] | 4,527 [2,341–6,938] (0.85×) | 4,768 [3,084–6,307] (0.90×) | 4,854 [3,213–6,560] (0.92×) | 4,545 [4,024–7,413] (0.86×) |
| c=16 p50 ms | 2.92 [2.75–3.43] | 3.40 [2.26–5.65] (0.86×) | 3.21 [2.50–3.79] (0.91×) | 3.18 [2.40–4.45] (0.92×) | 3.24 [2.11–3.71] (0.90×) |
| c=16 p99 ms | 5.47 [5.00–7.11] | 6.72 [4.08–24.6] (0.81×) | 6.52 [4.45–23.1] (0.84×) | 6.19 [4.19–15.0] (0.88×) | 6.72 [3.79–8.88] (0.81×) |
| c=16 campfire CPU ms/req | 0.69 [0.65–0.81] | 0.82 [0.55–1.15] (0.85×) | 0.76 [0.59–0.87] (0.92×) | 0.75 [0.57–1.01] (0.93×) | 0.79 [0.51–0.90] (0.88×) |
| c=16 thrust CPU ms/req | – | – | – | – | – |
| c=16 docker-proxy CPU ms/req | – | – | – | – | – |
| c=16 loadgen cores busy | 0.090 [0.080–0.12] | 0.13 [0.070–0.15] | 0.080 [0.080–0.14] | 0.11 [0.070–0.14] | 0.13 [0.070–0.16] |
| c=16 campfire cores busy | 3.69 [3.65–3.72] | 3.69 [2.70–3.79] | 3.60 [2.56–3.75] | 3.63 [3.15–3.71] | 3.72 [3.57–3.80] |
| avg response bytes | 6,277 [6,277–6,277] | 6,277 [6,277–6,277] | 6,277 [6,277–6,277] | 6,277 [6,277–6,277] | 6,277 [6,277–6,277] |
