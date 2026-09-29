# Cached page parts and quieter cookies: before and after

Measures this change against `main` at `898653e` (spliced gzip, header CSRF and the room index): native
release builds, the database from [`../header-csrf-20260927`](../header-csrf-20260927/report.md) (80
messages, a 419 KB room page), server on CPUs 8–11, `loadgen http` on 12–15, 16 clients, gzip, 2 s
warm-up then 8 s, 3 reps alternating the two binaries. Raw results: [`runs.txt`](runs.txt) (`index`
is `main`, `parts` this change).

The host was busy with other work during these runs (1-minute load average 13–17), so absolute
numbers are lower and noisier than on a quiet machine; alternating the binaries keeps the
comparison fair.

| Route | Before (req/s, median) | After | Change |
|---|---|---|---|
| Room page | 5,762 | 16,881 | 2.9× |
| Messages page (`?before=`) | 17,087 | 22,580 | 1.3× |
| Search | 5,692 | 16,097 | 2.8× |
| Sidebar | 15,586 | 17,207 | 1.1× |

## Where it comes from

gzip and the ETag of the real 466 KB room page on one core (release build, scratch benchmark of
`splice::PageParts`):

| | µs |
|---|---|
| Before: SHA-256 of the whole body for the ETag | 192 |
| Before: spliced gzip (messages stored, layout compressed live) | ~270 |
| After: split into parts, ETag from the parts, gzip from stored pieces | 42 |
| After, first request for a changed page (compressing its new pieces) | ~2,000, once |

The compressed page is within a few bytes of before (44,863 vs 44,841 bytes gzipped whole).

## Behavior checked end to end

For the room page, messages page and search: the gunzipped body is byte-identical to the identity
response, the ETag is the same for both encodings and stable across requests, and `If-None-Match`
gets a 304. A signed-in page view sets no cookies once the session has been refreshed in the last
hour and the room is the one last visited.
