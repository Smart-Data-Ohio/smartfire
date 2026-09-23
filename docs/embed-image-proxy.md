# Embed image proxy

Link preview cards show the linked page's OpenGraph image. The image bytes
are served from Smartfire itself through `GET /embeds/image/:signed`, so a
viewer's browser never contacts the remote host: no viewer IP address,
cookies, or `Referer` leak to the linked site. The server-side fetch runs
from the Smartfire host with no cookies.

## How it works

Every embed `<img>` points at the proxy with a signed source URL
(`Embeds::ImageProxy.signed_path`), rendered by
`action_text/attachables/_opengraph_embed`. The signature
(`embed_image` message verifier) binds the exact remote URL, so the
endpoint is not an open proxy: it serves only image URLs the server
itself rendered into an embed. Tampered signatures answer 404 without
touching the network, and the endpoint requires a signed-in member.

Fetches follow the same rules as the unfurl fetcher
(`Opengraph::Fetch`):

- Each hop resolves through `RestrictedHTTP::PrivateNetworkGuard` and the
  connection is pinned to the resolved public address. Loopback, private,
  and unresolvable hosts are refused with 404, including redirect
  targets. Redirects are capped at 10; non-HTTP targets are denied.
- Only `200` responses with a raster image content type
  (`jpeg`, `png`, `gif`, `webp`, `avif`, `bmp`, `ico`) are served.
  Anything else — HTML, text, and SVG in particular — answers 502.
  SVG is excluded on purpose: an `<img>` cannot run its scripts, but the
  same proxied URL navigated to directly would run them as same-origin
  script.
- Bodies are capped at 5 MB: an excessive `Content-Length` is rejected
  before reading, and the body is also read in chunks with the same cap
  in case the header lies. Timeouts, connection failures, and upstream
  errors answer 502. Every failure body is empty.

Successful responses carry `Cache-Control: public, max-age=3600` and
`Content-Disposition: inline`. The signature is deterministic per source
URL, so cached message fragments keep stable `src` attributes.

## Integration note

The `w2/link-embeds` branch renders its own card image directly from
`embed.image_url` (`app/views/link_embeds/_card.html.erb`). When that
merges, its image tag should use `Embeds::ImageProxy.signed_path` too,
which closes the per-viewer image disclosure documented in that
branch's privacy section.
