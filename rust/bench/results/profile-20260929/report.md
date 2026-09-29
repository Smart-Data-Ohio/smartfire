# CPU profiles of `main` at `9e2a110` (2026-09-29)

`bench/profile cpu --label main` with gperftools' `libprofiler.so` (SIGPROF at 1 kHz), native
release build with line tables, server on cores 8–11 and load generator on 12–15. Each target's
rollup is beside this file (`cpu-<target>.top.md`); the folded stacks the shares below were
computed from are ~150 MB each, so they aren't committed (`bench/.gitignore`) and come back by
rerunning the command.
Another project's fuzzer ran on cores 2, 6 and 18–23 throughout; a profile's shares are less
sensitive to that than throughput is.

The question was whether SIMD (as in omacom/ttfx: `#[target_feature]` tiers dispatched at runtime,
vector byte scans) would pay off here. Share of each route's CPU, inclusive:

| Share of CPU | room_show | messages_page | search | sidebar | post_message |
|---|---|---|---|---|---|
| gzip (zlib-rs) | 4.2% | 4.5% | ≈2% | **41.2%** | 10.0% |
| Handing reads to database threads (futex `syscall`) | 10.7% | 13.2% | 15.4% | 6.2% | 8.4% |
| `memcpy`/`memmove` (glibc, already AVX-512) | 11.7% | 10.6% | 7.2% | 5.4% | 5.9% |
| SHA-256 (sha2, already SHA-NI) | 7.7% | 0.5% | 6.1% | 4.2% | 0.7% |
| HTML/JSON escaping (scalar; the SIMD candidate) | 1.7% | 0.2% | 1.3% | 9.8% | 6.1% |
| SQLite | 11.5% | 14.1% | 21.6% | 10.8% | 40.9% |

- The byte-heavy work already runs in vectorized code chosen at runtime: zlib-rs, sha2 (SHA-NI),
  aes-gcm and polyval (AES-NI, carry-less multiply) and glibc's `memcpy`.
- Escaping is the one scalar byte loop that shows up: worth a few percent on the sidebar and on
  posts, and nothing on the pages served from cached parts.
- The 10,000-client fan-out spends 61% of its CPU in the `writev` system call (the kernel's TCP
  send path) and 27% in the cable code; nothing there is a byte loop.
- The largest single item was the sidebar's 41% in gzip: it has no cached message fragments, so it
  never got cached page parts and was compressed afresh on every request. Fixed in
  [`whole-page-parts-20260929`](../whole-page-parts-20260929/summary.md).
