# Shared presentation

`campfire_presentation` owns template-free facts used by the SPA API and machine protocols:
models, constants, time, number and text formatting, icon and reaction resolution,
freshness keys, and legacy JSON serializers. It sits below app and has no app, controller
or Askama dependency.

The sources, registries and formatter vectors came from the retired classic presentation
layer. Retained context and string/JSON helpers moved from `crates/view_kit` to keep this
boundary. Serialize legacy payloads with `helpers::to_rails_json` to preserve Rails escaping
and field order. Recorded vectors remain frozen.
