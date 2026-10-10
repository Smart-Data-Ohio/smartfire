# Shared server runtime

`campfire_runtime` owns authentication, request concerns, Active Storage serving and uploads,
rich text, attachment processing, messaging, and mail jobs with their JSON publication.
Its presenters load database facts and serialize legacy JSON without application templates.
`RequestContext` owns preferences, time zones and retained-page CSRF and flash scopes.
Retained-page macros live in `presenters::page`; request context is reexported by
`presenters::view_context`. Auth assets come from `campfire_static_assets`.

Legacy JSON values use the explicit bounded `campfire_app::json_cache` store.
There is no HTML fragment cache or ambient cache scope. Message mutation publication lives
in `campfire_messages::controllers::messages::rendered`.

`ws8_runtime_vectors.json` moved with the runtime. Tests include it here; its contents,
cookies and machine payloads are unchanged.
