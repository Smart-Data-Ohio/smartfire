# WS8br2 owned-view escaping review (PR #177)

Reference: Rails d7c7de92, with the approved 2e20b24c status/controller/layout overlay only.

The sweep includes `views/src/users.rs`, `views/src/users/`, `views/src/accounts.rs`,
`views/src/accounts/`, `views/src/{public_pages,welcome,first_runs,pwa}.rs`, their templates,
and the shared application title/style helpers used by those pages. Every raw-content,
HTML-trusting link/button/content-tag helper and safe-output boundary was inspected back
through its arguments. Attributes use the escaping attribute builder. Askama expressions
escape stored strings before block filters receive the rendered HTML.

| Boundary | Input provenance and result |
|---|---|
| `users/agent_profile.rs::agent_rooms` | Stored shared-room names and direct-room display names were sent directly to `link_to`. Fixed to `link_to_text(name, url, attrs)`. |
| Agent profile/badge templates | User name, provider/runtime, description, status note, kind/grant/activity/budget summaries use escaped Askama expressions. Subscription attributes use the attribute builder. |
| Cards, directory, human profile, transfer and ban controls | Name, bio, custom status, email and avatar title are escaped by expressions, `mail_to`, form helpers or attribute builders. Raw status/profile subviews are locally rendered templates. Call/message button bodies and hidden fields are locally generated HTML. |
| Profile settings, appearance and status popup | Field/error wrappers receive already rendered form tags. Choice labels use text helpers; select wrappers receive their escaped option HTML. Textarea values are escaped. Google connection buttons take literal labels. Raw GitHub/Fizzy/status/security fragments are owner render-function output. |
| Memberships and sidebar | Link filters receive rendered, escaped labels. Menu/name/title/data attributes use the attribute builder. Raw row/cache output is locally rendered HTML. Optional huddle participants are explicitly trusted server-rendered WS13 fragments, never a room/person label. |
| Account users, help contact, icons, logos, invite and settings | Names, email, icon titles/creator names and account name use escaped expressions/form/attribute helpers. Help-contact link attributes escape the interpolated name/address. Form filters contain locally rendered tags. |
| Audit HTML and CSV | Actor/target labels, details, user-agent/IP and filter values use escaped expressions, text option helpers or explicitly escaped option text. CSV retains Rails' quoting and formula handling; its values are not HTML. |
| Custom styles | The editor textarea escapes its value, including a closing textarea/script sequence. The application style helper deliberately emits the authorized custom CSS as Rails' `html_safe` style content; changing that would change the feature. |
| Welcome, first run, public, PWA and QR | Stored values go through expressions, form/text or attribute helpers. Public layout raw content is assembled from the page templates, not an input string. Shared page titles use the text helper. |

The only direct stored-text-to-HTML-trusting sink found was the agent room link. New
Rails-generated vectors cover actual tags, ampersands and quotes in agent room/name/bio,
provider/runtime/description/status-note; card name/bio/status/emoji; the entire profile
(name/bio/status/OOO note/GitHub input, DND person/title, membership and account names);
account member name/email; icon title/creator/form values; audit actor/target/details/IP/
user-agent; and the custom-style editor closing-tag sequence. Complete HTML/nav/application
page/CSV comparisons retain all bytes; no sanitizing or expectation masks are applied.
