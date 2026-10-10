# Shared presentation

`campfire_presentation` owns template-free facts used by the SPA API, machine protocols and
classic render adapters: models, constants, time/number/text formatting, icon and reaction
resolution, fragment-key formatting, and legacy JSON serializers. It sits below app and has
no app/controller or Askama dependency.

The sources, registries and formatter vectors came from `crates/views`; plain retained
context and string/JSON helpers moved from `crates/view_kit` to keep this dependency boundary. Classic HTML stays
there; its modules re-export these facts and implement local rendering traits for them.
Serialize legacy payloads with `helpers::to_rails_json` to preserve Rails escaping and field
order. Recorded vectors remain frozen.
