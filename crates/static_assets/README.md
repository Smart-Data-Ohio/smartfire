# Retained static assets

`campfire_static_assets` embeds `public/`, `media/{images,sounds,emoji}/` and
`auth/`. `build.rs` bundles retained auth CSS from the frontend's existing token,
motion and font sources via `../retained_pages/auth_build.rs`. It needs no Node,
SPA dist, `web/`, import map or classic stylesheet compiler.

The API provides logical URL helpers (`asset_path`, `try_asset_path`,
`image_path`, `audio_path`, `stylesheet_path`, `javascript_path`, `asset_url`),
`digested_path`, `manifest`, `manifest_json`, and `asset_bytes`. `serve` accepts
`StaticRequest` and returns `StaticResponse`, preserving MIME, GET/HEAD, Range,
conditional requests, encoding negotiation and the public cache policy.
`serve_embedded`, `resolve_asset_path` and `resolve_asset_url` share those rules
with the temporary classic adapter in `campfire_assets`. The server still adds
the existing immutable cache header and applies its existing security policies.

Media digests retain the former SHA1(content + assets version `1.0`) scheme;
auth CSS includes referenced font bytes in discovery order. `legacy-digests.json`
pins all 215 media URLs in the recorded Rails manifest. The build refuses to
assign changed bytes to those immutable URLs: a future change must archive the
old bytes and keep their original URL. Tests pin their SHA256 bytes and the
pre-extraction auth/font paths. Retaining all images also keeps old rich-text
icons and illustrations available without rewriting stored messages.

Brand icon licenses remain alongside their SVGs. The emoji MIT license is in
`media/emoji/GEMOJI-LICENSE`; font licenses stay beside their frontend sources.
Other original media/public/auth inputs retain the repository's MIT license,
copied here as `MIT-LICENSE`.

`campfire_assets` still builds/serves classic JS, CSS, vendored browser bundles,
overrides and import-map/tag helpers. Its build consumes this crate's retained
manifest/bytes to resolve classic CSS/JS references, without embedding another
copy of retained inputs. These adapters go with the later classic deletion.
The server tries classic files (including the combined `/assets/.manifest.json`)
before retained files; PWA worker/manifest/offline routes remain independently owned.
