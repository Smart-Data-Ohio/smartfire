# Porting our Rails views

This is a port. Copy the output of Smartfire's Rails templates, including whitespace, attribute
order, escaping, and the browser controllers they connect. Do not redesign a screen or silently
keep an upstream Campfire template because it compiles.

## Provenance and ownership

The templates were ported from Smartfire's Rails `app/views/**/*.erb` and `app/helpers/**/*.rb`
at the commit in `parity/reference.sha` (`78b9b1546`, including Rails PR #148,
`PRESENTATION_CACHE_VERSION = 3` and the Edge icon correction in #151). The Rails app has since
been removed; read those files in the git history when a template's origin matters.

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
Askama does not do that automatically: move statement tags to the following line while leaving
expression/block-render indentation in place. An expression that renders empty can still leave a
whitespace-only line.
Askama removes the final literal newline; use an explicit `{{ "\n" }}` when Rails emits it.
Do not normalize, trim, parse/reserialize, or widen masks to make an HTML comparison pass.

The byte goldens were recorded from the Rails app with real fixtures, a fixed clock and
test-only signing secrets (`reference-tools/views/core`, removed with the Rails app), and are
frozen: change one only with the behavior it pins. Fixed CSRF placeholders (`GLOBAL`,
`method:action`) and nonce (`NONCE`) were lent identically to both renderers. Signed stream names
and avatar URLs use our real SHA1-derived test keys, not upstream's goldens.

```sh
cargo test -j 4 -p campfire_views --test core
```

`tests/core.rs` compares complete strings and reports the first differing byte. The optional
diff directory receives actual/expected files for `diff -u`. Its injection test proves that a
title change, extra space and asset URL change are rejected. Show a new test failing before
fixing the port, or run it against an intentionally broken implementation.

`images.sh` uses a private copy of the built default parity seed. It generates complete
`messages.image` and `messages.image_large` goldens for David then JZ through one Rails cache,
plus filename preview probes. Run it after `run.sh`, which replaces the core golden directory.
`AttachmentView::filename_base` comes from the storage crate's `Filename::base` on the raw blob
filename, before sanitization: Rails uses `File.basename(filename, File.extname(filename))` for
image alt text, but its sanitized filename for links and file labels. The image tests compare
both filename operations and the complete preview bytes against Rails, including multiple dots,
no extension, dotfiles, paths, trailing dots, whitespace and HTML escaping.

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

The full message composition lives in `messages::MessageView`: `details` carries system-note,
thread, pin, edit, streaming, reply, forward, Drive, poll and step facts. `UserView::icon` is resolved
only for a bot without an uploaded avatar. Reactions are resolved through `IconSource`; the static
facts and grapheme/title probes come from pinned Rails, including gemoji Unicode aliases.

`MessageComponents` is the domain adapter for populated GitHub, Twitter, event, quote, Fizzy,
LinkedIn and generic embed children. Each entry is the owning partial's already rendered loop
body, including its whitespace. Never put request tokens, viewer-private data or unsanitized
text in these entries. Empty replacement containers render even without children, as in Rails.
WS8b supplies quotes/composition, WS14 events and Drive, and WS15 GitHub/Fizzy/X/LinkedIn/embeds.
These providers are still incomplete; empty-container goldens do not prove populated cards.

HTML fragments now include `ViewContext::base_url` in their key because Rails message attributes
contain absolute URLs. A presenter using `cached_message_fragment(id, updated_at, base_url)` must
pass the same origin used to render. That retained reader uses the default UTC zone;
non-UTC legacy renders use `cached_message_fragment_with_cards_in_zone` with the render's
zone. Writer and reader share the zone key. Collection fragments retain the domain
presentation key, including #179's viewer zone and origin. This preserves the forged-host
security property and prevents a warmed UTC fragment from serving another zone.

PR #151 corrects the Edge image to `external/install-edge.svg`. The current pin includes it;
the Edge golden renders the actual partial with no source substitution.

`fragment_cache::keys::MessageKey` ports `message_with_pr_cards_cache_key`: record version,
newest cards, optional embed references/PR thread stamp, pins, thread count, poll, note/stream
flags, agent steps, quote timestamps/names and version 3. `sidebar_membership` includes the
ordered huddle participant IDs and administrator flag. Keys are different in UTC and Hawaii
where Rails expands a `Time` through `to_a`. Quote-name digests use Ruby `Array#inspect`, not
JSON. WS8b must gather these inputs and wire these keys into its cache reads and writes, with
agent facts from WS11, huddle/sidebar inputs from WS13, event/Google/calendar/Drive inputs from
WS14 and card facts from WS15. The inherited message cache entry point still needs that wiring.
The process-local template digest is not ActionView::Digestor's shared Redis digest.
