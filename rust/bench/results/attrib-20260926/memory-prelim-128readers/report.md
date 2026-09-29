```
date: 2026-09-27T00:10:13+0200
host: 7.2.5-4-omarchy, 32 threads
server cpus: 4-7; loadgen cpus: 0-3
image: campfire-rust:perf-a654b08 sha256:82a2c82b7f7ff1d371c701e3d9f81b894f76c1120231f421a817d183748a54c3
env: WEB_CONCURRENCY=3 RAILS_MAX_THREADS=5 JOB_CONCURRENCY=3
routes: up,room_show,room_show_identity,messages_page,sidebar,post_message; concs: 1,16,64; secs: 5; cable: 1000; app env: RAILS_MAX_THREADS=128
```

Reps per configuration: proxy-thruster 1. Cells: median [min–max]; (×) is the gain over proxy-thruster (>1 is better).
Host load (1-min loadavg at start of each run): proxy-thruster 15.40

### HTTP up

| Metric | proxy-thruster |
|---|---|
| c=1 req/s | 9,006 |
| c=1 p50 ms | 0.099 |
| c=1 p99 ms | 0.22 |
| c=16 req/s | 21,303 |
| c=16 p50 ms | 0.70 |
| c=16 p99 ms | 1.83 |
| c=64 req/s | 14,998 |
| c=64 p50 ms | 3.66 |
| c=64 p99 ms | 13.7 |
| c=1 campfire CPU ms/req | 0.043 |
| c=1 thrust CPU ms/req | 0.063 |
| c=1 docker-proxy CPU ms/req | 0.013 |
| c=1 loadgen cores busy | 0.10 |
| c=1 campfire cores busy | 0.39 |
| c=16 campfire CPU ms/req | 0.053 |
| c=16 thrust CPU ms/req | 0.092 |
| c=16 docker-proxy CPU ms/req | 0.016 |
| c=16 loadgen cores busy | 0.25 |
| c=16 campfire cores busy | 1.13 |
| c=64 campfire CPU ms/req | 0.069 |
| c=64 thrust CPU ms/req | 0.14 |
| c=64 docker-proxy CPU ms/req | 0.027 |
| c=64 loadgen cores busy | 0.32 |
| c=64 campfire cores busy | 1.03 |
| avg response bytes | 103 |

### HTTP room_show

| Metric | proxy-thruster |
|---|---|
| c=1 req/s | 185 |
| c=1 p50 ms | 4.67 |
| c=1 p99 ms | 8.71 |
| c=16 req/s | 485 |
| c=16 p50 ms | 32.1 |
| c=16 p99 ms | 60.8 |
| c=64 req/s | 538 |
| c=64 p50 ms | 114 |
| c=64 p99 ms | 230 |
| c=1 campfire CPU ms/req | 5.05 |
| c=1 thrust CPU ms/req | 0.33 |
| c=1 docker-proxy CPU ms/req | 0.054 |
| c=1 loadgen cores busy | 0.020 |
| c=1 campfire cores busy | 0.93 |
| c=16 campfire CPU ms/req | 7.23 |
| c=16 thrust CPU ms/req | 0.53 |
| c=16 docker-proxy CPU ms/req | 0.098 |
| c=16 loadgen cores busy | 0.060 |
| c=16 campfire cores busy | 3.51 |
| c=64 campfire CPU ms/req | 6.80 |
| c=64 thrust CPU ms/req | 0.41 |
| c=64 docker-proxy CPU ms/req | 0.092 |
| c=64 loadgen cores busy | 0.060 |
| c=64 campfire cores busy | 3.66 |
| avg response bytes | 44,278 |

### HTTP room_show_identity

| Metric | proxy-thruster |
|---|---|
| c=1 req/s | 441 |
| c=1 p50 ms | 2.23 |
| c=1 p99 ms | 2.94 |
| c=16 req/s | 1,234 |
| c=16 p50 ms | 12.8 |
| c=16 p99 ms | 23.6 |
| c=64 req/s | 1,355 |
| c=64 p50 ms | 45.8 |
| c=64 p99 ms | 96.3 |
| c=1 campfire CPU ms/req | 1.87 |
| c=1 thrust CPU ms/req | 0.55 |
| c=1 docker-proxy CPU ms/req | 0.082 |
| c=1 loadgen cores busy | 0.050 |
| c=1 campfire cores busy | 0.82 |
| c=16 campfire CPU ms/req | 2.11 |
| c=16 thrust CPU ms/req | 0.80 |
| c=16 docker-proxy CPU ms/req | 0.11 |
| c=16 loadgen cores busy | 0.22 |
| c=16 campfire cores busy | 2.61 |
| c=64 campfire CPU ms/req | 2.05 |
| c=64 thrust CPU ms/req | 0.70 |
| c=64 docker-proxy CPU ms/req | 0.11 |
| c=64 loadgen cores busy | 0.24 |
| c=64 campfire cores busy | 2.77 |
| avg response bytes | 463,818 |

### HTTP messages_page

| Metric | proxy-thruster |
|---|---|
| c=1 req/s | 295 |
| c=1 p50 ms | 3.31 |
| c=1 p99 ms | 4.49 |
| c=16 req/s | 1,055 |
| c=16 p50 ms | 14.8 |
| c=16 p99 ms | 25.5 |
| c=64 req/s | 1,050 |
| c=64 p50 ms | 59.6 |
| c=64 p99 ms | 99.5 |
| c=1 campfire CPU ms/req | 3.19 |
| c=1 thrust CPU ms/req | 0.22 |
| c=1 docker-proxy CPU ms/req | 0.034 |
| c=1 loadgen cores busy | 0.010 |
| c=1 campfire cores busy | 0.94 |
| c=16 campfire CPU ms/req | 3.35 |
| c=16 thrust CPU ms/req | 0.27 |
| c=16 docker-proxy CPU ms/req | 0.030 |
| c=16 loadgen cores busy | 0.030 |
| c=16 campfire cores busy | 3.54 |
| c=64 campfire CPU ms/req | 3.48 |
| c=64 thrust CPU ms/req | 0.21 |
| c=64 docker-proxy CPU ms/req | 0.032 |
| c=64 loadgen cores busy | 0.040 |
| c=64 campfire cores busy | 3.65 |
| avg response bytes | 34,636 |

### HTTP sidebar

| Metric | proxy-thruster |
|---|---|
| c=1 req/s | 1,304 |
| c=1 p50 ms | 0.76 |
| c=1 p99 ms | 0.93 |
| c=16 req/s | 4,426 |
| c=16 p50 ms | 3.51 |
| c=16 p99 ms | 6.69 |
| c=64 req/s | 4,489 |
| c=64 p50 ms | 14.0 |
| c=64 p99 ms | 24.1 |
| c=1 campfire CPU ms/req | 0.65 |
| c=1 thrust CPU ms/req | 0.13 |
| c=1 docker-proxy CPU ms/req | 0.023 |
| c=1 loadgen cores busy | 0.020 |
| c=1 campfire cores busy | 0.85 |
| c=16 campfire CPU ms/req | 0.68 |
| c=16 thrust CPU ms/req | 0.14 |
| c=16 docker-proxy CPU ms/req | 0.024 |
| c=16 loadgen cores busy | 0.080 |
| c=16 campfire cores busy | 3.01 |
| c=64 campfire CPU ms/req | 0.68 |
| c=64 thrust CPU ms/req | 0.14 |
| c=64 docker-proxy CPU ms/req | 0.027 |
| c=64 loadgen cores busy | 0.090 |
| c=64 campfire cores busy | 3.05 |
| avg response bytes | 6,277 |

### HTTP post_message

| Metric | proxy-thruster |
|---|---|
| c=1 req/s | 1,026 |
| c=1 p50 ms | 0.79 |
| c=1 p99 ms | 11.9 |
| c=16 req/s | 1,999 |
| c=16 p50 ms | 4.90 |
| c=16 p99 ms | 27.8 |
| c=64 req/s | 2,034 |
| c=64 p50 ms | 31.0 |
| c=64 p99 ms | 51.5 |
| c=1 campfire CPU ms/req | 0.79 |
| c=1 thrust CPU ms/req | 0.14 |
| c=1 docker-proxy CPU ms/req | 0.018 |
| c=1 loadgen cores busy | 0.020 |
| c=1 campfire cores busy | 0.81 |
| c=16 campfire CPU ms/req | 0.88 |
| c=16 thrust CPU ms/req | 0.15 |
| c=16 docker-proxy CPU ms/req | 0.018 |
| c=16 loadgen cores busy | 0.040 |
| c=16 campfire cores busy | 1.76 |
| c=64 campfire CPU ms/req | 0.91 |
| c=64 thrust CPU ms/req | 0.17 |
| c=64 docker-proxy CPU ms/req | 0.023 |
| c=64 loadgen cores busy | 0.050 |
| c=64 campfire cores busy | 1.85 |
| avg response bytes | 2,005 |

### Action Cable, 1000 clients

| Metric | proxy-thruster |
|---|---|
| subscribed | 1,000 |
| paced post→all p50 ms | 22.1 |
| paced post→all p99 ms | 26.4 |
| paced POST p50 ms | 24.7 |
| max sustained msgs/s (to all) | 72.7 |
| deliveries/s | 72,723 |
| saturated post→all p50 ms | 47.1 |
| campfire CPU µs/delivery | 32.3 |
| thrust CPU µs/delivery | 13.4 |
| docker-proxy CPU µs/delivery | 18.8 |
| loadgen CPU µs/delivery | 23.2 |
| RssAnon peak MB: before | 352 |
| RssAnon peak MB: connected | 500 |
| RssAnon peak MB: paced | 517 |
| RssAnon peak MB: saturated | 534 |
| RssAnon peak MB: saturated_posted | 534 |
| RssAnon peak MB: saturated_drained | 534 |
| thrust RssAnon peak MB (saturated) | 153 |
| container memory.current peak MB (saturated) | 747 |
| peak threads | 135 |
