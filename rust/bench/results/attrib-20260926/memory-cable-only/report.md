```
date: 2026-09-27T00:08:26+0200
host: 7.2.5-4-omarchy, 32 threads
server cpus: 4-7; loadgen cpus: 0-3
image: campfire-rust:perf-a654b08 sha256:82a2c82b7f7ff1d371c701e3d9f81b894f76c1120231f421a817d183748a54c3
env: WEB_CONCURRENCY=3 RAILS_MAX_THREADS=5 JOB_CONCURRENCY=3
routes: ; concs: 1,16,64; secs: 5; cable: 1000; app env:
```

Reps per configuration: proxy-thruster 2, host-direct 2. Cells: median [min–max]; (×) is the gain over proxy-thruster (>1 is better).
Host load (1-min loadavg at start of each run): proxy-thruster 7.92/12.34, host-direct 11.77/15.79

### Action Cable, 1000 clients

| Metric | proxy-thruster | host-direct |
|---|---|---|
| subscribed | 1,000 [1,000–1,000] | 1,000 [1,000–1,000] |
| paced post→all p50 ms | 26.3 [22.6–30.0] | 16.3 [15.5–17.1] (1.61×) |
| paced post→all p99 ms | 34.4 [29.9–38.9] | 28.4 [23.5–33.4] (1.21×) |
| paced POST p50 ms | 28.6 [26.4–30.9] | 21.7 [21.3–22.0] (1.32×) |
| max sustained msgs/s (to all) | 53.1 [51.3–55.0] | 92.7 [90.7–94.6] (1.74×) |
| deliveries/s | 53,154 [51,301–55,006] | 92,645 [90,720–94,570] (1.74×) |
| saturated post→all p50 ms | 64.9 [61.4–68.4] | 36.2 [35.4–37.0] (1.79×) |
| campfire CPU µs/delivery | 43.9 [42.3–45.4] | 36.8 [36.4–37.2] (1.19×) |
| thrust CPU µs/delivery | 17.9 [16.8–19.1] | – |
| docker-proxy CPU µs/delivery | 22.0 [17.9–26.2] | – |
| loadgen CPU µs/delivery | 31.5 [30.7–32.2] | 24.8 [24.3–25.3] |
| RssAnon peak MB: before | 12.2 [12.2–12.2] | 12.3 [12.3–12.3] |
| RssAnon peak MB: connected | 170 [169–171] | 171 [171–171] |
| RssAnon peak MB: paced | 195 [194–196] | 197 [197–198] |
| RssAnon peak MB: saturated | 218 [208–228] | 232 [232–232] |
| RssAnon peak MB: saturated_posted | 215 [208–222] | 232 [232–232] |
| RssAnon peak MB: saturated_drained | 215 [208–222] | 232 [232–232] |
| thrust RssAnon peak MB (saturated) | 148 [138–156] | 0.000 [0.000–0.000] |
| container memory.current peak MB (saturated) | 399 [389–409] | 249 [248–250] |
| peak threads | 60.5 [39.0–82.0] | 126 [116–136] |
