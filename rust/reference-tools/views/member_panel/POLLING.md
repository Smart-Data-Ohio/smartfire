`polling.sh` builds the raw HTTP oracle for `Rooms::MembersController` from
the pinned Rails image and the reference-built `default` seed. `--check`
regenerates into scratch and compares every committed byte. Build the seed
with `parity/bin/seed build default` first; run with a workstream-owned
`PARITY_NAMESPACE`, `PARITY_OWNER` and the pinned `PARITY_IMAGE`.

`polling_cases.json` supplies the same SQL fixture setup to Rails and Rust.
Each scenario uses a private seed copy (Rust) or rollback transaction (Rails)
and a frozen clock. The tests use the seed's Rails-issued verified session
cookies. They require no scratch snapshots or untracked input files.

The 53 responses cover all 13 members-controller source cases, additional
status/presence/agent boundaries, raw JSON escaping, fresh avatar stamps,
viewer-specific stars, exact cache headers and conditional requests. Three
responses use the complete, unmodified parity seed: open, closed and direct
rooms. Nine Rust test groups compare the entire body and relevant headers.
These are Rust parity assertions; the original Ruby test file is not executed.

Pinned Rails leaves HTML entities and JS separators unescaped in this
`render json:` response. Rack adds an ETag despite `Cache-Control: no-store`.
Its middleware accepts an exact validator (304), while a wildcard or list
returns 200. A different viewer's star projection changes that validator.
Controller-level `fresh_when` keeps its separate wildcard/list semantics.

Agent reads and working-presence expiry call WS11's real `Agent` model.
Human status and lease readers are flagged read-only extractions from WS17
604fa0fe910fd597b903477742a725ff0badd727, pending that owner's merge. The small
viewer-qualified star query is flagged for WS8b's User::Starring API. Browser
interactions and end-to-end parity remain in the later end-to-end phase.
