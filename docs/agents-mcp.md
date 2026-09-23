# Smartfire MCP server

Smartfire exposes its agent API as a [Model Context Protocol](https://modelcontextprotocol.io)
server, so Claude, Cursor, Codex, and similar clients can use the workspace as
a tool. The endpoint is `POST /agents/mcp`, authenticated with the same agent
token as the [REST agent API](agents.md), and every tool delegates to the same
service code — and the same capability grants and rate limits — as its REST
counterpart.

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
`{ error, status }` structured. Malformed calls answer JSON-RPC errors
instead (`-32602` for an unknown tool or missing argument, `-32020` for a
header mismatch, `-32022` for an unsupported protocol version).

Rate-limited tools answer `isError` with `rate_limited` and a `retry_after`
in seconds (plus a `Retry-After` header). Buckets are shared with the REST
endpoints, per credential per minute:

| Tool | Shared bucket | Limit/min |
| ---- | ------------- | --------- |
| `poll_events`, `ack_events` | event polling | 120 |
| `get_approval` | approval reads | 120 |
| `get_context` | context reads | 120 |
| `post_message`, `request_approval`, `open_dm` | posting / approvals / DMs | 60 |
| `create_board_post` | board post creation | 30 |

The remaining tools have no throttle, like their REST counterparts.

## Client setup

Create an agent token on the bot page (shown once), grant the agent the
capabilities its tools need, and point the client at the endpoint with the
token in an `Authorization` header.

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
| `list_rooms` | — (own memberships) | — |
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
| `open_dm` | owner, prior contact, or `dm_anyone` | `POST /agents/dms` |

`tools/list` always returns the full set; per-tool enforcement happens at
call time, so a client can show every tool and let denials explain which
grant is missing.
