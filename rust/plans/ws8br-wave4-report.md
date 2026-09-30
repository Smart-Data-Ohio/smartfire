# WS8br Wave 4 — PARTIAL

Branch `rust/ws8br-rooms-http`, assigned worktree `rust-ws8br`, Rails pin `d7c7de92`. Received base `c4849d54`; main `21a7332f` is already merged through `9f356e2f`. Pushed read/quiet-refresh slice: `9a5fcea5`. Pushed join/preview slice: `e015d50e`. Pushed channel audit/header slice: `8bdb43ec`. This continuation also ports channel name/member-ID casts, request-dependent partial failures and the inherited direct-show callback failure. No PR or deployment.

## Ownership and retained work

The lead reassigned users/profiles/avatars/cards/bans/time zones, account pages and their re-diff, public/welcome/first-run/tour/PWA/QR to **WS8br2**, starting from `c4849d54`. None were implemented here. Shared layout, sidebar presenters, `RoomView`, `ShowView`, and the message-list seam remain stable.

Earlier pushed work remains: room authorization/soft-delete/durable enqueue, group DM create/reuse/rename/add/leave, category/favorite/inbound token controllers, switcher JSON, full sidebar sections and state, header/recipient broadcasts, empty-room shell regions and unread presenters. Historical detail and failing-first receipts are preserved in this report at `c4849d54`; the commands below are this continuation's actual reruns.

## Changes by file

- `controllers/rooms/reads.rs`, `controllers.rs`, `controllers/rooms.rs`: route both reads actions through normal auth/CSRF and alive membership lookup. POST advances to the newest root; DELETE accepts only a root in this room. JSON field order matches Rails. Broadcast only after the write commits, to the requester's reads/unreads stream.
- `db/models/membership.rs`: small flagged WS8a seam `mark_unread_before`, ordered by `(created_at,id)` with null predecessor at the first root; no-op saves retain the timestamp. Both read paths validate room/user/category association ownership. Stage/voice-specific validation and callback completion remain WS13; this seam changes no stage role/grant state and emits no model broadcast.
- `controllers/rooms/refreshes.rs`: quiet 204 before format/template negotiation, accounting for the pin-change stamp. Existing message rendering stays untouched. Changed pin count/list rendering is still a WS8b-m integration gap.
- `controllers/rooms/reads_tests.rs`: five combined HTTP tests cover read/unread state, first/second boundaries, another user's untouched membership, nonmember/deleted/bot guards, foreign/missing/thread rejection, tied timestamps and failed write rollback.
- `channels/tests/reads_test.rs`, `hub_test.rs`, `directory_test.rs`, `channels/broadcasts.rs`: real socket test proves both requesting-user sessions receive the actual Rails frames, another member stays silent, and rejected IDs, failed writes and removed access publish nothing. The existing session helper gains sibling-test visibility only.
- `reference-tools/rooms/reads_refresh.rb`, `vectors/reads_refresh.json`, `reads_discrimination.py`: real pinned Rails HTTP/status/state/recipient oracle; compiled failure against the received implementation, with source restored.
- `reference-tools/rooms/deferred_inventory.py`, `plans/ws8br-rails-cases.json`: retain all source case names/hashes while explicitly assigning the split-off files to WS8br2. No full Rails-file execution claimed.

- `controllers/rooms.rs`, `rooms/{opens,closeds}.rs`, `controllers.rs`: open nonmember preview and rejoin; aliases redirect through the same show-only fallback. Private/direct/voice/stage/board, deleted and missing rooms refuse rejoin; all mutation lookups remain scoped. Normal auth/CSRF, last-room cookie and root alert follow Rails.
- `db/models/membership.rs`: additional small WS8a `join_open` seam validates the open/alive room and user, inserts with the normal transaction timestamp/default involvement, and checks fresh membership inside the serialized writer. Repeated joins preserve the membership and publish nothing; no creation restriction or audit is added.
- `views/src/rooms.rs`, `templates/rooms/join.html`, `views/tests/room_join.rs`: escaped join-page body/nav bytes from Rails, exact title/body/sidebar facts. Existing shared layouts and presenters were not changed.
- `controllers/rooms/join_tests.rs`, `channels/tests/join_test.rs`, oracle/vector/discrimination: four combined HTTP tests cover all six Rails preview/rejoin declarations, double submit, bot/auth/CSRF, failed insert, and exact row HTML over a real joiner socket with another member silent.

- `rooms/{opens,closeds}.rs`: audit creation after the room/member commit; closed revisions snapshot actual granted/revoked names and audit only a diff after commit. An audit failure returns 500 with completed writes retained; model removal frames still precede the failure, controller row/header frames do not follow it.
- `rooms.rs`: small request-independent shared-header rendering helper; open update sends row then header globally, closed update sends rows then headers to retained members only. Presenter/layout APIs remain unchanged.
- `channel_audits_tests.rs`, `channels/tests/channel_audits_test.rs`, `channel_audits.rb`, vector and discriminator: three HTTP tests plus one real socket test verify actor/target/labels/details/IP, no-op auditing, forbidden update stability, post-commit outages, eight Rails HTTP transitions, 27 exact recipient HTML deliveries and the removed session's ordered disconnect. The audit comparison explicitly supplies the same proxy IP as Rails; the test helper otherwise has no peer address.
- Deferred inventory also includes the mixed `audit_log/rooms_audit_test.rb`: room cases WS8br, account cases WS8br2.

- `rooms.rs`: permitted scalar room names follow Active Model String casts (`t`/`f`, nil, numbers); unpermitted arrays/hashes are absent on update. Closed member IDs follow integer casts and nested-array predicates without flattening hashes. Nil name versus empty-string HTML/data attribute bytes remain a broader presenter acceptance gap.
- `rooms/closeds.rs`: request format governs the shared HTML partial lookup. JSON/XML/Turbo-only requests fail with 500 after domain/audit commits, before controller rows/headers; HTML, mixed JSON/HTML and wildcard succeed, matching the pinned Rails bug.
- `rooms/directs.rs`: namespace show inherits a missing `@room` callback at the pin and returns 500 after auth; the generic member-scoped room page remains working. Anonymous users still redirect and bot keys still receive 403.
- `coercions_tests.rs`, `coercions.rb`, `room_coercions.json`, discriminator: five HTTP tests compare 16 creates, 16 updates, eight ID casts, six partial formats and four direct-show callback failures against real Rails. They also verify persisted membership/audit state after rendering failure, plus generic page access guards.

No schema, Rails source, mask, parity allowlist, message-list/composer internals or owner-rendered partial changed. No new ignored test.

## Current verification

All commands rerun from this worktree; metadata alone ran from `rust/`. Both pinned `default` and `first_run` seeds exist. CI=1 forbids silent seeded skips. Three explicit ignores remain: recorder, push latency, and `manages_bots` pending WS11.

```bash
TMPDIR="$PWD/../.scratch" CARGO_TARGET_DIR="$PWD/target" mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 >/dev/null
```

Exit 0, no output.

```bash
python3 rust/reference-tools/rooms/check_workspace.py > .scratch/metadata-duplicates.log
python3 rust/reference-tools/rooms/deferred_inventory.py > .scratch/deferred-inventory.log
```

```text
Cargo TOML duplicate-key check: all manifests parse; no duplicate workspace dependency keys
Rails deferred inventory: 58 files, 512 source-declared cases; 0 Rails tests run, 0 Rails passes claimed
```

oracle:

```bash
PARITY_NAMESPACE=ws8br PARITY_IMAGE=ws8br-reference-d7c7de92 PARITY_OWNER=ws8br rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze -e RAILS_LOG_LEVEL=fatal rust/reference-tools/rooms/reads_refresh.rb > .scratch/reads-refresh.json 2> .scratch/reads-refresh-oracle.log
```

```text
Rails reads/refresh oracle: 9 read cases, 3 quiet refresh formats; reference d7c7de92
```

discrimination:

```bash
python3 rust/reference-tools/rooms/reads_discrimination.py > .scratch/reads-discrimination-summary.log 2>&1
```

```text
test result: FAILED. 0 passed; 5 failed; 0 ignored; 0 measured; 345 filtered out; finished in 0.53s
Reads discrimination: five compiled HTTP regressions rejected c4849d54; source restored
```

join-oracle:

```bash
PARITY_NAMESPACE=ws8br PARITY_IMAGE=ws8br-reference-d7c7de92 PARITY_OWNER=ws8br rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze -e RAILS_LOG_LEVEL=fatal rust/reference-tools/rooms/join.rb > .scratch/join.json 2> .scratch/join-oracle.log
```

```text
Rails join oracle: 9 HTTP cases, 2 join-page regions; reference d7c7de92
```

join-discrimination:

```bash
python3 rust/reference-tools/rooms/join_discrimination.py > .scratch/join-discrimination-summary.log 2>&1
```

```text
test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 346 filtered out; finished in 0.48s
Join discrimination: four compiled HTTP regressions rejected legacy lookup and absent dispatch; source restored
```

audit-oracle:

```bash
PARITY_NAMESPACE=ws8br PARITY_IMAGE=ws8br-reference-d7c7de92 PARITY_OWNER=ws8br rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze -e RAILS_LOG_LEVEL=fatal rust/reference-tools/rooms/channel_audits.rb > .scratch/channel-audits.json 2> .scratch/channel-audits-oracle.log
```

```text
Rails room audit oracle: 8 committed HTTP transitions; reference d7c7de92
```

audit-discrimination:

```bash
python3 rust/reference-tools/rooms/channel_audits_discrimination.py > .scratch/channel-audits-discrimination-summary.log 2>&1
```

```text
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 351 filtered out; finished in 0.50s
Channel audit discrimination: three compiled HTTP regressions rejected e015d50e; source restored
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 353 filtered out; finished in 7.52s
Channel header discrimination: compiled missing-header socket regression rejected; source restored
```

coercion-oracle:

```bash
PARITY_NAMESPACE=ws8br PARITY_IMAGE=ws8br-reference-d7c7de92 PARITY_OWNER=ws8br rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze -e RAILS_LOG_LEVEL=fatal rust/reference-tools/rooms/coercions.rb > .scratch/room-coercions.json 2> .scratch/room-coercions-oracle.log
```

```text
Rails room coercion oracle: 16 create casts, 16 update casts, 4 direct-show callbacks, 6 partial formats, 8 ID casts; reference d7c7de92
```

coercion-discrimination:

```bash
python3 rust/reference-tools/rooms/coercions_discrimination.py > .scratch/coercions-discrimination-summary.log 2>&1
```

```text
test result: FAILED. 0 passed; 5 failed; 0 ignored; 0 measured; 354 filtered out; finished in 0.49s
Coercion discrimination: five compiled HTTP regressions rejected 8bdb43ec; source restored
```

app:

```bash
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52100-52149 MAIL_TEST_PORT_RANGE=52100-52149 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire --bin campfire -- --test-threads=4 > .scratch/reads-app-final.log 2>&1
```

```text
test result: ok. 356 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 31.56s
```

views:

```bash
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire_views > .scratch/reads-views-final.log 2>&1
```

```text
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

clippy:

```bash
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings > .scratch/reads-clippy-final.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.69s
```

All four oracles exited 0 and their outputs are byte-identical to tracked vectors. The discriminators require compiled assertion failures: reads 5, join 4, audits 3 plus header socket 1, coercions 5. Auth and nonmember guards are included. App/views/clippy exit 0. No full workspace test run, full Rails Minitest run, browser/system/pixel acceptance or complete room parity is claimed.

## Cases grouped by file

`plans/ws8br-rails-cases.json` lists every source-declared case, line, hash and deferred owner. Full Rails Minitest run/pass counts are **0** for every file. Equivalent Rust coverage is separate:

| Rails file | Declared cases | Equivalent Rust subset / remaining |
| --- | ---: | --- |
| `rooms/reads_controller_test.rb` | 7 | All seven behaviors covered by five HTTP tests and one socket test; malformed parameter matrix and venue-specific writes remain. |
| `rooms/refreshes_controller_test.rb` | 4 | Quiet 204 and lost-membership 404 covered; changed-message full bytes and pin count/list integration remain WS8b-m/WS8br. |
| `rooms_controller_test.rb` | 29 | Previous delete/leave/guard/pagination/unread subsets retained; Six join/preview declarations now covered by four HTTP tests and one socket test; complete page, exact errors/flash and leave-failure rescue remain. |
| `rooms/opens_controller_test.rb` | 15 | Conversion/access subsets retained; icons, invalid forms and complete CRUD/form acceptance remain; scalar/member-ID casts and request partial formats, creation audit and unconfigured row/header callbacks are now covered. |
| `rooms/closeds_controller_test.rb` | 12 | Conversion/access subsets retained; icons, invalid forms and complete CRUD/form acceptance remain; creation/membership audit and unconfigured row/header callbacks are now covered. |
| `audit_log/rooms_audit_test.rb` | 17 | Room creation, actual membership diff/no-op, actor/target and committed audit failures covered here; direct/leave subsets retained. Invalid-icon no-audit case remains; five account cases belong to WS8br2. |
| `rooms/directs_controller_test.rb` | 29 | Previous cap/reuse/rename/add/leave/delete and directory-frame subsets retained; picker/settings/invalid rename/overflow/no-op/removed-member acceptance remain; inherited show failure and auth gates now covered. |
| `rooms/involvements_controller_test.rb` | 8 | Six HTTP transitions with exact recipient rows retained; exhaustive format/enum/venue/browser cases remain. |
| `rooms/members_controller_test.rb` | 13 | Endpoint remains unported; WS17 presence and WS11 agents must supply facts, WS8br owns response/auth/cache integration. |
| `rooms/categories_controller_test.rb` | 5 | Scoped assignment subset retained; exhaustive malformed/format and browser acceptance remain. |
| `rooms/favorites_controller_test.rb` | 6 | Scoped writes and all favorite-kind frames retained; malformed/format and browser acceptance remain. |
| `rooms/inbound_email_addresses_controller_test.rb` | 8 | Rotation/auth/redirect subset retained; edit/domain/notice/browser acceptance remains. |
| `room_categories_controller_test.rb` | 6 | Scoped mutations/complete category frame subset retained; broader coercions/formats/browser remain. |
| `switchers_controller_test.rb` | 5 | Complete seeded JSON bytes and guards retained; exhaustive input/format/browser acceptance remains. |
| `users/sidebars_controller_test.rb` | 14 | Complete frames/recipient state/cache subsets retained; query-count/configured-owner/browser acceptance remains. |
| `unfurl_links_controller_test.rb` | 9 | Existing upstream re-diff remains WS8br; embed behavior WS15e. |

Current own Rust counts: channel coercion HTTP 5, channel audit HTTP 3, channel callback socket 1, join HTTP 4, join socket 1, join views 1, reads HTTP 5, reads socket 1, room parity 19, inherited room tests 16, sidebar controller 4, switcher 4, directory socket 3, sidebar views 5, shell views 3, header views 1. Full browser files remain deferred; transferred users/account/public/tour/PWA/QR files belong to WS8br2 as the machine-readable inventory records.

## Precisely remaining, requested order

1. Finish room HTTP: open/closed icons/invalid forms and remaining CRUD/form parity, direct settings and failure paths, members JSON with WS17/WS11 facts, full refresh/pins integration, leave failure rescue and exact errors/flash/formats, broader involvement/categories/favorites cases. Voice/stage domain remains WS13, boards WS12; do not implement owner internals.
2. Integrate WS8b-m message list/composer/template/threads/pins/polls, WS17 viewer OOO facts and WS13 configured venue/header children; full seeded live room page byte/browser acceptance. The stable `rooms::room_message_list(ctx,&ShowView)` still defaults to the authorized zero-byte empty-room placeholder. Lending Rails owner fragments in shell goldens proves surrounding regions only.
3. Inbound-email address browser acceptance. Rails' existing redirect `/rooms/:id/edit` targets a missing generic action at the pin; preserve the observed redirect and report this gap.
4. Complete remaining owned Rails controller/system cases grouped by file, including query counts and valid agent-token guards when their owner auth lands. No full-file Rails pass count is inferred from Rust subset passes.
