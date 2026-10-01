# WS8bm2 report — partial message-features delivery

**PARTIAL.** Implemented polls/pins, saved items/reminder claim and rearm, scheduled messages, search/operators/date selections/tuple paging and full preloads for the current renderer, slash dispatch/autocomplete/`/play`, authorized quote endpoints and room Files. Merged WS9 main `4278cb1e`. Reminder quiet policy and the real durable handler are implemented; physical tagged sending remains a WS17 seam. Root provider composition/cache/quote integration, panel/composer mounting, agent/huddle integration, broader date/coercion parity and browser cases remain. This branch is not cutover-ready.

Worktree: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm2`; branch `rust/ws8bm2-message-features`. Base `9c8efaef6a4fd11e7b290db77dd524ed2ab0e5f9`, originally including main `21a7332f2d3c324f0862cdf448baf17a84395aa0`. Rails pin: `d7c7de9264c63015be398001d7a1094e7695a6db`. No PR or deployment.

Pushed implementation slices:

| Slice | SHA |
| --- | --- |
| Polls/pins | `a036f6e6dcb696ba36d4ff64a9a4806bc9c65e3c` |
| Saved/reminder claim/rearm | `90bb304a3092fad869469ed7f8f729980c4abdc1` |
| Scheduled messages | `f39afa8bd28c368bf4c9f80c317ea037b2047539` |
| Search | `53079ef4d33a40eb12ce66ec9ca951a8362f3f34` |
| Complete current-renderer search preloads | `f168c34855624c4bf25793a031634dad1fdb66f1` |
| Slash/autocomplete/play | `4be208b3ecb98b488deb7e3ae81af20f82df3252` |
| Quote endpoints/Files | `40b6a541157c751f58f3cca9e0bd0f3c667edd88` |
| WS9 main merge (merge commit) | `4acb20fc454fee0a7de6a6329768472c9eee7e44` |
| Reminder policy/registered job/tagged send seam | `166ae831a13acfeec83dcc77c679d1e9c1769b41` |
| Fresh-clone verification fixes | `b67efa3cfbab15f04fa37f5210bf3aa1117111a8` |

The final report-only commit follows these source commits; its own SHA is supplied in the final reply.

## Changes by file

Paths are relative to `rust/`; shortened controller/channel/job paths are under `crates/campfire/src/`, and template paths under `crates/views/`.

| Files | Changes |
| --- | --- |
| `controllers/rooms/polls.rs`, `controllers/messages/pins.rs`, `controllers/rooms/pins.rs` | Reachable human/member-gated poll create/show/vote and pin/unpin/list, formats/ballots/errors, cap/idempotency, quiet note and STI identity. |
| `controllers/message_features.rs` | Shared authorization/error/date adapters and numeric JSON message IDs; date grammar remains bounded. |
| `controllers/saved_items.rs`, `controllers/scheduled_messages.rs` | Owned CRUD/list/send HTTP, reminder/status behavior, visibility, pending/owner scope, lock-time rechecks, response headers/redirects and request-zone times. |
| `controllers/searches.rs`, `controllers/searches/{preloads,ports}.rs` | Parsed operator HTTP/chips, capped history/sections, tuple windows, preload adapter; 45 seeded behavior/byte/query tests. |
| `controllers/rooms/slash_commands.rs`, `huddle_readiness.rs` | Authorized transactional WS8a dispatch, explicit Rails JSON escaping, same-room thread selection, exact configuration readiness. Agent execution WS11; huddle launch WS13. |
| `controllers/autocompletable.rs`, `controllers/autocompletable/{icons,slash_commands}.rs` | Markdown user/icon/command picker JSON, room/global scopes, canonical icon/alias rank and limits, registered metadata, root-only filtering, auth/formats. |
| `controllers/rooms/{message_links,files}.rs` | Reference/source visibility, neutral/private quote frames and empty 404; cumulative upload/Drive listing, filters, pages and jump links. |
| `controllers/message_features/{tests,saved_tests,scheduled_tests,slash_tests,links_files_tests,reminder_tests}.rs` | 116 owned seeded tests; byte comparisons, actual HTTP/writer races/job rollback/sockets, all-reader SQL traces and the real reminder runner. Search adds 45 tests. |
| `controllers/presenters.rs`, `presenters/{rich_text,attachments,view_context,test_support}.rs` | Small flagged WS8b-m/WS6 seams: optional preload facts/resolver/icon access, capped recents, clock compatibility after WS9 merge. Shared root partial unchanged. |
| `controllers.rs`, `controllers/{rooms,messages}.rs` | Route/module inventory and existing bot-webhook helper exposure; retained WS9 routes when merging. |
| `channels/{message_features,sink}.rs`, `channels.rs` | Owned poll/pin/note/scheduled/slash commit-thread rendering and origin RAII; one actual socket append; no viewer/session-bound broadcast data. General root/quote composition stays WS8b-m. |
| `crates/db/src/database.rs` | Scoped writer origin survives commit callbacks; queued-writer observation synchronizes real lock races. |
| `db/models/{search_query,search}.rs`, `models/search_query/word_ranges.rs` | Bounded tuple ID query/probe, local date ranges, access-scoped joined/capped sections, capped recents and pinned Ruby Unicode word classification. |
| `db/models/message_rendering.rs` | Batch neutral rendering DTOs: source bodies, people/avatars, room/DM labels/icons, boosts, pins, threads, Drive IDs, steps and poll data. |
| `db/{autocomplete_users,command_suggestions}.rs`, `db/lib.rs` | Capped SQL autocomplete; duplicate-name scope covers the entire active room/global population; registered command metadata. |
| `db/models/{message_quote,room_files}.rs` | Authorized reference/source reads and bounded joined file/Drive listing; literal LIKE and fixed MIME groups, including NULL behavior. |
| `db/models/reminder_policy.rs`, `db/models/saved_item.rs` | Narrow reminder notification policy, membership seam and full Unicode reminder title, preserving Rails body truncation. |
| `db/slash_commands/time_parser.rs` | Exposed existing local-time resolver; retained WS9 strict zone handling. Full Ruby date grammar remains partial. |
| `crates/storage/src/blob.rs` | Bounded batch attached-message/blob lookup seams; no storage format change. |
| `jobs/reminders.rs`, `jobs.rs` | Registered real `SavedItem::ReminderPushJob`, policy/access rechecks, tagged payload/subscription enqueue seam, durable retry until WS17 sending lands. |
| `jobs/{periodic,tests}.rs` | Exposed actual reminder function for runtime checks; quote-refresh assertion checks its own class rather than unrelated retention jobs. |
| `config.rs`, `main.rs` | Huddle readiness flag/module wiring and owned reminder module registration. |
| `integrations.rs`, removed `integrations/search*.rs` | Removed inactive stock search sanitizer; active Ruby word classifier belongs to domain search. |
| `views/src/{pins,saved_items,scheduled_messages,searches,autocompletable,message_links,room_files}.rs`, `views/src/lib.rs` | Plain DTOs and Askama wrappers for owned pages/partials, picker JSON and sizes. |
| `views/src/helpers/forms.rs`, `views/src/messages/parts.rs` | Ordered hidden form fields; request-zone poll closing time. |
| `templates/rooms/pins/*`, `saved_items/*`, `scheduled_messages/*`, `searches/*`, `rooms/files/*`, `rooms/message_links/*`, `messages/message_links/*`, `polls/_poll.html` | Owned Rails list/count/frame/chip/section/page/stream/card bytes. Schedule control is reusable but not mounted into the shell. |
| `reference-tools/messaging/{features,saved,scheduled,search,preloads,slash,links_files,reminder_push}.rb`, corresponding `vectors/messaging/*.json` | Eight real pinned Rails oracles, committed SQL fixtures and exact expected bytes. Files membership timestamp regenerated under SQLite clock freezing. |
| `reference-tools/messaging/{features-reference-check,features-discriminate}.py` | Consumed-source byte/file-set verification and compiled discriminating mutations. |
| `plans/ws8bm2-report.md` | Tracked copy of the external report. |

## Design and parity evidence

Domain operations keep Rails validation/callback behavior and atomic index/unread/reference/job effects. Controllers authorize and render plain models. Scoped writer guards hold request origin through callbacks and restore on success/error/panic. Rendering does not introduce viewer/session/CSRF/nonce data into shared broadcasts.

Saved-item periodic claim/rearm uses the real frozen-clock worker. Rejected durable job insertion rolls back claim/inbox rows; successful retry fires once and rearm refreshes the same inbox activity. Scheduled update/cancel re-read under the writer lock; both send-between-lookup-and-lock races use actual HTTP and SQLite. Busy checks precede malformed update parameters. Send job rejection rolls back claim/message/history; a real socket receives one token-free append. Exact scheduled JSON covers UTC/Hawaii/New York offsets and gap handling; eight state partials, two schedule controls and an empty page match Rails.

Search quotes pinned Ruby word tokens for FTS and binds escaped operators. The `(created_at,id)` query loads at most 41 IDs and instantiates at most 40 messages, displaying them oldest first. Nonblank older requests require a reachable cursor; blank queries do not resolve it. Older windows omit side sections; capped initial sections and ten recents use fixed SQL reads. Day operators match UTC, Hawaii, New York's 23/25-hour days, São Paulo's midnight gap and Apia's missing day. All 771 Ruby word ranges are verified against the pin.

Search preloads every fact the current shared renderer consumes, including mention/SGID users and reply source bodies. Four versus sixteen mixed Markdown/mention/boost/poll messages have constant SELECT authorization events; the lazy path failed with `(55,199)`. Six complete message fragments match both real Rails and the lazy presenter from committed rows. Populated provider/quote cards absent from the inherited root composition remain unproven; this is not a claim of complete populated-search page parity.

Slash dispatch shares WS8a's domain dispatcher and atomic writer callback chain. Picker duplicate-name checks cover the complete active scope even outside the filtered page; avatar versions use the request zone. Rails explicit render-JSON and Jbuilder escaping differ, and each endpoint matches its own oracle. All 56 sounds plus five unknown/blank/case inputs match `/play` presentation. Twelve actual Rails huddle readiness cases pass. Agent registration metadata is real; invocation is still WS11.

Quotes authorize the quoting room/reference before source membership and source facts. Outsiders/deleted source rooms get only the private chip; source system notes remain private. Direct-room labels are viewer-neutral. Five complete bare-frame HTTP responses and two partials match bytes. Files match fifteen sections and sixteen Rails sizes; upload/Drive pages are independently capped, filters escape literal LIKE wildcards, and thread jump links retain their thread. The full HTTP SELECT execution trace covers all reader connections and cached statements; four versus sixteen combined file rows cost the same queries. Its private runner is shut down before tracing so unrelated queue/periodic reads cannot contaminate the count. The lazy-blob mutation still fails.

Reminder policy matches 27 real Rails cases: manual/presence DND, local quiet hours, cached meeting quiet, and manual/calendar OOO unless notifications stay on, including interval boundaries and malformed cached pairs. The registered job checks current room membership before policy or source reads, discards missing saved rows, and supplies full payload/tag/subscription IDs to the send boundary. Two real Rails job handoffs match, including an unclamped Unicode room title. Seven tests exercise the real runner and this boundary.

**WS17 send seam remains partial.** Tagged transport has not landed here. Allowed pushes with subscriptions remain one durable ready job rescheduled by sixty seconds; suppressed/empty-subscription jobs finish. Queue errors cannot acknowledge delivery. WS17 must replace the default callback with tagged pool enqueue. No physical push send is claimed.

Merged main `4278cb1e7a4529d5e2ecee51fccf69be6ed45257` with a merge commit, retaining both route inventories and clock-helper APIs. Locked metadata succeeded; no lockfile change was needed. The image `ws8bm2-reference:d7c7de92` was tagged from the installed reference image. The bounded verifier checks 72 consumed Rails source files against the pin and rejects two injected byte/file-set differences; it does not verify every image file. All eight oracles now reproduce byte-identically from independently rebuilt fresh-clone seeds under frozen process/SQLite time.

No schema, dependencies, lockfile, Rails source, root message partial, parity masks or allowlists were changed by this continuation. Cross-workstream touches are the small presenter/storage/clock/origin/config/job seams named above; WS8b-m, WS8b-r, WS11, WS13 and WS17 retain their integration boundaries.

## Failing-first and verification corrections

Historical failing-first checks rejected poll/pin gates, saved routes and scheduled routes before implementation. Stock search failed 25 cases; the Unicode parser probe failed independently. This continuation's initial slash/security run failed all thirteen checks, and initial quote/Files endpoint/security run failed all fourteen. Preload query measurement rejected the old lazy path; the absent reminder registry and old Unicode title clamp failed the new checks. Exact JSON/avatar-zone comparisons also failed before their fixes. Compiled mutation results below re-establish discrimination for the new continuation.

```text
test result: FAILED. 7 passed; 25 failed; 0 ignored; 0 measured; 389 filtered out; finished in 1.34s
test result: FAILED. 0 passed; 13 failed; 0 ignored; 0 measured; 430 filtered out; finished in 0.79s
test result: FAILED. 0 passed; 14 failed; 0 ignored; 0 measured; 467 filtered out; finished in 0.35s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 576 filtered out; finished in 0.16s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 582 filtered out; finished in 0.10s
```

The first fresh-clone run exposed a query-count test race: a background reader SELECT entered one measurement `(14,13)`. The corrected test quiesces that private runner and still measures the complete real HTTP stack, with no SQL exclusions. The first oracle replay exposed one SQLite-generated membership creation timestamp; process/SQLite freezing regenerated that actual Rails row. No expected HTML/JSON was normalized or masked. The failed fresh run was:

```text
test result: FAILED. 579 passed; 1 failed; 3 ignored; 0 measured; 0 filtered out; finished in 160.63s
```

An earlier quote-runner test incorrectly required unrelated retention jobs to be gone after quote completion. Its narrow assertion now rejects pending/running/failed quote jobs while allowing other classes. Production behavior did not change. Obsolete stock search/autocomplete assertions were replaced by actual Rails behavior checks, with no new ignores.

## Verification commands and raw summaries

The fresh checkout is `.scratch/fresh-clone` inside the assigned worktree, cloned from `https://github.com/Smart-Data-Ohio/smartfire.git` on this branch, then fast-forwarded to source commit `b67efa3cfbab15f04fa37f5210bf3aa1117111a8`. It independently compiled its own `rust/target` and rebuilt both seeds from committed recipes and the pinned reference image. No main-worktree target, seed, `.scratch` fixture or runtime file was copied into it. Its `.scratch` contains only verification logs/temp data created there. The final following commit changes the report only.

All Cargo commands below run in `.scratch/fresh-clone/rust`, except the explicitly named mutation loop in the assigned worktree root. Rust is 1.98.1, each Cargo run uses four jobs, and all seed-dependent suite tests run with `CI=1`.

Fresh-clone seed construction:

```sh
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 parity/bin/seed build default first_run > ../.scratch/fresh-seed.log 2>&1
```

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

Fresh-clone root, validating both independently built seeds:

```sh
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/campfire/verify_parity_seed.rb default > .scratch/final-validate-default.log 2>&1
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/campfire/verify_parity_seed.rb first_run > .scratch/final-validate-first-run.log 2>&1
```

Default seed raw summary fields:

```text
  "passed": 29,
  "failed": 0
```

First-run seed raw summary fields:

```text
  "passed": 4,
  "failed": 0
```

Fresh-clone root, bounded consumed-source verification and all eight actual Rails oracle replays:

```sh
PARITY_IMAGE=ws8bm2-reference:d7c7de92 python3 rust/reference-tools/messaging/features-reference-check.py > .scratch/final-source-check.log 2>&1
for oracle in features saved scheduled search preloads slash links_files reminder_push; do
  mkdir -p ".scratch/final-$oracle"
  cp -a rust/parity/.seed/default/. ".scratch/final-$oracle/"
  PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 rust/parity/bin/reference runner --storage "$PWD/.scratch/final-$oracle" --time 2026-03-02T16:00:00Z --freeze "rust/reference-tools/messaging/$oracle.rb" "/rails/storage/db/$oracle.json" > ".scratch/final-$oracle.log" 2>&1
  cmp ".scratch/final-$oracle/db/$oracle.json" "rust/vectors/messaging/$oracle.json" || exit 1
  rg '^WS8bm2' ".scratch/final-$oracle.log"
done
```

```text
WS8bm2 reference source check: 72 controller, model, helper and template files match d7c7de92
WS8bm2 reference check self-test: 2 injected source-byte/file-set differences rejected
WS8bm2 Rails oracle: 8 poll reads/ballots; 6 poll creates; 4 pin writes; 11 partials; 10 zone/date probes
WS8bm2 saved Rails oracle: 11 HTTP responses; 6 item partials; 1 empty page
WS8bm2 scheduled Rails oracle: 12 HTTP responses; 8 row partials; 1 empty page; 2 composer controls
WS8bm2 search Rails oracle: 15 parsed queries; 15 chip partials; 1 empty page; 1 clear stream; 18 zone/date selections; 1 populated sections partial; 1 load-older control; 1 empty older stream; 4 HTTP responses; 771 Unicode word ranges
WS8bm2 preload Rails oracle: 6 complete message fragments; 9 committed fixture tables
WS8bm2 slash Rails oracle: 20 dispatch responses; 17 picker responses; 61 play presentation fragments; 3 format responses; 12 huddle readiness cases
WS8bm2 links/files Rails oracle: 15 Files sections; 5 quote HTTP responses; 2 quote partials; 16 size values; 9 fixture tables
WS8bm2 reminder push Rails oracle: 27 policy cases; 2 captured real job payload/subscription handoffs; 3 fixture tables
```

Every `cmp` exited zero without output, against the already committed vector. No new normalization, mask or allowlist was used.

Assigned worktree root, re-run compiled mutations for this continuation (the script sets `CI`, private TMPDIR/target and the assigned socket/mail port ranges):

```sh
for group in preload slash files quote reminder; do
  python3 rust/reference-tools/messaging/features-discriminate.py "$group" > ".scratch/final-discriminate-$group.log" 2>&1 || exit 1
  cat ".scratch/final-discriminate-$group.log"
done
```

```text
preload-lazy: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 582 filtered out; finished in 1.81s
preload-pin: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 582 filtered out; finished in 0.78s
WS8bm2 discrimination: 2 compiled regressions detected; sources restored
slash-csrf: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 582 filtered out; finished in 2.06s
slash-token-duplicates: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 582 filtered out; finished in 0.43s
slash-icons-rank: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 582 filtered out; finished in 0.84s
slash-root-only: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 582 filtered out; finished in 0.42s
slash-job-atomic: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 582 filtered out; finished in 0.60s
slash-socket: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 582 filtered out; finished in 5.46s
WS8bm2 discrimination: 6 compiled regressions detected; sources restored
files-scope: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 582 filtered out; finished in 0.63s
files-literal: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 582 filtered out; finished in 0.47s
files-lazy: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 582 filtered out; finished in 0.82s
WS8bm2 discrimination: 3 compiled regressions detected; sources restored
quote-privacy: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 582 filtered out; finished in 0.52s
WS8bm2 discrimination: 1 compiled regressions detected; sources restored
reminder-registration: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 582 filtered out; finished in 0.12s
reminder-policy: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 582 filtered out; finished in 0.09s
reminder-membership: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 582 filtered out; finished in 0.09s
reminder-tag: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 582 filtered out; finished in 0.24s
WS8bm2 discrimination: 4 compiled regressions detected; sources restored
```

All sixteen intentional regressions reached and failed the intended compiled test; sources were restored and the tracked source diff was empty afterward. Earlier poll/pin/saved/scheduled/search mutation groups are historical evidence, not claimed as re-run by this filtered continuation loop.

Fresh-clone Rust directory, locked metadata, complete non-vendored workspace suite and clippy:

```sh
mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 > ../.scratch/final-metadata.json
TMPDIR="$PWD/../.scratch/tmp" CARGO_TARGET_DIR="$PWD/target" CI=1 CABLE_TEST_PORT_RANGE=52500-52549 MAIL_TEST_PORT_RANGE=52550-52599 mise exec rust@1.98.1 -- cargo test --locked -j4 --workspace --exclude html5ever -- --test-threads=4 --nocapture > ../.scratch/final-workspace-test.log 2>&1
TMPDIR="$PWD/../.scratch/tmp" CARGO_TARGET_DIR="$PWD/target" mise exec rust@1.98.1 -- cargo clippy --locked -j4 --workspace --exclude html5ever --all-targets -- -D warnings > ../.scratch/final-clippy.log 2>&1
```

Metadata, tests and clippy each exited zero. Metadata wrote valid JSON without a stdout summary. Test raw summaries appear below in execution order, with their original target labels:

```text
     Running unittests src/main.rs (target/debug/deps/campfire-d4df0af23b1f90e2)
test result: ok. 580 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 319.98s
     Running unittests src/lib.rs (target/debug/deps/campfire_assets-de6b6b844a46fb2e)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/reference.rs (target/debug/deps/reference-52334e28be9d328d)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.01s
     Running unittests src/lib.rs (target/debug/deps/campfire_cable-099573072d53381c)
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/disconnect.rs (target/debug/deps/disconnect-42cf31142ee556a4)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 44.52s
     Running tests/golden.rs (target/debug/deps/golden-a057e4bb65195f7c)
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.90s
     Running tests/protocol.rs (target/debug/deps/protocol-1c38858ec89913df)
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.13s
     Running unittests src/lib.rs (target/debug/deps/campfire_db-215201eeb95ea1a9)
test result: ok. 446 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 66.01s
     Running unittests src/lib.rs (target/debug/deps/campfire_jobs-2b4b600e1c1b7ef2)
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.77s
     Running tests/crash.rs (target/debug/deps/crash-a0e513b65522039f)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.04s
     Running unittests src/lib.rs (target/debug/deps/campfire_kit-c778c94af2123e34)
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
     Running tests/front.rs (target/debug/deps/front-58bf655879b34757)
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.02s
     Running tests/http.rs (target/debug/deps/http-e9043d1dc9499a1b)
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests/params_vectors.rs (target/debug/deps/params_vectors-85d6c6bc32e6f7f4)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
     Running tests/rails_vectors.rs (target/debug/deps/rails_vectors-c66f9244e0fc195e)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running unittests src/lib.rs (target/debug/deps/campfire_mail-7ce4648430c3074d)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
     Running tests/config.rs (target/debug/deps/config-76fc90a75b18de48)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/goldens.rs (target/debug/deps/goldens-0e5b54d85cab421b)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
     Running tests/inbound.rs (target/debug/deps/inbound-9b7df7acddd50156)
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 6.30s
     Running tests/security.rs (target/debug/deps/security-d6257643e5e860ea)
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
     Running tests/smtp.rs (target/debug/deps/smtp-3b7765b569fd2d4b)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.77s
     Running unittests src/lib.rs (target/debug/deps/campfire_richtext-4a8389a15be07fd2)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.35s
     Running tests/corpus.rs (target/debug/deps/corpus-8a400ffa0cacd0c6)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 21.57s
     Running tests/fork_regressions.rs (target/debug/deps/fork_regressions-47666ab224448bf8)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/hardening.rs (target/debug/deps/hardening-ad131644ca4fd322)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.77s
     Running tests/markdown_corpus.rs (target/debug/deps/markdown_corpus-cf0d596a4179c82f)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.21s
     Running tests/markdown_security.rs (target/debug/deps/markdown_security-bdf3e1cdacba4fed)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
     Running tests/reference_tests.rs (target/debug/deps/reference_tests-6fcdb456853a77f0)
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/sgid_json_corpus.rs (target/debug/deps/sgid_json_corpus-a2a8310065446f52)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.87s
     Running unittests src/lib.rs (target/debug/deps/campfire_routes-6b53a5291b6f9a17)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running unittests src/lib.rs (target/debug/deps/campfire_storage-81b9778f444bb28c)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
     Running tests/vectors.rs (target/debug/deps/vectors-7de607f29cb543fe)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 11.43s
     Running unittests src/lib.rs (target/debug/deps/campfire_views-7023f6bdfcce7286)
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
     Running tests/core.rs (target/debug/deps/core-fcd3834ff7bf58bc)
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
     Running unittests src/lib.rs (target/debug/deps/rails_compat-028047b1fc65456b)
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.85s
   Doc-tests campfire_assets
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_cable
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_db
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_jobs
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_kit
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_mail
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_richtext
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_routes
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_storage
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_views
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests rails_compat
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.12s
```

Cargo reports **1,646 passed, zero failed and 11 ignored**, including empty doctest targets. The 161 owned controller/search tests all ran and passed; no owned seeded test ignored or skipped. The origin and huddle readiness checks also pass. Existing ignores: application reference recorder, WS11 account-bot case, push latency; cable reference recorder; four DB scenario/export/fixture/2FA rollback workflows; mail rollback export; two kit doctest snippets. No new ignore was added.

Two additional conditional boundaries are explicit. The kit ACME/Pebble case returned early without its environment; storage ran non-media assertions but omitted version-dependent generated media byte comparisons. Raw messages:

```text
skipped: PEBBLE_MINICA isn't set
skipping byte comparisons that depend on versions: vectors have libvips "8.16.1" / "ffmpeg version 7.1.5-0+deb13u1 Copyright (c) 2000-2026 the FFmpeg developers", local libvips 8.18.6 / ffmpeg version n9.0.2 Copyright (c) 2000-2026 the FFmpeg developers
```

The separate `skipping locally: parity/.seed/default isn't built` message comes from the deliberate `missing_seed_may_skip_locally` helper test's empty temporary directory, not from a seeded controller bypass. Both real seeds validated, and `CI=1` enforces them.

## Grouped Rails behavior coverage

These are named reference behaviors mapped to passing grouped Rust tests and pinned byte oracles; the Rails controller/system suites themselves were not executed as suites. Counts do not imply complete root/browser parity. All ten deferred controller cases and all 45 browser cases remain owned by WS8bm2, coordinating the integration owners listed below.

| Rails controller test file under `test/controllers/` | Named behaviors covered / total | Deferred |
| --- | --- | --- |
| `rooms/polls_controller_test.rb` | 14 / 16 | Two whole-room/cache cases; partial bytes covered; WS8b-m integration |
| `messages/pins_controller_test.rb` | 6 / 6 | Browser/panel integration below |
| `rooms/pins_controller_test.rb` | 3 / 3 | Browser/panel integration below |
| `saved_items_controller_test.rb` | 12 / 12 | Physical reminder send WS17; browser below |
| `scheduled_messages_controller_test.rb` | 19 / 19 | Composer/sidebar mounting WS8b-r; browser below |
| `searches_controller_test.rb` | 36 / 36 | Missing populated provider composition and browser proof |
| `rooms/slash_commands_controller_test.rb` | 9 / 10 | Registered agent execution WS11 |
| `autocompletable/icons_controller_test.rb` | 6 / 6 | Browser picker below |
| `autocompletable/slash_commands_controller_test.rb` | 7 / 7 | Browser picker below |
| `autocompletable/users_controller_test.rb` | 5 / 5 | Broad odd-shape coercion below |
| `rooms/message_links_controller_test.rb` | 5 / 12 | Seven root/cache/refresh cases; WS8b-m integration |
| `rooms/files_controller_test.rb` | 8 / 8 | Shell/browser and odd-shape coercion below |
| **Total named controller behavior ports** | **130 / 140** | **10 deferred** |

| Rails system file under `test/system/` | Browser cases proved / total |
| --- | --- |
| `polls_test.rb` | 0 / 4 |
| `pins_saved_test.rb` | 0 / 7 |
| `slash_commands_test.rb` | 0 / 26 |
| `search_files_test.rb` | 0 / 4 |
| `scheduled_messages_test.rb` | 0 / 4 |
| **Total** | **0 / 45; all deferred, none attempted** |

## Rails test mapping and ownership

Covered controller behaviors are ported as grouped Rust tests and pinned HTTP/partial oracles; the Rails test suites themselves were not run here.

- `test/controllers/rooms/polls_controller_test.rb`: fourteen non-room-page cases covered: creates a poll; multiple anonymous close time; too few options; board rejection; replacement/broadcast; retraction; closed rejection; foreign options; non-member create/vote; bot create; viewer ballot; anonymous viewer ballot; outside-room show; bot show. Its final two room-page cases (anonymous cards carry no voter IDs; regular cards carry IDs) have exact partial assertions here, but full room-page/cache integration remains **deferred to WS8b-m**, with WS8bm2 responsible for poll endpoint/partial behavior.
- `test/controllers/messages/pins_controller_test.rb`: all six behaviors covered: creation plus four frames/note, idempotency, cap, unpin plus three frames, absent unpin and non-member rejection.
- `test/controllers/rooms/pins_controller_test.rb`: all three behaviors covered: newest-first jump/unpin list, empty list and non-member rejection.
- `test/system/polls_test.rb`: all four deferred to **WS8bm2**, coordinating composer/shell with WS8b-r and root cache with WS8b-m: builder/create/change/retract, anonymous reload markers, second-session live results, closed form absence.
- `test/system/pins_saved_test.rb`: all seven deferred to **WS8bm2**: menu/panel/live badge, same-tab note jump, compact note/no menu, panel unpin, save/remind/status/remove, tomorrow-9am fall-back and custom reminder. First four also require WS8b-r panel/shell wiring. Socket/partial tests here cover part of those behaviors but are not browser ports.

The following file mapping distinguishes completed request ports from deferred controller/system work. Deferred cases remain with **WS8bm2**, with the integration owners identified below:

- **Covered: 12/12 named Rails behavior ports pass.** `test/controllers/saved_items_controller_test.rb` (12 cases; 12 named Rust request tests pass, plus four additional checks): index lists saved messages with status filters; index hides items whose room access was lost; create saves with a reminder; create without a reminder leaves remind_at blank; create again updates the reminder instead of duplicating; create again after a fired reminder re-arms it; create rejects past and unparseable reminders; create is 404 for a message the user cannot see; update marks done and reopens; update rejects an invalid status; update and destroy are 404 for hidden or foreign items; destroy removes the item.
- **Covered: 19/19 individually named Rust request ports pass.** `test/controllers/scheduled_messages_controller_test.rb` (19 tests): index lists upcoming and past rows; index hides other people's rows; index shows stranded rows so they can be cancelled; creates a scheduled message; create rejects past times; create is 404 outside membership; updates text and time; update during an active claim is refused; update during an active claim redirects with a notice in HTML; update after the claim goes stale is allowed; update is 404 for sent rows and other people's rows; destroy cancels pending rows only; a send that lands between the lookup and the lock refuses the cancel; a send that lands between the lookup and the lock refuses the edit; destroy during an active claim is refused; send_now posts immediately; send_now drops rows without access; send_now drops rows the model rejects with the reason; bots are forbidden.
- **36/36 named Rust behavior ports pass; the complete current-renderer preload path is implemented; populated provider whole-page/browser parity remains partial.** `test/controllers/searches_controller_test.rb` (36 tests): index initial view; finding reachable messages; unreachable messages are not found; operator words are searched literally instead of raising; a leading operator returns 200; a boolean-looking query does not exclude terms; a quote character returns 200 with sensible results; create does not run the search; clear does not run the search; clear answers Turbo with streams that empty the header and page recents in place; clear leaves recents alone when it can't answer the requested format; the header renders at most ten recents even when older rows exceed the trim; clear without Turbo returns to the page it came from; index renders the page for a Turbo Stream request without an older-results cursor; the header search field renders on every signed-in page, empty outside search; the search page without a query lists recents instead of a watermark; a query with no results shows an empty state; results page through Load older results; an older window renders as a page without JavaScript; create saves the search term; create with no searchable words redirects back with a notice and records nothing; clear search history; from: narrows results to that author; in: narrows results to that room; in: a room the user is not in returns nothing; sections exclude soft-deleted rooms; has:pin narrows results to pinned messages; has:link narrows results to messages carrying a link; on: narrows results to that day; is:thread narrows results to thread messages; filter-only queries list without text and show chips; chips link back without their operator; boards, work threads, and events render as sections scoped to access; search sections cost the same queries regardless of section size; search sections label direct rooms neutrally; operator values cannot inject SQL or FTS syntax.
- **9/10 named behavior ports pass; agent execution is deferred to WS11 integration.** `test/controllers/rooms/slash_commands_controller_test.rb` (10 tests): shrug posts through the dispatcher; unknown commands answer an error without posting; status answers ephemeral confirmation; event answers an open_url; bare event opens the blank form; poll answers open_poll; agent commands invoke through the room; thread commands dispatch with the thread; non-members get 404; bots are forbidden.
- **6/6 named behavior ports pass.** `test/controllers/autocompletable/icons_controller_test.rb` (6 tests): requires authentication like the users endpoint; returns mixed brand and emoji matches with their payloads; orders prefix matches first and limits the results; returns no matches for blank or unknown queries; returns workspace icons with their stable image URL; lists every workspace icon for the picker Custom tab.
- **7/7 named behavior ports pass.** `test/controllers/autocompletable/slash_commands_controller_test.rb` (7 tests): lists built-ins with metadata; exposes takes_arguments for immediate and argument commands; includes the room's agent commands; agent commands registered without arguments run immediately; thread conversations hide root-only commands; filters by query; non-members get 404.
- **5/5 named behavior ports pass.** `test/controllers/autocompletable/users_controller_test.rb` (5 tests): search returns matching users; search results escape HTML in names; room search returns matching users; room search omits the Markdown token for duplicate display names; room search is scoped by membership.
- **5/12 named endpoint behavior ports pass; seven root/cache/refresh cases are deferred with WS8b-m.** `test/controllers/rooms/message_links_controller_test.rb` (12 tests): a member of the source room sees the quote card; a quote of a soft-deleted source room shows only the private chip; a non-member of the source room sees only the private chip; a non-member of the quoting room gets nothing; a reference from another room gets nothing; a same-room quote renders inline in the room; a cross-room quote renders a lazy frame in the room; editing the source enqueues a job that refreshes quoting cards over the stream; deleting the source clears quoting cards and busts their cache; two viewers of a cached direct-room quote see the same neutral label; quote cards cost the same queries regardless of card count; editing a message to add a permalink replaces its own card container.
- **8/8 named behavior ports pass.** `test/controllers/rooms/files_controller_test.rb` (8 tests): lists uploads newest first with jump links; lists Drive attachments as generic picker-only rows; type filters narrow uploads; filename search matches substrings and escapes wildcards; uploads page cumulatively; only the room's own files are listed; non-members get nothing; rendering costs the same queries for 4 files as for 16.
- **0/26 browser ports; all deferred.** `test/system/slash_commands_test.rb` (26 tests): typing slash opens the command picker with combobox semantics; picking poll by Enter runs it immediately; picking poll by click runs it immediately; picking event by Enter opens the form immediately; picking huddle by Enter runs it immediately; suggestion rows hint argument placeholders only; the close button dismisses the picker without sending; the close button does not overlap the first row's text; Escape closes the picker without sending; mentions and emoji pickers have no close button and still commit; the picker lists registered agent commands; agent commands that take arguments insert and wait; agent commands without arguments run immediately when picked; shrug posts through the picker; arguments close the slash picker and Enter posts; Enter submits while the picker's deactivating update is still pending; a submit queued during the live command check still runs the command; unknown slash words post as normal messages; double slash escapes a known command; a command registered after page load still runs; me renders as an action line; poll opens the poll builder; event navigates to the prefilled form; huddle reports when unconfigured; agent commands respond ephemerally until the agent replies; status sets the custom status.
- **0/4 browser ports; all deferred.** `test/system/search_files_test.rb` (4 tests): filter chips show parsed operators and remove them; a pasted permalink renders a quote card with a working jump link; a cross-room permalink loads its quote frame for members and outsiders; the Files tab lists uploads and Drive rows with working filters.
- **0/4 browser ports; all deferred.** `test/system/scheduled_messages_test.rb` (4 tests): schedules from the composer and lists in the Scheduled view; schedule send requires a draft; edits, sends now, and cancels from the Scheduled view; the sidebar links the Scheduled view.

All 61 `/play` presentation oracle fragments pass; room agent-command integration depends on WS11. Upload/message-root work remains with WS8b-m; room Files listing is WS8bm2. No test is handed back as completed merely because the underlying WS8a model exists.



## Remaining work in the requested order

1. **Search integration:** the current-renderer full preload path is complete. Populated GitHub/Fizzy/X/LinkedIn/events/embed/quote root composition remains WS8b-m with WS14/WS15; prove complete populated page and older-window bytes after it lands. Browser search/filter behavior remains unproven.
2. **Slash/autocomplete/play integration:** registered agent invocation and valid-agent bearer behavior require WS11; huddle launch requires WS13. HTTP/metadata/`/play` presentation is covered, but all 26 picker/composer browser cases remain.
3. **Message links/Files integration:** seven named root quote/cache/refresh cases remain: same-room inline card, cross-room lazy frame, source edit refresh, source deletion/cache bust, cached two-viewer neutral DM label, constant quote query cost, and editing a permalink replaces its own container. Coordinate WS8b-m root rendering and broadcasts; WS8bm2 retains the feature cases. Files' eight named HTTP behaviors pass; shell tab mounting and browser filters/download/jump interactions remain with WS8b-r integration.
4. **Cache/panel/auth/date/push gaps:** the root still reads `cached_message_fragment(id,updated_at,origin)` rather than the available Rails composite key, so poll-only updates can leave stale cards. Other owners' deletion/unpin writes still need origin scope. WS8b-r must mount pin panel/header/dialog and schedule composer/sidebar controls. Valid agent secrets still return 401 in the shared authentication seam; Rails' valid-agent endpoint 403 awaits WS11. WS9 strict zone handling is retained, but full Ruby `Date._parse`, natural-language/civil overflow/malformed ISO and option stripping/Array/Hash coercions remain partial, including odd file-filter/user-query/pager shapes. Reminder quiet policy/real job/payload are complete; physical tagged enqueue/send remains WS17, with durable sixty-second retry rather than false acknowledgment.
5. **Browser and deferred cases:** 45 named system cases remain unported, grouped above. Ten named controller behaviors remain (two whole-room poll cases, agent slash execution and seven root quote cases). Poll/pin-list constant-query proof and a periodic-close real runtime socket check remain. No browser/pixel matrix or Rust Docker image build was run. The full non-vendored workspace suite was verified from the fresh clone above; it does not close those omitted integration cases.

Resume at composite cache/root quote integration through small coordinated seams, then panel/auth/date/push boundaries and browser cases. No open product decision is required; remaining boundaries and missing verification are explicit. The external report is synchronized with this tracked copy.
