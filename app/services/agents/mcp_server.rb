module Agents
  # Model Context Protocol tool server for Smartfire agents, backing
  # POST /agents/mcp. Implements the stateless Streamable HTTP profile of
  # MCP spec revision 2026-07-28 (verified at modelcontextprotocol.io),
  # with the legacy initialize handshake kept so 2025-03-26 through
  # 2025-11-25 clients can use it too; see Agents::McpController for the
  # JSON-RPC framing. Every tool delegates to the same domain services as
  # the REST agent API — never duplicated logic — and returns a
  # ServiceResult with a JSON-safe payload.
  #
  # The official `mcp` gem (v1.6.0, maintained by the MCP project) was
  # considered and deliberately not used: its Rails transport keeps
  # session state in memory (single-process only, and sessions no longer
  # exist in 2026-07-28), and per-request agent auth plus per-tool grant
  # and throttle checks map directly onto the controller stack instead.
  class McpServer
    SUPPORTED_VERSIONS = %w[ 2026-07-28 2025-11-25 2025-06-18 2025-03-26 ].freeze
    MODERN_VERSION = "2026-07-28"
    SERVER_NAME = "smartfire"
    SERVER_VERSION = "1.0.0"
    ACK_EVENTS_MAX_IDS = 100
    INSTRUCTIONS = "Smartfire workspace tools for an AI agent acting as itself. " \
      "Read rooms and threads before posting, keep replies in the thread that mentioned the agent, " \
      "request human approval before external actions, and prefer get_context over guessing at history."

    class InvalidParams < StandardError; end

    # throttle is nil or [ limit, controller_path, action_name ]: the REST
    # bucket the tool shares, so switching surfaces cannot dodge limits.
    Tool = Data.define(:name, :description, :input_schema, :throttle)

    TOOLS = [
      Tool.new(
        name: "list_rooms",
        description: "List the agent's rooms that carry at least one granted capability (legacy agents: every member room), with kind flags (board, direct).",
        input_schema: { "type" => "object", "properties" => {} },
        throttle: nil
      ),
      Tool.new(
        name: "read_messages",
        description: "Read one conversation: a room's root messages (room_id) or a thread's messages (thread_id). Pass exactly one. Supports before/after message-id cursors and limit (default 50, max 100). Returns messages oldest-first with paging flags.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "room_id" => { "type" => "integer", "description" => "Room to read root messages from." },
            "thread_id" => { "type" => "integer", "description" => "Thread to read messages from." },
            "before" => { "type" => "integer", "description" => "Return messages before this message id." },
            "after" => { "type" => "integer", "description" => "Return messages after this message id." },
            "limit" => { "type" => "integer", "description" => "Max messages (default 50, max 100)." }
          }
        },
        throttle: nil
      ),
      Tool.new(
        name: "post_message",
        description: "Post a message to a room, or a reply inside a thread (thread_id). One of body or markdown_source is required; reply_to_message_id targets a message in the same conversation. Requires post_messages.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "room_id" => { "type" => "integer", "description" => "Room to post in." },
            "thread_id" => { "type" => "integer", "description" => "Thread to reply inside." },
            "body" => { "type" => "string", "description" => "Plain-text body." },
            "markdown_source" => { "type" => "string", "description" => "Markdown body (preferred)." },
            "reply_to_message_id" => { "type" => "integer", "description" => "Message to reply to." },
            "reply_notify_author" => { "type" => "boolean", "description" => "Notify the reply target's author." },
            "client_message_id" => { "type" => "string", "description" => "Idempotency key: a retry returns the original message." },
            "drive_file_ids" => { "type" => "array", "items" => { "type" => "string" }, "description" => "Drive file ids to attach." }
          },
          "required" => [ "room_id" ]
        },
        throttle: [ 60, "agents/messages", "create" ]
      ),
      Tool.new(
        name: "react",
        description: "React to a message with an emoji or :shortcode:. Idempotent: repeating the same content returns the existing reaction. Requires react.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "message_id" => { "type" => "integer", "description" => "Message to react to." },
            "content" => { "type" => "string", "description" => "Emoji or :shortcode:." }
          },
          "required" => %w[ message_id content ]
        },
        throttle: nil
      ),
      Tool.new(
        name: "poll_events",
        description: "Poll the agent's event ledger (mentions, replies, DMs, approvals, work assignments). Pass since (default 0) and limit (default 50, max 100); the response carries next_since for the next poll. Requires read_messages.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "since" => { "type" => "integer", "description" => "Return events after this event id." },
            "limit" => { "type" => "integer", "description" => "Max events (default 50, max 100)." }
          }
        },
        throttle: [ 120, "agents/events", "index" ]
      ),
      Tool.new(
        name: "ack_events",
        description: "Acknowledge event rows by id (idempotent, max 100 ids per call). Returns a per-id outcome or error. Requires read_messages.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "event_ids" => { "type" => "array", "items" => { "type" => "integer" }, "description" => "Event ids to acknowledge." }
          },
          "required" => [ "event_ids" ]
        },
        throttle: [ 120, "agents/events", "ack" ]
      ),
      Tool.new(
        name: "list_board_posts",
        description: "List a board's posts as work payloads, newest activity first. status is one work status or open/done/all (default open); owner is a user id, me, or agents; tag filters by tag. Requires read_messages.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "room_id" => { "type" => "integer", "description" => "Board room id." },
            "status" => { "type" => "string", "description" => "planned, in_progress, blocked, done, open, or all." },
            "owner" => { "type" => "string", "description" => "A user id, me, or agents." },
            "tag" => { "type" => "string", "description" => "A single tag." }
          },
          "required" => [ "room_id" ]
        },
        throttle: nil
      ),
      Tool.new(
        name: "create_board_post",
        description: "Create a board post: title (required), body (Markdown for the first message), tags, work_status, run_url (https), owner_id (defaults to the agent). Requires post_messages and manage_threads.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "room_id" => { "type" => "integer", "description" => "Board room id." },
            "title" => { "type" => "string", "description" => "Post title (required)." },
            "body" => { "type" => "string", "description" => "Markdown for the first message." },
            "tags" => { "description" => "Array or comma-separated string.", "type" => [ "array", "string" ], "items" => { "type" => "string" } },
            "work_status" => { "type" => "string", "description" => "planned, in_progress, blocked, done (default in_progress)." },
            "run_url" => { "type" => "string", "description" => "https run link." },
            "owner_id" => { "type" => "integer", "description" => "Eligible owner (defaults to the agent)." }
          },
          "required" => %w[ room_id title ]
        },
        throttle: [ 30, "agents/posts", "create" ]
      ),
      Tool.new(
        name: "update_board_post",
        description: "Update a board post the agent owns: work_status, note (recorded in history), tags (replaces the set; blank clears), run_url (blank clears). Each field updates only when given. Requires manage_threads.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "post_id" => { "type" => "integer", "description" => "Board post (work thread) id." },
            "work_status" => { "type" => "string", "description" => "planned, in_progress, blocked, done." },
            "note" => { "type" => "string", "description" => "Plain-text progress note." },
            "tags" => { "description" => "Array or comma-separated string.", "type" => [ "array", "string" ], "items" => { "type" => "string" } },
            "run_url" => { "type" => "string", "description" => "https run link." }
          },
          "required" => [ "post_id" ]
        },
        throttle: nil
      ),
      Tool.new(
        name: "set_result",
        description: "Replace the pinned result of a board post the agent owns. markdown is required (blank clears). Requires manage_threads.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "post_id" => { "type" => "integer", "description" => "Board post (work thread) id." },
            "markdown" => { "type" => "string", "description" => "Result markdown (blank clears)." }
          },
          "required" => %w[ post_id markdown ]
        },
        throttle: nil
      ),
      Tool.new(
        name: "list_work",
        description: "List the work threads the agent owns, newest first.",
        input_schema: { "type" => "object", "properties" => {} },
        throttle: nil
      ),
      Tool.new(
        name: "update_work",
        description: "Update a work thread the agent owns: work_status, note, tags, run_url. Same rules as update_board_post. Requires manage_threads.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "work_id" => { "type" => "integer", "description" => "Work thread id." },
            "work_status" => { "type" => "string", "description" => "planned, in_progress, blocked, done." },
            "note" => { "type" => "string", "description" => "Plain-text progress note." },
            "tags" => { "description" => "Array or comma-separated string.", "type" => [ "array", "string" ], "items" => { "type" => "string" } },
            "run_url" => { "type" => "string", "description" => "https run link." }
          },
          "required" => [ "work_id" ]
        },
        throttle: nil
      ),
      Tool.new(
        name: "request_approval",
        description: "Ask a human for authority before an external action: action (lowercase word), summary, optional room_id, payload, external_id (idempotency key; a repeat returns the existing request), expires_at or expires_in seconds (5 minutes to 7 days, default 24 hours). Requires external_action.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "action" => { "type" => "string", "description" => "Action name, e.g. deploy." },
            "summary" => { "type" => "string", "description" => "Plain-text summary for the decider." },
            "room_id" => { "type" => "integer", "description" => "Room the action concerns." },
            "payload" => { "description" => "Opaque JSON (max 4 KB)." },
            "external_id" => { "type" => "string", "description" => "Idempotency key." },
            "expires_at" => { "type" => "string", "description" => "ISO8601 expiry." },
            "expires_in" => { "type" => "integer", "description" => "Expiry in seconds." }
          },
          "required" => %w[ action summary ]
        },
        throttle: [ 60, "agents/approvals", "create" ]
      ),
      Tool.new(
        name: "get_approval",
        description: "Read one of the agent's approval requests with its effective status and decision. Requires external_action.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "approval_id" => { "type" => "integer", "description" => "Approval request id." }
          },
          "required" => [ "approval_id" ]
        },
        throttle: [ 120, "agents/approvals", "show" ]
      ),
      Tool.new(
        name: "get_context",
        description: "Load conversation context for a triggering message (message_id) or a thread outright: the message, its thread summary and root message, the last N messages ending at the trigger (limit, default 30, max 100) with agent/human authors, and the room. Requires read_messages.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "message_id" => { "type" => "integer", "description" => "Triggering message id." },
            "thread_id" => { "type" => "integer", "description" => "Thread id (or the thread the message must be in)." },
            "limit" => { "type" => "integer", "description" => "Max context messages (default 30, max 100)." }
          }
        },
        throttle: [ 120, "agents/contexts", "show" ]
      ),
      Tool.new(
        name: "open_dm",
        description: "Open (or reuse) the 1:1 DM with a human and post the agent's message. Requires post_messages, plus the agent's owner, prior contact, or the dm_anyone capability.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "user_id" => { "type" => "integer", "description" => "Human user id." },
            "body" => { "type" => "string", "description" => "Plain-text body." },
            "markdown_source" => { "type" => "string", "description" => "Markdown body (preferred)." },
            "client_message_id" => { "type" => "string", "description" => "Idempotency key." },
            "drive_file_ids" => { "type" => "array", "items" => { "type" => "string" }, "description" => "Drive file ids to attach." }
          },
          "required" => [ "user_id" ]
        },
        throttle: [ 60, "agents/dms", "create" ]
      ),
      Tool.new(
        name: "list_fizzy_boards",
        description: "List the boards the agent owner's Fizzy account can access. Requires the workspace-wide fizzy capability.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "account_id" => { "type" => "string", "description" => "Fizzy account id (defaults to the owner's linked account)." }
          }
        },
        throttle: [ 120, "agents/fizzy/boards", "index" ]
      ),
      Tool.new(
        name: "get_fizzy_board",
        description: "Show one Fizzy board with its columns, so a column id can be resolved before requesting a move. Requires the workspace-wide fizzy capability.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "board_id" => { "type" => "string", "description" => "Fizzy board id." },
            "account_id" => { "type" => "string", "description" => "Fizzy account id (defaults to the owner's linked account)." }
          },
          "required" => [ "board_id" ]
        },
        throttle: [ 120, "agents/fizzy/boards", "show" ]
      ),
      Tool.new(
        name: "search_fizzy_cards",
        description: "Full-text search of Fizzy cards through the agent owner's Fizzy account. Requires the workspace-wide fizzy capability.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "q" => { "type" => "string", "description" => "Search query." },
            "account_id" => { "type" => "string", "description" => "Fizzy account id (defaults to the owner's linked account)." }
          },
          "required" => [ "q" ]
        },
        throttle: [ 120, "agents/fizzy/cards", "search" ]
      ),
      Tool.new(
        name: "get_fizzy_card",
        description: "Show one Fizzy card, including its steps. Requires the workspace-wide fizzy capability.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "account_id" => { "type" => "string", "description" => "Fizzy account id." },
            "number" => { "type" => "integer", "description" => "Card number." }
          },
          "required" => %w[ account_id number ]
        },
        throttle: [ 120, "agents/fizzy/cards", "show" ]
      ),
      Tool.new(
        name: "create_fizzy_card",
        description: "Ask a human to approve creating a Fizzy card in a board. Returns the approval's id, status, and expiry; the card is created only when approved. A repeated external_id returns the existing request. Requires external_action.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "board_id" => { "type" => "string", "description" => "Fizzy board id." },
            "title" => { "type" => "string", "description" => "Card title." },
            "description" => { "type" => "string", "description" => "Card description." },
            "account_id" => { "type" => "string", "description" => "Fizzy account id (defaults to the owner's linked account)." },
            "external_id" => { "type" => "string", "description" => "Idempotency key." }
          },
          "required" => %w[ board_id title ]
        },
        throttle: [ 60, "agents/fizzy/card_actions", "create" ]
      ),
      Tool.new(
        name: "comment_on_fizzy_card",
        description: "Ask a human to approve commenting on a Fizzy card. Returns the approval's id, status, and expiry; the comment is posted only when approved. A repeated external_id returns the existing request. Requires external_action.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "number" => { "type" => "integer", "description" => "Card number." },
            "body" => { "type" => "string", "description" => "Comment body." },
            "account_id" => { "type" => "string", "description" => "Fizzy account id (defaults to the owner's linked account)." },
            "external_id" => { "type" => "string", "description" => "Idempotency key." }
          },
          "required" => %w[ number body ]
        },
        throttle: [ 60, "agents/fizzy/card_actions", "create" ]
      ),
      Tool.new(
        name: "move_fizzy_card",
        description: "Ask a human to approve moving a Fizzy card to another column. Returns the approval's id, status, and expiry; the card moves only when approved. A repeated external_id returns the existing request. Requires external_action.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "number" => { "type" => "integer", "description" => "Card number." },
            "column_id" => { "type" => "string", "description" => "Destination column id." },
            "account_id" => { "type" => "string", "description" => "Fizzy account id (defaults to the owner's linked account)." },
            "external_id" => { "type" => "string", "description" => "Idempotency key." }
          },
          "required" => %w[ number column_id ]
        },
        throttle: [ 60, "agents/fizzy/card_actions", "create" ]
      ),
      Tool.new(
        name: "close_fizzy_card",
        description: "Ask a human to approve closing a Fizzy card. Returns the approval's id, status, and expiry; the card closes only when approved. A repeated external_id returns the existing request. Requires external_action.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "number" => { "type" => "integer", "description" => "Card number." },
            "account_id" => { "type" => "string", "description" => "Fizzy account id (defaults to the owner's linked account)." },
            "external_id" => { "type" => "string", "description" => "Idempotency key." }
          },
          "required" => [ "number" ]
        },
        throttle: [ 60, "agents/fizzy/card_actions", "create" ]
      ),
      Tool.new(
        name: "reopen_fizzy_card",
        description: "Ask a human to approve reopening a Fizzy card. Returns the approval's id, status, and expiry; the card reopens only when approved. A repeated external_id returns the existing request. Requires external_action.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "number" => { "type" => "integer", "description" => "Card number." },
            "account_id" => { "type" => "string", "description" => "Fizzy account id (defaults to the owner's linked account)." },
            "external_id" => { "type" => "string", "description" => "Idempotency key." }
          },
          "required" => [ "number" ]
        },
        throttle: [ 60, "agents/fizzy/card_actions", "create" ]
      ),
      Tool.new(
        name: "pin_message",
        description: "Pin a message in its room, posting the pin note as the agent. Idempotent: pinning an already-pinned message succeeds without duplicating. Requires post_messages.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "message_id" => { "type" => "integer", "description" => "Message to pin." }
          },
          "required" => [ "message_id" ]
        },
        throttle: [ 60, "agents/pins", "create" ]
      ),
      Tool.new(
        name: "unpin_message",
        description: "Unpin a message. Unpinning a message that is not pinned still succeeds. Requires post_messages.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "message_id" => { "type" => "integer", "description" => "Message to unpin." }
          },
          "required" => [ "message_id" ]
        },
        throttle: [ 60, "agents/pins", "destroy" ]
      ),
      Tool.new(
        name: "register_slash_command",
        description: "Register a custom slash command for a room (name without the leading slash, lowercase). Invoking it delivers a slash_command event to this agent with the raw arguments. Re-registering the agent's own name updates its description. Names are unique per room. Requires post_messages.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "room_id" => { "type" => "integer", "description" => "Room to register the command in." },
            "name" => { "type" => "string", "description" => "Command name without the leading slash." },
            "description" => { "type" => "string", "description" => "Short description shown in the command picker." }
          },
          "required" => %w[ room_id name ]
        },
        throttle: [ 60, "agents/slash_commands", "create" ]
      ),
      Tool.new(
        name: "unregister_slash_command",
        description: "Unregister one of the agent's own custom slash commands in a room. Requires post_messages.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "room_id" => { "type" => "integer", "description" => "Room the command is registered in." },
            "name" => { "type" => "string", "description" => "Command name without the leading slash." }
          },
          "required" => %w[ room_id name ]
        },
        throttle: [ 60, "agents/slash_commands", "destroy" ]
      ),
      Tool.new(
        name: "create_poll",
        description: "Post a message carrying a poll: question plus 2-10 options, single or multiple choice, optionally anonymous and with an ISO8601 close time. Requires post_messages.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "room_id" => { "type" => "integer", "description" => "Room to post the poll in." },
            "question" => { "type" => "string", "description" => "Poll question (the message text)." },
            "options" => { "type" => "array", "items" => { "type" => "string" }, "description" => "2-10 option labels." },
            "multiple" => { "type" => "boolean", "description" => "Allow voting for several options." },
            "anonymous" => { "type" => "boolean", "description" => "Show counts without voter names." },
            "closes_at" => { "type" => "string", "description" => "ISO8601 close time." }
          },
          "required" => %w[ room_id question options ]
        },
        throttle: [ 60, "agents/polls", "create" ]
      ),
      Tool.new(
        name: "get_poll",
        description: "Read a poll with live counts (voter names unless anonymous). Requires post_messages.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "room_id" => { "type" => "integer", "description" => "Room the poll was posted in." },
            "poll_id" => { "type" => "integer", "description" => "Poll id." }
          },
          "required" => %w[ room_id poll_id ]
        },
        throttle: [ 120, "agents/polls", "show" ]
      ),
      Tool.new(
        name: "handoff_work",
        description: "Hand a work thread the agent owns to another agent with a context package: summary (required), links (up to 10 URLs), open_questions (up to 10). Ownership transfers and the receiver gets a work_handed_off event. Requires manage_threads.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "work_id" => { "type" => "integer", "description" => "Work thread id." },
            "receiver_agent_id" => { "type" => "integer", "description" => "Receiving agent id." },
            "summary" => { "type" => "string", "description" => "Handoff summary (required)." },
            "links" => { "type" => "array", "items" => { "type" => "string" }, "description" => "Context links (up to 10)." },
            "open_questions" => { "type" => "array", "items" => { "type" => "string" }, "description" => "Open questions (up to 10)." }
          },
          "required" => %w[ work_id receiver_agent_id summary ]
        },
        throttle: [ 60, "agents/work", "handoff" ]
      ),
      # --- Agent streaming, working presence, and steps (w3/agents-stream).
      # Each delegates to the same service as its REST counterpart.
      Tool.new(
        name: "start_stream",
        description: "Start a streaming message in a room, or a reply inside a thread (thread_id). markdown_source may start blank and fill in with append_stream; reply_to_message_id targets a message in the same conversation. Requires post_messages.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "room_id" => { "type" => "integer", "description" => "Room to stream in." },
            "thread_id" => { "type" => "integer", "description" => "Thread to reply inside." },
            "markdown_source" => { "type" => "string", "description" => "Initial Markdown body (may be blank)." },
            "reply_to_message_id" => { "type" => "integer", "description" => "Message to reply to." },
            "reply_notify_author" => { "type" => "boolean", "description" => "Notify the reply target's author." },
            "client_message_id" => { "type" => "string", "description" => "Idempotency key: a retry returns the original message." }
          },
          "required" => [ "room_id" ]
        },
        throttle: [ 60, "agents/streaming_messages", "create" ]
      ),
      Tool.new(
        name: "append_stream",
        description: "Append text to one of the agent's streaming messages (append), or replace its whole body (markdown_source). Pass exactly one. Requires post_messages.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "message_id" => { "type" => "integer", "description" => "Streaming message id." },
            "append" => { "type" => "string", "description" => "Markdown text to append." },
            "markdown_source" => { "type" => "string", "description" => "Replacement Markdown body." }
          },
          "required" => [ "message_id" ]
        },
        throttle: [ 240, "agents/streaming_messages", "update" ]
      ),
      Tool.new(
        name: "finalize_stream",
        description: "End one of the agent's streaming messages, firing every deferred side effect exactly once. Idempotent. Requires post_messages.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "message_id" => { "type" => "integer", "description" => "Streaming message id." }
          },
          "required" => [ "message_id" ]
        },
        throttle: [ 60, "agents/streaming_messages", "finalize" ]
      ),
      Tool.new(
        name: "set_presence",
        description: "Set the agent's working presence ('Thinking…', 'Running tests…'), shown next to its name in the room member list. Blank clears; omitted leaves it alone. Expires after 5 minutes unless refreshed, and clears when a stream finalizes.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "text" => { "type" => "string", "description" => "Presence text, max 140 characters (blank clears)." }
          }
        },
        throttle: nil
      ),
      Tool.new(
        name: "add_step",
        description: "Attach a structured step to one of the agent's messages (message_id) or to a work thread it owns (thread_id): name, status (pending, running, done, failed), input/output summaries, duration in milliseconds. Message steps need post_messages, thread steps need manage_threads.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "message_id" => { "type" => "integer", "description" => "The agent's own message id." },
            "thread_id" => { "type" => "integer", "description" => "Owned work thread id." },
            "name" => { "type" => "string", "description" => "Step name (required)." },
            "status" => { "type" => "string", "description" => "pending, running, done, failed (default running)." },
            "input_summary" => { "type" => "string", "description" => "Short input summary." },
            "output_summary" => { "type" => "string", "description" => "Short output summary." },
            "duration_ms" => { "type" => "integer", "description" => "Duration in milliseconds." }
          },
          "required" => [ "name" ]
        },
        throttle: [ 60, "agents/steps", "create" ]
      ),
      Tool.new(
        name: "update_step",
        description: "Update one of the agent's steps: name, status, summaries, duration. Each field updates only when given. Same grants as add_step.",
        input_schema: {
          "type" => "object",
          "properties" => {
            "step_id" => { "type" => "integer", "description" => "Step id." },
            "name" => { "type" => "string", "description" => "Step name." },
            "status" => { "type" => "string", "description" => "pending, running, done, failed." },
            "input_summary" => { "type" => "string", "description" => "Short input summary." },
            "output_summary" => { "type" => "string", "description" => "Short output summary." },
            "duration_ms" => { "type" => "integer", "description" => "Duration in milliseconds." }
          },
          "required" => [ "step_id" ]
        },
        throttle: [ 60, "agents/steps", "update" ]
      )
    ].freeze

    def self.tools
      TOOLS
    end

    def self.tool_named(name)
      TOOLS.find { |tool| tool.name == name }
    end

    def initialize(agent:, presenter:, credential: nil)
      @agent = agent
      @presenter = presenter
      @credential = credential
    end

    # Dispatches one tools/call. Raises InvalidParams for an unknown tool
    # or unusable arguments (JSON-RPC -32602); returns a ServiceResult
    # otherwise, including for grant denials and rate limits, which the
    # controller renders as an isError tool result.
    def call_tool(name, arguments)
      handler = "tool_#{name}"
      unless name.is_a?(String) && respond_to?(handler, true)
        raise InvalidParams, "Unknown tool: #{name.inspect}"
      end
      unless arguments.nil? || arguments.is_a?(Hash)
        raise InvalidParams, "arguments must be an object"
      end

      send(handler, arguments || {})
    end

    private
      def tool_list_rooms(_args)
        Rooms.list(agent: @agent)
      end

      def tool_read_messages(args)
        result = Reading.read(
          agent: @agent, room_id: args["room_id"], thread_id: args["thread_id"],
          before: args["before"], after: args["after"], limit: args["limit"]
        )
        return result unless result.ok?

        page = result.payload
        messages = @presenter.caching_thread_payloads do
          page[:messages].map { |message| @presenter.message_payload(message) }
        end

        ServiceResult.ok(page.merge(messages: messages))
      end

      def tool_post_message(args)
        room_id = args["room_id"].presence or raise InvalidParams, "Missing required argument: room_id"
        room = @agent.user.rooms.find_by(id: room_id)
        return ServiceResult.fail("Room not found", status: :not_found) unless room
        unless @agent.can?(:post_messages, room)
          return ServiceResult.fail("Forbidden: agent lacks post_messages capability", status: :forbidden)
        end
        if args["body"].blank? && args["markdown_source"].blank?
          raise InvalidParams, "body or markdown_source is required"
        end

        result = Posting.post(
          agent: @agent, room: room, thread_id: args["thread_id"],
          attributes: args.slice("body", "markdown_source", "reply_to_message_id", "reply_notify_author", "client_message_id").to_h.symbolize_keys,
          drive_file_ids: mcp_drive_file_ids(args)
        )
        return result unless result.ok?

        message = result.payload
        ServiceResult.ok(@presenter.message_payload(message).merge(thread_id: message.thread_id), status: :created)
      rescue ActiveRecord::RecordNotFound
        ServiceResult.fail("Reply target not found", status: :not_found)
      end

      def tool_react(args)
        message_id = args["message_id"].presence or raise InvalidParams, "Missing required argument: message_id"
        content = args["content"].presence or raise InvalidParams, "Missing required argument: content"

        Reactions.react(agent: @agent, message_id: message_id, content: content)
      end

      def tool_poll_events(args)
        unless @agent.has_capability_anywhere?(:read_messages)
          return ServiceResult.fail("Forbidden: agent lacks read_messages capability", status: :forbidden)
        end

        page = EventPolling.poll(agent: @agent, since: args["since"], limit: args["limit"], presenter: @presenter)

        ServiceResult.ok(page)
      end

      def tool_ack_events(args)
        ids = args["event_ids"]
        unless ids.is_a?(Array) && ids.any?
          raise InvalidParams, "event_ids must be a non-empty array"
        end
        if ids.size > ACK_EVENTS_MAX_IDS
          raise InvalidParams, "event_ids must contain at most #{ACK_EVENTS_MAX_IDS} ids"
        end
        unless @agent.has_capability_anywhere?(:read_messages)
          return ServiceResult.fail("Forbidden: agent lacks read_messages capability", status: :forbidden)
        end

        results = ids.map do |id|
          result = EventPolling.ack(agent: @agent, id: id)
          result.ok? ? { id: id, outcome: result.payload[:outcome] } : { id: id, error: result.error }
        end

        ServiceResult.ok({ results: results })
      end

      def tool_list_board_posts(args)
        room_id = args["room_id"].presence or raise InvalidParams, "Missing required argument: room_id"
        room = @agent.user.rooms.find_by(id: room_id)
        return ServiceResult.fail("Room not found", status: :not_found) unless room
        unless @agent.can?(:read_messages, room)
          return ServiceResult.fail("Forbidden: agent lacks read_messages capability", status: :forbidden)
        end
        unless room.board?
          return ServiceResult.fail("Room is not a board", status: :unprocessable_entity)
        end

        result = BoardPosts.list(agent: @agent, room: room, status: args["status"], owner: args["owner"], tag: args["tag"])
        return result unless result.ok?

        ServiceResult.ok(result.payload.map { |thread| Agents::WorkPayload.for(thread, agent: @agent) })
      end

      def tool_create_board_post(args)
        room_id = args["room_id"].presence or raise InvalidParams, "Missing required argument: room_id"
        room = @agent.user.rooms.find_by(id: room_id)
        return ServiceResult.fail("Room not found", status: :not_found) unless room
        unless @agent.can?(:post_messages, room)
          return ServiceResult.fail("Forbidden: agent lacks post_messages capability", status: :forbidden)
        end
        unless @agent.can?(:manage_threads, room)
          return ServiceResult.fail("Forbidden: agent lacks manage_threads capability", status: :forbidden)
        end
        unless room.board?
          return ServiceResult.fail("Room is not a board", status: :unprocessable_entity)
        end

        result = BoardPosts.create(
          agent: @agent, room: room, title: args["title"], body: args["body"], tags: args["tags"],
          work_status: args["work_status"], run_url: args["run_url"], owner_id: args["owner_id"]
        )
        return result unless result.ok?

        ServiceResult.ok(Agents::WorkPayload.for(result.payload, agent: @agent), status: :created)
      end

      def tool_update_board_post(args)
        post_id = args["post_id"].presence or raise InvalidParams, "Missing required argument: post_id"

        result = WorkThreads.update(
          agent: @agent, id: post_id,
          work_status: mcp_work_field(args, "work_status"),
          note: args["note"],
          tags: mcp_work_field(args, "tags"),
          run_url: mcp_work_field(args, "run_url")
        )
        return result unless result.ok?

        ServiceResult.ok(Agents::WorkPayload.for(result.payload, agent: @agent))
      end

      def tool_set_result(args)
        post_id = args["post_id"].presence or raise InvalidParams, "Missing required argument: post_id"

        result = WorkThreads.set_result(
          agent: @agent, id: post_id,
          markdown: args["markdown"], markdown_given: args.key?("markdown")
        )
        return result unless result.ok?

        ServiceResult.ok(Agents::WorkPayload.for(result.payload, agent: @agent))
      end

      def tool_list_work(_args)
        result = WorkThreads.list(agent: @agent)
        return result unless result.ok?

        ServiceResult.ok(result.payload.map { |thread| Agents::WorkPayload.for(thread, agent: @agent) })
      end

      def tool_update_work(args)
        work_id = args["work_id"].presence or raise InvalidParams, "Missing required argument: work_id"

        result = WorkThreads.update(
          agent: @agent, id: work_id,
          work_status: mcp_work_field(args, "work_status"),
          note: args["note"],
          tags: mcp_work_field(args, "tags"),
          run_url: mcp_work_field(args, "run_url")
        )
        return result unless result.ok?

        ServiceResult.ok(Agents::WorkPayload.for(result.payload, agent: @agent))
      end

      def tool_request_approval(args)
        fields = args.slice("action", "summary", "room_id", "payload", "external_id", "expires_at", "expires_in").to_h

        Approvals.create(agent: @agent, fields: fields, credential: @credential)
      end

      def tool_get_approval(args)
        approval_id = args["approval_id"].presence or raise InvalidParams, "Missing required argument: approval_id"

        Approvals.show(agent: @agent, id: approval_id)
      end

      def tool_get_context(args)
        ContextBuilder.build(
          agent: @agent, message_id: args["message_id"], thread_id: args["thread_id"],
          limit: args["limit"], presenter: @presenter
        )
      end

      def tool_open_dm(args)
        user_id = args["user_id"].presence or raise InvalidParams, "Missing required argument: user_id"
        if args["body"].blank? && args["markdown_source"].blank?
          raise InvalidParams, "body or markdown_source is required"
        end

        result = DirectMessages.open_and_post(
          agent: @agent, user_id: user_id,
          attributes: args.slice("body", "markdown_source", "client_message_id").to_h.symbolize_keys,
          drive_file_ids: mcp_drive_file_ids(args)
        )
        return result unless result.ok?

        room = result.payload[:room]
        message = result.payload[:message]
        ServiceResult.ok(
          {
            room: { id: room.id, name: room.name, direct: true },
            message: @presenter.message_payload(message),
            thread_id: message.thread_id
          },
          status: :created
        )
      end

      def tool_list_fizzy_boards(args)
        FizzyReads.boards(agent: @agent, account_id: args["account_id"])
      end

      def tool_get_fizzy_board(args)
        board_id = args["board_id"].presence or raise InvalidParams, "Missing required argument: board_id"

        FizzyReads.board(agent: @agent, board_id: board_id, account_id: args["account_id"])
      end

      def tool_search_fizzy_cards(args)
        query = args["q"].presence or raise InvalidParams, "Missing required argument: q"

        FizzyReads.search_cards(agent: @agent, query: query, account_id: args["account_id"])
      end

      def tool_get_fizzy_card(args)
        account_id = args["account_id"].presence or raise InvalidParams, "Missing required argument: account_id"
        number = args["number"].presence or raise InvalidParams, "Missing required argument: number"

        FizzyReads.card(agent: @agent, account_id: account_id, number: number)
      end

      def tool_create_fizzy_card(args)
        board_id = args["board_id"].presence or raise InvalidParams, "Missing required argument: board_id"
        title = args["title"].presence or raise InvalidParams, "Missing required argument: title"

        FizzyCardActions.create(
          agent: @agent,
          fields: {
            "kind" => "create",
            "account_id" => args["account_id"],
            "board_id" => board_id,
            "title" => title,
            "description" => args["description"],
            "external_id" => args["external_id"]
          },
          credential: @credential
        )
      end

      def tool_comment_on_fizzy_card(args)
        number = args["number"].presence or raise InvalidParams, "Missing required argument: number"
        body = args["body"].presence or raise InvalidParams, "Missing required argument: body"

        FizzyCardActions.create(
          agent: @agent,
          fields: {
            "kind" => "comment",
            "account_id" => args["account_id"],
            "number" => number,
            "body" => body,
            "external_id" => args["external_id"]
          },
          credential: @credential
        )
      end

      def tool_move_fizzy_card(args)
        number = args["number"].presence or raise InvalidParams, "Missing required argument: number"
        column_id = args["column_id"].presence or raise InvalidParams, "Missing required argument: column_id"

        FizzyCardActions.create(
          agent: @agent,
          fields: {
            "kind" => "move",
            "account_id" => args["account_id"],
            "number" => number,
            "column_id" => column_id,
            "external_id" => args["external_id"]
          },
          credential: @credential
        )
      end

      def tool_close_fizzy_card(args)
        number = args["number"].presence or raise InvalidParams, "Missing required argument: number"

        FizzyCardActions.create(
          agent: @agent,
          fields: {
            "kind" => "close",
            "account_id" => args["account_id"],
            "number" => number,
            "external_id" => args["external_id"]
          },
          credential: @credential
        )
      end

      def tool_reopen_fizzy_card(args)
        number = args["number"].presence or raise InvalidParams, "Missing required argument: number"

        FizzyCardActions.create(
          agent: @agent,
          fields: {
            "kind" => "reopen",
            "account_id" => args["account_id"],
            "number" => number,
            "external_id" => args["external_id"]
          },
          credential: @credential
        )
      end

      def tool_pin_message(args)
        message_id = args["message_id"].presence or raise InvalidParams, "Missing required argument: message_id"

        Pins.pin(agent: @agent, message_id: message_id)
      end

      def tool_unpin_message(args)
        message_id = args["message_id"].presence or raise InvalidParams, "Missing required argument: message_id"

        Pins.unpin(agent: @agent, message_id: message_id)
      end

      def tool_register_slash_command(args)
        room_id = args["room_id"].presence or raise InvalidParams, "Missing required argument: room_id"
        name = args["name"].presence or raise InvalidParams, "Missing required argument: name"

        SlashCommands.register(agent: @agent, room_id: room_id, name: name, description: args["description"])
      end

      def tool_unregister_slash_command(args)
        room_id = args["room_id"].presence or raise InvalidParams, "Missing required argument: room_id"
        name = args["name"].presence or raise InvalidParams, "Missing required argument: name"

        SlashCommands.unregister(agent: @agent, room_id: room_id, name: name)
      end

      def tool_create_poll(args)
        room_id = args["room_id"].presence or raise InvalidParams, "Missing required argument: room_id"
        question = args["question"].presence or raise InvalidParams, "Missing required argument: question"
        options = args["options"].presence or raise InvalidParams, "Missing required argument: options"
        raise InvalidParams, "options must be an array" unless options.is_a?(Array)

        Polls.create(
          agent: @agent, room_id: room_id, question: question, options: options,
          multiple: args["multiple"], anonymous: args["anonymous"], closes_at: args["closes_at"]
        )
      end

      def tool_get_poll(args)
        room_id = args["room_id"].presence or raise InvalidParams, "Missing required argument: room_id"
        poll_id = args["poll_id"].presence or raise InvalidParams, "Missing required argument: poll_id"

        Polls.show(agent: @agent, room_id: room_id, poll_id: poll_id)
      end

      def tool_handoff_work(args)
        work_id = args["work_id"].presence or raise InvalidParams, "Missing required argument: work_id"
        receiver_agent_id = args["receiver_agent_id"].presence or raise InvalidParams, "Missing required argument: receiver_agent_id"
        summary = args["summary"].presence or raise InvalidParams, "Missing required argument: summary"

        result = WorkHandoffs.create(
          agent: @agent, id: work_id,
          receiver_agent_id: receiver_agent_id,
          summary: summary,
          links: args["links"],
          open_questions: args["open_questions"]
        )
        return result unless result.ok?

        ServiceResult.ok(
          Agents::WorkPayload.for(result.payload[:thread], agent: @agent)
            .merge(handoff: result.payload[:handoff].payload),
          status: :created
        )
      end

      # --- Agent streaming, working presence, and steps (w3/agents-stream).

      def tool_start_stream(args)
        room_id = args["room_id"].presence or raise InvalidParams, "Missing required argument: room_id"
        room = @agent.user.rooms.find_by(id: room_id)
        return ServiceResult.fail("Room not found", status: :not_found) unless room
        unless @agent.can?(:post_messages, room)
          return ServiceResult.fail("Forbidden: agent lacks post_messages capability", status: :forbidden)
        end

        result = Streaming.start(
          agent: @agent, room: room, thread_id: args["thread_id"],
          attributes: args.slice("markdown_source", "reply_to_message_id", "reply_notify_author", "client_message_id").to_h.symbolize_keys
        )
        return result unless result.ok?

        message = result.payload
        ServiceResult.ok(@presenter.message_payload(message).merge(thread_id: message.thread_id, streaming: message.streaming?), status: :created)
      rescue ActiveRecord::RecordNotFound
        ServiceResult.fail("Reply target not found", status: :not_found)
      end

      def tool_append_stream(args)
        message_id = args["message_id"].presence or raise InvalidParams, "Missing required argument: message_id"

        result = Streaming.update(
          agent: @agent, id: message_id,
          append: args["append"], markdown_source: args["markdown_source"]
        )
        return result unless result.ok?

        message = result.payload
        ServiceResult.ok(@presenter.message_payload(message).merge(thread_id: message.thread_id, streaming: message.streaming?))
      end

      def tool_finalize_stream(args)
        message_id = args["message_id"].presence or raise InvalidParams, "Missing required argument: message_id"

        result = Streaming.finalize(agent: @agent, id: message_id)
        return result unless result.ok?

        message = result.payload
        ServiceResult.ok(@presenter.message_payload(message).merge(thread_id: message.thread_id, streaming: message.streaming?))
      end

      def tool_set_presence(args)
        # Like PATCH /agents/me, an omitted text leaves presence alone;
        # only an explicit blank clears it.
        unless args.key?("text")
          return ServiceResult.ok({ working_presence: @agent.working_presence_text })
        end

        WorkingPresence.set(agent: @agent, text: args["text"])
      end

      def tool_add_step(args)
        args["name"].presence or raise InvalidParams, "Missing required argument: name"

        result = Steps.create(
          agent: @agent,
          fields: args.slice("message_id", "thread_id", "name", "status", "input_summary", "output_summary", "duration_ms").to_h
        )
        return result unless result.ok?

        ServiceResult.ok(Steps.step_payload(result.payload), status: :created)
      end

      def tool_update_step(args)
        step_id = args["step_id"].presence or raise InvalidParams, "Missing required argument: step_id"

        result = Steps.update(
          agent: @agent, id: step_id,
          fields: args.slice("name", "status", "input_summary", "output_summary", "duration_ms").to_h
        )
        return result unless result.ok?

        ServiceResult.ok(Steps.step_payload(result.payload))
      end

      # Reads an updatable work field, returning the unset sentinel when
      # the key is absent so the model can tell it from an explicit blank.
      def mcp_work_field(args, key)
        args.key?(key) ? args[key] : ChannelThread::UNSET_WORK_VALUE
      end

      # :absent when the caller sent no Drive key, nil when the key held
      # no usable array (which fails validation), otherwise the id array.
      def mcp_drive_file_ids(args)
        return :absent unless args.key?("drive_file_ids")

        raw = args["drive_file_ids"]
        return nil unless raw.is_a?(Array)

        raw.map { |id| id.to_s.strip }.reject(&:blank?).uniq
      end
  end
end
