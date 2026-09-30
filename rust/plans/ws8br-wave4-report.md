# WS8br Wave 4 — PARTIAL

Branch `rust/ws8br-rooms-http`, assigned worktree `rust-ws8br`, Rails pin `d7c7de92`. Received base `c4849d54`; main `21a7332f` is already merged through `9f356e2f`. This continuation adds the read/quiet-refresh slice. No PR or deployment.

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
Rails deferred inventory: 57 files, 495 source-declared cases; 0 Rails tests run, 0 Rails passes claimed
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
test result: FAILED. 0 passed; 5 failed; 0 ignored; 0 measured; 340 filtered out; finished in 0.51s
Reads discrimination: five compiled HTTP regressions rejected c4849d54; source restored
```

app:

```bash
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52100-52149 MAIL_TEST_PORT_RANGE=52100-52149 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire --bin campfire -- --test-threads=4 > .scratch/reads-app-final.log 2>&1
```

```text
test result: ok. 342 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 31.17s
```

views:

```bash
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire_views > .scratch/reads-views-final.log 2>&1
```

```text
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

clippy:

```bash
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings > .scratch/reads-clippy-final.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 36.80s
```

The oracle exited 0 and its output is byte-identical to the tracked vector. The discrimination command exits 0 only when all five compiled HTTP tests fail; these include the nonmember guard. App/views/clippy exit 0. No full workspace test run, full Rails Minitest run, browser/system/pixel acceptance or complete room parity is claimed.

## Cases grouped by file

`plans/ws8br-rails-cases.json` lists every source-declared case, line, hash and deferred owner. Full Rails Minitest run/pass counts are **0** for every file. Equivalent Rust coverage is separate:

| Rails file | Declared cases | Equivalent Rust subset / remaining |
| --- | ---: | --- |
| `rooms/reads_controller_test.rb` | 7 | All seven behaviors covered by five HTTP tests and one socket test; malformed parameter matrix and venue-specific writes remain. |
| `rooms/refreshes_controller_test.rb` | 4 | Quiet 204 and lost-membership 404 covered; changed-message full bytes and pin count/list integration remain WS8b-m/WS8br. |
| `rooms_controller_test.rb` | 29 | Previous delete/leave/guard/pagination/unread subsets retained; join preview/join, complete page, exact errors/flash and leave-failure rescue remain. |
| `rooms/opens_controller_test.rb` | 15 | Conversion/access subsets retained; icons, invalid forms, creation audit and complete CRUD/stream acceptance remain. |
| `rooms/closeds_controller_test.rb` | 12 | Conversion/access subsets retained; icons, invalid forms, creation/membership audit and complete CRUD/stream acceptance remain. |
| `rooms/directs_controller_test.rb` | 29 | Previous cap/reuse/rename/add/leave/delete and directory-frame subsets retained; picker/settings/invalid rename/overflow/no-op/show callback/removed-member acceptance remain. |
| `rooms/involvements_controller_test.rb` | 8 | Six HTTP transitions with exact recipient rows retained; exhaustive format/enum/venue/browser cases remain. |
| `rooms/members_controller_test.rb` | 13 | Endpoint remains unported; WS17 presence and WS11 agents must supply facts, WS8br owns response/auth/cache integration. |
| `rooms/categories_controller_test.rb` | 5 | Scoped assignment subset retained; exhaustive malformed/format and browser acceptance remain. |
| `rooms/favorites_controller_test.rb` | 6 | Scoped writes and all favorite-kind frames retained; malformed/format and browser acceptance remain. |
| `rooms/inbound_email_addresses_controller_test.rb` | 8 | Rotation/auth/redirect subset retained; edit/domain/notice/browser acceptance remains. |
| `room_categories_controller_test.rb` | 6 | Scoped mutations/complete category frame subset retained; broader coercions/formats/browser remain. |
| `switchers_controller_test.rb` | 5 | Complete seeded JSON bytes and guards retained; exhaustive input/format/browser acceptance remains. |
| `users/sidebars_controller_test.rb` | 14 | Complete frames/recipient state/cache subsets retained; query-count/configured-owner/browser acceptance remains. |
| `unfurl_links_controller_test.rb` | 9 | Existing upstream re-diff remains WS8br; embed behavior WS15e. |

Current own Rust counts: reads HTTP 5, reads socket 1, room parity 19, inherited room tests 16, sidebar controller 4, switcher 4, directory socket 3, sidebar views 5, shell views 3, header views 1. Full browser files remain deferred; transferred users/account/public/tour/PWA/QR files belong to WS8br2 as the machine-readable inventory records.

## Precisely remaining, requested order

1. Finish room HTTP: open/closed icons/invalid forms/audit/stream parity, join/preview, direct settings and failure paths, members JSON with WS17/WS11 facts, full refresh/pins integration, leave failure rescue and exact errors/flash/formats, broader involvement/categories/favorites cases. Voice/stage domain remains WS13, boards WS12; do not implement owner internals.
2. Integrate WS8b-m message list/composer/template/threads/pins/polls, WS17 viewer OOO facts and WS13 configured venue/header children; full seeded live room page byte/browser acceptance. The stable `rooms::room_message_list(ctx,&ShowView)` still defaults to the authorized zero-byte empty-room placeholder. Lending Rails owner fragments in shell goldens proves surrounding regions only.
3. Inbound-email address browser acceptance. Rails' existing redirect `/rooms/:id/edit` targets a missing generic action at the pin; preserve the observed redirect and report this gap.
4. Complete remaining owned Rails controller/system cases grouped by file, including query counts and valid agent-token guards when their owner auth lands. No full-file Rails pass count is inferred from Rust subset passes.
