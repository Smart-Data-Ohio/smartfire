# PR #182 review fixes

Reviewed input: `9883d3fa1d2f797f84fd3e96e9e7e8472435cdb9`.
The PR was conflicting, so `origin/main` was merged in `2139a2b6` before the fixes.
The one thread-header conflict retains the owner's adapter and main's render-zone behavior.
Rails oracle: `d7c7de92`, with the approved #163 layout/assets drift unchanged.

## Changes

- Shared message fragments retain the exact Rails collection-helper key and add the
  exact `MessagesController#index` validator composition over the message and its
  rendered reply source. This covers source edits, legacy `edited_at`, and both
  creators' updates without changing database touches. The original collection-key
  goldens remain unchanged. Cache witnesses use the actual stored key.
- Reaction resolution and classification use the shared Ruby `String#strip`
  implementation. The human controller and database resolution path were audited;
  their shortcode pattern rejects Unicode whitespace before icon lookup. The remaining
  `trim_matches(':')` in the reaction module handles registry alt text, not whitespace.
- The token-leak mutation anchors the `Arc` conversion independently of surrounding
  formatting. The GitHub omission mutation targets the actual preloaded renderer.
  Mutation checks honor the caller's test port ranges.

The new `cache-reaction-review.rb` oracle records actual Rails room/edit/profile
requests, both exact key compositions, and eight repeated boost inputs. The committed
vector was regenerated from fresh seeds and reproduced byte for byte. Its reply has
a different creator from the source. Source edits and source-author renames occur
within one second, exercising microsecond keys. Both new regressions failed before
their fixes; the stronger different-author case also failed before adding the source
to the validator's records.

Public room-list/composer inputs are unchanged. This is a bounded review-fix slice;
the existing messaging behavior ledger's deferred flows retain their previous status.

## Final cache dependency audit (review of 8cc1e939)

PR #182 was conflicting again, so current main `59ad94de` was merged after the
focused cache fix. The three merge overlaps retain main's typed uncached quiet-stream
broadcast and webhook transport entry point, plus both sets of agent tests. The
message-controller test now calls that shared transport entry point.

Pinned Rails source remains `d7c7de92`; #163 changes only the approved layout/assets.
The **initial room list is uncached in Rails** (`app/views/rooms/show.html.erb:33`,
`:35`, `:37`). There is no `cache [message, reactors, ...]` block in
`app/views/messages/_message.html.erb:3`, nor an HTML `cache boost` block in
`app/views/messages/boosts/_boost.html.erb:1`. The latter's JSON counterpart does
use `json.cache! boost` (`app/views/messages/boosts/_boost.json.jbuilder:1`).
These distinctions matter: copying only the scrolling collection's key into the
new room cache cannot reproduce the uncached room's freshness.

### Literal Rails key compositions

The cached collections are `app/views/messages/index.html.erb:1`,
`app/views/channel_thread_messages/index.html.erb:1`, and
`app/views/rooms/refreshes/show.turbo_stream.erb:2`. Their callable key is exactly:

```ruby
newest_card = (message.github_pull_requests.map(&:updated_at) + message.fizzy_cards.map(&:updated_at) + message.twitter_posts.map(&:updated_at) + message.events.map(&:updated_at)).compact.max
embeds = message.link_embed_references.map { |reference| [ reference.id, reference.link_embed&.updated_at ] }
key = [ message, newest_card ]
key << embeds if embeds.any?
key << github_pr_threads_stamp(message.room_id) if message.github_pull_requests.any?
key << message.message_pins.map(&:updated_at).max
key << message.channel_thread&.messages_count
key << message.poll&.updated_at
key << message.system_note?
key << message.streaming?
key << message.agent_steps.map(&:updated_at).max
key << message_quote_stamp(message)
key << message_quote_names_digest(message)
key << MessagesHelper::PRESENTATION_CACHE_VERSION
key
```

| Key element | Exact Rails source/meaning |
| --- | --- |
| Message record | `app/helpers/github/pull_requests_helper.rb:38`; `cache_key_with_version`, **database id**, not `Message#to_key`'s client id (`app/models/message.rb:402`). |
| Newest card | `app/helpers/github/pull_requests_helper.rb:34`: compact maximum over GitHub PR, Fizzy card, X post, event `updated_at`; not over references or users. |
| Optional embeds | `app/helpers/github/pull_requests_helper.rb:37`: reference id and nullable embed timestamp, in association order. |
| Optional PR threads | `app/helpers/github/pull_requests_helper.rb:40`, `:147`: maximum mapping timestamp for the message's room. |
| Pins / thread / poll | `app/helpers/github/pull_requests_helper.rb:41`, `:42`, `:43`: separate slots; a reply counter, not a thread timestamp. |
| Note / streaming / steps | `app/helpers/github/pull_requests_helper.rb:44`, `:45`, `:46`: literal booleans and newest step timestamp. |
| Quote edits | `app/helpers/github/pull_requests_helper.rb:47`, `app/helpers/message_links_helper.rb:41`: compact maximum of source `updated_at` and `edited_at`. |
| Quote names | `app/helpers/github/pull_requests_helper.rb:48`, `app/helpers/message_links_helper.rb:52`: SHA256 of sorted Ruby `[[creator.name, room.name], ...].inspect`; nil room names remain nil. |
| Presentation version | `app/helpers/github/pull_requests_helper.rb:49`, `app/helpers/messages_helper.rb:161`: integer **3**, covering helper changes that template digests cannot detect. |

The page ETag array remains exactly
`[@messages, rendered_related_stamp(@messages), rendered_pin_stamp(@messages), 3]`
(`app/controllers/messages_controller.rb:24`). The related stamp is the maximum of
selected edited timestamps, reply source updated/edited timestamps, GitHub/X/link/event
card timestamps, polls, quote source updated/edited timestamps, selected creators'
timestamps, formatted UTC with **microseconds** (`:175` through `:187`). The pin
component is SHA256 over the ordered Ruby `[message_id, pin_id]` pairs (`:196`).
The previous room wrapper applies this composition to `[message, reply_source]`;
that alone misses other rendered users and can hide independent updates behind a maximum.
Both literal Rails compositions are preserved, with their existing oracle comparisons.

### Every rendered input and its invalidation

This follows every branch in `app/views/messages/_message.html.erb:3` through `:33`.
“Individual version” below means the record's literal Rails `cache_key_with_version`,
not a new timestamp maximum. These form the additional room reuse dependency array;
Rails itself needs no such array for its uncached initial room render.

| Rendered inputs | Rails source | Key / touch coverage in Rust's mounted fragment |
| --- | --- | --- |
| Message/client id, room/thread/creator ids, created/updated times, sort/targets/URLs, note/action/emoji classes | `app/helpers/messages_helper.rb:49`, `:79`, `:93`, `:103`, `:111`; `app/models/message.rb:402` | Original message version, flags, individual message version; source records also carry their versions. |
| Day separator and timestamp hooks | `app/views/messages/_message.html.erb:4`; `app/helpers/time_helper.rb:2` | Message created time; normal date/time hooks emit ISO time for browser localization, not viewer-local text. |
| System note actor, body, permalink, icon | `app/views/messages/_system_note.html.erb:6` | Message/body versions and actor's individual User version; static asset resolver scoped to the app. |
| Author name, bio/title, profile-card URLs, avatar, bot icon/role | `app/views/messages/_meta.html.erb:3`; `app/helpers/users/avatars_helper.rb:15`, `:29`; `app/models/user.rb:135`; `app/helpers/users_helper.rb:6` | Author's individual User version, avatar attachment/blob identities, workspace icon versions. |
| Room name/direct member names and optional room icon | `app/views/messages/_meta.html.erb:14`; `app/helpers/rooms_helper.rb:106`; `app/models/rooms/direct.rb:77` | Individual STI Room version and direct members' User versions. Search's `show_room_icon` local uses a separate key suffix. |
| Toolbar, pin badge, streaming state | `app/views/messages/_message.html.erb:16`, `:17`, `:19`; `app/views/messages/_pin_badge.html.erb:3` | Message flags, exact pin slots/pin digest, pin record set/versions; fixed toolbar assets. No viewer/session token. |
| Reply source existence/id, author name, text, forward-note text, attachment fallback filename, permalink/thread id | `app/views/messages/_context.html.erb:1`; `app/models/message.rb:262`; `app/helpers/messages_helper.rb:111` | Source Message/RichText/Room versions, source-author and source-mentioned User versions, attachment/blob identities; existing page validator includes reply source edited stamp. |
| Forward flag/note and lazy source endpoint | `app/views/messages/_context.html.erb:16` | Forwarding Message version; original source contents/access are fetched separately and are not embedded here. |
| Legacy/Markdown body, sanitized HTML, emoji/icon rendering, rich-text embedded blobs and mentions | `app/views/messages/_presentation.html.erb:5`; `app/helpers/messages_helper.rb:163`, `:188`; `app/views/users/_mention.html.erb:1`; `lib/rails_ext/action_text_attachables.rb:18` | Message/RichText versions and touch chain; every resolved mentioned User version, including reply/quote bodies (`app/models/message/mention_preloader.rb:66`); workspace icon record set/versions. |
| File presence/type, filename/base, metadata width/height, signed blob/variant URLs, download/share labels | `app/helpers/messages/attachment_presentation.rb:6`, `:29`, `:40`, `:58`, `:76`, `:88`, `:94`, `:100`; `app/models/message/attachment.rb:8` | Attachment/blob identities have no changing version; Rails blob updates touch attachments, then owner, then Room. Source owner versions protect filename previews. Variant transforms are fixed code/config; no variant row fields render. |
| Sound asset path, text/image and dimensions | `app/helpers/messages_helper.rb:221` | Message/body version; sound catalog/assets immutable for the app instance, presentation version 3. |
| Agent step order/count, name/status, duration, input/output summaries | `app/views/agent_steps/_steps.html.erb:5`; `app/models/agent_step.rb:24` | Exact newest-step helper slot plus independent step record set/versions. Agent's own profile is not rendered in the steps. |
| Poll identity/options/labels/order, counts/percentages, voter ids/names, anonymity/multiple choice, close time/state, token-free forms | `app/views/polls/_poll.html.erb:15`, `:21`, `:35`, `:44`, `:59` | Original separate poll timestamp; individual Poll/Option/Vote/User versions. Also `Poll#closed?`'s boolean (`app/models/poll.rb:68`) so room reload matches Rails at the deadline before the periodic closer writes. |
| Drive file ids/URLs, fixed generic chip (not remote file metadata) | `app/views/messages/_drive_attachments.html.erb:9`; `app/models/drive_attachment.rb:9`, `:18` | Drive row identities and owner-touch chain; no Drive credentials/private metadata enters the shared fragment. |
| GitHub order, privacy/lazy frame, repository/number/state, title/error/loading, author login/avatar, branches, review/checks, update time, URL, Discuss mapping/form | `app/views/github/pull_requests/_cards.html.erb:5`; `app/views/github/pull_requests/_card.html.erb:1`, `:13`, `:17`, `:26`, `:32`, `:44`, `:50`; `app/helpers/github/pull_requests_helper.rb:61` | Original card/mapping stamp slots plus every PR/reference/mapping version. Private content is fetched by the viewer endpoint. PR authors are provider fields, not local User rows. |
| X identity, error/loading, author/profile/avatar, text/clamping, media types/URLs/alt/dimensions, quote fields, posted time, counts/view URL | `app/views/twitter/posts/_card.html.erb:1`, `:13`, `:26`, `:41`, `:61`, `:78` | Original card stamp plus Post/Reference individual versions; nested quoted-provider data lives in the Post record. |
| Event title/state/series/time zone/start/end, venue name, Meet link, organizer name, lazy attendance endpoint | `app/views/rooms/events/_card.html.erb:1`, `:9`, `:12`, `:18`, `:22`, `:26`, `:29`; `app/models/message.rb:169` | Event-bearing messages already bypass the collection store and render current Event/Room/Organizer/Venue inputs in request context. No stale event fragment can be reused; attendance remains per viewer. |
| Quote source author, neutral room label, created time, excerpt/mention names/attachment fallback/forward note, jump URL; cross-room lazy frame | `app/views/messages/message_links/_cards.html.erb:6`; `app/views/messages/message_links/_card.html.erb:7`, `:8`, `:9`, `:11`, `:15`; `app/helpers/rooms_helper.rb:120` | Exact quote stamp/name digest; Source Message/RichText/Room/User/mentioned User versions and attachment/blob identities; Reference versions. Cross-room source contents/access stay behind their endpoint. |
| Fizzy sorted card ids, frame id/src | `app/views/fizzy/cards/_cards.html.erb:6`; `app/helpers/fizzy/cards_helper.rb:11`, `:17` | Original card stamp plus Card/Reference versions; payload and viewer's Fizzy account cache are not in this partial. |
| LinkedIn URL/URN embed player, fetched title/description/image or fallback chip | `app/views/linkedin/posts/_cards.html.erb:8`; `app/views/linkedin/posts/_card.html.erb:7`; `app/helpers/linkedin/posts_helper.rb:10`, `:23` | Exact embed-reference pairs plus LinkEmbed/Reference versions; suppression belongs to the Message version. |
| Generic embed site/title/description/image and message-specific URL/position | `app/views/link_embeds/_card.html.erb:7`, `:11`, `:14`, `:19`, `:23`; `app/models/link_embed_reference.rb:11`; `app/helpers/link_embeds_helper.rb:11`, `:24` | Exact embed pairs plus **reference** and embed individual versions, including URL/position changes with an unchanged card stamp. |
| Boost creation order/content/classification, grouped reactor ids/count/names, legacy booster title/name/bio/avatar/accessibility labels and delete controls | `app/views/messages/boosts/_reactions.html.erb:1`; `app/views/messages/boosts/_reaction.html.erb:4`, `:29`; `app/views/messages/boosts/_boost.html.erb:3`, `:5`, `:9`, `:14`; `app/models/boost.rb:5` | Boost touches Message; independent Boost and every booster/reactor User version. Removed the non-Rails nested HTML boost cache. |
| Reaction/boost shortcode title/character/brand or workspace image | `app/helpers/boosts_helper.rb:4`, `:15`, `:29`; `app/models/icons.rb:145`, `:173`, `:182` | Workspace icon record set/versions (also consulted by bot/room icons and Markdown); compiled brand/gemoji/config/assets live with the app. |
| Thread indicator count/hidden state/pluralized label and fixed icon | `app/views/messages/_thread_indicator.html.erb:10`; `app/models/channel_thread.rb:206` | Literal helper count slot; parent Message stamp from count refresh; Thread record identity/version. The thread's reply bodies/authors are not rendered by this indicator. |
| Unrenderable fallback | `app/helpers/messages_helper.rb:84`; `app/views/messages/_unrenderable.html.erb:1` | Dependencies disappearing change the individual record set; fixed template fallback. |
| URL origin, configuration/signing inputs and assets | `app/helpers/messages_helper.rb:93`; `app/views/users/_mention.html.erb:1`; `app/helpers/users/avatars_helper.rb:33` | Verified origin remains in the key; signer/config/asset resolver and cache share one app lifetime. Viewer CSRF tokens are excluded by Rails' forms. |

### Touch chains and exact record versions

The image's Rails gem tree is
`/usr/local/bundle/ruby/3.4.0/bundler/gems/rails-1a02651ac37f/`.
Paths below are relative to that tree when prefixed `gem:`.

| Dependency | Rails source and effect |
| --- | --- |
| Every timestamped record in the dependency array | `gem:activerecord/lib/active_record/integration.rb:97`, `:114`: `model_name.cache_key/id-UTC_updated_at_usec`; individual records, not Relation count/max keys. Expansion calls each record's versioned key. |
| STI Room / namespaced records | Same `gem:activerecord/lib/active_record/integration.rb:72`; the Rails oracle records `rooms/closeds`, `rooms/directs`, `github/pull_requests`, `twitter/posts`, `fizzy/cards`, `action_text/rich_texts`, `active_storage/attachments`, `active_storage/blobs` stems. |
| Blob, Attachment, DriveAttachment | No `updated_at` column: `cache_version` is nil and `cache_key_with_version` uses plain `model_name.cache_key/id` (`gem:activerecord/lib/active_record/integration.rb:79`, `:100`, `:117`; `gem:activerecord/lib/active_record/timestamp.rb:163`). `created_at` is **not** substituted as a changing version. |
| Message → Room | `app/models/message.rb:17`, `touch: true`. Creator, reply, forwarded source and thread associations at `:18` through `:21` do not touch the Message when their rows change. |
| Boost → Message → Room | `app/models/boost.rb:2`, `touch: true`; booster at `:3` has no touch. A User rename does not update any Boost or Message. |
| DriveAttachment → Message → Room | `app/models/drive_attachment.rb:9`, `touch: true`. |
| RichText → Message → Room | `gem:actiontext/app/models/action_text/rich_text.rb:46`, polymorphic `touch: true`; rich-text embedded attachments at `:55` use Active Storage. The legacy controller's edited timestamp remains separately covered by the exact Rails validator. |
| Attachment → owning User/Message/RichText | `gem:activestorage/app/models/active_storage/attachment.rb:25`, `touch: ActiveStorage.touch_attachment_records` (enabled in the reference); purge explicitly touches owner at `:54`, `:63`. |
| Blob update → Attachments → owners | `gem:activestorage/app/models/active_storage/blob.rb:44`, `:369`: each attachment is touched. Blob filename/metadata updates therefore advance owner versions without inventing a blob timestamp. |
| Pin/unpin | `app/models/message_pin.rb:91`: writes Room `pins_changed_at` and Message `updated_at` without normal Room touch; the original distinct pin slot and ETag pair digest remain. |
| Thread reply count | `app/models/channel_thread.rb:206`: writes count and parent Message stamp, deliberately leaving Thread `updated_at` alone. Its count stays in the original key. |
| Poll votes/close | `app/models/poll.rb:113`, `:77`: ballot changes touch Poll; close writes Poll timestamp. Poll, Option, Vote and voter associations do not touch Message or each other automatically (`app/models/poll.rb:5`, `app/models/poll_option.rb:4`, `app/models/poll_vote.rb:2`). |
| Cards, references and agent steps | `app/models/message.rb:40`, `:44`, `:48`, `:52`, `:55`, `:59`, `:66`; `app/models/link_embed_reference.rb:2`; `app/models/agent_step.rb:11`: their updates do not automatically touch Message; the exact helper aggregates and individual records both remain represented. |
| User/Room renames | `app/models/message.rb:18`, `app/models/boost.rb:3`, `app/models/poll_vote.rb:4`, `app/models/rooms/direct.rb:77`: no reverse touch; all rendered user roles and rooms must therefore appear independently in the added room dependency array. |

The added array is the sorted, deduplicated set of these literal record keys,
plus a poll's `closed?` boolean when present. It is **a Rust room-cache reuse guard**,
not a claim that pinned Rails has a missing `cache` call containing this array.
`rendered-dependencies.rb` obtains the keys via Rails models and
`ActiveSupport::Cache.expand_cache_key`; Rust asserts the entire expanded string
before and after every scenario. The original helper and ETag arrays are unchanged.

### Template digests

`gem:actionview/lib/action_view/renderer/partial_renderer/collection_caching.rb:69`
adds `digest_path_from_template` before expanding the callable key.
`gem:actionview/lib/action_view/helpers/cache_helper.rb:256` computes the Digestor
hash of the partial and its dependency tree, including explicit dependencies;
`:267` combines it with the key array. The recorded pinned digest is
**`4bc70e94df311f86c11d03b24ea441ae`**. The mounted collection prefix now uses that
literal Rails digest from `vectors/messaging/message-template-digest.txt`.
The store is app-local and cannot survive Rust binary/template/helper changes.

The exact discovered tree is recorded in the oracle JSON. Every real entry's
call site is listed below; all refer to `app/views/`:

| Digest dependency | Rails call site |
| --- | --- |
| `messages/_message` | `messages/index.html.erb:1` |
| `messages/system_note`, `toolbar`, `pin_badge`, `meta`, `streaming_indicator`, `context`, `presentation` | `messages/_message.html.erb:7`, `:16`, `:17`, `:18`, `:19`, `:20`, `:21` |
| `agent_steps/steps`, `polls/poll`, `messages/drive_attachments` | `messages/_message.html.erb:22`, `:23`, `:24` |
| `github/pull_requests/cards`, `twitter/posts/cards`, `rooms/events/cards`, `messages/message_links/cards`, `fizzy/cards/cards`, `linkedin/posts/cards`, `link_embeds/cards`, `messages/boosts/boosts`, `messages/thread_indicator` | `messages/_message.html.erb:25`, `:26`, `:27`, `:28`, `:29`, `:30`, `:31`, `:32`, `:33` |
| `github/pull_requests/card`, `twitter/posts/card`, `rooms/events/card`, `messages/message_links/card`, `linkedin/posts/card`, `link_embeds/card` | `github/pull_requests/_cards.html.erb:7`, `twitter/posts/_cards.html.erb:9`, `rooms/events/_cards.html.erb:5`, `messages/message_links/_cards.html.erb:9`, `linkedin/posts/_cards.html.erb:9`, `link_embeds/_cards.html.erb:9` |
| `messages/boosts/reactions`, `reaction`, `boost` | `messages/boosts/_boosts.html.erb:3`, `messages/boosts/_reactions.html.erb:9`, `:16` |

Rails' parser also reports three nonexistent leaf names (`inlines/inline`, `as/a`,
`nothings/nothing`) from dependency tracking; these have no template
bytes and remain in the recorded tree, without fabricated source files. Their parser inputs are respectively
`app/views/messages/message_links/_card.html.erb:2` (render inline),
`app/views/messages/boosts/_reactions.html.erb:16` (collection `as:`), and
`app/views/link_embeds/_card.html.erb:3` (render nothing). Helper-driven
mention and unrenderable renders are not discovered as explicit template render calls:
their real sources are `app/helpers/messages_helper.rb:192` and `:88`; helper changes
are covered by the presentation version and app lifetime, exactly as in Rails.

### Regressions for every discovered missing dependency

The oracle warms the real room twice, updates one input, reloads twice, and captures
whole Rails message partials plus exact record-key arrays. Fourteen independent cases
cover reactor, legacy booster, body/reply/quote mentioned users, poll voter, poll option,
room name, direct-room member, icon title, reference URL and clock-dependent poll close.
All twelve missing inputs failed before production edits. Two additional filename
cases (own attachment and reply-source attachment) already passed through the owner
version touch chain and still pass. Previous source-edit/author-rename regressions remain.
All changes/renames happen 125 ms after warm-up. The oracle reproduces byte for byte;
its fixture rows contain no user/session credentials or scanner-shaped secrets.

Final-round commands and raw summaries are recorded below after the gates finish.

## Previous round verification (8cc1e939)

The final checks use Rust 1.98.1 in the pinned media toolchain, freshly generated
`default` and `first_run` seeds, `CI=1`, two build jobs, eight test threads, and ports
54000–54049. Docker compilers share the unchanged host rustc slot pool. The release
input build runs the requested CI wrapper with one compiler under one host slot.
The fixture additions contain no scanner-shaped secret strings.

All required gates passed. The full workspace run includes `html5ever` and doctests:
**3375 passed, 0 failed, 12 existing ignores**, across 60 summaries. No missing-seed
skips occurred. Locked metadata completed with exit 0. Both named mutations reached
their intended assertions and were rejected; production sources were restored.

Commands:

```sh
cargo test --locked --workspace --no-fail-fast
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo metadata --locked --format-version 1
bash rust/ci/with-release-inputs.sh bash ./ci/cargo.sh build --locked --workspace --bins
PARITY_IMAGE=triage-reference-d7c7de92 python3 rust/reference-tools/messaging/check-goldens.py cache-reaction-review
python3 rust/reference-tools/messaging/check-owned-mutations.py cached-token-leak github-card-omitted
```

Raw summary lines (terminal color escapes removed from the CI build line):


Before the P2 fixes:

```text
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 1714 filtered out; finished in 2.27s
```

Before including a different source author:

```text
test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 1714 filtered out; finished in 2.09s
```

Final P2 regressions:

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1714 filtered out; finished in 2.11s
```

Final mutation rejections:

```text
cached-token-leak: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1715 filtered out; finished in 2.09s
github-card-omitted: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1715 filtered out; finished in 0.81s
WS8bm owned mutations: 2 rejected; 0 survived; production files restored
```

Full workspace tests and doctests:

```text
test result: ok. 1713 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 847.70s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.51s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.88s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 951 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 85.49s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.46s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.97s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.32s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.96s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.54s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.24s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.97s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.18s
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.41s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.41s
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
```

Strict Clippy:

```text
    Finished `dev` profile [unoptimized] target(s) in 57.50s
```

Release-input build:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m 39s
```

Rails oracle reproduction:

```text
WS8bm golden check: 1 Rails oracles re-run; 1 golden files byte-identical
```

Pinned Rails source verification:

```text
WS8bm reference source check: 63 controller, model, helper, template and icon files match d7c7de92
WS8bm reference check self-test: 2 injected source-byte/file-set differences rejected
```
