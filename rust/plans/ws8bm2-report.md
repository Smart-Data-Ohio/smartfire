# WS8bm2 report — partial provider/composer/date integration

**PARTIAL.** Public GitHub, X, event, LinkedIn and generic link-embed cards now render from batched persisted facts through the real message presenter. Ordinary room HTTP now mounts the published Markdown composer, slash/autocomplete registry and schedule-send control. Broader builder date parsing matches 175 pinned Rails cases, and request string coercion covers arrays/hashes with eight real reminder responses. The one remaining named controller behavior is agent invocation; it stays deferred under the lead's explicit WS11 instruction.

Physical tagged push sending (WS17), agent credentials/invocation (WS11) and huddle launch (WS13) remain named, flagged seams. All 45 browser cases remain deferred to the end-to-end Rust-server phase; no browser harness was built. Provider fetch/reference/write callbacks and private-card endpoints, the full owning shell/thread mount, and unprobed date/lookup coercions remain partial. This branch is not cutover-ready.

Worktree: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm2`; branch `rust/ws8bm2-message-features`. Rails pin: `d7c7de9264c63015be398001d7a1094e7695a6db`. This continuation started at received `bcbac9f85b182329fd54aa332a07fec609ab6255`. Main remains unmerged, as instructed while its asset-fingerprint golden fix lands. Earlier merge `4acb20fc` of WS9 main `4278cb1e` remains an ancestor. No PR or deployment.

## Pushed slices

Previously received polls/pins, saved/reminders, scheduled, search/preloads, slash/autocomplete/play, quotes/Files, reminder policy/job seam, root cache/private frames, panels and nine deferred controller behaviors remain present. Their historical reports and evidence are in branch history; the commands below describe this continuation only.

| Slice | Pushed source SHA |
| --- | --- |
| Public GitHub, LinkedIn and generic embeds | `c21349ef21fc034af8291a9f5e1b1abfbbd29c66` |
| Published WS14e event partial mount | `f9ee4d07253a70dbc3a5103ae194483caee9e6c9` |
| Published WS15e X cards and formatter mount | `4fcd5aa3b1836fbe432e93adc62e54f750207a20` |
| Published WS8b-r Markdown composer and schedule slot | `d9d00794b96a1307d197c79c2d66e803177d757f` |
| Broader calendar parsing and request string coercion | `2ceb6035f5a9e56e92397f8fa6f6320c9157338a` |
| Persisted cancellation and Drive mount assertions | `2453bc279bb85af84c9f24c5f51d903558ac13f8` |

**Fresh-clone verified source SHA: `2453bc279bb85af84c9f24c5f51d903558ac13f8`.** The following report-only commit has identical source; its pushed SHA is in the final reply.

## Changes by file and design

| Files relative to the assigned worktree | Changes |
| --- | --- |
| `rust/crates/db/src/models/message_rendering/{providers,event_cards,twitter}.rs`, `message_rendering.rs` | Read-only batch joins for public/private GitHub identity, PR discussion threads, raw embed-reference URLs, event/organizer/venue facts and X payloads. X ordering compares decimal identities beyond i64; domain facts carry no HTML or viewer tokens. |
| `rust/crates/campfire/src/controllers/presenters/provider_cards.rs`, `presenters.rs` | Small flagged root/provider adapter builds typed DTOs from preloads. Private/unknown GitHub rows expose neutral frames only; embed suppression and reference URLs are preserved. Meet URLs require HTTPS and a host. |
| `rust/crates/views/src/message_providers.rs`, `message_providers/events.rs`, `templates/message_providers/*` | Public GitHub loading/error/state/authors/branches/reviews/checks/Discuss and thread-link markup; generic embeds; LinkedIn player/chips. Actual container bytes match pinned Rails. |
| `rust/crates/views/src/events.rs`, `templates/rooms/events/*` | Reused the published WS14e pure card view from local remote ref `origin/rust/ws14e-events` at `d6d2dcc5`. The small adapter mounts populated cards and viewer-neutral lazy attendance frames; event writes/attendance endpoints stay with WS14e. |
| `rust/crates/views/src/twitter/*`, `templates/twitter/posts/_card.html` | Reused WS15e's published pure card/count/formatter views from `origin/rust/ws15e-embeds` at `a9e14853`. A tiny formatter URL-escape adapter reuses the existing view helper without adding a dependency. |
| `rust/crates/views/src/messages.rs`, `templates/messages/_message.html`, `lib.rs` | Typed optional provider fields and flagged root helper calls mount populated cards. Legacy fixtures without typed facts retain their existing fallback; ordinary presenters supply the new facts. |
| `rust/crates/views/src/messages/composer.rs`, `templates/messages/_composer.html` | Reused the published reusable WS8b-r composer from `origin/rust/ws8br-rooms-http` at `3e779615`. The owned schedule control fills its slot. Four complete Rails composers cover DM/root/thread and Drive metadata/share layouts. |
| `rust/crates/views/src/helpers/forms.rs` | Copied only the owner-provided namespace capability: thread IDs prefix field IDs while Rails parameter names stay unchanged. |
| `rust/crates/views/src/rooms.rs`, `templates/rooms/show/_composer.html`, `rust/crates/campfire/src/controllers/rooms.rs` | Small flagged optional shell seam. Ordinary room responses always supply Markdown facts, read the full builtin/room-agent registry and read only Google grant scopes for metadata-picker availability. Form rendering remains request-scoped, outside message fragments. Legacy WS6 shell fixtures retain the old fallback; broader shell ownership stays WS8b-r. |
| `rust/crates/db/src/slash_commands/calendar.rs`, `time_parser.rs`, `slash_commands.rs` | Separate builder calendar grammar with literal short years, compact civil defaults, named months, h/AMPM clocks, comma fractions, short offsets, invalid-date errors and Rails week/ordinal field defaults. Future-oriented slash parsing remains separate. |
| `rust/crates/campfire/src/controllers/message_features.rs`, `saved_items.rs`, `searches.rs`, `rooms/{files,slash_commands}.rs`, `autocompletable/{icons,slash_commands}.rs` | Explicit owned `to_s` sites use Ruby scalar/array/Parameters string forms. Kit-wide `Param::to_s`, WS9/auth casting and ActiveRecord lookup semantics were not changed. |
| `rust/crates/campfire/src/controllers/message_features/{provider_tests,composer_tests,date_tests,quote_integration_tests}.rs` | Actual persisted-row/container comparisons; zero-query preloaded rendering; private content/session exclusion; warm GitHub/embed/X cache refresh; ordinary Markdown/Drive/schedule/registry mount; 175 date probes, 15 coercions and eight exact reminder responses. |
| `rust/reference-tools/messaging/{providers,event_cards,twitter_preloads,composer,date_coercions}.rb`, corresponding `rust/vectors/messaging/*.json` | New actual pinned Rails fixtures. The canceled-event recipe explicitly updates cancellation after creation. No expected HTML was reconstructed from Rust. |
| `rust/reference-tools/embeds/{twitter_cards,twitter_text}.rb`, `rust/vectors/ws15e_twitter_*.json` | Owner-provided reference recipes/vectors: independently replayed against the pin before reuse, including 15 X cards, 19 formatter cases and 16 compact counts. Only the formatter portions of the text vector are asserted by the imported view tests; URL-extraction/domain behavior remains provider-owned. |
| `rust/reference-tools/messaging/{features-reference-check,features-discriminate,verify_oracles}.py` | 105 consumed reference files, seven new compiled mutation checks and 19 independently replayed oracle fixtures. |
| `rust/plans/ws8bm2-report.md` | Tracked copy of the external report. |

No schema, dependency, Cargo.lock, Rails-source, parity-mask or allowlist changes. No other worker branch was merged. The copied view files and small adapters are explicit cross-workstream touches; provider transports/write callbacks, Google endpoints, and broader room/message/thread shells retain their owners.

Provider rendering performs zero queries after preload, including 1 versus 18 GitHub/embed messages, distinct event containers and all persisted X containers. Warm related-row changes refresh root fragments without touching the message. X fixture identities exceed 64-bit integer range and sort numerically, including leading zeros. Shared fragments contain no private GitHub content, authenticity tokens or nonces. These checks establish read-only composition, not outbound fetch or write-callback delivery.

The composer uses the real request context for CSRF and exposes the full slash registry, including read-only agent metadata. It mounts the schedule dialog in the Markdown send row. Standalone thread facts match Rails, but the owning thread HTTP shell is not present on this base branch. Configured Drive-share slot bytes are proved; normal layout configuration/Google endpoints still need WS14g wiring. Browser interactions remain unproved.

Builder parsing is independent of slash-relative phrases. Actual Rails `Date._parse(..., false)` probes show that short years stay literal, that ordinal/week fields are recognized but ignored by TimeZone's civil conversion, and that a reminder array/hash string can still contain a parseable date. The port now reproduces the sampled behavior rather than dropping those parameters. Full Ruby date grammar, exact exceptional messages outside these probes, file-object coercion and odd ActiveRecord lookup/pager shapes remain partial. The slash commands' existing narrower calendar fallback has not been replaced by the new builder grammar; cross-path calendar grammar parity remains unproved.

## Fresh-clone verification

The remote branch was cloned at the verified source SHA, with no copied local vectors, seeds, secrets or Cargo build output. Both seeds were rebuilt in that checkout. All new test inputs are committed; scratch contains output and runtime seed copies only. The workspace suite and clippy use that clone's separate target directory. Concurrency and timing thresholds were unchanged.

From the assigned worktree root:

```sh
git clone --single-branch --branch rust/ws8bm2-message-features https://github.com/Smart-Data-Ohio/smartfire.git .scratch/ws8bm2-final/repo > .scratch/final-clone.log 2>&1
```

From `.scratch/ws8bm2-final/repo/rust` (the output/TMPDIR directory exists):

```sh
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 parity/bin/seed build default first_run > ../.scratch/final-seed.log 2>&1
mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 > ../.scratch/final-metadata.json
TMPDIR="$PWD/../.scratch/tmp" CARGO_TARGET_DIR="$PWD/target" CARGO_INCREMENTAL=0 CI=1 CABLE_TEST_PORT_RANGE=52500-52549 MAIL_TEST_PORT_RANGE=52550-52599 mise exec rust@1.98.1 -- cargo test --locked -j4 --workspace --exclude html5ever -- --test-threads=4 --nocapture > ../.scratch/final-workspace-test.log 2>&1
TMPDIR="$PWD/../.scratch/tmp" CARGO_TARGET_DIR="$PWD/target" CARGO_INCREMENTAL=0 mise exec rust@1.98.1 -- cargo clippy --locked -j4 --workspace --exclude html5ever --all-targets -- -D warnings > ../.scratch/final-clippy.log 2>&1
```

Clone, seed build, locked metadata, the final workspace suite and clippy exited 0. Metadata produced JSON and has no native summary line. Raw seed summaries:

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

The fresh suite totals **1684 passed, 0 failed, 11 existing ignored** across 46 raw summaries. It includes **150 owned feature tests and 45 owned search tests (195 total)**, with no owned ignores. No timing test flaked and no test concurrency/threshold was changed.

```text
test result: ok. 614 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 128.97s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.53s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.18s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 446 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 64.69s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.76s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.01s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.04s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.18s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 23.14s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.10s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.41s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.88s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.11s
test result: ok. 40 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.67s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
```

Raw fresh-clone clippy summary:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 53.16s
```

The eleven existing ignores are two reference recorders, WS11 bot management, the push-latency measurement, four database scenario/export/Rails/rollback tests, one mail rollback export and two kit doctests. Conditional ACME validation returned with PEBBLE_MINICA unset. Version-dependent media byte comparisons returned because this host has libvips 8.18.6/ffmpeg n9.0.2 while the vectors use 8.16.1/7.1.5. The missing-seed “skipping locally” line is the intentional empty-temporary-directory guard test; both required runtime seeds were rebuilt and validated. No owned seeded test skipped. html5ever is excluded explicitly by the standard workspace command above.


From `.scratch/ws8bm2-final/repo`:

```sh
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/campfire/verify_parity_seed.rb default > .scratch/final-validate-default.log 2>&1
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/campfire/verify_parity_seed.rb first_run > .scratch/final-validate-first-run.log 2>&1
python3 rust/reference-tools/messaging/features-reference-check.py > .scratch/final-reference-check.log 2>&1
python3 rust/reference-tools/messaging/verify_oracles.py > .scratch/final-oracle-replay.log 2>&1
```

All four exited 0. Validators closed with these raw lines (default, then first_run):

```text
  "passed": 29,
  "failed": 0
}
  "passed": 4,
  "failed": 0
}
```

Raw source check and independently replayed Rails summaries:

```text
WS8bm2 reference source check: 105 controller, model, helper and template files match d7c7de92
WS8bm2 reference check self-test: 2 injected source-byte/file-set differences rejected
WS8bm2 Rails oracle: 8 poll reads/ballots; 6 poll creates; 4 pin writes; 11 partials; 10 zone/date probes
WS8bm2 oracle replay: features.json byte-identical
WS8bm2 saved Rails oracle: 11 HTTP responses; 6 item partials; 1 empty page
WS8bm2 oracle replay: saved.json byte-identical
WS8bm2 scheduled Rails oracle: 12 HTTP responses; 8 row partials; 1 empty page; 2 composer controls
WS8bm2 oracle replay: scheduled.json byte-identical
WS8bm2 search Rails oracle: 15 parsed queries; 15 chip partials; 1 empty page; 1 clear stream; 18 zone/date selections; 1 populated sections partial; 1 load-older control; 1 empty older stream; 4 HTTP responses; 771 Unicode word ranges
WS8bm2 oracle replay: search.json byte-identical
WS8bm2 preload Rails oracle: 6 complete message fragments; 9 committed fixture tables
WS8bm2 oracle replay: preloads.json byte-identical
WS8bm2 slash Rails oracle: 20 dispatch responses; 17 picker responses; 61 play presentation fragments; 3 format responses; 12 huddle readiness cases
WS8bm2 oracle replay: slash.json byte-identical
WS8bm2 links/files Rails oracle: 15 Files sections; 5 quote HTTP responses; 2 quote partials; 16 size values; 9 fixture tables
WS8bm2 oracle replay: links_files.json byte-identical
WS8bm2 reminder push Rails oracle: 27 policy cases; 2 captured real job payload/subscription handoffs; 3 fixture tables
WS8bm2 oracle replay: reminder_push.json byte-identical
WS8bm2 quote integration Rails oracle: 4 containers; 7 messages; 3 fixture tables
WS8bm2 oracle replay: quote_integration.json byte-identical
WS8bm2 root cache Rails oracle: 6 composite keys; 12 provider frame containers; 10 fixture tables
WS8bm2 oracle replay: root_cache.json byte-identical
WS8bm2 panels Rails oracle: 4 pin panels; 2 sidebar links
WS8bm2 oracle replay: panels.json byte-identical
WS8bm2 date Rails oracle: 69 Time.zone.parse cases in 3 zones
WS8bm2 oracle replay: date_inputs.json byte-identical
WS8bm2 provider Rails oracle: 11 GitHub containers; 7 embed/LinkedIn containers; 8 fixture tables
WS8bm2 oracle replay: providers.json byte-identical
WS8bm2 event cards Rails oracle: 3 populated containers; 5 fixture tables
WS8bm2 oracle replay: event_cards.json byte-identical
WS8bm2 broader date/coercion Rails oracle: 106 calendar cases; 15 parameter string/presence probes; 8 reminder HTTP responses
WS8bm2 oracle replay: date_coercions.json byte-identical
WS8bm2 composer Rails oracle: 4 complete Markdown composers including thread and Drive-share controls
WS8bm2 oracle replay: composer.json byte-identical
WS8bm2 X preload Rails oracle: 3 populated containers; 4 persisted posts
WS8bm2 oracle replay: twitter_preloads.json byte-identical
WS8bm2 oracle replay: twitter_cards.json byte-identical
WS8bm2 oracle replay: twitter_text.json byte-identical
WS8bm2 oracle replay: 19/19 independently replayed fixtures byte-identical
```

## Failing first and compiled discrimination

Actual compiled failing-first tests are in `.scratch/{provider,event,twitter,composer}-failing-first.log`, `.scratch/date-coercion-failing-first.log` and `.scratch/param-failing-first.log`. Provider privacy/shared-response assertions failed before implementation; the X/event/composer tests failed on empty or legacy root composition; the date and coercion tests failed on actual Rails mismatches. A test-helper compile error during composer setup was corrected and is not counted as a failing-first result.

Raw failing-first summaries, in the log order above:

```text
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 604 filtered out; finished in 0.51s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 608 filtered out; finished in 0.13s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 609 filtered out; finished in 0.09s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 610 filtered out; finished in 0.43s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 613 filtered out; finished in 1.16s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 614 filtered out; finished in 0.00s
```

The focused continuation command below ran after all mutation sources were restored, with 150 owned feature tests passing. It is also covered by the final fresh-clone suite:

```sh
TMPDIR="$PWD/.scratch/tmp" CARGO_TARGET_DIR="$PWD/rust/target" CARGO_INCREMENTAL=0 CI=1 CABLE_TEST_PORT_RANGE=52500-52549 MAIL_TEST_PORT_RANGE=52550-52599 mise exec rust@1.98.1 -- cargo test --manifest-path rust/Cargo.toml --locked -j4 -p campfire controllers::message_features -- --test-threads=4 --nocapture > .scratch/final-owned-tests.log 2>&1
```

```text
test result: ok. 150 passed; 0 failed; 0 ignored; 0 measured; 467 filtered out; finished in 17.58s
```

From the assigned worktree root, these compiled mutations ran separately, each restoring its source afterward:

```sh
CARGO_INCREMENTAL=0 python3 rust/reference-tools/messaging/features-discriminate.py provider- > .scratch/provider-mutations.log 2>&1
CARGO_INCREMENTAL=0 python3 rust/reference-tools/messaging/features-discriminate.py event- > .scratch/event-mutations.log 2>&1
CARGO_INCREMENTAL=0 python3 rust/reference-tools/messaging/features-discriminate.py x- > .scratch/twitter-mutations.log 2>&1
CARGO_INCREMENTAL=0 python3 rust/reference-tools/messaging/features-discriminate.py composer- > .scratch/composer-mutations.log 2>&1
CARGO_INCREMENTAL=0 python3 rust/reference-tools/messaging/features-discriminate.py calendar- > .scratch/calendar-mutations.log 2>&1
CARGO_INCREMENTAL=0 python3 rust/reference-tools/messaging/features-discriminate.py coercion- > .scratch/coercion-mutations.log 2>&1
```

Each rejected the intended compiled regression: private GitHub disclosure, normalized URL replacing the reference URL, unsafe Meet URLs, reversed numeric X order, missing thread field namespaces, converting literal short years to 20xx, and dropping array-valued reminder dates. Raw closing lines:

```text
provider-private-content: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 607 filtered out; finished in 0.60s
provider-reference-url: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 607 filtered out; finished in 0.14s
WS8bm2 discrimination: 2 compiled regressions detected; sources restored
event-meet-scheme: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 608 filtered out; finished in 0.63s
WS8bm2 discrimination: 1 compiled regressions detected; sources restored
x-identity-order: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 609 filtered out; finished in 0.13s
WS8bm2 discrimination: 1 compiled regressions detected; sources restored
composer-thread-field-ids: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 613 filtered out; finished in 0.06s
WS8bm2 discrimination: 1 compiled regressions detected; sources restored
calendar-short-year: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 616 filtered out; finished in 2.29s
WS8bm2 discrimination: 1 compiled regressions detected; sources restored
coercion-array-reminder: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 616 filtered out; finished in 0.89s
WS8bm2 discrimination: 1 compiled regressions detected; sources restored
```

## Grouped Rails behavior coverage

Counts below are named Rails reference behaviors mapped to passing grouped Rust tests and pinned oracles; **the Rails controller suites were not run as suites**. They do not imply complete provider, shell or browser parity.

| Rails controller file under `test/controllers/` | Named behaviors covered / total | Remaining integration |
| --- | --- | --- |
| `rooms/polls_controller_test.rb` | 16 / 16 | Browser phase; additional performance/runtime proof below |
| `messages/pins_controller_test.rb` | 6 / 6 | Action menus/browser phase |
| `rooms/pins_controller_test.rb` | 3 / 3 | Browser phase; additional performance proof below |
| `saved_items_controller_test.rb` | 12 / 12 | WS17 physical send seam; browser phase |
| `scheduled_messages_controller_test.rb` | 19 / 19 | Composer mounted; thread-shell/browser phase |
| `searches_controller_test.rb` | 36 / 36 | Read-only provider composition mounted; provider endpoints/callbacks and populated whole-page proof |
| `rooms/slash_commands_controller_test.rb` | 9 / 10 | WS11 agent invocation seam |
| `autocompletable/icons_controller_test.rb` | 6 / 6 | Browser phase |
| `autocompletable/slash_commands_controller_test.rb` | 7 / 7 | Browser phase; WS11 invocation seam |
| `autocompletable/users_controller_test.rb` | 5 / 5 | Odd ActiveRecord lookup/pager shapes remain |
| `rooms/message_links_controller_test.rb` | 12 / 12 | Browser phase |
| `rooms/files_controller_test.rb` | 8 / 8 | WS8b-r Files tab/shell; odd-shape coercions; browser phase |
| **Total named controller behavior ports** | **139 / 140** | **One WS11 agent seam** |

| Rails system file under `test/system/` | Browser cases proved / total |
| --- | --- |
| `polls_test.rb` | 0 / 4 |
| `pins_saved_test.rb` | 0 / 7 |
| `slash_commands_test.rb` | 0 / 26 |
| `search_files_test.rb` | 0 / 4 |
| `scheduled_messages_test.rb` | 0 / 4 |
| **Total** | **0 / 45; reserved for end-to-end phase, none attempted** |


## Remaining work, in priority order

1. **Provider/owner integration:** batched populated root rendering is complete for the rows above. WS15g/WS15e/WS14e retain fetch requests/transports, reference synchronization, provider-write broadcasts, private GitHub/Fizzy card endpoints and event attendance endpoints. Complete human edit replacement targets still need the owner adapters. Prove populated full search/older-window bytes with those integrated callbacks; container goldens and read-only preloads do not establish whole-page provider parity. Audit request-origin scopes on other owners' deletion/unpin paths.
2. **Owning shell:** ordinary room Markdown/schedule mounting is complete; reusable thread bytes are proved. Wire the owning thread shell, configured Google/Drive-share availability/endpoints (WS14g), action menus, Files tab and full WS8b-r shell. Preserve correct voice/stage/board STI reply-control identifiers when that shell supplies broader room facts. Legacy WS6 fixture fallback remains explicitly partial.
3. **Date/coercion:** 175 actual Rails builder cases and 15 parameter probes pass. Remaining unprobed Ruby formats include negative/long years, variable compact/zone forms and exact exceptional messages. Odd ActiveRecord array/hash lookup semantics, user filtering, structured pager/link parameters, option stripping and file-object coercions still need actual Rails differentials. No universal Ruby date/coercion parity is claimed.
4. **Additional non-browser proof:** poll/pin-list constant-query measurements and a periodic poll-close job's real runtime socket delivery remain unverified. Existing vote/pin sockets and quote/provider zero-query checks do not establish those separate paths.
5. **Explicit deferred seams:** physical tagged push send stays at `rust/crates/campfire/src/jobs/reminders.rs` (WS17); agent credentials remain at `concerns.rs::WS11_AGENT_AUTHENTICATION_SEAM` and agent command execution at the dispatcher (WS11, the last 1/140 named controller behavior); huddle launch stays at the WS13 dispatcher seam. None was implemented here.
6. **End-to-end phase:** all 45 browser cases inventoried above remain deferred against the Rust server. No browser harness, browser/pixel matrix or Rust Docker build was attempted.

No open product decision is required. The external report and tracked copy are byte-identical. The implementation remains partial at the explicit boundaries above.
