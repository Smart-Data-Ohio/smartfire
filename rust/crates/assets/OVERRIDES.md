# Frontend overrides

Files in `overrides/` shadow the reference app's assets of the same logical path (the path under
`app/javascript`, `app/assets/*` or a vendored gem's asset directory), so the Rust app could change
its frontend without editing the reference Rails app. `build.rs` puts that directory first on the
load path, and skips it when it doesn't exist.

**There are none.** Until cutover the Rust app serves our Rails app's assets byte for byte (the
digested paths, the import map and the modulepreload set included), so `overrides/` is absent.
Upstream's JavaScript overrides were removed when the crate was pointed at our tree:

| File | Why it went |
|---|---|
| `models/file_uploader.js` | Dropped the `X-CSRF-Token` header. We keep Rails' token CSRF, so the reference's upload sends it. |
| `controllers/copy_to_clipboard_controller.js` | Added a `url` value for upstream's token-free cached markup. Our markup is our Rails', which passes `content`. |
| `lib/autocomplete/base_autocomplete_handler.js` | Fixed an upstream bug our file doesn't share (ours differs in other ways too). |

Rails at the current reference pin (`d7c7de92`, #151) renders the Edge install icon from
`external/install-edge.svg`. Neither app ships a root `install-edge.svg` override.

Adding one needs the lead's approval: it is a deliberate divergence from the reference.
