# Message attachment processing (#226)

The Rust implementation now follows the approved Rails attachment concern and
`Message::AttachmentProcessingJob`. Thread posts, forwards, attachment replacements
and thread webhook replies schedule processing after the outer transaction commits.
Root posts and the existing imported-message job keep Rails' guarded inline path:
a media error cannot undo or fail a committed post. Decoding uses the existing
storage/ffmpeg adapter off the database writer; generated JPEGs commit before their
WebP variants are opened. Their ordinary analysis jobs remain deferred.

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
