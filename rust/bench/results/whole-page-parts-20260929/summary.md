# Store the compressed form of every digested page (2026-09-29)

Pages with cached message fragments (room, messages, search) were split into parts whose
compressed pieces are kept and reused. Every other page, the sidebar among them, was gzipped from
scratch on each request: 41% of the sidebar's CPU (`bench/results/profile-20260929`). Now a page
that `Rack::ETag` digests anyway (a 200 or 201 without an ETag or Last-Modified of its own), and
that has no cached fragments, is one text part: the SHA-256 it already needs keys its stored
piece. Bodies under 1 KB are still compressed afresh.

`bench/attrib --configs native-main,native-parts --reps 3 --concs 1,16 --secs 5`: `native-main` is
`9e2a110`, `native-parts` the same plus this change, both native release builds, interleaved per
rep. The host carried another project's load throughout (1-minute load average 14–18), which hits
both configurations alike.

| Route, 16 clients | Before req/s | After req/s | Change |
|---|---|---|---|
| sidebar | 10,683 [10,631–11,132] | 19,400 [19,019–19,717] | **1.82×** |
| sidebar, CPU per request | 0.35 ms | 0.19 ms | **1.87×** |
| room_show | 16,710 [15,646–17,734] | 16,927 [16,624–17,302] | 1.01× |
| messages_page | 19,651 [18,622–20,181] | 19,149 [19,054–19,983] | 0.97× |
| search | 19,754 [19,380–19,997] | 19,783 [19,281–20,234] | 1.00× |
| post_message | 4,803 [4,463–4,849] | 4,650 [4,524–4,840] | 0.97× |

Response sizes are unchanged. The pages that already used cached parts are the same within noise
(their ranges overlap). Full tables, from `bench/attrib-report`, are in [`report.md`](report.md).
