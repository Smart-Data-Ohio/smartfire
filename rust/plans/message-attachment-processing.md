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

The unchanged-assignment regression was added in `1e494524b` before fixing the
model's early return. Fresh Rails queued one processing job for that assignment;
the Rust control reported:

```
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2837 filtered out; finished in 0.47s
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
