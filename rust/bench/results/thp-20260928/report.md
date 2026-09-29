# No transparent huge pages (2026-09-28)

The production image before (`app`) and after (`nothp`) opting out of transparent huge pages:
`PR_SET_THP_DISABLE` for the process and jemalloc's `thp:never`. Host: 32 cores, THP `[always]`.

## Idle memory

After boot and one `/up` request (`idle.txt`), anonymous RSS by CPUs given to the container:

| CPUs | Before | After |
|---|---|---|
| 1 | 15 MB | 10 MB |
| 2 | 37 MB | 11 MB |
| 4 | 43 MB | 11 MB |
| 32 | 160 MB | 15 MB |

In the bench harness (`SERVER_CPUS=8-11`, four cores): idle `memory.current` 47 → 14 MB, anon
45 → 11 MB.

## Throughput

`bench/run --apps rust --reps 1` with `SUITES=http`, one run per image back to back, then the
`nothp` image again (`nothp-rerun`). The host was shared with another project's load test
(load average around 15), so single runs carry a few percent of noise; no rep-to-rep comparison
is meaningful below that.

| Route, concurrency | Before req/s | After req/s | After (rerun) req/s |
|---|---|---|---|
| room_show c=16 | 19,900 | 19,883 | 20,293 |
| room_show c=64 | 19,398 | 19,610 | 20,159 |
| messages_page c=64 | 23,189 | 23,363 | 23,388 |
| sidebar c=64 | 12,579 | 12,383 | 12,695 |
| search c=64 | 22,692 | 23,947 | 24,184 |
| avatar c=64 | 380,817 | 459,982 | 458,112 |
| static_css c=64 | 459,547 | 474,015 | 464,876 |
| up c=64 | 133,564 | 134,325 | 132,590 |
| post_message c=16 | 5,475 | 5,434 | 5,491 |
| post_message c=64 | 5,518 | 5,191* | 5,475 |

Throughput is the same within noise; huge pages bought nothing measurable here. The full
per-route results, latencies included, are in each run's `report.md` and `rust-1.json`.

\* That cell also counted 1,805 load generator errors: failed connects or sends, not HTTP errors
(every response was a 200, p99 17 ms). The rerun had none, at the same rate as before; the
errors came from the busy host, not the change.
