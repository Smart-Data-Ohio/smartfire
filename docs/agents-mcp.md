# Smartfire MCP server

Smartfire exposes its agent API as a [Model Context Protocol](https://modelcontextprotocol.io)
server, so Claude, Cursor, Codex, and similar clients can use the workspace as
a tool. The endpoint is `POST /agents/mcp`. Members can connect a client by URL
and sign in with OAuth, or use an agent token as with the [REST agent API](agents.md).
Every tool uses the same service code, capability grants, and rate limits as its
REST counterpart. OAuth connections authenticate only at the MCP endpoint.

## Protocol

Stateless [Streamable HTTP](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http),
spec revision **2026-07-28**: one JSON-RPC request or notification per POST,
answered with a single JSON object. The server keeps no sessions and streams
nothing, which the spec permits. Legacy clients (`2025-03-26` through
`2025-11-25`) handshake with `initialize` and then send unversioned requests;
`server/discover` and `ping` are also implemented. Unknown RPC methods answer
404, and GET/DELETE answer 405.

Tool results carry both `structuredContent` (the machine-readable payload)
and a `text` part with the same payload serialized as JSON. A denied or
failed tool call is still HTTP 200 with a JSON-RPC result: `isError: true`,
the reason as text (the same message the REST endpoint renders), and
`{ error, status }` structured. Budget denials add the REST 429's
`cap`, `limit`, and `retry_after` fields. Malformed calls answer
JSON-RPC errors instead (`-32602` for an unknown tool or missing
argument, `-32020` for a header mismatch, `-32022` for an unsupported
protocol version).

Unparseable bodies answer HTTP 400 with `-32700`, and malformed
envelopes — including batch arrays, which the spec forbids — answer HTTP
400 with `-32600`. Unknown RPC methods stay 404 with `-32601`, as the
spec requires.

Rate-limited tools answer `isError` with `rate_limited` and a `retry_after`
in seconds (plus a `Retry-After` header). Buckets are shared with the REST
endpoints, per credential per minute:

| Tool | Shared bucket | Limit/min |
| ---- | ------------- | --------- |
| `poll_events`, `ack_events` | event polling | 120 |
| `get_approval` | approval reads | 120 |
| `get_context` | context reads | 120 |
| `list_fizzy_boards`, `get_fizzy_board`, `search_fizzy_cards`, `get_fizzy_card` | Fizzy board and card reads | 120 |
| `get_poll` | poll reads | 120 |
| `post_message`, `request_approval`, `open_dm` | posting / approvals / DMs | 60 |
| `create_fizzy_card`, `comment_on_fizzy_card`, `move_fizzy_card`, `close_fizzy_card`, `reopen_fizzy_card` | Fizzy card actions | 60 |
| `pin_message`, `unpin_message` | pinning / unpinning | 60 |
| `register_slash_command`, `unregister_slash_command` | command registration | 60 |
| `create_poll` | poll creation | 60 |
| `handoff_work` | work handoffs | 60 |
| `start_stream`, `finalize_stream` | streaming messages | 60 |
| `append_stream` | streaming updates | 240 |
| `add_step`, `update_step` | agent steps | 60 |
| `set_presence` | working presence | none (like `PATCH /agents/me`) |
| `create_board_post` | board post creation | 30 |

The remaining tools have no throttle, like their REST counterparts.
`ack_events` charges its bucket once per id — matching REST's one-id-per-ack
accounting — and rejects batches over 100 ids with `-32602`.

On top of the per-tool buckets, every POST counts against a coarse
per-credential cap of 600 requests/minute across all methods
(`initialize` and `tools/list` included, since they have no tool
bucket). Past it the endpoint answers HTTP 429 with a `Retry-After`
header and `{ "error": "rate_limited" }`, like the REST endpoints.

## Connect by URL

In Smartfire, open **Settings → Agents → Connect an agent** and copy the MCP
URL, such as `https://chat.example.com/agents/mcp`. Your administrator must
configure a public HTTPS address and enable OAuth. Cloud clients must be able
to reach that address from their servers. A client's plan or workspace policy
may limit custom connectors.

Add the URL in your client and choose OAuth authentication:

| Client | Setup |
| --- | --- |
| [ChatGPT](https://developers.openai.com/api/docs/guides/custom-mcp-server) | On the web, open **Plugins**, select **+ → Add custom MCP server**, and enter the Server URL. Choose OAuth and CIMD, then create and install the plugin. Sign in when prompted. |
| [Claude](https://support.claude.com/en/articles/11175166-get-started-with-custom-connectors-using-remote-mcp) | Open **Customize → Connectors → Add custom connector**, enter the URL, and choose **Sign in now**. Use Claude's published identity or automatic registration. A team owner may need to add the connector first. |
| [Cursor](https://cursor.com/docs/mcp) | Add the URL to `~/.cursor/mcp.json` as below. Enable the server in **Customize** and complete the OAuth sign-in prompt. |
| [Grok](https://docs.x.ai/grok/connectors) | Open [Connectors](https://grok.com/connectors), choose **New Connector → Custom**, enter the URL, and complete authentication. |
| [MCP Inspector](https://github.com/modelcontextprotocol/inspector) | Run `npx @modelcontextprotocol/inspector`. Select Streamable HTTP, enter the URL, and use the OAuth flow in Auth settings before connecting. |

Cursor's URL-only configuration lets it discover OAuth and register its client:

```json
{
	"mcpServers": {
		"smartfire": { "url": "https://chat.example.com/agents/mcp" }
	}
}
```

### Review consent

Smartfire asks you to sign in, including your second factor if enrolled. The
consent page shows the client app's name, its return hostname, and an unverified
app warning or a badge for an app on your computer. Choose a new personal agent
or one you own, then select rooms and abilities. For an existing agent, the page
shows which rooms and abilities change. These changes affect its other
connections too.

Personal agents can read and post messages, react, and manage threads in the
rooms you grant. They cannot exceed your own live access. Confirm your password
before approval. **Deny** returns to the client without creating a connection.

### Manage connections

Under **Settings → Agents**, each agent lists its connections with the client
name, connection date, expiry, and last use. **Revoke** stops that connection's
tokens immediately. **Reconnect** shows the URL and instructions for starting
sign-in again from the client. Choose the existing agent on the next consent
page. Deactivating an agent stops all its connections.

Administrators can open **Admin → Bots → Connected apps** and **Block** an app.
Blocking stops all connections through that client and prevents new approvals.
Unblocking lets unexpired, unrevoked connections work again.

An OAuth connection lasts at most **180 days**. After that, reconnect and review
consent again. Access tokens last one hour. Refresh tokens rotate on each use
and expire after 30 idle days. Neither can outlive the connection. Separately,
static agent tokens default to **90 days**, with a "never" option when issued.

> [!WARNING]
> Clients that refresh in parallel can reuse the same refresh token. Smartfire
> treats reuse as a breach and revokes the entire connection, including tokens
> the first refresh issued. The card shows "Disconnected" with "Disconnected
> because an old sign-in token was reused. Reconnect to continue."
> Reconnect from the client and make it serialize refresh requests.

## Token client setup

Under **Settings → Agents**, create or select your personal agent and issue a
token after password confirmation. Copy the secret, which is shown once.
Grant the rooms and abilities its tools need, then point the client at the
endpoint with the token in an `Authorization` header. Administrators manage
workspace-agent tokens on the bot page.

Claude Code:

```sh
claude mcp add --transport http smartfire https://smartfire.example.com/agents/mcp \
  --header "Authorization: Bearer $AGENT_TOKEN"
```

Generic JSON config (Cursor, Codex, and other MCP clients accept a variant
of this; check the client's docs for the exact key names):

```json
{
  "mcpServers": {
    "smartfire": {
      "url": "https://smartfire.example.com/agents/mcp",
      "headers": { "Authorization": "Bearer $AGENT_TOKEN" }
    }
  }
}
```

A quick smoke test over HTTP:

```sh
curl https://smartfire.example.com/agents/mcp \
  -H "Authorization: Bearer $AGENT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}'
```

## Tools

| Tool | Capability | REST equivalent |
| ---- | ---------- | --------------- |
| `list_rooms` | member rooms with a granted capability (legacy: all) | — |
| `read_messages` | `read_messages` in the room | bot message listing |
| `post_message` | `post_messages` in the room | `POST /rooms/:id/agents/messages` |
| `react` | `react` in the room | bot boosts |
| `poll_events`, `ack_events` | `read_messages` | `GET /agents/events`, ack |
| `list_board_posts` | `read_messages` in the board | `GET /rooms/:id/agents/posts` |
| `create_board_post` | `post_messages` + `manage_threads` | `POST /rooms/:id/agents/posts` |
| `update_board_post`, `set_result` | ownership + `manage_threads` | `PATCH /agents/work/:id`, result |
| `list_work`, `update_work` | ownership (+ `manage_threads` for writes) | `GET/PATCH /agents/work` |
| `request_approval`, `get_approval` | `external_action` | `/agents/approvals` |
| `get_context` | `read_messages` | `GET /agents/context` |
| `open_dm` | `post_messages` + owner, prior contact, or `dm_anyone` | `POST /agents/dms` |
| `list_fizzy_boards`, `get_fizzy_board`, `search_fizzy_cards`, `get_fizzy_card` | workspace-wide `fizzy` | `GET /agents/fizzy/...` |
| `create_fizzy_card`, `comment_on_fizzy_card`, `move_fizzy_card`, `close_fizzy_card`, `reopen_fizzy_card` | workspace-wide `external_action` | `POST /agents/fizzy/card_actions` |
| `pin_message`, `unpin_message` | `post_messages` in the room | `POST`/`DELETE /agents/messages/:id/pin` |
| `register_slash_command`, `unregister_slash_command` | `post_messages` in the room | `POST`/`DELETE /rooms/:id/agents/slash_commands` |
| `create_poll`, `get_poll` | `post_messages` in the room | `POST`/`GET /rooms/:id/agents/polls` |
| `handoff_work` | ownership + `manage_threads` | `POST /agents/work/:id/handoff` |
| `start_stream`, `append_stream`, `finalize_stream` | `post_messages` in the room | streaming messages endpoints |
| `set_presence` | the agent itself | `PATCH /agents/me` (`working_presence`) |
| `add_step`, `update_step` | `post_messages` (message steps) / `manage_threads` (thread steps) | `POST`/`PATCH /agents/steps` |

`tools/list` returns the everyday-chat tools for personal agents. It excludes
approvals, Fizzy actions, slash-command registration, and work handoffs.
Personal agents can DM only their owner. Workspace agents receive the full
tool set. Every tool still checks permissions when called.

"Prior contact" for `open_dm` means the agent's ledger holds a
`mention`, `reply`, or `direct_message` row with the human as actor —
written when the human mentions the agent, replies to the agent, or
posts in a direct room with the agent. It counts whatever the row's
outcome, and survives the human leaving or the room being deleted. See
[Agent DMs](agents.md#agent-dms) for the exact rule.

The Fizzy tools read through the agent owner's linked Fizzy account
and write only through human approval: each write tool creates a
`fizzy.*` approval and returns its id, status, and expiry, exactly
like `POST /agents/fizzy/card_actions`. Completions arrive through
`poll_events` with a `fizzy_action` payload. See
[Fizzy reads](agents.md#fizzy-reads) and
[Fizzy write actions](agents.md#fizzy-write-actions) for the gates.
