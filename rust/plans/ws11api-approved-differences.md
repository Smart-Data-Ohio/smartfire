# Approved agent API differences

The current approvals below cover committed-file ownership, proxy security headers
and authorization races. The JPEG/video crash approvals from PR #192 are history,
superseded by #226/#233's deferred attachment processing. They no longer authorize
different posting statuses or missing generated files.

Both fresh closed-thread cases now return 201 in Rails and Rust. Their real processing
jobs run after the post commits and retain the JPEG variant and video preview/WebP
files. Current evidence is
[`agent_review192r2_attachment.json:13`](../vectors/agent_review192r2_attachment.json#L13),
its [JPEG file retention:127](../vectors/agent_review192r2_attachment.json#L127),
and [`agent_review192r3_attachment.json:13`](../vectors/agent_review192r3_attachment.json#L13).
The Rust consumers execute the queued job and compare complete state/files at
[`agent_review_r2_tests.rs:310`](../crates/campfire/src/controllers/agent_review_r2_tests.rs#L310)
and [`agent_review_r3_tests.rs:79`](../crates/campfire/src/controllers/agent_review_r3_tests.rs#L79).
See [message attachment processing](message-attachment-processing.md#evidence) and
[the pin refresh](parity-pin-refresh.md#changed-golden-attribution) for the replacement
of the old crash diagnostics with current Rails captures. Native media sizes/checksums
still require the pinned runtime; they are not masked.

## Historical JPEG approval (PR #192 second review; superseded by #226/#233)

At the old Rails pin `d7c7de92`, with Rails gem `1a02651ac37f`, posting a fresh JPEG
in a closed thread returned Rails 500 versus Rust 201. Rails raised `IOError: closed stream`
from `IO.copy_stream` in `activestorage/lib/active_storage/service/disk_service.rb:23`:
`ActiveStorage::VariantWithRecord#transform_blob` had closed its transformed output
before `ChannelThread#post_message!` committed and the deferred upload used it.
The message, reopened thread and media rows had committed, but the variant file was
absent. The maintainer approved Rust's successful response and staged-variant discard
at that checkpoint instead of reproducing the crash. That discard is retired;
the current JPEG vector requires the generated file to exist.

## Historical video approval (PR #192 third review; superseded by #226/#233)

At the same old pin, posting a fresh uploaded copy of fixture video 9 returned Rails
500 with `ActiveStorage::FileNotFoundError` from `DiskService#stream` at
`activestorage/lib/active_storage/service/disk_service.rb:152`. The variant transform
opened the preview before its deferred upload committed. Rails rolled back the message,
thread reopening, preview/variant rows and jobs; the separately uploaded source survived.
The maintainer approved Rust 201 with retained source/preview/WebP files, using Rails
preprocessing outside the posting transaction as the successful state oracle.
The current vector captures an ordinary successful Rails post followed by its actual
processing job instead; there is no current video crash or rollback exception.

## Committed file ownership and missing-file serving (PR #192 fourth and fifth reviews)

Staged original/preview/variant files are retained immediately when COMMIT
succeeds, before fallible model callbacks. A callback exception still propagates
and cancels later ordinary callbacks, matching pinned Rails' transaction runner.
It cannot revoke ownership of committed files or the notifications for committed
durable jobs. Failed writes, savepoints and failed COMMIT still remove staged
files. The callback experiment and its committed receipt live in
`reference-tools/agents/review192r4_commit_callbacks.rb` and
`review-192-r4-callbacks.json`.

This file-retention guarantee is a deliberate state difference. A Rails
after-commit callback that raises before Active Storage's deferred upload leaves
the image blob, attachment and variant rows committed without the image file;
later callbacks, including the upload, are skipped. Rust retains the staged file
whenever COMMIT succeeds, independently of those fallible callbacks. Both return
500 for the callback exception and skip later ordinary callbacks. The maintainer
approved preserving committed file ownership instead of reproducing Rails'
missing-file state. This callback-failure difference is independent of the retired
JPEG/video posting approvals.

Representation requests preserve Rails' handled missing-file responses. Processed
records are reused without regenerating files: an existing WebP can be served
without its intermediate JPEG preview. When the final variant file is absent,
the redirect still returns 302 to the signed disk URL, whose follow-up is an empty
404; the representation proxy returns an empty 404 with the image content type,
disposition and no-cache header. Rows, files and jobs stay unchanged. These are
ordinary handled responses, not an approved crash difference. The three scenarios
and their exact bodies/headers are recorded in
`agent_review192r5_representations.json`.
Rails permits representation requests to reuse a committed variant record even
without its file; missing-file serving uses the handled responses above. This is
separate from fresh posting and its deferred processing job, whose current vectors
require retained generated files. The video oracle retains every attachment's
record_id and record_type, with exact foreign keys and generated row-count checks.

## HTTP/1.1 proxy security headers (PR #203 follow-up)

The maintainer explicitly approved retaining Rust's six security defaults on
Active Storage representation and blob proxy responses. Pinned Rails uses
ActionController::Live's response and omits them with HTTP/1.1, including
successful streams, ranges and handled empty 404/416 responses:

| Approved extra Rust header | Required value |
| --- | --- |
| Permissions-Policy | camera=(self), display-capture=(self), microphone=(self), notifications=(self) |
| Referrer-Policy | strict-origin-when-cross-origin |
| X-Content-Type-Options | nosniff |
| X-Frame-Options | SAMEORIGIN |
| X-Permitted-Cross-Domain-Policies | none |
| X-XSS-Protection | 0 |

No other extra/missing name or changed value is approved. Date, X-Request-Id
and X-Runtime are per-request values allowed only by those names; their format
and single-value cardinality remain checked. Content-Transfer-Encoding is
compared, including its absence on proxies. CSP is compared byte for byte:
fixed test-only nonce-generator entropy is an input on both sides, not an
output/header mask. Production nonce generation stays random.

`review192r5_missing_representations.rb` and `blob_proxy_headers.rb` now use
`Rack::Builder.parse_file(config.ru)` with Rack::Deflater, explicitly assert the
request's SERVER_PROTOCOL is HTTP/1.1 and retain it in each raw receipt. Every
response header and all values are captured. The all-header comparator rejects
unexpected names, changed unapproved values and duplicate values, including on
approved names. The previous nine-name projection and HTTP/1.0 default hid
this difference; the inventory's old claim of exact proxy header parity is
superseded by this explicit approval. Bodies, statuses, media bytes and stored
state remain exact for these proxy requests.

## Current authorization and owner-transfer races (#205 review)

The maintainer approved retaining Rust's stricter current-state checks. Astra's
`/home/riels/.cache/rust-port/ws11apir/review-205-r1/behavior-race-results.md`
classifies 13 of 34 committed prewriter races and four of ten private response
races as inherited differences, not batching or #205 regressions. Its baseline
89 cases / 100 requests has zero differences; the remaining 21 prewriter and six
private cases match exactly. This approval does not change production checks.

Both create surfaces recheck membership, manage_threads grants and suspension
inside the writer. Membership deletion gives Rust 404 versus Rails' invalid-owner
422; revoked manage grants give Rust 403 versus Rails 201 with two jobs; suspension
gives Rust 403 versus Rails' invalid-owner 422. Seven noncreate write surfaces
reject a newly suspended sender with Rust 404 while Rails' earlier Agent snapshot
continues with 200/201. Rust preserves the injected changes, writes no source jobs
and does not proceed on the stale authorization. MCP retains HTTP 200 with the
corresponding error/success envelope differences.

Four REST/MCP owner transfers before identity selection or during the private
repository reply redact private title/head/base in Rust, while Rails retains its
earlier owner association. Rust also skips the earlier identity's external read
when the transfer precedes identity selection. Public fields remain visible;
all 21 persisted table snapshots and jobs agree. Ordinary allowance, disconnect
and account relink behavior remain unchanged. Current owner/account checks,
IdentityGuard and final sealing stay authoritative; this narrow approval does not
allow unrelated response, authorization or state differences.
