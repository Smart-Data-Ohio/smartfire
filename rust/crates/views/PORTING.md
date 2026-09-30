# Porting our Rails views

This is a port. Copy the output of Smartfire's Rails templates, including whitespace, attribute
order, escaping, and the browser controllers they connect. Do not redesign a screen or silently
keep an upstream Campfire template because it compiles.

## Reference and ownership

Read the matching `app/views/**/*.erb` and `app/helpers/**/*.rb` first. The pin is recorded in
`.claude/delegation/rust-port/decisions.md`. The current pin is `fec615be`, including Rails PR #148
and `PRESENTATION_CACHE_VERSION = 3`. WS6's image is `ws6-reference-fec615be`: the existing reference
runtime with the pin's actual `app`, `config`, `lib`, `db`, `test`, `vendor` and `public` trees.
The generator refuses an older presentation version.

Domain owners port their own templates and queries. WS6 owns the layout, helpers and shared
partials. Templates consume plain view models, never database rows. A controller gathers data
before rendering; no Askama expression should perform a query.

## View models, helpers and escaping

Give the template a struct in the matching module, `#[derive(Template)]`, and a
`#[template(path = "domain/name.html")]`. Templates live at the ERB path minus `.erb`.
Use `use crate::helpers::{self as h, filters};` when using a block helper. For example:

```jinja
{% let form = h::form_with(h::routes::rooms_directs()).class("multi-select-bar__form") %}
{% filter form_with(form) %}
  <button type="submit">Message (0)</button>
{% endfilter %}
```

`h::attrs()` preserves Ruby hash insertion order. Build options in the same order as the Ruby.
`image_tag` moves `src` and the dimensions where Rails puts them; `tag.*` uses `builder_tag`,
while legacy `image_tag`/input helpers use `legacy_tag`. Boolean attributes and `aria-*` values
have different serialization rules. Use the helper instead of inventing equivalent markup.

The ERB escaper handles HTML, SVG and JSON templates. A plain string is escaped;
`h::Html`/`h::raw` is an already safe buffer. Only use `raw` for trusted template output or
HTML sanitized by its owning renderer. A block filter receives already rendered HTML.
The manifest deliberately keeps Rails' HTML escaping inside JSON (`&amp;` in the small-logo
URL); converting every value to a JSON string changes the reference bytes.

Askama borrows array literals when calling helpers. Use `[].as_slice()` and
`["rooms"].as_slice()` when a helper expects a slice. Rails route helper variants with query
or format options are available through `h::routes::NAME.path_with(...)`.

## Layout regions and domain adapters

`layouts::Application::new(ctx, content)` wraps an already rendered body. Fill `head`, `nav`,
`footer`, `sidebar`, `member_panel` and `thread_panel` with exactly the output Rails captured
in those `content_for` blocks. Set `page_title` and `body_class` to the corresponding instance
variables. Alternatively extend `layouts/application.html`, implement `layouts::Page`, and
override its blocks. Implement `Page::has_sidebar` when a page fills that block; the layout
uses it for the drawer toggle. Rails `provide` can be represented by filling the same region
or setting the title/description before rendering; there is no global mutable slot registry.

`layouts::Public` renders without a workspace context and receives the public stylesheet tag.
`layouts::Mailer` receives safe HTML; `layouts::TextMailer` receives plain text. Render a Turbo
frame through `layouts::frame(ctx, page.as_head(), page.as_content())` with those template
blocks exposed in the template derive.

`ViewContext::chrome` and `CurrentUser::preferences` carry domain facts. `ChromeSource` is the
adapter boundary: the app's domain owner supplies `Chrome` and `UserPreferences`. Defaults are
incomplete integration values, not proof that a configured feature is absent:

| Input | Owner supplies |
|---|---|
| `brand_icon_names` | Icons' ordered client registry |
| `google_picker`, `google_drive` | Google config and the current user's Drive authorization |
| `huddle_configured`, voice settings | Huddle configuration (WS13) and persisted user settings |
| `global_search_query`, `recent_searches` | Searches controller's squished query and ten ordered searches |
| `notification_sounds` | Presence/calendar policy decisions and quiet-window epochs |
| `time_zone` | `Zone::for_user(users.time_zone)` after authentication, per request |

`IconSource` resolves a name to `AvatarIcon`; `shared::IconField` accepts that resolved value
and the stored editable name separately. Unknown icons render no preview. `avatar_tag_with_icon`
accepts an icon only when the owner has established that the user is a bot without an uploaded
avatar. Uploaded images win. `avatar_tag` adds the shared profile-card trigger.

Time helpers take a `Zone` and a `jiff::Timestamp`. `time_ago_in_words` also takes the request's
clock explicitly. Do not substitute UTC for a recognized saved Rails name or IANA identifier.
The generated zone allowlist keeps TZInfo's case-sensitive acceptance; Jiff's lookup alone
would accept inputs Rails rejects. `to_fs(:db)` uses UTC, matching Rails.

## Whitespace and byte checks

Erubi removes lines containing only Ruby statements, including their indentation and newline.
Askama does not do that automatically. `reference-tools/views/erubi_trim.py` moves statement
tags to the following line while leaving expression/block-render indentation in place. Review
its result: an expression that renders empty can still leave a whitespace-only line.
Askama removes the final literal newline; use an explicit `{{ "\n" }}` when Rails emits it.
Do not normalize, trim, parse/reserialize, or widen masks to make an HTML comparison pass.

Add a state to `reference-tools/views/core/goldens.rb` (or your domain's equivalent). It renders
in our reference container with real fixtures, fixed clock and test-only signing secrets. Fixed
CSRF placeholders (`GLOBAL`, `method:action`) and nonce (`NONCE`) are lent identically to both
renderers. Signed stream names and avatar URLs are generated using our real SHA1-derived test
keys, not copied from upstream's goldens. The cache-key generator does not patch Rails helpers.

Run from `rust/`, with the worker's own target and named image:

```sh
bash reference-tools/views/core/build_reference.sh
PARITY_IMAGE=ws6-reference-fec615be PARITY_OWNER=ws6 STORE=/home/riels/.cache/rust-port/ws6/core-reference reference-tools/views/core/run.sh
TMPDIR=/home/riels/.cache/rust-port/ws6/tmp WS6_VIEW_DIFF_DIR=/home/riels/.cache/rust-port/ws6/view-diffs mise exec rust@1.98.1 -- cargo test -j 4 -p campfire_views --test core
```

`tests/core.rs` compares complete strings and reports the first differing byte. The optional
diff directory receives actual/expected files for `diff -u`. Its injection test proves that a
title change, extra space and asset URL change are rejected. Show a new test failing before
fixing the port, or run it against an intentionally broken implementation.

Generated helper tables come from the Rails vectors via `generate_tables.py`; format the two
generated Rust files with the pinned rustfmt after regenerating. `config/locales/en.yml` adds
no app translations; `TranslationsHelper` and Rails' English date-helper defaults are tested.

## Worked examples

1. **PWA manifest:** `pwa::Manifest` carries the account name, both logo paths, base URL and
   asset resolver. `templates/pwa/manifest.json` mirrors the ERB interpolation and text.
   `pwa_endpoints_match_rails` compares the endpoint body with Rails' actual HTTP response;
   it also compares the service worker's complete source bytes. These are the two smallest
   endpoint examples, with no database types in the templates.
2. **Multi-select bar:** `shared::MultiSelectBar { exit_button }` ports the actual shared form.
   `multi_select_bar_matches_rails_with_both_controls` checks both the Clear and exit variants,
   including the request's per-form token and Erubi statement trimming.
3. **Icon field:** `shared::IconField` takes the parent's builder, scope and a resolved preview.
   `shared_icon_field_matches_rails_for_every_kind` renders it inside a persisted room form,
   checking no icon, unknown name, brand image and emoji against Rails.

## Shared fragment forms and keys

Ordinary request forms retain tokens. The five PR #148 message-tree forms explicitly omit them:
reaction chip, legacy boost delete, poll vote/retract and PR Discuss. Use
`form.authenticity_token(false)` or `attrs().attr("authenticity_token", false)` for `button_to`.
The page's CSRF meta token supplies Turbo's header. Never cache current-user state, a CSRF
token or a CSP nonce. The layout's quick-reaction forms are request forms outside the cache and
retain their tokens. `messages/_actions.html` is rendered once in the application layout, never
inside a message. The two-viewer message and legacy-boost tests exercise the actual fragment cache,
including cache hits, pre-read fragments, nested boosts and request-free broadcasts. No message
fragment may contain token fields, token slots or session nonces. WS4's single token-slot mechanism
remains defence in depth for other cacheable forms; do not add a second token mechanism.

`fragment_cache::keys::MessageKey` ports `message_with_pr_cards_cache_key`: record version,
newest cards, optional embed references/PR thread stamp, pins, thread count, poll, note/stream
flags, agent steps, quote timestamps/names and version 3. `sidebar_membership` includes the
ordered huddle participant IDs and administrator flag. Keys are different in UTC and Hawaii
where Rails expands a `Time` through `to_a`. Quote-name digests use Ruby `Array#inspect`, not
JSON. The owning message/sidebar presenter must gather these inputs and wire these keys into
its cache reads and writes. The inherited message cache entry point still needs that wiring.
The process-local template digest is not ActionView::Digestor's shared Redis digest.

## Pixel parity with WS19

Register the real domain state and covered templates in `parity/screens.yml`; follow
`parity/SCREENS.md` for fixtures, interaction steps and the engine/viewport/theme matrix.
Have WS19 provide matching seeded reference/candidate servers and its worker-safe capture
launcher. For WS6 use ports 46000–46099, worker-prefixed Docker names and cache/worktree scratch.
The comparison invocation for existing servers is:

```sh
parity/bin/compare --expected http://127.0.0.1:46000 --actual http://127.0.0.1:46001 \
  --only 'your/domain/state' --engines chromium --viewports desktop,phone \
  --schemes light,dark --no-allowlist --out target/ws6-layout-parity
```

Do not use the checkout's inherited generic Docker launcher without fixing its worker namespace
or using WS19's updated launcher. Byte goldens do not prove pixel parity. WS6 has not run the
screen matrix: this checkout has no parity seeds, its inventory is inherited upstream, and
the optional chrome providers/domain pages remain incomplete. No masks or allowlists changed.
