# Search, message quotes, and the Files tab

## The search field

Every signed-in page has a search field at the trailing end of the top
bar, after the page's own header actions and before the help menu, so
it stays in one spot as you move between pages. Press **/** (outside a text field) or
**Ctrl/⌘+Shift+F** (anywhere) to focus it. Focusing it opens your recent
searches (the last ten, newest first) as a combobox: typing filters
them, **↑**/**↓** move, **Enter** opens the highlighted one or, with
nothing highlighted, submits what you typed, and **Esc** closes the
list. Submitting goes through `POST /searches`, which records the query
in your recent searches and redirects to the results page; the field
keeps the query there. The list's **Clear** button (and the one on the
results page) empties your recent searches in place, without leaving
the page you are on.

The header is a size container: once the member or thread panel (or a
narrow viewport) leaves it under 68rem, the room's header actions drop
their labels to keep the field; under 46rem the field folds into a
search button that expands it over the whole header row. The field
gives up its own width (down to 9rem) before the page title does, so
room names keep their full width at typical desktop sizes.

The results page (`/searches?q=`) reads top down: a way back to your
last room, the result count, operator chips, board/work-thread/event
sections, then the messages. Without a query it shows a short hint and
your recent searches. Only "Load older results" (a request with a
`before` cursor) answers as a Turbo Stream; every other request,
including Turbo form redirects that inherit a Turbo Stream `Accept`
header, renders the page.

## Search operators

The search page (`/searches`) accepts Slack-style operators alongside
plain words. Operators may repeat: multiple `from:` or `in:` values
combine with OR, everything else combines with AND.

| Operator | Meaning |
| --- | --- |
| `from:@name` | Messages whose creator's name contains `name` (case-insensitive; the `@` is optional) |
| `in:#room` | Messages in rooms whose name contains `room` (case-insensitive; the `#` is optional) |
| `has:link` | Messages carrying a link (a stored anchor or a bare `http(s)` URL) |
| `has:file` | Messages with an uploaded file or a Drive attachment |
| `has:image` | Messages with an uploaded image |
| `has:pin` | Pinned messages |
| `before:YYYY-MM-DD` | Messages created before the date |
| `after:YYYY-MM-DD` | Messages created after the date |
| `on:YYYY-MM-DD` | Messages created on the date |
| `is:thread` | Messages posted inside a thread |

Examples: `from:@jz launch`, `in:#designers has:file mockup`,
`on:2026-09-01 deploy`, `is:thread has:pin`. A query of only operators
(`from:@jz`) lists everything matching, newest first.

Whatever remains after the valid operators are removed is searched as
before: every word-character run is quoted as an FTS5 phrase, so words
like `AND` or `NOT` match literally and can never break the query.
Operators whose value is missing or unparseable (`has:bogus`,
`before:2026-13-45`) stay plain text. Every operator value reaches the
database only through bound parameters with `LIKE` wildcards escaped,
so `from:@%` matches a literal percent and quote characters cannot
break out of the query.

Parsed operators render as removable chips above the results; each chip
links back to the same search without its operator, and the query text
stays editable in the top-bar search field. `from:` and `in:` values are single
tokens (no spaces); direct rooms have no name, so `in:` never matches
them. `has:image` only sees uploaded files, because Drive attachments
store no MIME type. Quiet system notes (pin notes and the like) never
match, even for filter-only queries.

Message results page newest-first through "Load older results", as
before. Boards, work threads (outside boards), and events matching the
operator-free text render as capped side sections above the messages,
each scoped to rooms the viewer belongs to and further narrowed by
`in:`; they show only on the first page.

## Message quotes

Pasting a message permalink (`/rooms/:room_id/@:message_id`) quotes the
source under the message as a card showing the author, the room, a
200-character excerpt, and the time, with a jump link to the source.
Direct rooms show a neutral "a direct message" label that is the same
for every viewer, since direct-room names differ per viewer. Quotes in
the same room render inline; quotes from another room load lazily
through a per-viewer frame, and viewers who cannot access the source
room see only a plain "Message in a private room" chip with no author,
excerpt, or time.

A quote follows its source: editing the source refreshes quoting cards
over the room stream through a background job (batched and capped;
quotes past the cap refresh on the next load through the cache key),
and deleting it clears them. Quote state rides in the message fragment
cache key (the sources' newest edit stamp) and in the message list
etag, so cached pages and conditional GETs stay correct. Each message
quotes at most ten permalinks, taken from text outside code spans and
fenced blocks. The sync ignores missing messages, self-links, and
system notes. Thread permalinks (`?thread=&message_id=`) do not quote
yet; only the `/@` form does.

## Files tab

Each channel's header links to a Files tab (`/rooms/:room_id/files`)
listing what the room shared, newest first, in two sections:

- **Uploads**: files attached to the room's messages, with type
  filters (All, Images, Videos, Documents, Other), filename search,
  and cumulative "Load more" paging. Each row shows the filename, size,
  content type, author, date, and a jump link to its message.
- **Drive files**: files pinned through the Drive picker. Only the
  stored file id is used (picker-only data, never a Drive API call),
  so names, kinds, and times resolve per viewer in the browser through
  the same upgrade as message attachments, and Drive rows take no part
  in type filters or filename search.

Both sections scope to room membership: non-members get a 404, and each
section renders a bounded number of queries no matter how many rows the
room holds. Thumbnails are future work; rows show a file icon today.
