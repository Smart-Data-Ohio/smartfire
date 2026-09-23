# Link embeds and LinkedIn post cards

A Markdown message containing links renders previews beneath the message:

- **Generic links** render Discord-style embeds: the site name, the linked
  title, a short description, and the page's OpenGraph image as a
  thumbnail. Up to 3 links per message.
- **LinkedIn post links** (`linkedin.com/posts/…` and
  `linkedin.com/feed/update/urn:li:(activity|share|ugcPost):…`) render a
  LinkedIn-styled card: the LinkedIn mark, the fetched title and excerpt,
  the page image, and a "View post on LinkedIn" link. A "Show embedded
  post" button swaps in LinkedIn's official embed player on click only.

The stored message body is never changed; references live in the
`link_embed_references` join table and are re-synced when a message is
created or its Markdown source is edited.

## Which URLs get embeds

`LinkEmbed::UrlClassifier` scans the rendered message text outside code
spans and fenced blocks (labeled links resolve through their hrefs) and
keeps up to 3 unique URLs in order of appearance. These URLs are skipped
— they have their own cards or need none:

- GitHub pull request URLs (PR cards) and X post URLs (X cards)
- LinkedIn post URLs (LinkedIn cards, synced through the same rows)
- Google Drive file URLs (preview chips) and Fizzy workspace URLs
- Internal permalinks (anything under `/rooms/<id>`: message links,
  event links, thread links)

Only Markdown messages sync embeds. Legacy Trix bodies already carry
their stored OpenGraph attachments, and a second card beneath would
render every old unfurl twice.

## Suppression and removal

- Wrapping a link in angle brackets suppresses its embed, as in Discord:
  `<https://example.com>` renders as a plain link with no card. The
  brackets are read from the raw Markdown source.
- The author can remove all embeds from a message with the **Remove
  embeds** menu action. It sets `messages.embeds_suppressed` and clears
  both card containers via Turbo Stream; the references stay so a later
  edit still re-syncs them. Only the author sees the action, and only
  while the message has at least one renderable embed (a generic card
  with fetched text, or any LinkedIn card — login-gated pages still
  render a link chip).

## Where the data comes from

Embed rows are keyed by normalized URL (downcased host, default ports
and fragments dropped) in the `link_embeds` table, shared by every
message linking the page. Fetching happens in `LinkEmbed::FetchJob`,
never inline in a request: on first reference, and again once the cached
result outlives its TTL — 24 hours for a usable preview, 1 hour for a
negative result (login gate, missing page, network failure). Rendering a
stale card re-enqueues its fetch, so an expired result refreshes on view;
the fetch-request claim still bounds this to one enqueue per URL per
10-minute window. The job is idempotent, safe to enqueue concurrently,
and records failures as `fetch_error` instead of retrying forever.

The fetch client (`LinkEmbed::Fetcher`) reuses the existing
`Opengraph::Location`/`Opengraph::Fetch` pipeline, so it inherits the
SSRF guard (hostnames resolved through `PrivateNetworkGuard` with the
resolved address pinned for the connection, redirect targets
re-resolved), explicit 5 s open/read/write timeouts, the 5 MB body cap,
and no cookies (no `Cookie` header is ever set). On top of that each
embed fetch has an overall 10 s deadline across all its redirects and
reads (`LinkEmbed::Fetcher::FETCH_DEADLINE_SECONDS`, enforced with
`Timeout`, with `Net::HTTP`'s silent idempotent retry disabled so the
retry cannot swallow the deadline's own fire) and follows at most 3
redirects (`LinkEmbed::Fetcher::MAX_REDIRECTS`); a slow drip or a
redirect loop records the usual negative result. The parser
(`LinkEmbed::MetadataParser`) reads OpenGraph tags first, then the
Twitter-card equivalents, then the document `<title>` and meta
description; `og:site_name` falls back to the page host. There is no
image proxy, so card images render directly from their `https:` URL
(`img-src` is already https-wide); only public hosts whose image answers
a HEAD request with an image content type are kept, and anything else
renders without its image. Everything from the page is treated as
untrusted text: tags are stripped and the values are truncated and
escaped on render, and the response body is never logged.

After a fetch, the card container is broadcast via Turbo Stream replace
to each room (or thread) with a referencing message, over the existing
membership-gated message stream, so embeds fill in live.

## LinkedIn specifics

LinkedIn post pages are usually login-gated. When the public page yields
no usable OpenGraph data, the card degrades to a compact link chip ("in
**View post on LinkedIn") instead of rendering nothing. `/posts/` URLs
carry no URN, so only `/feed/update/urn:li:…` links offer the "Show
embedded post" button, which loads
`https://www.linkedin.com/embed/feed/update/<urn>` in an iframe on click
— the player (and its tracking) never loads until the reader asks. Only
numeric URN ids are recognized (`urn:li:(activity|share|ugcPost):<digits>`,
enforced in `Linkedin::PostUrl::PATTERN`), so a crafted link cannot smuggle
an unexpected path into the player URL. The
Content Security Policy allows that host in `frame-src` (report-only,
like the rest of the policy; see the initializer comment). No LinkedIn
brand icon ships with the workspace set, so the card uses a small CSS
"in" badge in LinkedIn blue.

The click-to-load iframe carries
`sandbox="allow-scripts allow-same-origin allow-popups"` plus
`referrerpolicy="strict-origin-when-cross-origin"`. That is the minimum
the official player needs, verified against the live player in headless
Chrome: scripts run the player, same-origin keeps its storage and
requests working, and popups let its links open new tabs. Deliberately
withheld: `allow-forms`, `allow-top-navigation` (the player can never
navigate the Campfire page), and `allow-downloads`. The referrer policy
keeps room URLs out of LinkedIn's logs: LinkedIn sees only the Campfire
origin, never which room or message linked the post.

## Accessibility and layout

Cards are plain articles, links, and buttons: fully keyboard operable
with visible link text, decorative images hidden from assistive
technology (`alt=""` plus `aria-hidden` on their redundant links), and
no animation to reduce. Layouts stack at phone width.
