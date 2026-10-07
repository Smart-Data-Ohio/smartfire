# Frontend overrides

Files in `overrides/` shadow the reference app's assets of the same logical path (the path under
`app/javascript`, `app/assets/*` or a vendored gem's asset directory), so the Rust app could change
its frontend without editing the reference Rails app. `build.rs` puts that directory first on the
load path, and skips it when it doesn't exist.

The lead explicitly requested the post-pin #163 status popup. `people.css` and
`controllers/profile_card_controller.js` are exact copies from `2e20b24c`, held here so a build
against the pinned Rails archive also serves the approved status assets. Their compiled bytes
and digests are checked strictly against the post-change Rails precompile. Every other asset
remains pinned; these files contain no Rust-specific frontend changes.
Upstream's JavaScript overrides were removed when the crate was pointed at our tree:

| File | Why it went |
|---|---|
| `models/file_uploader.js` | Dropped the `X-CSRF-Token` header. We keep Rails' token CSRF, so the reference's upload sends it. |
| `controllers/copy_to_clipboard_controller.js` | Added a `url` value for upstream's token-free cached markup. Our markup is our Rails', which passes `content`. |
| `lib/autocomplete/base_autocomplete_handler.js` | Fixed an upstream bug our file doesn't share (ours differs in other ways too). |

Rails at the current reference pin (`parity/reference.sha`, including #151) renders the Edge install icon from
`external/install-edge.svg`. Neither app ships a root `install-edge.svg` override.

Adding one needs the lead's approval: it is a deliberate divergence from the reference.

S8's installed SPA also overrides `initializers/service_worker.js` and
`controllers/notifications_controller.js`. Both read the worker URL rendered by the classic
layout's provisional head metadata and register it at scope `/`. The initializer reconciles
after both page loads and Turbo visits, including sign-in when a stored preference overrides
the signed-out default. Neither override unregisters the root registration, preserving its
push subscription when the UI changes. The copied `web/` assets remain unchanged.
