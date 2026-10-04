The behavioral template map is `template-coverage.json`. It reconciles the cutover inventory
at `78b9b1546`: **207 of the 208 declarations have Rails byte receipts; one is not reachable**.
It also inventories newer declarations and port-only files: 283 of 284 Rails declarations have
receipts, and 290 of 298 Rust template files have receipts. The eight remaining Rust files are
retained renderers outside the production path; their active replacements are mapped separately.
The map does not label those live Rails counterparts dead.

Each receipt names an annotated Rust test and its committed Rails oracle. Each declaration
records the fixture branch and render chain. Output witnesses identify an actual string in a
golden (file, JSON pointer and literal). Wrapper/helper/JSON-only declarations instead record
composition evidence, since their source may contain no literal markup. For example:

- `rooms/stage/_role_event` uses the explicitly selected role-event cases in the 400-render
  stage corpus, rather than credit from an ordinary stage page.
- `github/pull_requests/_write_actions` uses the real write-response corpus; a lazy card frame
  does not cover it. Fizzy containers and loaded cards/chips likewise use separate receipts.
- `accounts/users/_next_page_container` uses the new 501-user pagination fixture. An ordinary
  settings page without another page earns no credit.
- `messages/_unrenderable` uses a real GET of a row with a missing creator. Ordinary successful
  message-state renders earn no credit for the rescue branch.

The lone unreachable Rails declaration, `users/autocompletables/_template`, has no render
reference in application controllers, helpers or views. Current room forms construct their
picker directly, and the mention picker serves JSON. The fresh HTML request returns 406.
Port-only retired Lexxy HTML renderers and duplicated/older room/sidebar/agent composition
files are identified explicitly in the Rust map.

The new producer, `reference-tools/template_coverage/http.rb`, runs 20 actual controller
requests over the default seed, freezes the clock and fixes rendering CSRF values. It records
74 ActionView template identifiers and their source hashes. The captured sources were verified
against the pinned `d7c7de92` tree. It captures whole response bytes for streams and whole
`main-content` bytes for application pages. Shared layouts have independent existing byte
receipts; excluding their head avoids conflating the old parity pin with post-pin JS fixes.
Comparisons preserve whitespace, fields, URLs and asset references within the captured region.

Regenerate the new fixture with the pinned reference image:

```sh
PARITY_IMAGE=ws8bm2-reference:d7c7de92 PARITY_NAMESPACE=template-coverage \
  PARITY_OWNER=template-coverage parity/bin/reference runner --seed default \
  --time 2026-03-02T16:00:00Z --freeze reference-tools/template_coverage/http.rb \
  vectors/template_coverage_http.json
```

`coverage/check-templates --write` merges screenshot state IDs with behavioral receipt IDs
without pretending byte tests are screenshots. `coverage/test-templates.py` runs every mapped
receipt plus the whole views suite using four nextest workers. Set `CI=1` so absent seeds fail.
Rust paths resolve from the workspace, and Rails paths follow `CAMPFIRE_REFERENCE` when set.
The views guard rejects missing/stale Rust or Rails entries, missing tests/oracles, unrecorded
or absent branch witnesses, and missing original cutover declarations. Fresh controller receipts
also require the named Rails declaration in that response's ActionView trace. Its negative controls
remove an entry, empty a receipt, invent a test name, remove a branch witness and substitute
a different controller branch. The empty map failed first with 298 missing Rust files.

The production room shell (`rooms::Show`) and the separate composition renderer each have
their own full-page receipts. The three composition composers explicitly select Drive `none`,
`legacy` and `picker` in cases 0, 3 and 4. Their shared form markup alone is not evidence
that all three renderers ran. The composition navigation fallback has no production caller;
all 38 composition-page cases supply the separate navigation renderer instead.

The fresh controller test failed before the fixes on pagination append indentation, the
missing-creator rescue partial's surrounding newlines, and the deleted-room response's final
newline. The additional original-room case exposed missing whitespace around the welcome
invitation on an empty timeline. The populated invitation also exposed a redundant empty
expression boundary in the mounted message list. Four Rust templates and that list adapter
now preserve Rails' exact bytes; the reference outputs are generated, not edited. Prior room
receipts used different rooms, more than 40 messages, or an explicitly false invitation input.
The new requests cover the original room both empty and after posting a real message.

Validation commands (from `rust/`, with `CI=1`, all three seeds present and nextest on `PATH`):

```sh
mise exec rust@1.98.1 -- python3 parity/coverage/test-templates.py
mise exec rust@1.98.1 -- cargo nextest run --locked -p campfire_views --test template_coverage -j 4
mise exec rust@1.98.1 -- cargo clippy --locked --workspace --exclude html5ever --all-targets -- -D warnings
ruby parity/coverage/check-templates
```

The nine screen follow-ups are reconciled below (`src/` paths are relative to `crates/campfire/`). Template coverage establishes server output;
it does not close a named browser/release declaration merely by citing a domain test. Existing
behavior scripts and domain ledgers remain the acceptance source for those declarations.
Pure pixel/geometry-only work is excluded.

| Follow-up | Existing behavioral owner/evidence | Boundary remaining outside this template run |
| --- | --- | --- |
| Successful 2FA/session lifecycle | `src/app/two_factor_tests.rs`, `src/app/admin_two_factor_tests.rs`; auth full-page receipts | Browser enrollment and one-time reveals stay in the security acceptance ledger. |
| Permission boundaries | Bot/access/member-panel HTTP and golden matrices in the map | No grant from an empty or denied page to its populated branch. |
| CRUD transitions | Human-work, board, message, Slack, GitHub and fresh replacement/create/delete response receipts | Browser mutation declarations remain with their named behavior scripts. |
| Cross-user realtime | Huddle grant/stream socket receipts; message/provider broadcast tests; `parity/behavior/messages.mjs` | Sender/recipient browser timing remains a browser assertion. |
| Agent execution | `src/controllers/agent_http_tests.rs`, bot contracts and agent runtime/event/job tests | Provider execution lifecycle remains with the API/runtime ledger. |
| RTC | `src/controllers/rooms/remaining_call_tests.rs`, `src/channels/huddle_effects_tests.rs`, stage/participant render matrices | Microphone/camera permission and real media transport are not template bytes. |
| OAuth and Picker | `src/app/google_api_tests.rs`, Google profile/login receipts; `parity/behavior/picker-readiness.mjs` | External consent and the real Picker SDK require provider/browser acceptance. |
| Periodic jobs | `src/controllers/message_features/periodic_delivery_tests.rs`, `src/jobs/periodic.rs`; scheduled behavior script | Scheduler lifecycle remains with its durable queue/job acceptance. |
| Installed/offline PWA | `src/controllers/pwa.rs`, exact manifest/worker receipts | Installed service-worker lifecycle needs a real browser, beyond static endpoint bytes. |
