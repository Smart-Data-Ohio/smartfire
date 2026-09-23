# AI agents

First slice of [roadmap milestone 3](../ROADMAP.md#3-ai-agents-as-first-class-participants).
See the [first-slice design](design/agent-identity-slice-1.md) for the full plan.

## Identity

An agent is a row in `agents`, 1:1 with a bot `User`. `kind` is `personal`
(requires an `owner_id`) or `workspace` (requires an owner or managing group on
create; backfilled rows may have no owner, rendered as "no owner recorded").
`Agent#active?` is false while `suspended_at` is set or the bot user is not
active. Suspending an agent revokes all of its capability grants in the same
transaction. Deactivating or banning a person suspends every agent they own, so those
agents' Bearer tokens answer 401 and their bot keys get 403 from every
capability check.

## Credentials

`agent_credentials` holds Bearer agent tokens. Credentials store only a SHA256 digest
plus a display identifier; the secret is shown once at creation. Revoked or
expired credentials return 401 on the next request. The legacy `bot_key` URL
path is frozen and unchanged. Bot keys (`<id>-<token>`) authenticate the
same way: `users.bot_token_digest` holds the token's SHA-256 digest,
compared in constant time after an id lookup, and the UI shows the key
once, on the page that follows creating the bot or generating a new key
(the bots page's curl examples use a `BOT_KEY` placeholder). Existing keys
kept their value. The retired plaintext `users.bot_token` column is never
read or written; leftover values are nulled by
`bin/rails bots:clear_plaintext_tokens` (also run once by the periodic
runner) wherever a digest exists. The column itself stays because
migrations must remain strictly additive. A bot row without a digest
cannot authenticate until its key is reset.

### Rate limits

Agent endpoints throttle per credential per minute, so one busy
credential cannot starve others sharing the agent:

- event polling and acks: 120/minute each
- approval reads, context reads, and Fizzy board and card reads: 120/minute each
- posting messages, requesting approvals, cancelling approvals, opening
  DMs, pull-request actions, and Fizzy card actions: 60/minute each
- creating board posts: 30/minute
- the whole MCP endpoint: 600/minute per credential across all methods,
  on top of the per-tool buckets its tools share with the endpoints
  above

Overflowing a bucket returns 429 with a `Retry-After` header in
seconds and a `{ "error": "rate_limited" }` body. Human session
requests are not throttled, and neither are the frozen legacy
bot-key endpoints, which carry no credential to key a bucket on.
These limits are separate from the
20-deliveries-per-minute-per-room delivery guard below.

## Capability grants

`agent_grants` rows scope what an agent may do: `agent_id`, nullable `room_id`
(`NULL` means workspace-wide), `capability`, `granted_by_id`, `revoked_at`, and
a partial unique index over active rows. Capabilities are `read_messages`,
`post_messages`, `react`, `manage_threads`, `external_action`, `fizzy`, and `dm_anyone`.
`dm_anyone` is granted workspace-wide only: the grant form rejects a room
scope, and the DM check counts only active workspace-wide grants (a
room-scoped row, if one predates the validation, grants nothing).

`read_messages`, `post_messages`, and `react` are enforced through
the `AgentAuthorization` concern (`require_agent_capability`) on the bot
message endpoints, the bot boost endpoints,
`POST /rooms/:room_id/agents/messages` (JSON, Bearer-only), and the event
polling endpoints below; `external_action` is enforced on the approval
endpoints (see Approvals), `manage_threads` is enforced on the agent
work endpoints (see Work threads), `fizzy` is enforced on the agent
Fizzy read endpoints (see Fizzy reads), and `dm_anyone` is enforced on the
agent DM endpoints (see Agent DMs). Enforcement reads the database on every
request; nothing is cached.

Room membership still applies on top of grants: every endpoint returns 404 for
rooms the agent's user is not a member of, so a workspace-wide grant never
bypasses membership. Missing capabilities deny with 403 and a JSON error body;
401 stays reserved for authentication failures (bad, revoked, or expired
credential; suspended agent; deactivated user).

### Legacy fallback

`Agent#legacy_capabilities?` is true when **no `agent_grants` rows exist for
the agent at all, revoked or not**. A legacy agent keeps `read_messages`,
`post_messages`, and `react` in rooms it belongs to. Once any grant has ever
been created, only active grants count: revoking the last grant removes access
rather than restoring the fallback.

### Cascade revocation

Revocation persists in the same transaction as the triggering change:

- Destroying a membership revokes that agent's grants in that room.
  Workspace-wide grants survive; the membership check itself still forbids the
  next post with a 404.
- Destroying a room revokes its room-scoped grants.
- Suspending an agent revokes all of its grants.
- Deactivating, banning, or destroying the agent's user revokes all of its
  grants.

Removing an agent from a closed room or revoking its grant therefore forbids
its next post immediately.

## Event delivery and activity ledger

`agent_events` is an append-only ledger (never backfilled from historical
messages) with `agent_id`, `event_type`, optional `room_id`, `message_id`,
`agent_credential_id`, `actor_id`, `outcome`, `detail`, JSON `metadata`, and
`created_at`, indexed on `[agent_id, created_at]`. Deliverable types are
`mention`, `direct_message`, `reply`, `approval_decided`,
`github_action_completed`, `fizzy_action_completed`, `work_assigned`, and `work_unassigned`;
ledger-only types are `posted`
(written whenever the agent posts through any endpoint) and the suppression
rows `delivery_suppressed_rate_limit`, `delivery_suppressed_hop_limit`, and
`delivery_suppressed_revoked`. Outcomes are `pending`, `delivered`,
`acknowledged`, and `suppressed`.

A message creates one pending event per recipient agent: mentions of the
agent's user, replies to the agent's messages (a reply wins over a mention
when both apply), and any message in a direct room with the agent. The
agent never receives its own messages, and bots without an agent row keep
the legacy webhook path only.

`Agent::DeliveryJob` re-checks room membership and the `read_messages`
grant at perform time, then marks the row `delivered` and enqueues its
webhook POST when one is configured. Polling is the primary path, so a
missing webhook still delivers. Revocation between enqueue and perform
writes `delivery_suppressed_revoked`; a message deleted before delivery
marks the row suppressed without a new row.

### Polling

`GET /agents/events?since=<id>&limit=<n>` (Bearer-only, JSON, ordered by
id, max 100) returns the agent's own deliverable rows as a bare JSON
array with the message payload resolved at query time. The last
scanned row id travels in the `X-Smartfire-Next-Since` response
header, which the client passes back as `since` to page forward; pass
`?envelope=1` for the `{ events: [...], next_since: <id> }` object
form instead. Rows for messages the agent can no longer read
(membership or grant revoked, message deleted) are omitted, and so are
work rows whose thread is gone without a snapshot — but every scanned
row still advances the cursor, so a fully dropped page returns no rows
with a cursor that moves past them. A deleted thread's
`work_unassigned` row is the exception: it returns its pre-destroy
snapshot marked `thread_deleted: true`.
`POST /agents/events/:id/ack` marks a row `acknowledged` and is
idempotent. Both require `read_messages`
(`Agent#has_capability_anywhere?` at the endpoint, per-room `Agent#can?`
per row and per ack).

Message event rows carry a `pull_request` key: the PR context object when
the message lives in a pull-request discussion thread, explicit null
otherwise. The object is `url`, `owner`, `repo`, `number`, `title`,
`state`, `head_branch`, `base_branch`, `review_decision`, and
`checks_state` (`checks_state` mirrors the card's check status). `title`,
`head_branch`, and `base_branch` are null unless the repository is known
public or the agent owner's own linked GitHub account can read it (the
card rule, with the owner as viewer); work `links` entries follow the same
rule for their `title`. See
[GitHub pull request cards](github.md#pull-request-threads).

An agent with the `post_messages` capability may also attach Drive files
when it posts through the agent message API by sending
`message[drive_file_ids][]`; only the ids are stored, exactly as for a
member's post. The `message` object itself carries `drive_attachments`: the message's
Drive attachments as `[{ file_id, url }]` (`url` is the
`https://drive.google.com/open?id=...` link), `[]` when there are none.
Never names: bots receive no Drive credentials. See
[Google Drive attachments](google-drive.md#attachments).

### Rate limit and loop guard

At most 20 deliveries per agent per room per minute, counted from
`agent_events` with the count and the insert sharing the agent's row
lock, so concurrent enqueues cannot over-deliver; excess writes
`delivery_suppressed_rate_limit` and is dropped, not queued. Agent-to-agent chains carry a hop count and a
trigger chain id: human messages start at 0, and an agent's message
carries its trigger's hop plus one, where the trigger is the most
recent mention, direct message, reply, or work assignment event
`pending` for, `delivered` to, or `acknowledged` by the agent in any
room within the last five minutes. Pending rows count so a fast
polling agent cannot restart the chain at hop 0, and bridging rooms
carries the chain instead of resetting it. The agent's own `posted`
rows, suppression rows, and approval decisions are never triggers,
and neither the request body nor the reply target influences the
hop; a message with no recent trigger is a new root at 0. A chain
reaching hop 3 writes `delivery_suppressed_hop_limit` instead of
delivering, so two agents mentioning each other stop with both
suppressions in the ledger.

Messages from bots without an agent row carry hops too: a reply
continues its source message's highest recorded hop (including the
sender's `posted` row), and a root post continues the most recent
room message within the window that mentioned the bot or replied to
it. The legacy webhook path honors the same hop limit and simply
stops posting once a chain reaches hop 3. Work assignments carry the
assigning agent's chain the same way: an assignment at hop 3 writes
`delivery_suppressed_hop_limit` with the thread fields instead of
`work_assigned` or `work_unassigned`, so two agents assigning posts
to each other stop. Human assignments always start a new root.

### Webhooks

The webhook payload gains an additive
`agent: { id, name, owner, delivery_id }` key (`owner` is the owner's name
or null) when posted through event delivery. The legacy bot webhook path
sends the unchanged payload without that key. Agent deliveries also carry
the same additive `pull_request` key as polling: the PR context object in
a pull-request discussion thread, null otherwise. The agent webhook's
`message` object also carries the same `drive_attachments` array as
polling (`[{ file_id, url }]`, never names); the legacy path omits it.

Every webhook POST, agent or legacy, resolves through
`RestrictedHTTP::PrivateNetworkGuard` and pins the connection to the
resolved public address: loopback and private destinations are
refused instead of posted to, and a hostname that resolves to
nothing fails the delivery.

A 200 response with a text or attachment body becomes a sync reply
to the triggering message: inside its thread when it has one (board
posts included), otherwise a root message referencing it. A reply
that cannot be stored (a locked thread, a deleted parent) is logged
for agent deliveries and the POST still counts as delivered; legacy
deliveries raise. A webhook that does not answer within 7 seconds
records a timeout in the agent's ledger row and retries like any
transport failure, with no timeout message; legacy bots keep the
"Failed to respond within 7 seconds" message.

The `room.path` in every delivery is the plain room path. No payload ever
carries the bot key. Legacy bots (bot users without an `Agent` row)
additionally receive `reply_url`: a signed path, fresh per delivery, that
expires after 15 minutes and posts replies through the bot posting
endpoint (`POST` to it with the reply body, exactly like the old keyed
`room.path`). The reply URL authenticates create only: reads, edits,
deletes, and boosts through a valid reply URL answer 403. Expired,
tampered, or wrong-room tokens authenticate nobody, so those requests
redirect to sign-in like any unauthenticated request. Receivers with a
stored bot key keep using it, and
agent-backed bots keep posting back with their own agent token; new
integrations should prefer an agent token. Integrations that still
`POST` to the old keyed `room.path` must switch: that path is now the
plain room path and answers without bot authentication.

### Verifying signatures

Every webhook POST carries an `X-Smartfire-Timestamp` header (unix
seconds) and, when the bot has a signing secret, an
`X-Smartfire-Signature: sha256=<hmac>` header with the HMAC-SHA256
of `"#{timestamp}.#{raw_body}"`: the timestamp header, a dot, and
the raw request body. Each agent has its own secret, generated
on its first delivery and shown to admins and the agent's owner on
the bot edit page with a reset control; legacy bots sign too once
a secret is generated for them on the same page. Verify with a
constant-time comparison over the raw bytes, and reject timestamps
older than 5 minutes to bound replays:

```ruby
require "openssl"

timestamp = request.headers["X-Smartfire-Timestamp"].to_s
signature = request.headers["X-Smartfire-Signature"].to_s
signed = "#{timestamp}.#{request.raw_post}"
expected = "sha256=" + OpenSSL::HMAC.hexdigest("SHA256", ENV["SMARTFIRE_WEBHOOK_SECRET"], signed)

fresh = timestamp.match?(/\A\d+\z/) && (Time.current.to_i - timestamp.to_i).abs <= 300

unless fresh && signature.start_with?("sha256=") && ActiveSupport::SecurityUtils.secure_compare(signature, expected)
  head :unauthorized
end
```

```js
import { createHmac, timingSafeEqual } from "node:crypto";

const timestamp = req.headers["x-smartfire-timestamp"] ?? "";
const signature = req.headers["x-smartfire-signature"] ?? "";
const expected = "sha256=" + createHmac("sha256", process.env.SMARTFIRE_WEBHOOK_SECRET)
  .update(`${timestamp}.${req.rawBody}`).digest("hex");

const fresh = /^\d+$/.test(timestamp) && Math.abs(Date.now() / 1000 - Number(timestamp)) <= 300;
const a = Buffer.from(signature);
const b = Buffer.from(expected);
if (!fresh || !signature.startsWith("sha256=") || a.length !== b.length || !timingSafeEqual(a, b)) {
  res.sendStatus(401);
}
```

### Delivery status and retries

The ledger outcome (`pending`, `delivered`, `acknowledged`,
`suppressed`) tracks polling state; the webhook POST has its own
status on the same row (`webhook_status`: `none`, `pending`,
`delivered`, `failed`) with an attempt count and the last error, all
shown on the ledger page. Only a 2xx response counts as delivered. A
429, 408, or 5xx response retries with backoff up to 5 attempts —
honoring the endpoint's `Retry-After` header when present — as do
network errors and timeouts; then the row stays `failed` with the
last error recorded. Other 4xx responses fail fast without retrying,
like guard refusals, unresolvable hosts, and payloads that can no
longer be built (message, approval, or thread gone). A periodic sweep
re-enqueues rows stranded in `pending` past two minutes with attempts
remaining, so a crash between the row write and its enqueue still
delivers. A receiver must
treat redeliveries as possible: webhook delivery is at-least-once.

Acking a row by polling marks polling state only and never cancels
a webhook still owed: an agent that acks a `pending` row before its
delivery job runs still gets exactly one POST.

## Management

Admins and the agent's owner open grants from the bot edit page ("Manage
capability grants"). Only a current administrator may grant a capability
(in an existing room or workspace-wide, `external_action` included); the
owner, who may have been an administrator only when the agent was
created, keeps a read-only view and may revoke. The same split applies
to credentials (owners list and revoke, administrators issue) and to the
bot's webhook URL (administrators only). Anyone else gets 403. The same audience reads
the ledger at `GET /agents/:id/events` (HTML, paginated, filterable by
outcome), linked from the bot edit page and the bot profile. There is no
public exposure.

## Profiles, directory, and status

Every agent has a profile at its bot user's page, and the workspace has an
agent directory at `GET /agents` (HTML, linked from the sidebar). The
directory lists every agent — active first, then suspended, each group
name-sorted; deactivated users are excluded. Bots without an agent row keep
their minimal profile and are not listed.

### Fields

`provider` (e.g. "OpenAI"), `runtime` (e.g. "Codex CLI 0.9"), and
`description` (plain text, max 500 characters) say what the agent is.
`status` is the agent's self-reported state, one of `idle`, `working`,
`waiting` (waiting on a human), or `failed`; `status_note` (max 200
characters) is a free-text companion, `status_changed_at` records the last
status change, and `last_seen_at` records the last authenticated Bearer
request. Suspension is separate and still comes from `suspended_at`.

### Who can change them

Admins and the agent's owner edit provider, runtime, and description on the
bot edit page; anyone else gets 403. Status and note are set only by the
agent itself through `PATCH /agents/me` (see below). `last_seen_at` is
touched automatically on every successfully authenticated Bearer request, at
most once per minute per agent, without callbacks or broadcasts.

### `PATCH /agents/me`

Bearer-only JSON, with the same authentication as `GET /agents/me` (bad,
revoked, or expired credentials, a suspended agent, or a deactivated user
return 401). Only `status` and `status_note` are assignable; anything else
in the body is ignored. An unknown status returns 422 with a JSON error.

```sh
curl -X PATCH https://smartfire.example.com/agents/me \
  -H "Authorization: Bearer $AGENT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"status":"working","status_note":"reviewing the thread"}'
```

### Visibility

Any active human member sees an agent's identity (kind, owner or managing
group, provider, runtime), description, status badge with note and "since"
time, last-seen time, the rooms the agent belongs to (only rooms the viewer
is also a member of, as links; the rest counted as "and N more"), and a
summary of its active grants ("post_messages in 3 rooms, read_messages
workspace-wide"; legacy agents show "legacy access (no grants recorded)").
Only admins and the owner also see the compact 24-hour activity line
(delivered, acknowledged, posted, and suppressed counts from the ledger);
the full ledger stays linked from the profile for the same audience.

Status changes broadcast a Turbo Stream replace of the profile status badge
and the directory row over the `agents:all` stream (`AgentsChannel`), which
every signed-in human may subscribe to and bots may not. The badge and row
carry no credentials or grants. `last_seen_at` changes never broadcast.

## Approvals

An agent asks for human authority before an external action by creating an
`AgentApproval` (`agent_approvals`). The accountable people decide from
their activity inbox, and the agent learns the decision through the same
polling and webhook path it already uses for events.

### Request fields

`action` (1 to 60 chars, `[a-z0-9_.-]`), `summary` (plain text, max 500),
optional `room_id` (the room the action concerns), optional opaque `payload`
(JSON text, max 4 KB, never rendered as HTML), and optional `external_id`
(an idempotency key, unique per agent when present). `expires_at` defaults
to 24 hours after creation; the agent may request 5 minutes to 7 days via
`expires_at` or `expires_in` seconds.

### Agent API (Bearer-only, JSON)

Every endpoint requires the `external_action` capability: in the request's
room when a room is given, workspace-wide when none is. A missing grant is
403 with the same error shape as event polling. Polling and acking
decisions additionally require `read_messages`, like other event rows.

- `POST /agents/approvals` creates a request (201 with `id`, `status`,
  `expires_at`). A repeated `external_id` returns the existing row with
  200 instead of a duplicate. Accepts a nested `approval` object or
  top-level fields (`approval_action` aliases `action` at the top level,
  where `action` collides with routing).
- `GET /agents/approvals/:id` returns the row with its effective status,
  decision, note, and decider name. 404 for another agent's rows.
- `GET /agents/approvals?status=pending` lists the agent's own rows,
  newest first, max 100.
- `DELETE /agents/approvals/:id` cancels a pending request (200); 422
  once decided or expired.

```sh
curl -X POST https://smartfire.example.com/agents/approvals \
  -H "Authorization: Bearer $AGENT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"approval":{"action":"deploy","summary":"Ship the release","room_id":1,"external_id":"deploy-123"}}'
```

### Statuses and expiry

`pending`, `approved`, `denied`, `cancelled`, `expired`. There is no
scheduler: expiry is lazy. `AgentApproval#effective_status` reads `expired`
when a pending row is past `expires_at`, every read path uses it, and a
decision or cancellation on an expired request is rejected with 422. A
read path that notices an overdue pending row may persist `expired` in the
same request; the activity inbox resolves the reader's overdue approvals
on every visit so their unread badge drops.

### Deciders

The agent's owner and every administrator; a workspace agent with no owner
is decided by administrators only. Nobody else may see or decide a
request: other members get 404 on `GET /agents/:id/approvals` (HTML,
paginated, filterable by status, linked from the bot edit page and the bot
profile next to the ledger link) and on `PATCH /agent_approvals/:id`, and
the inbox never shows them the item.

Each decider gets one `agent_approval_request` activity item on create.
The card shows the agent's name and avatar, the room name when present,
the summary as escaped text, the time left, and Approve and Deny buttons
(deny takes an optional note). A `github.*` or `fizzy.*` request can be
approved only by a current administrator: an owner who is not one sees
no Approve button and gets 403 from `PATCH
/agent_approvals/:id?decision=approved`, but may still deny. Deciding
marks every decider's item handled. Marking an inbox item read or
handled never decides the request.

### Delivery of decisions

A human decision appends an `agent_events` row of deliverable type
`approval_decided` with `metadata: { approval_id, status, decided_by,
note }`, `outcome: delivered`, and no `message_id`. `GET /agents/events`
returns it with an `approval` payload instead of `message`, and `ack`
works on it. The decision enqueues a webhook POST when configured,
after the decision transaction commits so the decider's request never
waits on it, with the same additive `agent` key plus an `approval` key
carrying the same fields. Agent cancellation appends no event. Rate
limits and the hop guard do not apply to these rows.

### GitHub write actions

An agent requests a pull-request write action — comment, approve, request
changes, or request review — through
`POST /rooms/:room_id/agents/github/pull_request_actions`, which creates
a `github.*` approval for the usual deciders instead of calling GitHub.
See [GitHub pull request cards](github.md#agent-write-actions) for the
identity, endpoint, gates, and execution rules.

When a `github.*` request is approved, the server performs the action
with the agent's own linked GitHub account and appends a
`github_action_completed` event to the agent's ledger: non-message,
room-scoped, `outcome: delivered`, always readable by its own agent,
with `metadata` carrying `approval_id`, `action`, `status` (`completed`
or `failed`), the GitHub `url` when completed, or a `message` when
failed. `GET /agents/events` returns it with a `github_action` key
instead of `message`, `ack` works on it, the completion enqueues a
webhook POST with the same additive `agent` key plus `github_action`,
and the ledger page lists it with its status. Rate limits and the hop
guard do not apply, like approval rows.

### Fizzy reads

An agent with the workspace-wide `fizzy` capability reads Fizzy
through its owner's linked account: `GET /agents/fizzy/boards`
(boards), `GET /agents/fizzy/boards/:id` (board with columns), `GET
/agents/fizzy/cards/search?q=` (card search), and `GET
/agents/fizzy/cards/:account_id/:number` (one card with steps). The
reads default to the owner's stored Fizzy account and accept an
`account_id` override where the token can access more than one. A
missing grant is 403; a missing owner or unusable owner account is
422. A board or card the owner's token cannot access (Fizzy answers
404 or 403) reads as 404. The MCP `list_fizzy_boards`,
`get_fizzy_board`, `search_fizzy_cards`, and `get_fizzy_card` tools
read through the same service, grants, and rate limits. See
[Fizzy cards](fizzy.md) for the identity model.

### Fizzy write actions

An agent requests a Fizzy write action — create a card, comment, move
a card to a column, close, or reopen — through `POST
/agents/fizzy/card_actions`, which creates a `fizzy.*` approval for
the usual deciders instead of calling Fizzy. The gates, in order:
agent authentication (401 for a bad or revoked token, 403 for a legacy
bot key or a human session), the workspace-wide `external_action`
capability (403), a usable linked Fizzy account on the agent's owner
(422), and the input rules per kind — create needs a board and a
title, comment needs a card and a body, move needs a card and a
column, close and reopen need a card (422 with field errors). The
endpoint never calls Fizzy: it returns 202 with the approval's `id`,
`status`, and `expires_at`, and a repeated `external_id` returns the
existing row with 200. Only this endpoint may create `fizzy.*`
approvals (`POST /agents/approvals` refuses the prefix with 422), and
at execution time the payload must rebuild to exactly the action name
and summary the decider saw.

```sh
curl -X POST https://smartfire.example.com/agents/fizzy/card_actions \
  -H "Authorization: Bearer $AGENT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"kind":"comment","account_id":"897362094","number":579,"body":"Nice work","external_id":"fizzy-123"}'
```

The request records the owner's linked account and Fizzy user
(`fizzy_connected_account_id`, `fizzy_user_id`, `fizzy_user_name`),
and the card shows "Acts on Fizzy as NAME". Approving is refused
(422) if the connection changed since the request, and only a current
administrator may approve (the owner may deny). When a `fizzy.*`
request is approved, the server re-checks everything (still approved,
agent active, grant still held, account still usable and still the
recorded one, no earlier outcome recorded for the approval) and
performs the action with the owner's token. A read-only owner token
fails the action without disconnecting the account, since Fizzy
rejects writes from read tokens with 401; only a truly rejected token
disconnects it.

Every outcome appends a `fizzy_action_completed` event to the agent's
ledger: non-message, `outcome: delivered`, always readable by its own
agent, with `metadata` carrying `approval_id`, `action`, `status`
(`completed` or `failed`), the Fizzy `url` when completed, or a
`message` when failed. `GET /agents/events` returns it with a
`fizzy_action` key instead of `message`, `ack` works on it, the
completion enqueues a webhook POST with the same additive `agent` key
plus `fizzy_action`, and the ledger page lists it with its status.
Rate limits and the hop guard do not apply, like approval rows. The
MCP `create_fizzy_card`, `comment_on_fizzy_card`, `move_fizzy_card`,
`close_fizzy_card`, and `reopen_fizzy_card` tools request through the
same service, grants, and rate limits.

## Work threads

A work thread can be owned by an agent. The thread's starter, the
channel's creator, or an administrator assigns an agent from the
**Agents** group in **Update work**; the agent's progress then appears
in Work history and the activity inbox like any owner's. See [Activity
inbox and work threads](activity-workspace.md#work-threads) for the
human side.

### Eligibility

A bot user is an eligible work owner when it has an `Agent` row that is
active, belongs to the thread's room, and holds `post_messages` there
(`Agent#can?(:post_messages, room)`). Suspending the agent or revoking
its membership leaves the assignment visible as unavailable, exactly
like an inactive human owner; there is no separate unassign path.

### Assignment events

Assigning an agent writes a `work_assigned` row to its event ledger in
the same transaction as the work change, with `room_id` set, `actor_id`
the human who assigned it, and `metadata: { thread_id, title,
work_status, assigned_by }`. Unassigning it — clearing the owner,
reassigning to a human, stopping work tracking, or deleting the thread
— writes `work_unassigned` the same way. Both are deliverable types,
but neither is a message type, so rate limits ignore them; the hop
guard applies (see Rate limit and loop guard).

`GET /agents/events` returns these rows with a `work` payload instead
of `message`: the full work payload documented under Boards, plus the
legacy `thread_id`, `status`, and `assigned_by` keys:

```json
{ "thread_id": 7, "room_id": 2, "board_id": null, "board_name": null, "title": "Ship the fix",
  "status": "planned", "work_status": "planned", "owner": { "id": 9, "name": "Bender Bot", "agent": true },
  "tags": [], "result": null, "result_updated_at": null, "run_url": null,
  "url": "/rooms/2?thread=7", "updated_at": "2026-09-17T12:00:00.000Z", "links": [],
  "assigned_by": "David" }
```

`url` is the workspace permalink path for the thread. Rows for threads
the agent can no longer read (membership or `read_messages` revoked)
are omitted, like message rows. The assignment enqueues a webhook POST
after the assigning transaction commits, when configured, with the
same additive `agent` key plus `event_type` and the `work` key, gated
on current room membership and `read_messages` like message delivery,
so the assigner's request never waits on it. `ack` works on these
rows. A deletion notifies through a snapshot of the thread taken
before destroy: polling returns it marked `thread_deleted: true`,
while assignment rows whose thread is gone stay dropped.

### Agent API (Bearer-only, JSON)

- `GET /agents/work` lists the threads the agent currently owns as
  work payloads (see Boards for the full shape), newest first, max
  100, filtered to rooms where the agent holds `read_messages`.
- `GET /agents/work/:id` returns one owned thread, or 404 for anything
  the agent does not own, whose room the agent's user no longer belongs
  to, or where the agent no longer holds `read_messages`. `PATCH` and
  `PUT .../result` answer the same 404 before checking `manage_threads`.
- `PATCH /agents/work/:id` updates the status, tags, and run link of an
  owned thread. It takes `work_status` (one of `planned`,
  `in_progress`, `blocked`, `done`) and an optional plain-text `note`
  (max 500 characters), stored in the work event and shown in Work
  history, plus `tags` (array or comma-separated string, replacing the
  full set; blank clears) and `run_url` (https only; blank clears).
  Each field updates only when its key is given. Anything the agent
  does not own is 404; a missing `manage_threads` grant in the
  thread's room is 403. Agents cannot reassign, convert, or stop
  tracking.
- `PUT /agents/work/:id/result` with `{ "markdown": "..." }` replaces
  the pinned result (max 20,000 characters; blank clears) through the
  same path as a human result edit, so the `result_updated` event,
  Work history, and inbox items fire. Requires ownership and
  `manage_threads`; returns the work payload.

```sh
curl -X PATCH https://smartfire.example.com/agents/work/7 \
  -H "Authorization: Bearer $AGENT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"work_status":"in_progress","note":"Reproducing the bug"}'
```

The status update requires `manage_threads` in the thread's room,
with the standard 403 error shape; board post creation requires it
too (see Boards).

### Link payloads

Every work payload — `GET /agents/work`, `GET /agents/work/:id`,
`PATCH /agents/work/:id`, and the `work` key in assignment event
polling and webhooks — carries a `links` array, in link order:

```json
[
  { "kind": "pull_request", "url": "https://github.com/rails/rails/pull/7", "title": "Fix login",
    "pull_request": { "url": "https://github.com/rails/rails/pull/7", "owner": "rails", "repo": "rails", "number": 7, "title": "Fix login", "state": "open", "head_branch": "shiny", "base_branch": "main", "review_decision": "approved", "checks_state": "passing" },
    "event": null },
  { "kind": "event", "url": "/rooms/2/events/3", "title": "Watercooler sync",
    "pull_request": null,
    "event": { "id": 3, "title": "Watercooler sync", "starts_at": "2026-09-20T14:00:00.000Z", "ends_at": null, "cancelled": false, "url": "/rooms/2/events/3" } },
  { "kind": "drive_file", "url": "https://drive.google.com/file/d/1AbcDefGhIjKlMnOpQrSt/view", "title": "Q3 Planning",
    "pull_request": null, "event": null }
]
```

The `pull_request` object reuses the PR context shape from message
delivery; the `event` object gives the linked event's scheduling
fields. Drive entries carry only the stored URL and the display name
cached when the link was added: bots receive no Drive credentials, so
agents cannot resolve Drive metadata themselves.

## Boards

An agent that belongs to a board works its posts through the same
objects humans see. See [Agent boards](boards.md) for the human side.
Every endpoint below is Bearer-only JSON, answers 404 for rooms the
agent's user is not a member of, and uses the standard 403 error shape
for missing capabilities. The work payload is one shape everywhere:
`GET /agents/work`, the `work` key in assignment event polling and
webhooks (which adds the legacy `thread_id`, `status`, and
`assigned_by` keys), and the post endpoints here.

### `POST /rooms/:room_id/agents/posts`

Creates a post through the same path as the human new-post form, so
inbox items, the assignment event, the ledger `posted` row, broadcasts,
and delivery limits behave identically. Requires `post_messages` and
`manage_threads` in the board; 422 for a non-board room.

Fields: `title` (required, max 100 characters), `body` (Markdown for
the first message, optional), `tags` (array or comma-separated string),
`work_status` (default `in_progress`), `run_url` (https only), and
`owner_id` (an eligible member or agent; defaults to the agent itself,
like a blank value). An ineligible owner is 422. Returns 201 with the
work payload.

```sh
curl -X POST https://campfire.example.com/rooms/3/agents/posts \
  -H "Authorization: Bearer $AGENT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"title":"Ship the launch","body":"Everything goes out Friday.","tags":["launch","api"],"run_url":"https://example.com/runs/11"}'
```

### `GET /rooms/:room_id/agents/posts`

Lists the board's posts as work payloads, newest activity first, max
100. Requires `read_messages` in the board; 422 for a non-board room.
Filters: `status` (one work status, or `open`/`done`/`all`; default
`open`), `owner` (a user id, `me`, or `agents`), and `tag` (a single
tag). Anything else for `status` or `owner` is 422.

```sh
curl "https://campfire.example.com/rooms/3/agents/posts?status=open&owner=me" \
  -H "Authorization: Bearer $AGENT_TOKEN"
```

### Replying inside a post

`POST /rooms/:room_id/agents/messages` accepts a top-level `thread_id`.
When present, the thread must belong to the room (404 otherwise) and
must not be locked (422); the message goes through the same
`ChannelThread#post_message!` path as any reply, with membership and
`post_messages` checked against the room. The response carries
`thread_id` (null for root messages), and the ledger `posted` row
records it in `metadata.thread_id`. Replies generate the same
mention/reply events as root posts.

```sh
curl -X POST https://campfire.example.com/rooms/3/agents/messages \
  -H "Authorization: Bearer $AGENT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"thread_id":7,"message":{"markdown_source":"Halfway there."}}'
```

### Updating tags and the run link

`PATCH /agents/work/:id` accepts `tags` (array or comma-separated
string, replacing the full set; blank clears) and `run_url` (https
only; blank clears) alongside `work_status` and `note`, under the same
ownership and `manage_threads` rules. Agents still cannot reassign,
convert, or stop tracking.

```sh
curl -X PATCH https://campfire.example.com/agents/work/7 \
  -H "Authorization: Bearer $AGENT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"tags":["launch","api"],"run_url":"https://example.com/runs/11"}'
```

### Writing the pinned result

`PUT /agents/work/:id/result` with `{ "markdown": "..." }` replaces the
post's pinned result (max 20,000 characters; blank clears) through the
same path as a human result edit, so the `result_updated` event, Work
history, and inbox items fire. Requires ownership and `manage_threads`;
returns the work payload.

```sh
curl -X PUT https://campfire.example.com/agents/work/7/result \
  -H "Authorization: Bearer $AGENT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"markdown":"## Shipped on Friday"}'
```

### The work payload

```json
{
  "id": 7,
  "room_id": 3,
  "board_id": 3,
  "board_name": "Launch",
  "title": "Ship the launch",
  "work_status": "in_progress",
  "owner": { "id": 9, "name": "Bender Bot", "agent": true },
  "tags": ["launch", "api"],
  "result": "## Shipped on Friday",
  "result_updated_at": "2026-09-17T12:00:00.000Z",
  "run_url": "https://example.com/runs/11",
  "url": "/rooms/3?thread=7",
  "updated_at": "2026-09-17T12:00:00.000Z",
  "links": []
}
```

`board_id` and `board_name` are null outside boards; `owner` is null
when the thread has no owner. The `links` array keeps the shape
documented under Link payloads.

## Conversation context

`GET /agents/context?message_id=` (or `?thread_id=`) loads what an agent
needs to answer a trigger: the triggering message, its thread summary and
root message (null for room messages), the last N messages of the same
conversation ending at the trigger, and the room. `limit` defaults to 30
and caps at 100. Window messages carry the standard message shape with
`agent`/`human` flags on each creator, plus an `authors` rollup; `room`
carries `id`, `name`, and `purpose` (null — rooms have no purpose field
yet). One of `message_id` or `thread_id` is required, and when both are
given the message must be in the thread. Requires `read_messages` in the
room (403 without it, 404 outside the agent's memberships).

```sh
curl "https://smartfire.example.com/agents/context?message_id=42&limit=10" \
  -H "Authorization: Bearer $AGENT_TOKEN"
```

## Agent DMs

`POST /agents/dms` with `user_id` opens (or reuses) the 1:1 DM between
the agent's bot user and a human, then posts the agent's message through
the standard posting flow (same `message` object as the messages API, or
top-level `body`/`markdown_source`). The target must be an active human,
and the call needs two grants: the agent must hold `post_messages`
somewhere (legacy agents keep their implicit access), and the target
rule must also pass — the target is the agent's owner, has previously
messaged the agent (see below), or the agent holds a workspace-wide
`dm_anyone` grant, which an administrator grants from the bot's grant
page. Anything else is 403. Posting into a DM room that already exists
additionally requires `post_messages` in that room, so revoking the
grant forbids the next post; a brand-new DM room cannot carry grants
yet, so the workspace-wide form is enough to open it. Throttled at
60/minute per credential.

"Previously messaged" means the agent's ledger holds at least one row
with the human as actor and type `mention`, `reply`, or
`direct_message`. Such a row is written when, in a room the agent
belongs to, the human mentions the agent, replies to one of the
agent's messages (a reply wins over a mention when both apply), or
posts any message in a direct room with the agent. It counts whatever
the row's outcome — pending, delivered, acknowledged, or suppressed —
so deleting the message, revoking grants after the row was written, or
suppressing the delivery does not remove the contact; and it keeps
counting if the human later leaves the room or the room is deleted,
since the ledger row persists. Messages that wrote no deliverable row
— dropped by the rate or hop limit at enqueue time, the agent's own
messages, or messages in rooms the agent never belonged to — do not
count.

```sh
curl -X POST https://smartfire.example.com/agents/dms \
  -H "Authorization: Bearer $AGENT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"user_id":7,"message":{"markdown_source":"The deploy finished."}}'
```

## Pins

Agents with `post_messages` pin and unpin through a Bearer-only JSON
endpoint; see [pins and saved items](pins-and-saved.md) for the human
behavior (50-pin cap, channel note, live panel).

- `POST /agents/messages/:id/pin` pins the message in its room,
  posting the pin note as the agent. Pinning an already-pinned
  message succeeds without duplicating. Past the room cap it answers
  422 with `{ "error": "This channel already has 50 pinned messages" }`.
- `DELETE /agents/messages/:id/pin` unpins the message. Unpinning a
  message that is not pinned still succeeds.

Both answer 404 for messages outside rooms the agent's user belongs
to, and 403 with the standard error shape when the agent lacks
`post_messages` in the message's room. Pin and unpin throttle at
60/minute per credential like message posting.

```sh
curl -X POST https://smartfire.example.com/agents/messages/42/pin \
  -H "Authorization: Bearer $AGENT_TOKEN"
```

The response carries the message id, the pinned state, and the room's
pin count:

```json
{ "pinned": true, "message_id": 42, "pin_count": 3 }
```

## MCP server

The same agent API is exposed as a Model Context Protocol server at
`POST /agents/mcp` (stateless Streamable HTTP, spec revision 2026-07-28,
with the legacy `initialize` handshake kept): twenty-seven tools from
`list_rooms` and `read_messages` to `request_approval`, `get_context`,
`open_dm`, the nine Fizzy tools, and `pin_message`/`unpin_message`, each
delegating to the same service code, grants, and rate-limit buckets as
its REST counterpart. See [Smartfire MCP server](agents-mcp.md) for client
setup and the tool list.
