# Static assets

`campfire_static_assets` embeds `public/` and `media/{images,sounds,emoji}/`. `build.rs`
digests them; it needs no Node, SPA dist or browser inputs.

The API provides logical URL helpers (`asset_path`, `try_asset_path`,
`image_path`, `audio_path`, `asset_url`), `digested_path`, `manifest`,
`manifest_json`, and `asset_bytes`. `serve` accepts `StaticRequest` and returns
`StaticResponse`, preserving MIME, GET/HEAD, Range, conditional requests,
encoding negotiation and the public cache policy. `serve_embedded`,
`resolve_asset_path` and `resolve_asset_url` share those rules for byte
endpoints. The server adds the existing immutable cache header and applies its
existing security policies.

Media digests retain the former SHA1(content + assets version `1.0`) scheme.
`legacy-digests.json` pins all 215 media URLs in the recorded Rails manifest.
The build refuses to assign changed bytes to those immutable URLs: a future
change must archive the old bytes and keep their original URL. Tests pin their
SHA256 bytes. Retaining all images also keeps old rich-text icons and
illustrations available without rewriting stored messages.

Brand icon licenses remain alongside their SVGs. The emoji MIT license is in
`media/emoji/GEMOJI-LICENSE`. Other original media/public inputs retain the
repository's MIT license, copied here as `MIT-LICENSE`.

The server serves the manifest and bytes directly. PWA worker, manifest and
offline routes remain independently owned by `campfire_spa`. The React UI builds with pnpm.
