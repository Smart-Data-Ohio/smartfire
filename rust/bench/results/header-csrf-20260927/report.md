# Forgery protection by Sec-Fetch-Site: before and after

Measures dropping CSRF tokens for `Sec-Fetch-Site` checks against the spliced-gzip commit
(`abcaa09`), the same way as [`../splice-20260927/report.md`](../splice-20260927/report.md): native
release builds, a fresh database filled by [`../splice-20260927/fill.py`](../splice-20260927/fill.py)
(80 messages, a 419 KB room page), server on CPUs 8–11, `loadgen http` on 12–15, 16 clients, gzip,
2 s warm-up then 8 s, 3 reps alternating the two binaries. Raw results: [`runs.txt`](runs.txt)
(`new` is the spliced-gzip build, `csrf` this change).

| Route | Before (req/s, median) | After | Change |
|---|---|---|---|
| Room page | 5,406 | 5,886 | +8.9% |
| Messages page (`?before=`) | 16,345 | 17,336 | +6.1% |
| Search | 5,531 | 6,088 | +10.1% |

Spread across reps was under 1% in every cell.

## Behavior checked end to end

- Two loads of the room page are byte-identical with the same ETag, and `If-None-Match` with it
  gets a 304 (before, every page carried a fresh masked token, so ETags never matched).
- The page has no `csrf-token` meta tag and no `authenticity_token` fields.
- Posting a message: `Sec-Fetch-Site: same-origin` 200; no header over plain HTTP 200;
  `cross-site` 422; `same-origin` with a foreign `Origin` 422.
- The served `models/file_uploader.js` sends no `X-CSRF-Token`.

## Found along the way

A room with a long history makes its page much slower, before and after this change. With 236k
messages in the room (left by an earlier POST benchmark), the room page fell from ~5,800 to
~140–270 req/s. `messages` has no `(room_id, created_at)` index, so `last_page`'s
`ORDER BY created_at DESC LIMIT 50` sorts the whole room: 60 ms per query, 0.02 ms with the index.
`Message::paged` also counts the whole room (3.4 ms) to learn whether it has more than 50.
