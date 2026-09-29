```
date: 2026-09-27T00:12:17+0200
host: 7.2.5-4-omarchy, 32 threads
server cpus: 4-7; loadgen cpus: 0-3
image: campfire-rust:perf-a654b08 sha256:82a2c82b7f7ff1d371c701e3d9f81b894f76c1120231f421a817d183748a54c3
env: WEB_CONCURRENCY=3 RAILS_MAX_THREADS=5 JOB_CONCURRENCY=3
routes: up,room_show,room_show_identity,messages_page,sidebar,post_message; concs: 1,16,64; secs: 5; cable: 1000; app env:
```

Reps per configuration: proxy-thruster 1. Cells: median [min–max]; (×) is the gain over proxy-thruster (>1 is better).
Host load (1-min loadavg at start of each run): proxy-thruster 17.01

### HTTP up

| Metric | proxy-thruster |
|---|---|
| c=1 req/s | 4,167 |
| c=1 p50 ms | 0.21 |
| c=1 p99 ms | 1.10 |
| c=16 req/s | 19,952 |
| c=16 p50 ms | 0.73 |
| c=16 p99 ms | 2.21 |
| c=64 req/s | 18,005 |
| c=64 p50 ms | 3.16 |
| c=64 p99 ms | 9.84 |
| c=1 campfire CPU ms/req | 0.083 |
| c=1 thrust CPU ms/req | 0.13 |
| c=1 docker-proxy CPU ms/req | 0.026 |
| c=1 loadgen cores busy | 0.10 |
| c=1 campfire cores busy | 0.34 |
| c=16 campfire CPU ms/req | 0.055 |
| c=16 thrust CPU ms/req | 0.096 |
| c=16 docker-proxy CPU ms/req | 0.017 |
| c=16 loadgen cores busy | 0.26 |
| c=16 campfire cores busy | 1.10 |
| c=64 campfire CPU ms/req | 0.056 |
| c=64 thrust CPU ms/req | 0.12 |
| c=64 docker-proxy CPU ms/req | 0.021 |
| c=64 loadgen cores busy | 0.29 |
| c=64 campfire cores busy | 1.00 |
| avg response bytes | 103 |

### HTTP room_show

| Metric | proxy-thruster |
|---|---|
| c=1 req/s | 192 |
| c=1 p50 ms | 4.82 |
| c=1 p99 ms | 9.00 |
| c=16 req/s | 824 |
| c=16 p50 ms | 19.2 |
| c=16 p99 ms | 32.2 |
| c=64 req/s | 844 |
| c=64 p50 ms | 73.8 |
| c=64 p99 ms | 128 |
| c=1 campfire CPU ms/req | 4.88 |
| c=1 thrust CPU ms/req | 0.25 |
| c=1 docker-proxy CPU ms/req | 0.052 |
| c=1 loadgen cores busy | 0.010 |
| c=1 campfire cores busy | 0.94 |
| c=16 campfire CPU ms/req | 4.32 |
| c=16 thrust CPU ms/req | 0.28 |
| c=16 docker-proxy CPU ms/req | 0.044 |
| c=16 loadgen cores busy | 0.040 |
| c=16 campfire cores busy | 3.56 |
| c=64 campfire CPU ms/req | 4.34 |
| c=64 thrust CPU ms/req | 0.24 |
| c=64 docker-proxy CPU ms/req | 0.047 |
| c=64 loadgen cores busy | 0.040 |
| c=64 campfire cores busy | 3.66 |
| avg response bytes | 44,267 |

### HTTP room_show_identity

| Metric | proxy-thruster |
|---|---|
| c=1 req/s | 362 |
| c=1 p50 ms | 2.61 |
| c=1 p99 ms | 4.51 |
| c=16 req/s | 1,080 |
| c=16 p50 ms | 14.4 |
| c=16 p99 ms | 27.5 |
| c=64 req/s | 1,371 |
| c=64 p50 ms | 45.6 |
| c=64 p99 ms | 78.1 |
| c=1 campfire CPU ms/req | 2.19 |
| c=1 thrust CPU ms/req | 0.65 |
| c=1 docker-proxy CPU ms/req | 0.11 |
| c=1 loadgen cores busy | 0.080 |
| c=1 campfire cores busy | 0.79 |
| c=16 campfire CPU ms/req | 2.32 |
| c=16 thrust CPU ms/req | 0.92 |
| c=16 docker-proxy CPU ms/req | 0.13 |
| c=16 loadgen cores busy | 0.23 |
| c=16 campfire cores busy | 2.50 |
| c=64 campfire CPU ms/req | 1.95 |
| c=64 thrust CPU ms/req | 0.70 |
| c=64 docker-proxy CPU ms/req | 0.096 |
| c=64 loadgen cores busy | 0.17 |
| c=64 campfire cores busy | 2.67 |
| avg response bytes | 463,818 |

### HTTP messages_page

| Metric | proxy-thruster |
|---|---|
| c=1 req/s | 244 |
| c=1 p50 ms | 3.60 |
| c=1 p99 ms | 8.46 |
| c=16 req/s | 1,030 |
| c=16 p50 ms | 15.3 |
| c=16 p99 ms | 25.0 |
| c=64 req/s | 1,105 |
| c=64 p50 ms | 56.5 |
| c=64 p99 ms | 90.8 |
| c=1 campfire CPU ms/req | 3.73 |
| c=1 thrust CPU ms/req | 0.28 |
| c=1 docker-proxy CPU ms/req | 0.033 |
| c=1 loadgen cores busy | 0.010 |
| c=1 campfire cores busy | 0.91 |
| c=16 campfire CPU ms/req | 3.32 |
| c=16 thrust CPU ms/req | 0.28 |
| c=16 docker-proxy CPU ms/req | 0.031 |
| c=16 loadgen cores busy | 0.040 |
| c=16 campfire cores busy | 3.42 |
| c=64 campfire CPU ms/req | 3.26 |
| c=64 thrust CPU ms/req | 0.21 |
| c=64 docker-proxy CPU ms/req | 0.034 |
| c=64 loadgen cores busy | 0.040 |
| c=64 campfire cores busy | 3.60 |
| avg response bytes | 34,636 |

### HTTP sidebar

| Metric | proxy-thruster |
|---|---|
| c=1 req/s | 1,271 |
| c=1 p50 ms | 0.77 |
| c=1 p99 ms | 1.06 |
| c=16 req/s | 4,453 |
| c=16 p50 ms | 3.48 |
| c=16 p99 ms | 6.62 |
| c=64 req/s | 4,768 |
| c=64 p50 ms | 13.1 |
| c=64 p99 ms | 23.0 |
| c=1 campfire CPU ms/req | 0.66 |
| c=1 thrust CPU ms/req | 0.13 |
| c=1 docker-proxy CPU ms/req | 0.020 |
| c=1 loadgen cores busy | 0.020 |
| c=1 campfire cores busy | 0.84 |
| c=16 campfire CPU ms/req | 0.66 |
| c=16 thrust CPU ms/req | 0.14 |
| c=16 docker-proxy CPU ms/req | 0.025 |
| c=16 loadgen cores busy | 0.090 |
| c=16 campfire cores busy | 2.94 |
| c=64 campfire CPU ms/req | 0.64 |
| c=64 thrust CPU ms/req | 0.14 |
| c=64 docker-proxy CPU ms/req | 0.026 |
| c=64 loadgen cores busy | 0.10 |
| c=64 campfire cores busy | 3.04 |
| avg response bytes | 6,277 |

### HTTP post_message

| Metric | proxy-thruster |
|---|---|
| c=1 req/s | 1,047 |
| c=1 p50 ms | 0.78 |
| c=1 p99 ms | 11.7 |
| c=16 req/s | 2,160 |
| c=16 p50 ms | 4.46 |
| c=16 p99 ms | 21.0 |
| c=64 req/s | 2,254 |
| c=64 p50 ms | 27.9 |
| c=64 p99 ms | 47.8 |
| c=1 campfire CPU ms/req | 0.77 |
| c=1 thrust CPU ms/req | 0.14 |
| c=1 docker-proxy CPU ms/req | 0.013 |
| c=1 loadgen cores busy | 0.020 |
| c=1 campfire cores busy | 0.81 |
| c=16 campfire CPU ms/req | 0.79 |
| c=16 thrust CPU ms/req | 0.14 |
| c=16 docker-proxy CPU ms/req | 0.018 |
| c=16 loadgen cores busy | 0.040 |
| c=16 campfire cores busy | 1.71 |
| c=64 campfire CPU ms/req | 0.76 |
| c=64 thrust CPU ms/req | 0.15 |
| c=64 docker-proxy CPU ms/req | 0.021 |
| c=64 loadgen cores busy | 0.050 |
| c=64 campfire cores busy | 1.72 |
| avg response bytes | 2,005 |

### Action Cable, 1000 clients

| Metric | proxy-thruster |
|---|---|
| subscribed | 1,000 |
| paced post→all p50 ms | 22.5 |
| paced post→all p99 ms | 34.5 |
| paced POST p50 ms | 25.4 |
| max sustained msgs/s (to all) | 73.4 |
| deliveries/s | 73,408 |
| saturated post→all p50 ms | 47.7 |
| campfire CPU µs/delivery | 31.3 |
| thrust CPU µs/delivery | 13.3 |
| docker-proxy CPU µs/delivery | 17.1 |
| loadgen CPU µs/delivery | 22.3 |
| RssAnon peak MB: before | 343 |
| RssAnon peak MB: connected | 491 |
| RssAnon peak MB: paced | 517 |
| RssAnon peak MB: saturated | 532 |
| RssAnon peak MB: saturated_posted | 532 |
| RssAnon peak MB: saturated_drained | 532 |
| thrust RssAnon peak MB (saturated) | 156 |
| container memory.current peak MB (saturated) | 743 |
| peak threads | 107 |
