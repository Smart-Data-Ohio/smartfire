# PR #206 review regressions

These producers run the pinned Rails app, rather than deriving expected outcomes from Rust.
`settings.rb`, `casts.rb` and `dispatch-review.rb` adapt the reviewer's reproductions;
`deletion-races.rb` destroys a board with the actual `Room::DestroyJob` at the dispatcher
boundary and verifies that a healthy board still finishes. All producers check the existing
settings or dispatch source hashes before executing.

Run from the repository root with a validated `default` parity seed:

```bash
export PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2
export PARITY_NAMESPACE=pr206review PARITY_OWNER=pr206review PARITY_CPUS=1
rust/parity/bin/reference exec --seed default -- bin/rails runner --skip-executor /work/reference-tools/board_automations/review/settings.rb > rust/vectors/board_automation_review_settings.json
rust/parity/bin/reference exec --seed default -- bin/rails runner --skip-executor /work/reference-tools/board_automations/review/casts.rb > rust/vectors/board_automation_review_casts.json
rust/parity/bin/reference exec --seed default -- bin/rails runner --skip-executor /work/reference-tools/board_automations/review/dispatch-review.rb > rust/vectors/board_automation_review_dispatch.json
rust/parity/bin/reference exec --seed default -- bin/rails runner --skip-executor /work/reference-tools/board_automations/review/deletion-races.rb > rust/vectors/board_automation_review_deletion_races.json
```

The corpus contains 45 complete HTTP responses with persisted rules, assignments and audits,
48 model cast/validation cases, 19 full dispatch runs, and two actual Rails deletion races.
For malformed SLA arrays that raise in Rails, the HTTP regression explicitly expects Rust's
approved 400 response and the same unchanged persisted facts. Accepted arrays still match
Rails exactly. The model corpus includes all 48 cases from the review, including the four
decimal-prefix cast differences.

`review_pr206_dispatch_with_production_richtext_jobs_and_bind_limit` uses the full `TestApp`
dispatcher, production reference callbacks and broadcast sink. Its SQL probe covers all readers
and the writer through broadcast rendering, counting SELECT executions before any assertion
queries. The job runner is stopped so queued job facts remain inspectable. It compares complete
claim, inbox, job, note-body, plain-text and quiet-message facts with Rails, and independently
compares every batched Turbo message partial byte-for-byte with the existing shared renderer.
New/repeat SLA and digest read growth is checked at 10 and 100 boards; the 100-board cases run
with SQLite's variable limit set to 64 on every reader and the writer. A further 32-note case
loads 64 distinct quoted messages from permalink-containing titles under the same limit.
Rails also verifies that this case creates all 64 quote references.
The associated GitHub discussion loader is additionally checked with 96 IDs under the
64-variable limit, comparing the batched results with individual loads.

The settings probe measures the complete authenticated GET and rendering path with 10/100
assignment choices. The deletion tests use two database handles and the normal Rust room
destruction operation, covering disappearance before room preload, creator preload and the
fresh SLA claim check. Concurrent sweeps verify exactly-once claims and repeat no-ops.

From `rust/`, run the review tests with the pinned toolchain and seeds:

```bash
CI=1 mise exec rust@1.98.1 -- cargo test --locked -p campfire -p campfire_db review_pr206 -- --test-threads=4 --nocapture
```

The workstream report records the original failing runs, final full-app counts, workspace,
strict-clippy and release-input verification receipts. Reviewer scratch inputs are read-only;
these checked-in copies contain no paths into another worker's checkout.
