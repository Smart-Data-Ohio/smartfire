# WS15e services for WS11 adapters (explicit seams)

Rails pin: d7c7de92. WS11 owns registration, authentication, throttling, generic MCP envelopes,
AgentApproval human decisions and event-webhook delivery. This document adds no live route.
The WS15e domain services are complete and native tested; their callers must still establish
Current.agent/current credential through WS11's real authentication layer.

## Authenticated read adapters

REST requires JSON, a Bearer agent credential (browser cookies/bot keys never substitute),
active agent/user and a workspace-wide `fizzy` grant. Read throttle: 120. Return
`Cache-Control: no-store`. Services recheck agent/grant/owner/link state; never supply a token
from the request. Resolve the service base through `fizzy::client::api_base_url()` and pass
`Network::system()` for production.

Call `integrations::fizzy::agent_reads::read(app, network, base, authenticated_agent_id, operation)`:

| Rails adapter | Operation and raw values |
| --- | --- |
| GET /agents/fizzy/boards | `Read::Boards { account: params["account_id"] }` |
| GET /agents/fizzy/boards/:id | `Read::Board { account: params["account_id"], board: params["id"] }` |
| GET /agents/fizzy/cards/search | `Read::Search { account: params["account_id"], query: params["q"] }` |
| GET /agents/fizzy/cards/:account_id/:number | `Read::Card { account: params["account_id"], number: params["number"] }` |

Missing raw values are JSON null. Keep numeric values and strings intact so Ruby coercion,
ID checks and default owner-account selection happen in the domain. REST returns
`ReadResult.status` and `ReadResult.body()`. MCP tools call the same operation and translate
that result into WS11's existing ServiceResult/MCP envelope; service errors are not HTTP 200
successes. Reuse the Rails MCP tool names/argument schema, not new aliases.

## Write-request adapter

POST /agents/fizzy/card_actions requires the authenticated agent's workspace-wide
`external_action` grant; write throttle: 60; JSON and no-store. It requests approval only.
Pass these raw fields to `agent_requests::create` on one writer transaction:
`account_id`, `kind`, `board_id`, `number`, `column_id`, `title`, `description`, `body`,
`external_id`. Pass the authenticated credential's row id separately and the request's actual
Jiff TimeZone (not UTC unless the request is UTC). Use `ArEncryption` from app secrets.

The service owns connected owner credentials, replay/expiry, action validation, daily budgets,
approval snapshot and activity-inbox writes. Return its status/body. It performs no Fizzy HTTP.
MCP write tools map their schema into these same raw fields and call this service.

## Human approval, execution and sweep

After WS11 authenticates/authorizes the human and writes an approved decision, call
`agent_job::enqueue_approved(tx, approval_id)` **in that same transaction**. The hook emits
Fizzy::PerformAgentActionJob; the app sink persists the durable row before commit. Do not
call `perform` inline or enqueue a second post-commit job. The registered job uses the shared
`action_claims::FIZZY` and `rewrite_running`, one external attempt and legacy/indexed claim
identity. Never parse stored action numbers through f64: use `Action::from_stored`.

`action_claims` already schedules the 30-second stale-claim sweep. Preserve its status-guarded
rewrite/audit and webhook events; WS11 registers the real `AgentEventWebhookJob` handler and
delivery state. The current WS15e job descriptor is an explicit handler seam, not a webhook
implementation.

## Streaming finalization and imports

WS11 calls `Message::sync_external_references(tx, true)` only after its actual finalization
policy permits reference fanout. Quiet/suspended streams must not claim or enqueue fetches.
WS16 uses the existing `enqueue=false` reference/backfill mode in its quiet-import transaction.
Do not remove those flags or infer authorization merely because a message row exists.

## Acceptance still owned by WS11

Authenticated REST/MCP/controller/approval/webhook cases remain individually deferred in the
WS15e report. Native domain tests are not credited as adapter authentication tests. In
particular retain forged-token, room-only grant, suspension/deactivation, owner-only token,
replay, no-write-before-approval and no-network-on-denial assertions when wiring these callers.
