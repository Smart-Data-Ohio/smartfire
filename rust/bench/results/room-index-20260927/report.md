# Room paging index: before and after

Measures `index_messages_on_room_id_and_created_at` (added on boot) and the cheaper
`Message::paged` against `main` at `a9a9ba3`, on a room with a long history.

## Setup

- Native release builds, server on CPUs 8–11, `loadgen http` on 12–15, 16 clients, gzip, 2 s
  warm-up then 8 s. One run each (the gap is far outside run-to-run noise). Raw results:
  [`runs.txt`](runs.txt) (`csrf` is `main`, `index` this change).
- The database from [`../header-csrf-20260927`](../header-csrf-20260927/report.md)'s first run, after
  its POST benchmark left 235,767 messages in the room, with the new index dropped, as a database the
  Rails app created would be. The `index` build created the index when it booted.

| Route | Before (req/s) | After | Change |
|---|---|---|---|
| Room page | 95 | 6,051 | 64× |
| Messages page (`?before=`) | 87 | 17,972 | 208× |

After the change, the long room serves as fast as the 80-message room in the earlier reports
(5,886 and 17,336 req/s).

## Queries (sqlite3, 235,767 messages in the room)

| Query | Before | After |
|---|---|---|
| Last page (`ORDER BY created_at DESC LIMIT 40`) | 60 ms (sorts the room in a temp B-tree) | 0.02 ms (walks the index) |
| `paged?` | 3.4 ms (`COUNT(*)` of the room) | 0.02 ms (`LIMIT 1 OFFSET 40`) |
| Creating the index, once | | 49 ms |
