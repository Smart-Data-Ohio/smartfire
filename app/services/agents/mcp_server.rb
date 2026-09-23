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

      def tool_pin_message(args)
        message_id = args["message_id"].presence or raise InvalidParams, "Missing required argument: message_id"

        Pins.pin(agent: @agent, message_id: message_id)
      end

      def tool_unpin_message(args)
        message_id = args["message_id"].presence or raise InvalidParams, "Missing required argument: message_id"

        Pins.unpin(agent: @agent, message_id: message_id)
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
