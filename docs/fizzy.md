# Fizzy cards

Smart Data tracks work in [Fizzy](https://app.fizzy.do) (37signals'
open-source kanban tool). This slice links conversations to Fizzy
cards: card links unfurl into live cards, any message can become a
card, and agents can read and (with approval) write cards. It mirrors
how the [GitHub integration](github.md) is built.

Out of scope for this slice: a board-to-room webhook feed, and a
workspace-level Fizzy token.

## What it does

A message containing a `https://app.fizzy.do/<account_id>/cards/<number>`
URL renders a live card beneath the message: board name, number, title,
status (column name, Maybe?, Closed, Postponed), assignees with avatars,
tags, step progress (done/total), last-active time, and a link out to
Fizzy. The stored message body is never changed; references live in the
`fizzy_card_references` join table and are re-synced when a message is
created or its Markdown source is edited. Comment links (a card URL
with a trailing path) unfurl their card the same way.

- Up to 4 cards render per message, and URLs inside code spans and
  fenced blocks are ignored, using the same extraction helpers the PR
  cards use.
- Card content is fetched with the *viewer's* own linked Fizzy token
  and cached per viewer for 5 minutes (`fizzy_card_caches`). Rendering
  with a missing or stale cache enqueues `Fizzy::FetchCardJob`, which
  refreshes that viewer's row and broadcasts fresh (content-free) card
  frames; each viewer's frames then reload with their own token. At
  most one fetch per viewer and card runs per 5-minute window, however
  many renders race.
- Posting a message warms the author's own cache row so their card
  fills in without a reload; other viewers fetch on view.
- Fizzy URLs are excluded from the generic OpenGraph unfurl
  (`UnfurlLinksController` answers 204) so a card does not render twice.

## Visibility

Every viewer only sees cards they can access in Fizzy. Fetching with
the viewer's token *is* the gate: when Fizzy answers 200 the card
renders, and when it answers 404 or 403 the viewer sees only a minimal
link chip ("Fizzy card #123") — the same per-viewer rule the private
GitHub cards use. Viewers without a connected account see the chip
with a "Connect Fizzy to preview" hint.

## Connect your Fizzy account

Card previews and card creation run as the member's own Fizzy user
through a per-user personal access token — never a shared token, and
administrators get no special powers. Linking happens on the profile
page: paste a token, the app validates it with `GET /my/identity`,
and stores the returned account and user with the encrypted token.
The token is never logged or shown again; unlinking deletes it.

Generate the token in Fizzy under your profile → API → Personal access
tokens. Previews work with a **Read** token; creating cards needs
**Read + Write**. Linking a token whose identity can see several Fizzy
accounts uses the first one for the board picker and agent defaults;
card links themselves always carry their own account, so previews work
in every account the token can access.

If Fizzy later rejects the token (401), the account is marked
disconnected with a reason and the profile offers a reconnect instead
of a first-time connect. Deactivating a member disconnects their
account the same way.

## Create a Fizzy card from a message

Any room member with a linked token can turn a message into a card
from the message actions menu (**Create Fizzy card**). The form shows
the source message, a board picker (the member's boards via the API),
a title prefilled from the message's first line, and a description
prefilled with the message text plus a permalink back to it.

Creating posts the card to Fizzy as the member's own user, then posts
a reply in the same conversation with the new card's URL so it unfurls
there. Members without a linked token get a connect prompt instead of
the form. A read-only token is called out ("generate a Read + Write
token") without disconnecting the account; only a truly rejected
token marks it disconnected.

## Agent access

Agents read Fizzy through `GET /agents/fizzy/boards` (list), `GET
/agents/fizzy/boards/:id` (board with columns), `GET
/agents/fizzy/cards/search?q=` (search), and `GET
/agents/fizzy/cards/:account_id/:number` (show, with steps), all as
the agent *owner's* linked Fizzy account. Reads need the workspace-wide
`fizzy` capability. Writes — create, comment, move to a column, close,
reopen — go through `POST /agents/fizzy/card_actions`, need the
workspace-wide `external_action` capability, and only run after a
human approves, exactly like the GitHub write-action flow. See [AI
agents](agents.md#fizzy-write-actions) for the endpoint, gates,
approval, and completion event.

## Configuration

No environment variables are required. All Fizzy calls use per-user
tokens against `https://app.fizzy.do`; set `FIZZY_API_BASE_URL` only
to point the client at a self-hosted Fizzy installation. The same
setting drives card URL extraction and card links: only card URLs on
the configured host unfurl into cards, and link fallbacks point at
that host, so a self-hosted Fizzy works end to end.

```sh
curl -H "Authorization: Bearer $FIZZY_TOKEN" -H "Accept: application/json" \
  https://app.fizzy.do/897362094/cards/579.json
```
