# GIF API for the SPA

The picker UI can import `gifActions` from `frontend/src/sync/gif-actions.ts`.
Its async functions use the application's authenticated API client and return
generated DTOs. `send` accepts a room id, an optional thread id, a slug, a
stable client message id, and optional Markdown. Keep the same client message
id when retrying. It returns the stored `MessageDTO` and updates the message
store after success.

| Method | Endpoint | Response type |
| --- | --- | --- |
| `config()` | `GET /api/v1/gifs/config` | `GifConfig` with `enabled` |
| `search(query, page = 1)` | `GET /api/v1/gifs/search?q=...&page=...` | `GifList` |
| `trending(page = 1)` | `GET /api/v1/gifs/trending?page=...` | `GifList` |
| `suggestions(query)` | `GET /api/v1/gifs/suggestions?q=...` | `GifSuggestions` |
| `autocomplete(query)` | `GET /api/v1/gifs/autocomplete?q=...` | `GifSuggestions` |
| `categories()` | `GET /api/v1/gifs/categories` | `GifCategoryList` |
| Preview URL | `GET /api/v1/gifs/media?u=...` | Image or video bytes |
| `send(input)` | `POST /api/v1/rooms/:id/messages` or `/threads/:id/messages` | `MessageDTO` |

`GifList` carries `items`, `page`, and `hasNext`. Each `GifItem` has `slug`,
`klipyId` as a string, `title`, `attribution`, `previewUrl`, nullable
`stillUrl`, `width`, and `height`. Use the returned proxy paths. When there
is no still, `stillUrl` is null. Queries accept 1 to 200 characters and pages
1 to 10000. Terms replies contain at most ten suggestions.

`CreateMessage.gifSlug` also works on thread openers and board post briefs.
It can accompany `attachmentSignedIds` within the ten-file limit. It cannot
accompany the legacy `attachmentSignedId`. The server resolves the slug,
downloads a rendition, and commits its blob with the message. Rejected model
writes and concurrent duplicate sends do not retain unused imported blobs.
Image GIF attachments use the stored animated original as their thumbnail URL.
The mock uses the same contract. Client uploads cannot set reserved `klipy_`
metadata; GIF/WebP imports require a blob-bound server signature for animated previews.
The stored blob keeps `uploader_id`, `klipy_id`, `klipy_slug`, `klipy_title`,
and `klipy_attribution` in existing metadata. No schema migration is needed.

The deploy's Google integration configure step reads `KLIPY_API_KEY` from the `production` GitHub Actions environment in `Smart-Data-Ohio/smartfire`; an unset or empty secret preserves the VM's existing value.

Every endpoint requires sign-in. Writes require the normal CSRF token.
Without `KLIPY_API_KEY`, config returns `enabled: false` and discovery,
media, and new GIF sends return `404 NotFound`. Provider failures return
`503 Unavailable`. Rate limits return `429 RateLimited` with `Retry-After`.
Configuration and limits are in [self-hosting.md](self-hosting.md#gif-search-optional).

The Effect functions live in `frontend/src/api/gif-endpoints.ts`. Runtime
schemas in `frontend/src/api/schema/gif.ts` pin against generated types.
`pnpm gen` regenerates them from `crates/api_types/src/gif.rs`.

The mock server enables GIFs by default. `createMockServer({ gifsEnabled:
false })` exercises the unavailable state. `frontend/mock/fixtures/gifs.ts`
provides `hello-wave` and `celebration-cheer`, two-frame GIF bytes and a PNG
still. Preview and stored-file paths work through Vite mock middleware and
the test mock Fetch transport. No key or external media request is needed.

The native request paths, pagination, `file[size][format]` responses, and
terms responses were checked against [KLIPY GIF docs](https://docs.klipy.com/gifs-api)
and [suggestions docs](https://docs.klipy.com/search-suggestions-and-autocomplete).
The allowlist follows [KLIPY network requirements](https://docs.klipy.com/network-requirements).
The native docs document `content_filter=high`, but do not document
`rating=pg`. Requests send both. The effect of `rating`, and extra customer
and filtering parameters on categories, items, and terms, remains partial
without a live key. Production proxying and rehosting need the custom
agreement described by [KLIPY integration requirements](https://docs.klipy.com/integration-requirements).
