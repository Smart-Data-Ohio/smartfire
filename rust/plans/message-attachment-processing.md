# Message attachment processing (#226)

The Rust implementation now follows the approved Rails attachment concern and
`Message::AttachmentProcessingJob`. Thread posts, forwards, attachment replacements
and thread webhook replies schedule processing after the outer transaction commits.
An unchanged blob assignment also schedules Rails' save callback.
Root posts and the existing imported-message job keep Rails' guarded inline path:
a media error cannot undo or fail a committed post. Decoding uses the existing
storage/ffmpeg adapter off the database writer; generated JPEGs commit before their
WebP variants are opened. Their ordinary analysis jobs remain deferred.
Agent root attachment posts also follow Rails' save/process/broadcast/fan-out
order. Broadcasting before inline processing would let recovery claim the blob
first, leaving its metadata and first presentation pending. The existing corrupt
MOV HTTP golden exposed that ordering defect; a new regression covers both valid
and corrupt root uploads. Broadcast recovery queries only missing video previews.

Scheduling uses the existing nullable blob columns. Atomic claims last 15 minutes,
renew only for the owner token, and recover at inclusive expiry. Guarded release and
failure writes cannot clear a newer claim. Three decoder errors leave the durable
`failed` token and no expiry, preventing views from starting new retry batches.
Rails' decoder retries use polynomial waits (3 and 18 seconds plus 15% jitter).
Refused enqueues preserve separate 1/2/4/8/15-minute cooldowns, capped at 15 minutes.
The terminal marker and terminal queue outcome commit together, including a retry
of that commit without another decoder run.

Successful processing touches every message still attached to the blob and emits
Rails' exact `messages/presentation` replace with `maintain_scroll="true"`. An edit
of the scheduling message skips only that owner. Rendering recovers missing video
previews from the PR-card cache-key path, even on a fragment-cache hit, and from
ordinary attachment presentation and detached broadcasts. Pending and permanently
failed videos omit the poster and have no permanent spinner or error UI.

## Evidence

The test-only commit `b28ed7439` precedes production changes. Against main's Rust
implementation, concurrent views, expired-lease recovery, terminal failure and
restart tests reported:

```
test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 2820 filtered out; finished in 0.86s
```

`reference-tools/messaging/attachment_processing.sh` verifies the pinned Rails
image (`d7c7de9264c63015be398001d7a1094e7695a6db`) and overlays only the four approved
#226 Ruby files read from the reference checkout. It runs the existing isolated
seed harness, frozen clock and canonical media tools. The vector records full
message/blob rows including lease columns, queued jobs, retry/cooldown times,
presentation HTML before/after processing, broadcast bytes, and source/JPEG/WebP
sizes and SHA256s. Regenerate with:

```sh
bash reference-tools/messaging/attachment_processing.sh > vectors/message_attachment_processing.json
```

The same overlay regenerated the agent attachment and thread/JPEG boundary vectors;
the old diagnostics now execute the actual deferred Rails job instead of describing
the superseded transactional crash. Rust assertions compare the rows, file checksums,
HTML and broadcast payloads without masking their contents. Additional regressions
cover replacement PATCHes, shared owners, refused enqueues, stale releases, cached
recovery and an orphaned worker claim recovered by a newly started runner.

No Rails source, schema, asset or pin changes are required.

## Astra review follow-up (#233)

Completion now uses the fallible message publisher in an after-commit callback,
instead of the ordinary model event sink that logs and drops rendering failures.
The touch still commits before rendering, and a failed completion reaches the
job's three-attempt retry handler. A missing owner is skipped; other owners still
receive their completion. Unrelated model callbacks retain their existing rescue
behavior.

Root webhook replies now drain the presenter's recovery requests after releasing
the reader. Both agent and legacy sync replies use that helper. The presenter
audit also found full-message rendering in the composer's direct response
(including duplicate submissions and warm collection caches), reply tombstone
broadcasts, imported-mail completion, and quiet-stream completion. These paths now
preserve recovery. Ordinary committed message/presentation broadcasts already
recover using the committing connection, avoiding a recursive writer call.
Room, thread, search and controller rendering use the shared draining adapters.
Reactions, quote/thread controls, directory/board rows and other component-only
renderers do not render an attachment or its PR-card cache key in Rails. The
notifier, huddle and digest producers normally create attachment-free notes;
their full-message event handlers also need recovery when a video is attached.
The dispatcher follow-up below covers those handlers.

The reuse regression again posts once, performs its real processing job, records
the JPEG and WebP identities and bytes, and then posts/processes the same source
again. Both artifacts and the existing variant record must survive unchanged.

The failure controls in test-only commit `6a9964307` ran against the reviewed
`3fd4814b6` production sources. The completion control left a generated JPEG but
zero retry jobs and zero frames; the root webhook control left zero recovery jobs.
Fresh pinned Rails output recorded one retry (execution 1, cleared lease) and one
webhook recovery (token suffix 0, no preview/poster), respectively. Both Rust
assertions failed:

```text
test result: FAILED. 19 passed; 2 failed; 0 ignored; 0 measured; 2820 filtered out; finished in 16.71s
```

Regenerate the new failure vector through the same pinned overlay harness:

```sh
bash reference-tools/messaging/attachment_processing.sh \
  reference-tools/messaging/attachment_processing_failures.rb \
  > vectors/message_attachment_processing_failures.json
```

The completion regression runs the real queue worker, injects a detached-render
failure after scheduling, checks the Rails retry/lease state, restores rendering,
advances the frozen clock to the retry and verifies the completion frame. The
webhook regression uses the real corrupt-MOV decoder path and subscribed append.
Additional controls cover duplicate composer responses, quiet-stream completion
and imported-mail recovery.

Follow-up validation used the canonical media image, `CI=1`, all three required
seeds, eight test threads and an immutable test executable. The full app binary
includes the affected message, webhook, mail, channel and reuse suites. Strict
workspace Clippy ran with `--locked --workspace --exclude html5ever --all-targets
-- -D warnings`, two Cargo build jobs and the unchanged rustc throttle:

```text
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 2825 filtered out; finished in 4.49s
test result: ok. 2831 passed; 0 failed; 13 ignored; 0 measured; 0 filtered out; finished in 480.34s
Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 09s
```

Inherited acknowledgement window: a restart after publishing completion but
before durable queue acknowledgement can publish completion twice. Fresh Rails
also publishes twice when replaying the same serialized job, as Astra's probes
confirmed. This port retains that behavior and does not claim exactly-once
publication or add deduplication.

The unchanged-assignment regression was added in `1e494524b` before fixing the
model's early return. Fresh Rails queued one processing job for that assignment;
the Rust control reported:

```
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2837 filtered out; finished in 0.47s
```

## Dispatcher recovery follow-up (#233)

`StepParentChange` rendered the whole parent message but was absent from the
after-commit recovery dispatch. Creating or updating an agent step on a video
with no preview therefore published posterless HTML without scheduling its
preview. Fresh pinned Rails output queues one processing job on both paths,
with token suffix `0`, a 900-second lease and no poster yet.

The dispatch now recognizes every full-message renderer: ordinary message,
replacement and presentation partials; step parent changes; quiet-stream
completion; GitHub notifier messages; digest notes; and stage-ended notes.
Recovery runs after the renderer releases its reader, on the committing
connection. A single bound JSON-list query handles digest batches without
per-message recovery reads or exceeding SQLite's variable limit. Existing
per-blob claims still prevent duplicate enqueues.

The sweep checked all 21 broadcast kinds in `channels::sink`. The remaining
handlers render only child cards, badges, directory rows, thread steps,
thread indicators or room/huddle controls, or send removals/control frames.
They do not render an attachment or its PR-card collection cache key in Rails.
Fresh Rails also confirms that a system note can render an attached video and
must recover it, whereas a thread-indicator replacement must not enqueue.

Test-only commit `c252e077e` added real authenticated step POST/PATCH controls
and a notifier/digest/system-note/stage-note dispatcher sweep, with a negative
thread-indicator control. Against `d2623ea89` production sources, all three
tests failed on missing recovery jobs and NULL leases; HTTP statuses and video
frames already matched Rails:

```text
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 2844 filtered out; finished in 2.00s
```

Regenerate the event vector through the same pinned overlay and isolated seed:

```sh
bash reference-tools/messaging/attachment_processing.sh \
  reference-tools/messaging/attachment_processing_events.rb \
  > vectors/message_attachment_processing_events.json
```

Validation uses the canonical media image, `CI=1`, all three required seeds,
eight test threads and an immutable test executable. Strict workspace Clippy
keeps the rustc throttle and two Cargo build jobs. The inherited publication
before acknowledgement window remains unchanged; replay after a later owner's
render failure can also republish an earlier owner's completion, as Rails does.

```text
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 2825 filtered out; finished in 5.00s
test result: ok. 2834 passed; 0 failed; 13 ignored; 0 measured; 0 filtered out; finished in 489.12s
Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 16s
```

## Validation

The canonical toolchain image ran the affected app, database, queue/crash,
storage/media vector and view tests with `CI=1`, the `default`, `first_run` and
`agents_ui` seeds, and eight test threads. All 14 view integration binaries
also passed. Strict Clippy used `--workspace --exclude html5ever --all-targets --
-D warnings`. The workspace binary build used `ci/with-release-inputs.sh`;
the rustc throttle remained in place and Cargo used two build jobs.

```text
regressions
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 2826 filtered out; finished in 3.85s
campfire
test result: ok. 2826 passed; 0 failed; 13 ignored; 0 measured; 0 filtered out; finished in 894.75s
campfire_db
test result: ok. 1379 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 92.91s
campfire_jobs
test result: ok. 59 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.85s
job crash recovery
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.01s
campfire_storage
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
storage vectors
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.74s
campfire_views
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
strict Clippy
Finished `dev` profile [unoptimized + debuginfo] target(s) in 41.60s
release-input build
Finished `dev` profile [unoptimized + debuginfo] target(s) in 56.20s
```
