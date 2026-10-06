# Asset golden drift report

## Result and scope

The 15 complete auth pages and six application-layout states now compare asset
references by logical path, with **every identified live pipeline URL field checked
against the current Rust asset pipeline manifest**. Explicitly frozen fixture
fields remain byte-exact. All other bytes remain exact. The same
shared helper covers the other live-asset app, runtime Markdown and core views
comparisons. Page goldens were not regenerated or hand-edited.

The Rust asset crate's strict Rails exports were regenerated from main
`2e20b24c3f2be9db8a646a1352c159b4afacad0e` using the reference tooling. These
continue to check Propshaft digests, compiled bytes, tags/importmap, Link headers
and static responses without applying the page comparison helper.

The push and pull-request filters now cover every Rails input found to be read
by the port's build, runtime or normal test suites. Permissions, concurrency,
branch filters, all jobs and gates remain byte-identical to the base.

## Reproduction and inventory

The initial worktree and fresh base both reproduced the same auth-page failure.
See [asset-golden-drift-reproduction.md](asset-golden-drift-reproduction.md) for
commands, raw baseline summaries and the 15 page case names.

Failing tests at `2e20b24c`:

- app: `app::full_page_tests::complete_auth_templates_match_fifteen_seeded_rails_pages_without_masks`.
- views/core: `application_layout_matches_rails_in_every_state`.
- assets/reference: `manifest_matches_the_reference_precompile`,
  `compiled_files_are_byte_identical_to_the_reference_precompile`,
  `stylesheet_link_tag_all_matches_the_reference`,
  `javascript_importmap_tags_match_the_reference`,
  `public_files_are_served_like_action_dispatch_static`.

All 28 fingerprint-bearing golden/vector files (the search includes bare
fingerprinted paths in manifests and compiled-hash exports as well as URLs):

`-[0-9a-f]{8}\.[A-Za-z]` (also covers importmap URLs):

- `rust/crates/assets/tests/reference/compiled_sha256.json`
- `rust/crates/assets/tests/reference/javascript_importmap_tags.html`
- `rust/crates/assets/tests/reference/link_header.txt`
- `rust/crates/assets/tests/reference/manifest.json`
- `rust/crates/assets/tests/reference/static_responses.json`
- `rust/crates/assets/tests/reference/stylesheet_link_tag_all.html`
- `rust/crates/campfire/src/ws8_runtime_vectors.json`
- `rust/crates/richtext/tests/markdown/expected.json`
- `rust/crates/views/tests/golden/core/helpers.json`
- `rust/crates/views/tests/golden/core/images.json`
- `rust/crates/views/tests/golden/core/layouts/application_admin_full.html`
- `rust/crates/views/tests/golden/core/layouts/application_bot.html`
- `rust/crates/views/tests/golden/core/layouts/application_everything_on.html`
- `rust/crates/views/tests/golden/core/layouts/application_member.html`
- `rust/crates/views/tests/golden/core/layouts/application_searches_query.html`
- `rust/crates/views/tests/golden/core/layouts/application_signed_out.html`
- `rust/crates/views/tests/golden/core/layouts/public.html`
- `rust/crates/views/tests/golden/core/layouts/public_described.html`
- `rust/crates/views/tests/golden/core/pages/public_pages_about.html`
- `rust/crates/views/tests/golden/core/pages/public_pages_privacy.html`
- `rust/crates/views/tests/golden/core/pages/public_pages_terms.html`
- `rust/crates/views/tests/golden/core/pages/pwa_manifest.json`
- `rust/crates/views/tests/golden/core/partials/first_paint_controller_preloads.html`
- `rust/crates/views/tests/golden/core/partials/shared_icon_field_brand.html`
- `rust/vectors/auth_full_pages.json`
- `rust/vectors/kit_security.json`
- `rust/vectors/sudo_views.json`
- `rust/vectors/two_factor_views.json`

The app auth/sudo/two-factor vectors, runtime Markdown brand-icon vectors,
core views goldens and all six asset reference exports compare against **live
Rails asset inputs**. A pinned database seed does not freeze those inputs:
`campfire_assets/build.rs` reads the current reference files at compile time,
and the app serves the resulting embedded files. These comparisons were
sensitive to Rails asset changes. Core helper/message fixtures also contain
some serialized input URLs; those are fixture data rather than new pipeline
lookups, while the same renders use live assets for their surrounding UI.

`richtext/tests/markdown/expected.json` supplies its own frozen icon catalog to
`markdown_corpus.rs`; it does not resolve paths through the live asset pipeline.
It stays byte-exact and unchanged. No separate frozen Rails source copy is used
by the app/core views tests. `kit_security.json` captures asset URLs and Link
headers, but the security-header replay asserts selected security/cache headers
rather than the captured page/Link bytes. Its asset/base.css request URL can
drift independently; the captured people.css Link header is not asserted.

## Changes by file

| Files | Change |
| --- | --- |
| `test-support/asset_goldens.rs` | Shared test-only comparison plus five discrimination tests. Validate actual URLs against `campfire_assets::manifest()`, substitute logical paths on both sides, then compare all remaining bytes. |
| `crates/campfire/src/app.rs` | Include the shared test helper; no production dependency or code change. |
| `crates/campfire/src/app/{full_page_tests,sudo_tests,two_factor_tests,session_management_tests}.rs` | Route live-asset page/content golden checks through the helper. |
| `crates/campfire/src/rich_text.rs` | Use the helper for live brand-asset Markdown/canonical HTML and stored/scheduled/forwarded HTML comparisons; remaining domain fields still compare exactly. |
| `crates/views/tests/core.rs` | Use the same helper for core comparisons, avatar/preload goldens, and a mutation test on a genuinely rendered application layout. |
| `crates/views/tests/review/mod.rs` | Route reaction-body golden checks through the shared core comparator, alongside the other existing helper/message comparisons. |
| `crates/assets/tests/reference/{manifest.json,compiled_sha256.json,stylesheet_link_tag_all.html,javascript_importmap_tags.html,static_responses.json}` | Rails-generated replacements for the changed people.css and profile-card-controller.js bytes/digests. `link_header.txt` was re-exported but remained identical. |
| `crates/assets/vendor/MANIFEST.md` | Generated provenance now identifies the exact current-main source SHA; vendored gem bytes/load path are unchanged. |
| `.github/workflows/rust.yml` | Add identical push/PR path filters, each documented at its reader boundary. |
| `AGENTS.md` | Update the description of when Rust CI runs. |
| `plans/asset-golden-drift-reproduction.md` | Step 1 reproduction and inventory commit. |
| `plans/asset-golden-drift-report.md` | This final report. |

Paths are relative to `rust/` except `.github/workflows/rust.yml`.

The comparison changes only fingerprint bytes. It never parses/reserializes the
HTML or importmap JSON, sorts tags, removes tags, masks whitespace, removes
nonces/tokens, or accepts a wrong actual digest even if the golden contains that
same wrong URL. Logical paths, tag order/count/attributes, importmap keys,
ordering, preload declarations and all non-asset content remain exact. The
asset manifest is computed by the same Propshaft-compatible build pipeline that
embeds the current files, including referenced-file bytes and asset version.
Already-digested logical vendor names stay exact, and a bare `/assets/` prefix
in service-worker code/comments is preserved as ordinary text.

## Strict Rails export regeneration

```sh
# The image was built from the clean 2e20b24c worktree before the reproduction commit.
PARITY_NAMESPACE=assetdrift-current \
  PARITY_REFERENCE_APP_IMAGE=assetdrift-current:app \
  PARITY_IMAGE=assetdrift-current-reference PARITY_OWNER=assetdrift \
  PARITY_RUNTIME=docker rust/parity/bin/reference build
# Checked image GIT_REVISION == 2e20b24c3f2be9db8a646a1352c159b4afacad0e.
REFERENCE_IMAGE=assetdrift-current-reference ASSET_EXPORT_PREFIX=assetdrift \
  REFERENCE_SHA=2e20b24c3f2be9db8a646a1352c159b4afacad0e \
  rust/crates/assets/script/revendor
```

```text
Exported 20 vendored files and 463 compiled assets to /work/assets
```

Rails computed `people-8adb2aea.css` and
`controllers/profile_card_controller-ca4bd34b.js`. The strict asset tests
passed with those exports. On later reference bumps the same export command
must still refresh strict asset vectors: page-digest tolerance does not replace
same-byte Propshaft verification.

## Mutation evidence

The new shared tests reject dropped, reordered and duplicated stylesheets,
wrong/missing CSS digests (including when actual and golden agree on the bad
URL), wrong JS importmap digests, changed importmap keys/type, and non-asset
attribute/whitespace changes. A frozen golden fingerprint with the correct
current actual URL is accepted.

The additional views test renders an entire application layout with live
stylesheet/importmap helpers, then deliberately changes that rendered page:

```sh
# From rust/, with CI=true and TMPDIR pointing at worktree .scratch/:
mise exec rust@1.98.1 -- cargo test --locked -j 4 -p campfire_views \
  --test core live_application_layout_rejects -- --nocapture
```

```text
drop stylesheet: page differs at byte 1511 after validating asset digests (48206 vs 48283 bytes)
reorder stylesheets: page differs at byte 1511 after validating asset digests (48283 vs 48283 bytes)
wrong digest: wrong asset digest: /assets/people-00000000.css; pipeline requires /assets/people-8adb2aea.css
test live_application_layout_rejects_dropped_reordered_stylesheets_and_wrong_digests ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 33 filtered out; finished in 0.01s
```

These rejection assertions also run in the fresh-clone views suite.

## CI filters and lint

Added `app/assets/**`, `app/javascript/**`, `config/importmap.rb`,
`test/fixtures/**`, `db/schema.rb`, plus these directly observed readers:

- `vendor/javascript/**`: Propshaft load path and importmap application modules.
- `public/**`: embedded static files.
- `config/initializers/assets.rb`: assets version in digest inputs.
- `config/icons.yml`: live comparison with the vendored brand catalog.
- `db/migrate/**`: schema migration-version checks against filenames.
- `app/models/sound.rb`: built-in sound-catalog verification.

The file contains a one-line reader comment for every added path. The entire
workflow suffix from `permissions: {}` onward was compared byte-for-byte with
the base; branch filters are unchanged and push/PR path lists are identical.

```sh
actionlint
# zizmor ran with online audits enabled using the locally authenticated GitHub client.
uvx zizmor --no-progress .github/workflows
```

```text
actionlint exit: 0
zizmor all workflows with online audits exit: 0
No findings to report. Good job! (2 ignored, 19 suppressed)
```

## Fresh-clone verification

Source verified: `1df846e0403f84158f11c2ef7c54a8010fb504e1` (commits 1-3).
The final report commit changes only documentation. Clone:
`.scratch/fresh-clone`, made with `git clone --no-hardlinks --single-branch
--branch rust/asset-golden-drift . .scratch/fresh-clone`. It has its own Cargo
`rust/target`, and no pre-existing vectors, source fixtures or seeds were copied
from untracked worker scratch.

The pinned Rails archive was created afresh under the clone's `.scratch/`:

```sh
mkdir -p .scratch/fresh-clone/.scratch/reference-pin
git -C .scratch/fresh-clone archive "$(cat rust/parity/reference.sha)" \
  | tar -x -C .scratch/fresh-clone/.scratch/reference-pin
CAMPFIRE_REFERENCE="$PWD/.scratch/fresh-clone/.scratch/reference-pin" \
  PARITY_NAMESPACE=assetdrift-fresh PARITY_IMAGE=assetdrift-reference \
  PARITY_OWNER=assetdrift PARITY_RUNTIME=docker \
  .scratch/fresh-clone/rust/parity/bin/seed build default first_run
# The same env was used for each seed's Rails validation:
# reference runner --seed NAME --time 2026-03-02T16:00:00Z --freeze \
#   reference-tools/campfire/verify_parity_seed.rb NAME
```

```text
seed: default -> parity/.seed/default (6.1M)
seed: first_run -> parity/.seed/first_run (1.5M)
  "passed": 29,
  "failed": 0
  "passed": 4,
  "failed": 0
```

Cargo did **not** receive `CAMPFIRE_REFERENCE`, so fixtures/assets/schema were
read from the fresh clone's live Rails root. `CI=true` makes missing-seed paths
fail rather than silently return. Commands from the fresh clone's `rust/`:

```sh
export CI=true CARGO_BUILD_JOBS=4 TMPDIR="$PWD/../.scratch"
mise exec rust@1.98.1 -- cargo test --locked -j 4 \
  -p campfire -p campfire_db -p campfire_views -p campfire_assets --no-fail-fast
mise exec rust@1.98.1 -- cargo clippy --locked --workspace \
  --exclude html5ever --all-targets -- -D warnings
mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1
```

Raw final summaries, in Cargo execution order (app, assets unit/reference,
database, views unit/core, then their doctests):

```text
test result: ok. 403 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 38.30s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.57s
test result: ok. 445 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 55.22s
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
test result: ok. 34 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 5m 19s
cargo metadata --locked exit: 0
```

No test concurrency was lowered and no deadlines/timing thresholds changed.
There were no silent seed-dependent skips. The app's three explicit existing
ignores remain: `manages_bots` (WS11), `push_latency` (measurement), and
`channels::tests::golden::record_reference` (interactive recording). The DB's
four existing explicit ignores require external Rails differential/rollback
artifacts: `scenario_matches_ruby`, `export_database_for_rails`,
`fixtures_match_ruby_row_for_row`, `read_rails_rollback_changes`. They are listed
here rather than represented as executed parity checks.

### First fresh attempt and baseline comparison

The first fresh-clone app run observed one unrelated intermittent runner-test
failure: `jobs::tests::ws8_quote_refresh_jobs_execute_in_the_real_app_runner`.
Its wait predicate finishes when the quote-refresh job disappears, while the
following assertion requires **every** queue row to be gone. Its failed snapshot
contained only an unrelated ready `Retention::PruneJob`. The test and runner
are unchanged by this branch; no production asset path is involved.

```text
test result: FAILED. 402 passed; 1 failed; 3 ignored; 0 measured; 0 filtered out; finished in 51.59s
```

To compare rather than assume inheritance, the same fresh clone was checked
out at unmodified `2e20b24c` and the complete seeded app suite rerun:

```text
test jobs::tests::ws8_quote_refresh_jobs_execute_in_the_real_app_runner ... ok
test result: FAILED. 397 passed; 1 failed; 3 ignored; 0 measured; 0 filtered out; finished in 37.95s
```

The base's only failure was the original 15-page asset drift. The runner race
did not recur in that base run; its classification as a race follows the
unchanged predicate and failed row snapshot, not a deterministic baseline
reproduction. After restoring the branch, the entire app/DB/views/assets run
passed with the same concurrency and deadlines. The first-run log was retained
at `.scratch/fresh-tests.log`, baseline at `.scratch/fresh-base-app.log`, and
final results at `.scratch/fresh-final-tests.log`; clippy and metadata logs are
also retained under `.scratch/`.

## Boundaries and remaining work

No Rails app implementation was changed. No parity allowlist/mask was added or
widened; no fixture secrets, test timing, ignored-test annotations, Cargo
manifest or lockfile were changed. The only cross-workstream code touches are
test comparisons in auth, runtime Markdown and core views; no controller,
domain or job behavior changed. No Rails functional tests were ported or
deferred by this test-infrastructure task.

The unrelated runner-test race remains outside this branch and is reported for
the lead's test-stability work. The seven existing explicit ignored/manual tests
remain unexecuted as described above. For `5f2588a4`, all requested ordinary suites
and locked clippy/metadata checks passed on the final fresh-clone source. The branch is
pushed; no PR or deployment is part of this task.

## Astra review fixes (PR #168)

Both P2 findings are addressed in the test helper and views test callers. Remote
`main` remained `2e20b24c3f2be9db8a646a1352c159b4afacad0e`; no merge was needed.
Implementation source: `bd98dfa58ca6dff6108c8f98ea27848481135089`.

### Local URL fields and exact surrounding bytes

`test-support/asset_goldens.rs` now identifies href/src/content attribute values,
importmap string values, PWA JSON URL values, and Link header entries using source
ranges. Keys, comments (including unfinished comments), raw-text elements,
visible text and unrelated attributes are never asset substitution sites.
Only an entire local `/assets/` path with an eight-hex fingerprint and a logical
path present in the Rust manifest can normalize. Absolute external URLs, nested
non-asset paths and URLs whose query contains an asset path stay exact; query
and fragment suffixes of genuine local asset URLs also remain exact.

Known live asset URLs still require the actual fingerprint to match the current
pipeline. Non-pipeline strings remain ordinary bytes. No HTML, JSON or headers
are reserialized, and field count, tag order/count and surrounding bytes are
still checked. Positive tests cover meta content, quoted/unquoted src values,
PWA JSON src, importmap values and Link entries. Negative tests also cover
comment/raw-text content, JSON keys, unknown paths and unchanged external URLs.

The five review mutations plus the unchanged external-URL control were added
before the helper fix. This reproduced all six unexpected outcomes:

```text
test result: FAILED. 0 passed; 6 failed; 0 ignored; 0 measured; 34 filtered out; finished in 0.00s
```

After the fix, all five digest-only mutations reject and the unchanged external
URL passes. The regression test names are:

- `rejects_digest_changes_in_external_urls`
- `rejects_digest_changes_in_visible_text`
- `rejects_digest_changes_in_query_values`
- `rejects_digest_changes_in_unrelated_attributes`
- `rejects_digest_changes_in_nested_non_asset_paths`
- `leaves_unchanged_external_urls_byte_exact`

These are committed alongside the positive URL-context checks, frozen/live
field checks and existing tag/importmap mutation tests; they run in both app
and views test harnesses.

### Frozen fixture fields and live surrounding UI

`crates/views/tests/core.rs` exposes the field-scoped comparator.
`crates/views/tests/review/mod.rs` supplies the serialized room-icon and reaction
Image URLs from each MessageView as frozen input. Their unique expected output
fields are selected by ordinal, then kept byte-exact on both sides. Ambiguous
fixture provenance fails instead of granting a global URL exclusion. The
surrounding UI assets keep live validation and logical-path comparison. The
shared helper tests cover two uses of the same logical asset with only one field
frozen, so this does not exempt an entire asset or page from checking.

A committed rendered-message test uses a deliberately stale frozen fixture URL
to reproduce the divergence without depending on untracked input. For both
`room_icon` and `reactions`, the unchanged frozen field passes, a changed frozen
field rejects, and a wrong live dropdown-icon digest rejects.

The review's actual SVG mutation was also replayed against a scratch reference
archived from Rails `2e20b24c`, using the supplied
`/home/riels/.cache/rust-port/driftr/mutations/github.svg`. The pipeline's GitHub
fingerprint changes from `07b84d6c` to `21ad1b9d`.

Before fixing the caller:

```text
message-state-room_icon: wrong asset digest: /assets/icons/brands/github-07b84d6c.svg; pipeline requires /assets/icons/brands/github-21ad1b9d.svg
message-state-reactions: wrong asset digest: /assets/icons/brands/github-07b84d6c.svg; pipeline requires /assets/icons/brands/github-21ad1b9d.svg
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 39 filtered out; finished in 0.03s
```

After the fix, the complete core views suite passes with that mutated SVG. This
includes the byte-identical frozen message cases and the live GitHub icon-field
render. The shared live digest test rejects a deliberately incorrect GitHub URL
against the actual changed manifest:

```text
test shared_icon_field_matches_rails_for_every_kind ... ok
test review::message_partial_branches_match_rails ... ok
wrong live digest: wrong asset digest: /assets/icons/brands/github-22222222.svg; pipeline requires /assets/icons/brands/github-21ad1b9d.svg
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.41s
```

Mutation commands from this worktree (only scratch Rails inputs were changed):

```sh
mkdir -p .scratch/review-fixes/reference
git archive 2e20b24c | tar -x -C .scratch/review-fixes/reference
cp /home/riels/.cache/rust-port/driftr/mutations/github.svg \
  .scratch/review-fixes/reference/app/assets/images/icons/brands/github.svg
CI=true CAMPFIRE_REFERENCE="$PWD/.scratch/review-fixes/reference" TMPDIR="$PWD/.scratch" \
  mise exec rust@1.98.1 -- cargo test --manifest-path rust/Cargo.toml --locked -j 4 \
  -p campfire_views --test core -- --nocapture
```

### Fresh-clone results for the fixes

A new clone was created at `.scratch/review-fixes/fresh-clone` with
`git clone --no-hardlinks --single-branch --branch rust/asset-golden-drift . PATH`.
It built its own Cargo target. Both seeds were rebuilt in this clone from the
revision-verified `assetdrift-reference` image at `parity/reference.sha`, and
validated by Rails (`default`: 29 passed/0 failed; `first_run`: 4 passed/0 failed).
Cargo used the clone's live Rails root, without a CAMPFIRE_REFERENCE override.
The final report commit is documentation only.

Commands from the fresh clone's `rust/`:

```sh
export CI=true CARGO_BUILD_JOBS=4 TMPDIR="$PWD/../.scratch"
mise exec rust@1.98.1 -- cargo clippy --locked --workspace --exclude html5ever --all-targets -- -D warnings
mise exec rust@1.98.1 -- cargo test --locked -j 4 -p campfire -p campfire_assets -p campfire_db -p campfire_views --no-fail-fast
export PATH="$PWD/../../bin:$PATH"
mise exec rust@1.98.1 -- cargo nextest run --locked -j 4 --profile ci --no-fail-fast -p campfire -p campfire_assets -p campfire_db -p campfire_views
mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1
```

The runner was CI's pinned `cargo-nextest 0.9.146`, copied read-only from the
existing toolchain image into this worktree's scratch bin directory using an
owned `assetdrift-nextest-copy` container, removed after the copy. It used the
unchanged `ci` profile: default test concurrency, no retries, no altered
deadlines or timing thresholds. The combined app/assets/db/views run passed:

```text
    Starting 950 tests across 6 binaries (7 tests skipped)
     Summary [ 152.473s] 950 tests run: 950 passed, 7 skipped
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 43s
cargo metadata --locked exit: 0
```

The nextest JUnit counts confirm every requested suite ran:

| Suite | Passed | Failed | Explicitly ignored |
| --- | ---: | ---: | ---: |
| app | 412 | 0 | 3 |
| assets unit | 5 | 0 | 0 |
| assets reference | 8 | 0 | 0 |
| db | 445 | 0 | 4 |
| views unit | 36 | 0 | 0 |
| views core | 44 | 0 | 0 |

The preceding libtest run also completed the assets, db, views and all available
doctest targets. Their raw summaries (assets unit/reference, db, views unit/core,
assets/db/views doctests) were:

```text
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.53s
test result: ok. 445 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 65.14s
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.40s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

The first fresh attempt again observed the unchanged quote-refresh test race
already reported above: its target quote job was gone, but the assertion rejected
an unrelated running Retention::PruneJob. The raw app summary was:

```text
test result: FAILED. 411 passed; 1 failed; 3 ignored; 0 measured; 0 filtered out; finished in 41.56s
```

The second fresh attempt also hit the unchanged OpenGraph
`parses_pathological_pages_quickly` one-second bound under heavy host load:
1.1385674 seconds, alongside the quote-refresh race. Its app summary was:

```text
test result: FAILED. 410 passed; 2 failed; 3 ignored; 0 measured; 0 filtered out; finished in 49.16s
```

The host load average observed during that run was 44.07. Its log is retained
at `.scratch/review-fixes/fresh-tests-second.log`. The third, warm run again
observed only the quote-refresh race:

```text
test result: FAILED. 411 passed; 1 failed; 3 ignored; 0 measured; 0 filtered out; finished in 41.11s
```

That log is retained as `.scratch/review-fixes/fresh-tests-final.log`. Each
libtest run used default test concurrency and unchanged timing thresholds.

No job code, predicate, deadline, timing threshold, concurrency setting or ignored
test was changed. The first failed log is retained as
`.scratch/review-fixes/fresh-tests-first.log`; the passing nextest log is
`.scratch/review-fixes/fresh-nextest.log`, and clippy/metadata logs are under
`.scratch/review-fixes/`. All nine additional shared
regressions and the rendered-message regression ran normally. The same seven
explicit manual/ignored tests remain; no seed-dependent test silently skipped.

Scope remained these two P2s plus their tests and this report. Page goldens,
strict Rails exports, vendored assets, Rails app files, workflow filters,
permissions, concurrency, job gates, Cargo manifests and lockfile are unchanged
from `5f2588a4`. No additional asset oracle, merge, PR creation or deployment was
performed.
