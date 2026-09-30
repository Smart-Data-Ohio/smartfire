# Review follow-up: sanitized attachment filenames (2026-09-30)

Verified source commit: `3ef4fd645ac2a6e155a5534ee55b7c242e5aa1a4`.
Identification fix: `8c303dc014bd0e1224a6727ef3289f2922143f26`.
Failing-first base: `76e922725258948c9da59bae78305ac26fb46b15`.
Rails oracle: `d7c7de9264c63015be398001d7a1094e7695a6db`.

## Change and Rails audit

`crates/storage/src/storage.rs::identify_blob` now passes `Filename#sanitized` to Marcel,
matching pinned `ActiveStorage::Blob::Identifiable#identify_content_type`'s `filename.to_s`.
The source audit also covered `Blob#extract_content_type`, `Blob#filename`,
`ActiveStorage::Filename#sanitized`, disk disposition and storage URL handling.
`NewBlob::content_type` already sanitizes filenames for byte and file uploads; URL/disposition
paths already sanitize too. Rails persists the original filename string in the database, and
Rust continues to match that. Base/extension methods intentionally operate on the original name,
as Rails' `File.basename`/`File.extname` do.

The audit additionally found that `app/services/messages/forwarder.rb::copy_attachment_to`
explicitly supplies `filename.to_s` to `create_and_upload!`, so a forwarded copy stores the sanitized
name. `Storage::stage_copy` now matches that assignment. It continues to retain the source MIME,
metadata and bytes, and the original upload's row retains its raw filename. These are the only two
production changes.
No schema, queue timing, view, asset golden, threshold, mask or unrelated source change was needed.

## Failing first

Added tests and pinned vectors before changing production code. On `76e92272`, ran with
`CARGO_BUILD_JOBS=2`, dev/test debug 0, incremental 0, `CI=true`, disk-backed worktree `TMPDIR`
and eight test threads:

```sh
mise exec rust@1.98.1 -- cargo test --locked -p campfire_storage blob_filenames_match_pinned_rails -- --test-threads=8
mise exec rust@1.98.1 -- cargo test --locked -p campfire signed_filename -- --test-threads=8
```

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 8 filtered out; finished in 0.03s
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 684 filtered out; finished in 0.74s
```

- Storage corpus: `file.png/` with `hello` identified as `image/png`, versus Rails'
  `application/octet-stream` (sanitized `file.png-`).
- Signed slash HTTP regression: a SQLite trigger rejects durable AnalyzeJob inserts; old Rust
  returns 500 rather than Rails' 302. Correct behavior commits the attachment, sets identified
  and analyzed metadata, and queues no job.
- Signed trailing-space HTTP regression: old Rust identifies `application/octet-stream`, whereas
  Rails identifies `image/png` (sanitized `file.png`), leaves analyzed unset, and enqueues one job.

After the identification change, the same commands passed:

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out; finished in 0.64s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 684 filtered out; finished in 0.63s
```

The forward-copy regression was added before its production change, against `8c303dc0` (whose
copy path was unchanged from `76e92272`). It fails because `file.png/` is stored unchanged instead
of Rails' `file.png-`:

```sh
mise exec rust@1.98.1 -- cargo test --locked -p campfire_storage forwarded_filenames_match_pinned_rails -- --test-threads=8
```

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 9 filtered out; finished in 0.03s
```

Both storage corpus regressions pass after the copy fix:

```sh
mise exec rust@1.98.1 -- cargo test --locked -p campfire_storage filenames_match_pinned_rails -- --test-threads=8
```

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out; finished in 0.42s
```

The final HTTP regressions also create and persist a copy, which is included in Rails readback:

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 684 filtered out; finished in 0.76s
```

The shared direct-upload test helper now restores the declared content type as well as empty
metadata after staging, so test setup accurately models a blob created before identification.
HTTP tests use real admin authentication, CSRF and SQLite. Real durable jobs are held by the
existing AFTER INSERT trigger for inspection; no analyzer or queue mock is used in Rust.

## Pinned differential corpus and readback

`reference-tools/attachments/generate_filenames.rb` generated
`vectors/attachment_filenames.json` from the pinned Rails app: **141 filenames × four byte
inputs = 564 cases**. Inputs are `hello`, empty bytes, a PNG and a valid PDF. Names cover trailing
slashes, backslashes, repeated/path separators, spaces, every ASCII control 1–31 at the start,
middle and end, leading/trailing NUL stripping, DEL, non-ASCII whitespace, bidi override,
replacement punctuation, accented/composed/decomposed Unicode, Japanese, emoji and Unicode
slash/backslash lookalikes. Interior NUL is excluded because Ruby's File.extname rejects it.
The corpus compares sanitized and raw persisted names, identified MIME, identification metadata,
ordinary upload metadata, and deferred analyzer selection. Each case tests signed-blob
identification, ordinary byte upload, and file staging. It also checks **560 forward copies**: stored sanitized names, retained MIME/metadata and identical
bytes. The four raw-NUL names are excluded from copying because Rails' Blob#open rejects them
before a copy can be made. Both original rows and stored copies are checked through row lookup.
The full corpus regenerated byte-for-byte in the independent final fresh clone.

Two further Rails vectors are actual signed logo assignments with authenticated admin sessions
and forgery protection enabled. `file.png/` returns 302 with NullAnalyzer, zero analysis jobs,
and identified/analyzed metadata; `file.png ` returns 302 with Vips ImageAnalyzer, one job,
and identified metadata only.

The HTTP tests exported their actual committed SQLite rows and uploaded files. Pinned Rails'
`reference-tools/attachments/verify_filenames.rb` read both back, validating the account, blob and
attachment, bytes, both filename forms, MIME, analyzer, metadata, signed lookup, foreign keys,
and durable analysis job counts:

```text
Rails filename_slash readback: valid account/blob/attachment and copied blob; raw and sanitized names, bytes, MIME, analyzer, metadata and 0 analysis jobs match
Rails filename_space readback: valid account/blob/attachment and copied blob; raw and sanitized names, bytes, MIME, analyzer, metadata and 1 analysis jobs match
```

Generation and readback commands (from `rust/`, with `PARITY_NAMESPACE=attach`,
`PARITY_OWNER=attach`, `PARITY_IMAGE=attach-reference:d7c7de92`):

```sh
parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze reference-tools/attachments/generate_filenames.rb
parity/bin/reference runner --storage ../.scratch/filename-final-readback/filename_slash --time 2026-03-02T16:00:00Z --freeze reference-tools/attachments/verify_filenames.rb filename_slash
parity/bin/reference runner --storage ../.scratch/filename-final-readback/filename_space --time 2026-03-02T16:00:00Z --freeze reference-tools/attachments/verify_filenames.rb filename_space
```

## Final fresh-clone verification

Independent `git clone --no-local` under `.scratch/fresh-filenames-final`, checked out at
`3ef4fd645ac2a6e155a5534ee55b7c242e5aa1a4`, with an empty target directory. Both pinned seeds
were built in this clone using `parity/bin/seed build default first_run`. Rails validated all
29 default and four first-run checks, with zero failures. Corpus regeneration is byte-identical.

Locked metadata passed in the worktree and final clone/CI image. Strict TOML parsing checked
all 13 workspace manifests and the workspace dependency table:

```text
duplicate-key check: 13 workspace manifests parsed; 75 unique workspace dependency keys; 0 duplicates
```

The full workspace, including app, db, views, assets, storage and doctests, passed:
**1775 passed, 0 failed, 12 existing ignores**. There are no inherited failures and no added
ignores. All attachment regressions ran with the real parity seed. No asset golden helper change
was required. Clippy with `-D warnings` and the normal binary build passed.

All checks used `CARGO_BUILD_JOBS=2`; all tests used `--test-threads=8`. The host compiler throttle
and its shared slot locks remained in use, through the same container wrapper documented in the
previous follow-up below. The Docker command is identical except the source bind is now
`.scratch/fresh-filenames-final:/src`. No global cargo configuration was changed. The canonical
CI toolchain image (`attach-toolchain-ci:1.98.1`,
`sha256:80bed826ce3b998e8ba75b85b25d18055066760982413e4483dd9779c5f053b2`)
runs the pinned media tools, so the storage byte parity gate executes in full.

Final clone check arguments:

```sh
cargo metadata --locked --format-version 1
cargo test --locked --workspace --exclude html5ever --no-fail-fast -- --test-threads=8
cargo clippy --locked --workspace --exclude html5ever --all-targets -- -D warnings
cargo build --locked --workspace --bins
```

Raw full-suite result lines in execution order:

```text
test result: ok. 683 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 98.13s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.51s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.79s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.02s
test result: ok. 445 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 45.22s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.72s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.54s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.25s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.64s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 34.07s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.53s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.36s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.72s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.49s
test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.66s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Raw clippy and binary-build completion lines, respectively:

```text
    Finished `dev` profile [unoptimized] target(s) in 1m 28s
    Finished `dev` profile [unoptimized] target(s) in 44.89s
```

Both fresh-clone targets from this follow-up were deleted after their checks (2.8 GB each),
and no `.scratch/*/rust/target*` directory remains. The primary worktree `rust/target` is retained.
No test process or `attach-filename` container remains running. Scratch logs and readback rows
are outputs only; no test depends on them.

Changed files: `crates/storage/src/storage.rs` (identification and copy assignment),
`crates/storage/tests/vectors.rs` (two corpus regressions),
`crates/campfire/src/controllers/presenters/attachments/tests.rs` (two HTTP regressions and
accurate direct-upload setup/readback), `reference-tools/attachments/generate_filenames.rb`,
`reference-tools/attachments/verify_filenames.rb`, `vectors/attachment_filenames.json`, and
this report. No other production path or workstream changed.

Earlier follow-ups below retain their historical commands and results.

---

# Review follow-up: Rails NullAnalyzer (2026-09-30)

Verified source/merge commit: `1760c69c90223c2fd646caafec532b0d19e1eb3e`.
Fix commit: `3b0e4350ee0ed4cbc03e3895b9024d9f8cf17f2e`.
Failing-first base: `3ac47dd0f8cc1a3f0761dd3581023a208c90dc75`.
Merged `origin/main` at `b66199b7e2364e4bc4a8fda7fda3193cec112f98` (#166, WS15e), with a merge commit.
Rails oracle: `d7c7de9264c63015be398001d7a1094e7695a6db`.

## Change

`controllers/presenters/attachments.rs::enqueue_analysis` now uses the existing Rails-compatible
`Analyzer::for_content_type(...).analyze_later()` selector after identification. Image, video and
audio still persist `ActiveStorage::AnalyzeJob` inside the triggering write. Text and PDF select
`NullAnalyzer`: the attachment commits first, then an after-commit hook runs a separate writer
transaction, merges `analyzed: true` into current metadata and touches the attached records.
It queues no analysis job and needs no storage read or media process. Already-analyzed and deleted
blobs are no-ops. A failure in that separate write propagates the production 500 while preserving
the primary attachment and parent fields, as Rails does.

The pinned gem's `ActiveStorage::Attachment#analyze_blob_later`,
`Blob::Analyzable#analyze_later`, and `Analyzer::NullAnalyzer.analyze_later?` were read directly
from the reference image. This keeps inline after-commit work after commit, per decisions.md,
while preserving the durable-job atomicity exception. All existing callers use this helper.
No Rails source, schema, masks, test timing thresholds or unrelated jobs were changed.

Merge resolution in `controllers/messages.rs` preserves signed assignment and transactional
analysis while retaining main's `Message::edit` path and `markdown_source`. An initial fresh
build caught two missed conflict regions; they were resolved and the unpublished merge amended
before successful verification. No conflict markers remain.

## Failing first and regressions

Before changing production code, on `3ac47dd0`, with fresh test-created databases, admin sessions
and real CSRF, ran (both commands used `CARGO_BUILD_JOBS=2`, dev/test debug 0,
incremental 0, `CI=true` and disk-backed worktree `TMPDIR`):

```sh
mise exec rust@1.98.1 -- cargo test --locked -p campfire null_analyzer -- --test-threads=8
mise exec rust@1.98.1 -- cargo test --locked -p campfire attachment_analyzers_match_pinned_rails -- --test-threads=8
```

```text
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 424 filtered out; finished in 0.53s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 426 filtered out; finished in 0.53s
```

- Rejecting durable AnalyzeJob inserts: signed text logo assignment returned 500 rather than 302.
- Commit boundary: text inserted one durable analysis job instead of zero. The retained test also
  verifies metadata is unset inside the attachment transaction and in an earlier after-commit hook.
- Failing inline metadata update: old code returned 302 instead of Rails' 500. The fixed regression
  checks that the parent fields and attachment remain committed, identified stays true, analyzed
  stays unset, and no analysis job exists.
- Five-type differential: text metadata lacked `analyzed: true`. The fixed test checks all five
  identified MIME types, response statuses, analyzer timing, metadata and job counts, and rejects
  job inserts for image, video and audio to verify their assignments still roll back atomically.

After the fix, all original eight attachment regressions and four new regressions passed:

```sh
mise exec rust@1.98.1 -- cargo test --locked -p campfire attachments::tests -- --test-threads=8
```

```text
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 415 filtered out; finished in 2.40s
```

Real durable jobs are held by a SQLite AFTER INSERT trigger for inspection; no queue/analyzer mock
is used. Fixtures and expected data are committed; optional readback output is created by the test,
not read as a dependency. Fixture SHA256 checks verify identical inputs.

## Pinned Rails differentials and readback

`reference-tools/attachments/generate_analyzers.rb` uses direct-upload blobs, authenticated admin
requests, forgery protection, real uploaded bytes and Rails' test job adapter. It generated
`vectors/attachment_analyzers.json`; regeneration in the independent fresh clone is byte-identical.
The tiny valid WAV/PDF inputs are committed in `reference-tools/attachments/fixtures/`.

| Input | Identified MIME | Rails analyzer | HTTP | AnalyzeJobs | Metadata immediately after request |
| --- | --- | --- | --- | --- | --- |
| text | text/plain | NullAnalyzer | 302 | 0 | identified, analyzed |
| image | image/png | ImageAnalyzer::Vips | 302 | 1 | identified |
| video | video/quicktime | VideoAnalyzer | 302 | 1 | identified |
| audio | audio/x-wav | AudioAnalyzer | 302 | 1 | identified |
| PDF | application/pdf | NullAnalyzer | 302 | 0 | identified, analyzed |

The same generator injects a failing null metadata-update trigger. Rails returns 500 with the
new parent name and attachment committed, only identified metadata, and zero AnalyzeJobs.
Rust matches those vector fields.

`ATTACHMENT_ANALYZER_READBACK_DIR` exports per-type Rust snapshots during the differential test.
Pinned Rails `verify_analyzers.rb` validates the rows, signed lookup, bytes, MIME, analyzer,
metadata, job count and attachment foreign keys:

```text
Rails text readback: valid account/blob/attachment; bytes, MIME, analyzer, metadata and 0 analysis jobs match
Rails image readback: valid account/blob/attachment; bytes, MIME, analyzer, metadata and 1 analysis jobs match
Rails video readback: valid account/blob/attachment; bytes, MIME, analyzer, metadata and 1 analysis jobs match
Rails audio readback: valid account/blob/attachment; bytes, MIME, analyzer, metadata and 1 analysis jobs match
Rails pdf readback: valid account/blob/attachment; bytes, MIME, analyzer, metadata and 0 analysis jobs match
```

Commands for generation and each readback (from `rust/`, with `PARITY_NAMESPACE=attach`,
`PARITY_OWNER=attach`, `PARITY_IMAGE=attach-reference:d7c7de92`):

```sh
parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze reference-tools/attachments/generate_analyzers.rb
parity/bin/reference runner --storage ../.scratch/null-analyzer-readback/text --time 2026-03-02T16:00:00Z --freeze reference-tools/attachments/verify_analyzers.rb text
# The readback command was also run for image, video, audio and pdf, with each matching directory.
```

## Fresh-clone verification after main

Independent clone under `.scratch/fresh-null-analyzer`, created with `git clone --no-local`,
checked out at the verified merge commit. Both seeds were built from the pinned Rails image:
`parity/bin/seed build default first_run`. Rails seed verification passed 29 default checks and
4 first-run checks, with zero failures. Locked metadata passed in the worktree, clone and CI image.
Strict TOML parsing passed all 13 workspace manifests: 75 unique workspace dependency keys,
zero duplicates. No lockfile repair was necessary after the merge.

The first host run passed every target except the new main storage guard, which correctly rejects
host libvips 8.18.6 / ffmpeg 9.0.2 when `CI=true`. This was resolved by rebuilding and rerunning the
**entire** workspace in the pinned CI toolchain image, rather than skipping or disabling the gate.
Its libvips 8.16.1 / ffmpeg 7.1.5 execute all storage byte/row/metadata comparisons.

CI image: `attach-toolchain-ci:1.98.1`, image ID
`sha256:80bed826ce3b998e8ba75b85b25d18055066760982413e4483dd9779c5f053b2`.
The container wrapper retains the host rustc slot/lock protocol and always joins that shared pool;
its lock directory and slot configuration are mounted. No global cargo configuration was changed,
no compiler-throttle bypass was used. Build jobs were 2 and test threads 8 throughout this follow-up.

Exact container invocation used for the checks, expressed as the scratch helper:

```sh
#!/usr/bin/env bash
set -euo pipefail
attach_root=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-attach
attach_container=$1
shift
exec docker run --rm --name "$attach_container" --user 1000:1000 \
  --volume "$attach_root/.scratch/fresh-null-analyzer:/src" \
  --volume /home/riels/.cargo:/cargo \
  --volume /home/riels/.cache/rust-port:/home/riels/.cache/rust-port:ro \
  --volume "$attach_root/.scratch/container-rustc-throttle.sh:/home/riels/.cache/rust-port/rustc-throttle.sh:ro" \
  --volume /tmp/rust-port-rustc-slots:/tmp/rust-port-rustc-slots \
  --workdir /src/rust --env TMPDIR=/src/.scratch/tmp --env CI=true \
  --env CARGO_HOME=/cargo --env CARGO_BUILD_JOBS=2 \
  --env CARGO_PROFILE_DEV_DEBUG=0 --env CARGO_PROFILE_TEST_DEBUG=0 \
  --env CARGO_INCREMENTAL=0 --env RUSTFLAGS=-Clink-arg=-fuse-ld=mold \
  attach-toolchain-ci:1.98.1 "$@"
```

Check arguments:

```sh
cargo test --locked --workspace --exclude html5ever --no-fail-fast -- --test-threads=8
cargo clippy --locked --workspace --exclude html5ever --all-targets -- -D warnings
cargo build --locked --workspace --bins
cargo metadata --locked --format-version 1
```

Full fresh-clone workspace and doctests: **1771 passed, 0 failed, 12 ignored**.
The existing ignores are reported, with no extra ignore added by this branch. All requested
app/db/assets/views/storage targets passed. Main's scoped asset golden helper already covers the
affected goldens; no golden, normalization or allowlist change was needed.

Raw workspace summaries, in execution order:

```text
test result: ok. 681 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 75.60s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.47s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.73s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.02s
test result: ok. 445 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 42.92s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.43s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.04s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.41s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.29s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 21.16s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.49s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.17s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.87s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.69s
test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.40s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Clippy and normal binary build, respectively:

```text
    Finished `dev` profile [unoptimized] target(s) in 55.99s
    Finished `dev` profile [unoptimized] target(s) in 48.40s
```

The 2.8 GB fresh-clone target was deleted after the checks. Old fresh/baseline target directories
were also removed/confirmed absent; zero extra target directories and zero task test containers
remain. The report is the only change after the verified source commit. No PR opened.

---

# Original implementation and initial verification (historical)

# Durable attachment analysis and signed blob assignment

Verified implementation commit: `116f0d27428097704a76fa5c6e849ad9b70d6b6d`.
Base and independently fetched `origin/main`: `2e20b24c`.
Rails oracle: `d7c7de9264c63015be398001d7a1094e7695a6db`, verified from the image's `GIT_REVISION`.
Branch: `rust/durable-attachment-analysis`. No Rails changes, migrations, parity masks, timing-threshold changes, or reduced test concurrency.

## Changes and design

- `crates/campfire/src/jobs.rs`: registers `AnalyzeJob`, implements `db::Job` with `CLASS = "ActiveStorage::AnalyzeJob"`, and dispatches it through the durable runner. Arguments are `{blob_id}`. Integrity failures retain their storage error type and use Rails' ten-attempt polynomial retry policy. Deleted blobs finish without side effects. The ad hoc sender/API is now test-only because analysis was its last production caller.
- `crates/campfire/src/active_storage.rs`: shares analysis between the durable handler and synchronous message processing. Checks `analyzed` before media work and again inside the writer. A repeated delivery does not read the file, update metadata, or touch records. Merges extracted metadata into the latest row so concurrent metadata changes are retained. Blob saves touch users, accounts, workspace icons and messages; messages also touch their rooms. Media work stays off the writer; typed errors survive the bounded media pool.
- `crates/campfire/src/controllers/presenters/attachments.rs`: enqueues `Event::job(&AnalyzeJob { blob_id })` while inserting/replacing the attachment in its writer transaction. Removes the old post-commit `Pending`/`analyze_later` interface. Handles signed strings with the app's **ActiveStorage** verifier, purpose **blob_id**, and current app clock; attaches the existing row. Identifies direct-upload blobs before assignment and saves identification in the same transaction. Reassigning the same blob reuses its attachment and emits no purge job.
- `controllers/accounts.rs`, `accounts/bots.rs`, `users/profiles.rs`, `users.rs`, `first_runs.rs`: remove the old after-write enqueue. All existing writes already had reachable writer transactions; no separate transaction was necessary.
- `controllers/messages.rs`: converts the direct ad hoc enqueue in updates, supports signed existing blobs in create/update, and persists analysis with attachment writes. Message creation still processes its representation synchronously; the durable analysis job makes the committed attachment recoverable if the request/process stops first. Both analysis paths use the same idempotent routine.
- `crates/db/src/models/message.rs`: preserves the attachment when its blob is assigned again, matching `Attached::Changes::CreateOne#find_attachment`.
- `crates/storage/src/blob.rs`, `storage.rs`: add `is_identified` and Rails' `identify_without_saving` file step (at most 4 KB), without writing outside the assignment transaction.
- `controllers/presenters/attachments/tests.rs`: eight real database/request regressions, with authenticated admin sessions and real CSRF, rejecting SQLite triggers, stop/restart, duplicate job delivery, signed-ID cases and rollback-compatible row export.
- `controllers/presenters/test_support.rs`: adds a named-seed boot helper for the first-run regression.
- `reference-tools/attachments/generate.rb`, `vectors/attachment_assignments.json`: generated signed-ID and callback/request vectors from our pinned Rails, with `create_before_direct_upload!`, actual uploaded PNG bytes, and forgery protection enabled. Regeneration from the fresh clone is byte-identical to the committed JSON.
- `reference-tools/attachments/verify_rust.rb`: pinned Rails validates and downloads Rust-written records, attachments and storage bytes.

## Failing-first evidence

Before changing production code, on `2e20b24c`, ran:

```sh
CI=true CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 \
  mise exec rust@1.98.1 -- cargo test --locked -j 4 -p campfire attachments::tests --no-fail-fast
```

Raw result:

```text
test result: FAILED. 0 passed; 5 failed; 0 ignored; 0 measured; 401 filtered out; finished in 0.47s
```

Observed failures:

| Regression | Base behavior | Correct assertion |
| --- | --- | --- |
| `durable_analysis_logo_upload_is_atomic` | 302 despite rejecting durable AnalyzeJob inserts | 500; account name/blob count unchanged |
| `durable_analysis_message_update_is_atomic` | 302 despite rejecting durable AnalyzeJob inserts | 500; body/timestamp/attachment rolled back |
| `durable_analysis_retry_does_not_reanalyze_or_touch` | second analysis hits the rejecting metadata-update trigger (`reanalyzed`) | no-op success |
| `signed_blob_logo_assignment_matches_rails` | valid signed PNG returns 500 | pinned Rails 302, existing blob reused |
| `signed_blob_workspace_assignment_uses_the_existing_blob` | shared assignment returns `Could not find or build blob: expected attachable` | attaches the existing PNG row |

The same core assertions are retained. Subsequent tests exercise first run, signup, profile, bot create/update, shutdown/restart with a persisted ready job, repeated delivery through the real runner, missing blobs, and same-blob reassignment. No queue or analyzer mock is used.

## Rails differentials and rollback compatibility

Generated at `2026-03-02T16:00:00Z` from the pinned Rails image:

| Signed blob case | Rails icon POST | Rails logo PATCH | Rust logo PATCH |
| --- | --- | --- | --- |
| valid | 302 | 302 | 302 |
| before expiry | 302 | 302 | 302 |
| exactly at expiry | 500 | 500 | 500 |
| expired | 500 | 500 | 500 |
| wrong purpose | 500 | 500 | 500 |
| tampered digest | 500 | 500 | 500 |
| tampered payload | 500 | 500 | 500 |
| valid signature, missing blob | 404 | 404 | 404 |

Rails schedules one AnalyzeJob for a newly attached, unanalyzed image; an analyzed same-blob reassignment schedules zero and preserves its attachment ID. The Rust restart test compares the durable job count and final 64×64 analyzed metadata with those Rails outputs. Duplicate deliveries and a deleted-blob delivery drain without updates.

**Scope discrepancy in the supplied brief:** `2e20b24c` has no ported Rust workspace-icon controller. The declared route falls through to the unported endpoint. Thus an icon HTTP 422 cannot be reproduced on this base. This was raised during work. No unrelated icon controller was implemented: the regression uses the real shared assignment transaction for WorkspaceIcon, and pinned Rails reads and validates that Rust-written icon. Full HTTP rollback and signed-ID parity are proven for the callers present on this main. The future icon controller must use this transaction seam; its HTTP port/WorkspaceIcon validation and cache callbacks remain with the rooms/accounts workstream.

Fresh-clone Rust export followed by pinned Rails readback:

```text
Rails readback: 4 valid records; 4 valid attachments; shared signed blob; PNG bytes and analyzed metadata match; attachment foreign keys clean
```

Rails verifies Account, User, Message and WorkspaceIcon validations, every attachment's validation and signed ID, shared blob identity, downloaded PNG bytes, exact analyzed metadata, and foreign keys in the attachment/icon tables. The unmodified seed and exported snapshot both have the same unrelated orphaned fixture message (`messages` 935962042); no new foreign-key violation was introduced. Readback checks do not claim that pre-existing fixture is valid.

Reproduction commands (from `rust/`; namespace/image are test-only):

```sh
export PARITY_NAMESPACE=attach PARITY_OWNER=attach PARITY_IMAGE=attach-reference:d7c7de92
parity/bin/seed build default first_run
parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze \
  reference-tools/attachments/generate.rb > vectors/attachment_assignments.json
# Choose a new, absent output directory for VACUUM INTO:
ATTACHMENT_READBACK_DIR=/absolute/worktree/.scratch/readback CI=true \
  mise exec rust@1.98.1 -- cargo test --locked -j 4 -p campfire \
  signed_blob_reassignment_preserves_rows_and_rails_can_read_them -- --nocapture
parity/bin/reference runner --storage /absolute/worktree/.scratch/readback \
  --time 2026-03-02T16:00:00Z --freeze reference-tools/attachments/verify_rust.rb
```

## Fresh-clone verification

Cloned committed branch with `git clone --no-local --single-branch --branch rust/durable-attachment-analysis . .scratch/fresh` (no shared target). Rebuilt both seeds in that clone from the pinned reference image, then ran the pinned Rails seed validator: **29 passed / 0 failed** for default, **4 passed / 0 failed** for first_run. `CI=true` makes missing seeds fail; none of the app tests silently skipped. Scratch/temp directories are created before use, not required untracked fixtures.

From the fresh clone's `rust/`, using `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_INCREMENTAL=0`, and a disk-backed `TMPDIR` under the assigned `.scratch/`:

```sh
CI=true mise exec rust@1.98.1 -- cargo test --locked -j 4 --workspace --exclude html5ever --no-fail-fast
CARGO_BUILD_JOBS=4 mise exec rust@1.98.1 -- cargo clippy --locked --workspace --exclude html5ever --all-targets -- -D warnings
mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 >/dev/null
mise exec rust@1.98.1 -- cargo build --locked -j 4 -p campfire --bins
CI=true mise exec rust@1.98.1 -- cargo test --locked -j 4 -p campfire attachments::tests -- --nocapture
```

`cargo metadata --locked` returned exit code 0; Cargo.lock stayed unchanged. Raw clippy and normal-binary build completion:

```text
Finished `dev` profile [unoptimized] target(s) in 30.87s
Finished `dev` profile [unoptimized] target(s) in 30.44s
```

Raw attachment result (fresh clone, also exported for Rails readback):

```text
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 401 filtered out; finished in 1.84s
```

Fresh full workspace totals across unit, integration and doctest summaries: **1464 passed, 7 failed, 11 explicitly ignored**. The seven failures are the unchanged fingerprint goldens below; no new failure. Host storage tests execute but version-dependent media byte comparisons skip because host libvips is 8.18.6. Therefore also ran storage from the fresh clone in CI's pinned toolchain image (`campfire-toolchain-ci-rust-speedups`, rustc 1.98.1, libvips 8.16.1 and ffmpeg 7.1.5), with a separate `target-canonical`, four build jobs and the default test concurrency. The media byte/dimension comparisons **execute**, with no version skip:

```text
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.13s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Canonical command:

```sh
docker run --rm --name attach-storage-ci --user 1000:1000 \
  -v /absolute/worktree/.scratch/fresh:/src -v /home/riels/.cargo:/cargo \
  -v /absolute/worktree/.scratch/canonical-tmp:/ci-tmp -w /src/rust \
  -e TMPDIR=/ci-tmp -e CI=true -e CARGO_HOME=/cargo \
  -e CARGO_TARGET_DIR=/src/rust/target-canonical -e CARGO_BUILD_JOBS=4 \
  -e CARGO_PROFILE_DEV_DEBUG=0 -e CARGO_PROFILE_TEST_DEBUG=0 -e CARGO_INCREMENTAL=0 \
  -e RUSTFLAGS=-Clink-arg=-fuse-ld=mold campfire-toolchain-ci-rust-speedups \
  cargo test --locked -j 4 -p campfire_storage --no-fail-fast -- --nocapture
```

## Inherited failures and main

An independent fresh checkout of fetched `origin/main` at `2e20b24c`, with the same pinned seeds and command, reports **1456 passed, 7 failed, 11 ignored**. Failure names and assertion output are identical on origin/main, the working branch and the fresh branch clone (only panic process IDs/backtrace hints normalized for comparison):

- `app::full_page_tests::complete_auth_templates_match_fifteen_seeded_rails_pages_without_masks`
- `application_layout_matches_rails_in_every_state`
- `compiled_files_are_byte_identical_to_the_reference_precompile`
- `javascript_importmap_tags_match_the_reference`
- `manifest_matches_the_reference_precompile`
- `public_files_are_served_like_action_dispatch_static`
- `stylesheet_link_tag_all_matches_the_reference`

All identify the #163 asset drift: `controllers/profile_card_controller.js` and `people.css`, propagating into importmaps/stylesheets/full-page goldens. PR #168 was checked before the final report commit and remains OPEN/unmerged; origin/main remains `2e20b24c`. It was therefore not available to merge. No inherited golden was edited or masked.

Raw full-workspace summaries from the fresh branch clone (in Cargo's execution order):

```text
test result: FAILED. 405 passed; 1 failed; 3 ignored; 0 measured; 0 filtered out; finished in 33.27s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: FAILED. 3 passed; 5 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.49s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.59s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.02s
test result: ok. 445 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 40.19s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.31s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.55s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.90s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.30s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.53s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.70s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.50s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.79s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.55s
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: FAILED. 27 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.88s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

## Ad hoc queue audit and remaining boundaries

`rg -n 'perform_later' crates/campfire/src -g '*.rs'` on the base found only two production ad hoc call sites: the shared `attachments::analyze_later` helper and the direct message-update call. All their callers are converted. No other production `App::jobs.perform_later` use needs conversion. Remaining invocations are in `jobs/tests.rs`, `app/tests.rs`, and `channels/tests/hub_test.rs`; durable `JobQueue::perform_later` invocations there already persist rows. Those tests and their best-effort queue semantics remain intact. Other production jobs use the transactional event sink.

No write caller was outside a reachable transaction. Upload/identification file work runs before the write, but blob metadata, attachments, parent writes, audits and durable enqueue commit or roll back together. This changes Rails' Redis atomicity intentionally, per decisions.md. Analysis retry is the requested no-op after success; missing/deleted records also finish safely. No unrelated job, schema or Rails behavior was converted.

Rails tests covered by these targeted differentials: signed `CreateOne` assignment, verifier purpose/expiry/tampering, attachment analysis callback, same-blob reassignment, metadata/touch handling and rollback-compatible storage reads. Full WorkspaceIcon controller/domain validations are not ported here; the existing role/authorization/sudo/CSRF and account/profile/first-run tests still execute. Production deployment is outside this worker's scope. Branch pushed; no PR opened.
