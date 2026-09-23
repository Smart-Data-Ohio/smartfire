require "net/http"

module Fizzy
  # Authenticated Fizzy REST API client backing card previews, the
  # create-from-message flow, and the agent endpoints. All calls run as
  # the token owner's own Fizzy identity — never a workspace token.
  #
  # Endpoint shapes follow the Fizzy API docs (docs/api in
  # github.com/basecamp/fizzy): account-scoped paths under
  # https://app.fizzy.do with a personal access token. Never raises for
  # transport problems without mapping them: callers rescue
  # Client::Error. Never logs tokens, headers, or bodies.
  class Client
    API_BASE_URL = "https://app.fizzy.do"
    TIMEOUT = 10

    class Error < StandardError; end
    class Unauthorized < Error; end
    class NotFound < Error; end
    # A 403 means the token cannot see or do the thing — not that the
    # request was malformed — so read paths treat it like a 404 instead
    # of surfacing an error.
    class Forbidden < Error; end
    class Refused < Error; end

    # Ids interpolated into API paths (account, board, column): Fizzy
    # issues alphanumerics, and anything else would be a path traversal.
    ID_FORMAT = /\A[A-Za-z0-9_-]+\z/

    # The identity behind a token: { "accounts" => [...] }, each with
    # "id", "name", "slug", and the token owner's "user" in it. Raises
    # Unauthorized when Fizzy rejects the token. Used to validate a
    # pasted token at link time.
    def self.identity_for(token)
      new(token: token).identity
    end

    def initialize(token:, base_url: ENV.fetch("FIZZY_API_BASE_URL", API_BASE_URL))
      @token = token
      @base_url = base_url.chomp("/")
    end

    # GET /my/identity.json
    def identity
      get("/my/identity.json")
    end

    # GET /{account}/boards.json
    def boards(account_id)
      get("/#{checked_id(account_id)}/boards.json")
    end

    # GET /{account}/boards/{board}.json
    def board(account_id, board_id)
      get("/#{checked_id(account_id)}/boards/#{checked_id(board_id)}.json")
    end

    # GET /{account}/boards/{board}/columns.json
    def columns(account_id, board_id)
      get("/#{checked_id(account_id)}/boards/#{checked_id(board_id)}/columns.json")
    end

    # GET /{account}/cards/{number}.json, including steps.
    def card(account_id, number)
      get("/#{checked_id(account_id)}/cards/#{checked_number(number)}.json")
    end

    # GET /{account}/search.json?q=
    def search(account_id, query)
      get("/#{checked_id(account_id)}/search.json?q=#{CGI.escape(query.to_s)}")
    end

    # POST /{account}/boards/{board}/cards.json. Returns the created
    # card (Fizzy renders it with 201 and a Location header).
    def create_card(account_id, board_id, title:, description: nil)
      payload = { card: { title: title }.tap { |card| card[:description] = description if description.present? } }
      post("/#{checked_id(account_id)}/boards/#{checked_id(board_id)}/cards.json", payload)
    end

    # POST /{account}/cards/{number}/comments.json. Returns the created
    # comment.
    def create_comment(account_id, number, body:)
      post("/#{checked_id(account_id)}/cards/#{checked_number(number)}/comments.json", { comment: { body: body } })
    end

    # POST /{account}/cards/{number}/triage.json. Moves the card into
    # a column. Returns true.
    def move_to_column(account_id, number, column_id:)
      post("/#{checked_id(account_id)}/cards/#{checked_number(number)}/triage.json", { column_id: column_id })
      true
    end

    # POST /{account}/cards/{number}/closure.json. Returns true.
    def close_card(account_id, number)
      post("/#{checked_id(account_id)}/cards/#{checked_number(number)}/closure.json", {})
      true
    end

    # DELETE /{account}/cards/{number}/closure.json. Returns true.
    def reopen_card(account_id, number)
      delete("/#{checked_id(account_id)}/cards/#{checked_number(number)}/closure.json")
      true
    end

    private
      def checked_id(value)
        value = value.to_s
        raise Error, "Invalid Fizzy id" unless value.match?(ID_FORMAT)

        value
      end

      def checked_number(value)
        value = value.to_s
        raise Error, "Invalid Fizzy card number" unless value.match?(/\A\d+\z/)

        value
      end

      def get(path)
        request(Net::HTTP::Get.new(path, headers))
      end

      def post(path, payload)
        request(Net::HTTP::Post.new(path, headers), payload.to_json)
      end

      def delete(path)
        request(Net::HTTP::Delete.new(path, headers))
      end

      def request(message, body = nil)
        message.body = body if body

        uri = URI(@base_url)
        response = Net::HTTP.start(uri.host, uri.port, use_ssl: uri.scheme == "https",
          open_timeout: TIMEOUT, read_timeout: TIMEOUT) do |http|
          http.request(message)
        end

        case response
        when Net::HTTPSuccess
          parse_body(response)
        when Net::HTTPUnauthorized
          raise Unauthorized, "Fizzy rejected the linked token"
        when Net::HTTPNotFound
          raise NotFound, "Not found in Fizzy"
        when Net::HTTPForbidden
          raise Forbidden, "Fizzy refused: #{fizzy_message(response)}"
        when Net::HTTPUnprocessableEntity
          raise Refused, "Fizzy refused: #{fizzy_message(response)}"
        else
          raise Error, "Fizzy returned #{response.code}"
        end
      rescue Unauthorized, NotFound, Forbidden, Refused
        raise
      rescue Error
        raise
      rescue StandardError => error
        Rails.logger.warn "Fizzy::Client request failed: #{error.class}"
        raise Error, "Could not reach Fizzy (#{error.class.name.demodulize.titleize})"
      end

      def parse_body(response)
        JSON.parse(response.body.presence || "{}")
      rescue JSON::ParserError
        {}
      end

      # Fizzy's own error message, kept inline. A zero-width space splits
      # "@[" so a hostile message can never become a mention token.
      def fizzy_message(response)
        parsed = parse_body(response)
        message = parsed["message"] || parsed["error"] || parsed["errors"]&.to_s
        message = message.to_s.gsub("@[", "@\u200B[").gsub(/[\r\n]+/, " ").strip
        message.presence || "request was not allowed"
      end

      def headers
        {
          "Accept" => "application/json",
          "Content-Type" => "application/json",
          "User-Agent" => "Smartfire-Fizzy",
          "Authorization" => "Bearer #{@token}"
        }
      end
  end
end
