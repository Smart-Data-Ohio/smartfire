class Agents::McpController < ApplicationController
  include AgentApiThrottle

  allow_agent_access only: %i[ create method_not_allowed ]

  # Same agent-only posture as the other agent endpoints: a session-cookie
  # request that trips forgery protection gets the 403 its missing token
  # deserves.
  rescue_from ActionController::InvalidAuthenticityToken, with: :reject_session_request
  rescue_from ActionDispatch::Http::Parameters::ParseError, with: :render_parse_error

  before_action :ensure_agent_token, only: %i[ create method_not_allowed ]

  ERROR_PARSE = -32700
  ERROR_INVALID_REQUEST = -32600
  ERROR_METHOD_NOT_FOUND = -32601
  ERROR_INVALID_PARAMS = -32602
  ERROR_INTERNAL = -32603
  ERROR_HEADER_MISMATCH = -32020
  ERROR_UNSUPPORTED_VERSION = -32022

  BASE64_HEADER_SENTINEL = /\A=\?base64\?(.+)\?=\z/

  # GET/DELETE on the MCP endpoint: a modern server answers old clients
  # with 405 (spec 2026-07-28, backward compatibility). Real traffic is
  # POST-only; the server keeps no sessions and streams nothing, which
  # the spec permits for a server with nothing to stream.
  def method_not_allowed
    render json: rpc_error(nil, ERROR_INVALID_REQUEST, "Only POST is supported on the MCP endpoint"),
      status: :method_not_allowed
  end

  # POST /agents/mcp (agent-token-only, JSON). One JSON-RPC request or
  # notification per POST, answered with a single JSON object. Protocol
  # version 2026-07-28 is negotiated per request (MCP-Protocol-Version
  # header plus _meta); legacy clients handshake with initialize and then
  # send unversioned requests, which read as 2025-03-26.
  def create
    no_store_response!

    if forged_origin?
      render json: rpc_error(nil, ERROR_INVALID_REQUEST, "Invalid origin"), status: :forbidden
      return
    end

    begin
      envelope = JSON.parse(request.raw_post)
    rescue JSON::ParserError
      render json: rpc_error(nil, ERROR_PARSE, "Parse error"), status: :bad_request
      return
    end

    unless envelope.is_a?(Hash) && envelope["jsonrpc"] == "2.0" && envelope["method"].is_a?(String)
      id = envelope.is_a?(Hash) ? envelope["id"] : nil
      render json: rpc_error(id, ERROR_INVALID_REQUEST, "Invalid request")
      return
    end

    # A notification carries no id and wants no answer.
    unless envelope.key?("id")
      head :accepted
      return
    end

    id = envelope["id"]
    method = envelope["method"]
    params = envelope["params"].is_a?(Hash) ? envelope["params"] : {}

    version = resolve_protocol_version(id, method, params)
    return if performed?

    modern = version == Agents::McpServer::MODERN_VERSION
    if modern && (header_error = validate_modern_headers(method, params))
      render json: rpc_error(id, ERROR_HEADER_MISMATCH, header_error), status: :bad_request
      return
    end

    dispatch_method(id, method, params, version, modern)
  end

  private
    def ensure_agent_token
      reject_session_request unless authenticated_by.agent_token? && Current.agent
    end

    def reject_session_request
      render json: { error: "Forbidden: #{"Bearer"} agent token required" }, status: :forbidden
    end

    # Rails parses JSON params before the action runs, so garbage with a
    # JSON content type never reaches the envelope parser below; answer it
    # the same way.
    def render_parse_error
      render json: rpc_error(nil, ERROR_PARSE, "Parse error"), status: :bad_request
    end

    # The Origin header, when present, must match this host; anything else
    # answers 403 without touching the protocol (DNS rebinding guard).
    def forged_origin?
      origin = request.headers["Origin"].presence
      return false if origin.blank?

      begin
        host = URI.parse(origin).host
      rescue URI::InvalidURIError
        return true
      end

      host.blank? || !host.casecmp?(request.host)
    end

    # Per-request version: the header wins, must match _meta when both are
    # present, and initialize may carry it in protocolVersion. Unversioned
    # requests read as legacy 2025-03-26. Renders and returns nil when the
    # version is unusable.
    def resolve_protocol_version(id, method, params)
      header_version = request.headers["MCP-Protocol-Version"].presence
      meta = params["_meta"].is_a?(Hash) ? params["_meta"] : {}
      meta_version = meta["io.modelcontextprotocol/protocolVersion"].presence

      if header_version && meta_version && header_version != meta_version
        render json: rpc_error(id, ERROR_HEADER_MISMATCH,
          "Header mismatch: MCP-Protocol-Version header value #{header_version.inspect} does not match body value #{meta_version.inspect}"),
          status: :bad_request
        return nil
      end

      requested = header_version || meta_version
      requested ||= params["protocolVersion"].presence if method == "initialize"
      requested ||= "2025-03-26"

      unless Agents::McpServer::SUPPORTED_VERSIONS.include?(requested)
        render json: rpc_error(id, ERROR_UNSUPPORTED_VERSION, "Unsupported protocol version",
          { supported: Agents::McpServer::SUPPORTED_VERSIONS, requested: requested }), status: :bad_request
        return nil
      end

      requested
    end

    # Modern requests mirror the method and tool name into headers so
    # intermediaries can route without parsing the body; the server must
    # reject mismatches (and missing headers) rather than trust either
    # side alone. Legacy clients send none of these and skip validation.
    def validate_modern_headers(method, params)
      mcp_method = request.headers["Mcp-Method"].presence
      return "Header mismatch: Mcp-Method header is required" if mcp_method.nil?
      if mcp_method != method
        return "Header mismatch: Mcp-Method header value #{mcp_method.inspect} does not match body value #{method.inspect}"
      end

      if method == "tools/call"
        mcp_name = request.headers["Mcp-Name"].presence
        return "Header mismatch: Mcp-Name header is required" if mcp_name.nil?

        decoded = decode_header_value(mcp_name)
        return "Header mismatch: Mcp-Name header value is malformed" if decoded.nil?

        expected = params["name"].to_s
        if decoded != expected
          return "Header mismatch: Mcp-Name header value #{mcp_name.inspect} does not match body value #{expected.inspect}"
        end
      end

      nil
    end

    def decode_header_value(value)
      match = BASE64_HEADER_SENTINEL.match(value)
      return value unless match

      decoded = Base64.strict_decode64(match[1]).force_encoding(Encoding::UTF_8)
      decoded.valid_encoding? ? decoded : nil
    rescue ArgumentError
      nil
    end

    def dispatch_method(id, method, params, version, modern)
      case method
      when "initialize"
        render json: rpc_result(id, {
          protocolVersion: version,
          capabilities: { tools: { listChanged: false } },
          serverInfo: { name: Agents::McpServer::SERVER_NAME, version: Agents::McpServer::SERVER_VERSION },
          instructions: Agents::McpServer::INSTRUCTIONS
        })
      when "server/discover"
        render json: rpc_result(id, modern_result({
          supportedVersions: Agents::McpServer::SUPPORTED_VERSIONS,
          capabilities: { tools: {} },
          _meta: { "io.modelcontextprotocol/serverInfo" => { name: Agents::McpServer::SERVER_NAME, version: Agents::McpServer::SERVER_VERSION } },
          instructions: Agents::McpServer::INSTRUCTIONS
        }, modern))
      when "tools/list"
        render json: rpc_result(id, modern_result({
          tools: Agents::McpServer.tools.map do |tool|
            { name: tool.name, description: tool.description, inputSchema: tool.input_schema }
          end
        }, modern))
      when "tools/call"
        call_tool(id, params, modern)
      when "ping"
        render json: rpc_result(id, modern_result({}, modern))
      when /\Anotifications\//
        render json: rpc_result(id, {})
      else
        render json: rpc_error(id, ERROR_METHOD_NOT_FOUND, "Method not found: #{method}"), status: :not_found
      end
    end

    def call_tool(id, params, modern)
      tool = Agents::McpServer.tool_named(params["name"])
      unless tool
        render json: rpc_error(id, ERROR_INVALID_PARAMS, "Unknown tool: #{params["name"].inspect}")
        return
      end

      if tool.throttle
        limit, controller_path, action_name = tool.throttle
        retry_after = agent_api_throttle_retry_after(limit, controller_path: controller_path, action_name: action_name)
        if retry_after
          response.set_header("Retry-After", retry_after.to_s)
          render json: rpc_result(id, modern_result({
            content: [ { type: "text", text: "rate_limited: retry after #{retry_after} seconds" } ],
            structuredContent: { error: "rate_limited", retry_after: retry_after },
            isError: true
          }, modern))
          return
        end
      end

      server = Agents::McpServer.new(agent: Current.agent, presenter: self, credential: current_credential)
      result = server.call_tool(params["name"], params["arguments"])

      if result.ok?
        body = result.payload.as_json
        render json: rpc_result(id, modern_result({
          content: [ { type: "text", text: JSON.generate(body) } ],
          structuredContent: body,
          isError: false
        }, modern))
      else
        render json: rpc_result(id, modern_result({
          content: [ { type: "text", text: result.error } ],
          structuredContent: { error: result.error, status: result.status.to_s },
          isError: true
        }, modern))
      end
    rescue Agents::McpServer::InvalidParams => error
      render json: rpc_error(id, ERROR_INVALID_PARAMS, error.message)
    rescue StandardError => error
      Rails.logger.error("MCP tools/call failed: #{error.class}: #{error.message}")
      render json: rpc_error(id, ERROR_INTERNAL, "Internal error")
    end

    def current_credential
      scheme, token = request.authorization.to_s.split(" ", 2)
      return nil unless scheme&.casecmp?("Bearer") && token.present?

      AgentCredential.find_by(token_digest: AgentCredential.digest(token.strip))
    end

    def rpc_result(id, result)
      { jsonrpc: "2.0", id: id, result: result }
    end

    def rpc_error(id, code, message, data = nil)
      error = { code: code, message: message }
      error[:data] = data unless data.nil?

      { jsonrpc: "2.0", id: id, error: error }
    end

    # Modern results carry the MRTR envelope marker; legacy clients get
    # the bare shape their schemas expect.
    def modern_result(result, modern)
      modern ? result.merge(resultType: "complete") : result
    end
end
