# Unicode casing parity

Reference: Rails `d7c7de9264c63015be398001d7a1094e7695a6db`, Ruby 3.4.10
(`2b0b7728dc7f0561c35c3d8c4489945c94b783ad`), Unicode 15.0.0.
`reference-tools/unicode_casing_parity.rb` records the expectations in
`vectors/unicode_casing_parity.json`; its output was also reproduced in the pinned
`ws19b-ci-reference` image. The existing `rails_compat::unicode` tables already match
these Ruby outputs. Their implementation and tables are unchanged.

| Finding | Fix | Regression |
| --- | --- | --- |
| 1. Inbound author, authenticated domains, relay IDs | Ruby downcase for the author bind and domain comparison; full fold for relay IDs | Actual inbound routing selects the member for `ΟΣ@example.com`; DKIM/DMARC/SPF cases cover `É` and final sigma; `ß`/`ss` and `ς`/`σ` relay IDs match, with the topmost header still authoritative. Domain folding is explicitly rejected. |
| 2. Search and filenames | Downcase in the shared LIKE helper and filename bind; existing SQL and escaping retained | `in:ΟΣ`, `from:ΟΣ`, board/work thread names, event descriptions, and `οσ_%\\.txt` match, including escaped LIKE metacharacters. |
| 3. Failed-sign-in audit and labels | Downcase both lookup binds | The second `ΟΣ@example.com` failure is suppressed after `οσ@example.com`; an existing `οσ@local` account makes the malformed email input `ΟΣ@local` a recognized label. |
| 4. Name-sorted lists | Downcase the sort keys | Separate tests cover preloaded direct rooms, account sidebar labels, huddle notices, huddle grant participants, each stage role, the agent directory, and both human/agent message-payload owner lists. Ruby orders `οςa` before `ΟΣ`; listener hand priority is preserved. |
| 5. GitHub handles and reviewers | Downcase saved/manual handles, failed-save previews, verified-login claims, reviewer lookup, and notification keys | `ΟΣ` becomes `οσ`; the reviewer receives an actual inbox item. Recorded cases also ensure ordinary downcase preserves `ß` and `ς`, embedded NUL, and Ruby's Unicode version. |
| 6. GitHub identity guards | Full fold for approval identity and verified claimant comparisons | `ß`/`SS` and `ς`/`σ` match; wrong account IDs and blank identities do not match; composed/decomposed accents remain distinct. Verified claimants keep their identity. |
| 7. DND exception ordering | `ORDER BY LOWER(users.name)` | Actual profile projection orders `A\0a` before `a\0z`; the fixture also preserves SQLite's ordering of `É` and `é`. |

The 23 behavioral regressions fail before the implementation changes. The initial
thread-search fixture required a valid board work status; after that fixture correction,
its pre-fix run fails because the matching section is absent. The helper-oracle check
passes before and after the change. All 24 targeted tests pass after the changes.

The `default` and `first_run` seeds were copied into this worktree and validated again
against the pinned Rails image: 29/29 and 4/4 checks passed. Workspace verification uses
Rust 1.98.1 and the CI toolchain image (`campfire-toolchain-ci-rust-speedups`), including
the pinned libvips 8.16.1 and ffmpeg 7.1.5. The host's rustc throttle was not changed.
New fixture values contain no real credential-scanner prefixes.
All generated scratch targets and the task's cache/fixture directory were removed
after verification; raw logs and reference-test outputs are retained outside the worktree.

Commands (from the repository root, with the CI script's required `RUNNER_TEMP`,
worktree-local `CARGO_HOME` and `CARGO_TARGET_DIR`, and `CI=1`):

```sh
bash rust/ci/cargo.sh nextest run --locked --workspace --profile ci --test-threads 4
bash rust/ci/cargo.sh test --locked --workspace --doc
bash rust/ci/cargo.sh clippy --locked --workspace --all-targets -- -D warnings
cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1
bash rust/ci/with-release-inputs.sh bash ./ci/cargo.sh build --locked --workspace --bins
```

All commands above exited 0. Nextest ran all 3,670 enabled tests; the 10 skips are
existing `#[ignore]` tests. The doctest command has no runnable examples and two
existing ignored database examples. All 24 Unicode tests passed in the pinned
workspace run as well as in the targeted run.

Raw targeted summaries:

```text
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 1795 filtered out; finished in 1.23s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 1147 filtered out; finished in 0.30s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 54 filtered out; finished in 0.11s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 48 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 78 filtered out; finished in 0.00s
```

Raw workspace/gate summaries (terminal color escapes removed):

```text
     Summary [1054.628s] 3670 tests run: 3670 passed, 10 skipped
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 51s
    Finished `test` profile [unoptimized + debuginfo] target(s) in 16.53s
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
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
cargo metadata --locked: OK (13 workspace members)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 04s
```
