# Asset golden drift reproduction

Base: `2e20b24c3f2be9db8a646a1352c159b4afacad0e` (#163). Parity seed pin:
`d7c7de9264c63015be398001d7a1094e7695a6db` (`parity/reference.sha`).
Both `default` and `first_run` seeds were built by Rails in the revision-verified
`assetdrift-reference` image and validated with `verify_parity_seed.rb`.
The app uses the **live worktree** for assets, not the seed's reference pin.

## Baseline commands and results

```sh
mkdir -p .scratch/reference-pin
git archive "$(cat rust/parity/reference.sha)" | tar -x -C .scratch/reference-pin
# Image's GIT_REVISION was checked against reference.sha before tagging.
docker tag ws19b-ci-reference assetdrift-reference
CAMPFIRE_REFERENCE="$PWD/.scratch/reference-pin" PARITY_NAMESPACE=assetdrift \
  PARITY_IMAGE=assetdrift-reference PARITY_OWNER=assetdrift PARITY_RUNTIME=docker \
  rust/parity/bin/seed build default first_run
# From rust/; TMPDIR points to this worktree's .scratch/ throughout.
CI=true mise exec rust@1.98.1 -- cargo test --locked -j 4 -p campfire --no-fail-fast
CI=true mise exec rust@1.98.1 -- cargo test --locked -j 4 -p campfire_assets -p campfire_views --no-fail-fast
```

Raw app summary:

```text
test result: FAILED. 397 passed; 1 failed; 3 ignored; 0 measured; 0 filtered out; finished in 45.69s
```

The only failing app test is
`app::full_page_tests::complete_auth_templates_match_fifteen_seeded_rails_pages_without_masks`.
All 15 cases fail at byte 5270: `sign_in`, `incompatible_browser`, `transfer`,
`setup`, `challenge`, `backups`, `backups_signed_out`, `sessions_one`, `sessions_two`,
`sudo_continue`, `sudo_`, `sudo_password`, `sudo_totp`, `sudo_google`,
`sudo_password_totp_google`. Actual is `people-8adb2aea.css`; expected is
`people-b8926caa.css`. No seed-dependent tests silently skipped. The three
existing explicit ignores are the bot-key WS11 test, a job performance measurement,
and the live-reference cable recording test.

Additional baseline failures:

- views `application_layout_matches_rails_in_every_state` (all six application layouts).
- assets `manifest_matches_the_reference_precompile`.
- assets `compiled_files_are_byte_identical_to_the_reference_precompile`.
- assets `stylesheet_link_tag_all_matches_the_reference`.
- assets `javascript_importmap_tags_match_the_reference`.
- assets `public_files_are_served_like_action_dispatch_static`.

```text
test result: FAILED. 3 passed; 5 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.88s
test result: FAILED. 27 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
```

## Fingerprinted golden/vector inventory

Exhaustive tracked JSON/HTML/text golden/vector files matching
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

All `views/tests/golden/core` entries listed above are consumed by `tests/core.rs`
and `tests/review/mod.rs` with `campfire_assets` supplying live asset paths, style
tags and importmap tags. They break when their referenced Rails bytes change.
`auth_full_pages.json`, `sudo_views.json`, and `two_factor_views.json` likewise
render live assets in app tests. `ws8_runtime_vectors.json`'s Markdown brand icons
are resolved through live `AppRichText::icons`; broadcasts/domain-only parts do
not render assets. The asset crate's six `tests/reference` files compare live
Rust precompile/serving against a frozen Rails **export**, so they are sensitive
to changes and must remain strict for same-byte Propshaft verification.

`kit_security.json` contains captured Link headers and base.css request paths;
its security tests assert the selected security/cache headers, not those captured
Link headers or a full page. A change to the base.css digest can invalidate the
captured request path; the people.css Link capture itself is not asserted.

`richtext/tests/markdown/expected.json` is compared using the serialized icon
catalog in that same frozen vector (`markdown_corpus.rs`), not live Rails assets.
Its paths do not drift when Rails assets change. There is no separate frozen
Rails source tree used by the app or core views tests; freezing the **seed** does
not freeze the assets they render.

`rg -l people-b8926caa rust` additionally identifies the manifest and compiled-hash
exports, the stylesheet/static-response exports, all six application layouts,
`kit_security.json`, and `auth_full_pages.json`. No golden was edited for this
reproduction.
