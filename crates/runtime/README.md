# Shared server runtime

`campfire_runtime` takes the former web layer's authentication, request concerns, Active
Storage serving/uploads, rich text, attachment processing, messaging and mail jobs. Its
presenters load database facts and serialize legacy JSON without classic page templates.
`RequestContext` owns preferences, time zones and retained-page CSRF/flash scopes independently
of the classic application Layout.

`crates/web` re-exports these services and facts for existing callers. It keeps classic page,
fragment, broadcast and HTML-cache adapters, including mail's publication callback. Those
adapters implement local rendering traits on the shared types. No HTML cache was moved here:
JSON cache users access the existing app-owned cache through `campfire_app::cache`.
Retained documents load their auth assets directly through `campfire_static_assets`.

`ws8_runtime_vectors.json` moved with the runtime. Include paths from tests point here; its
contents, cookies and machine payloads are unchanged.
