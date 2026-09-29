# Spliced gzip: before and after

Measures the change that splices precompressed message fragments into gzipped pages
(`crates/kit/src/deflater/splice.rs`) against the commit before it (`3fe9f61`).

## Setup

- Native release builds of both commits (the workspace's release profile), not Docker images:
  Docker wasn't available on the host. The Rails reference wasn't run, so there are no Rails
  ratios here; `bench/run` gives those.
- A fresh database filled over HTTP by [`fill.py`](fill.py): the first-run account and 80 posted
  messages of mixed rich text (links, lists, code) in one room. Its room page is 466 KB
  uncompressed (the seed's is 464 KB).
- Server pinned to CPUs 8–11, `loadgen http` to 12–15, 16 concurrent clients, `Accept-Encoding:
  gzip`, a 2 s warm-up then 8 s measured, 3 reps with the two binaries alternating. Host load
  average ~1. Raw results: [`runs.txt`](runs.txt).

## Throughput (req/s, median of 3)

| Route | Before | After | Change |
|---|---|---|---|
| Room page | 2,527 | 5,461 | 2.16× |
| Messages page (`?before=`) | 3,709 | 16,523 | 4.45× |
| Search | 2,123 | 5,526 | 2.60× |

The spread across reps was under 1.5% for every cell.

## Size and correctness

| Route | Before (gzip bytes) | After | Change |
|---|---|---|---|
| Room page | 44,741 | 44,893 | +0.3% |
| Messages page | 37,921 | 38,238 | +0.8% |
| Search | 54,972 | 55,464 | +0.9% |

For each route the gunzipped body is byte-identical to the `identity` response (after masking the
per-request CSRF token), and the response headers are unchanged (`Transfer-Encoding: chunked`, no
`Content-Length`).

## Where the time went

gzip of the 466 KB room page on one core, from a scratch benchmark of `splice::gzip`:

| | µs |
|---|---|
| Whole body at level 6 (before) | 1,032 |
| Spliced (after) | 271 |
| of which: live head (37 KB) / live tail with dictionary (18 KB) | 134 / 93 |
| of which: locating fragments / looking up pieces / CRC-32 | 11 / 3 / 6 |

Compressing each message independently was rejected: it made the room page 4.4× larger,
because consecutive messages share most of their markup. Chaining each piece to its predecessor
through a preset dictionary keeps what they share.
