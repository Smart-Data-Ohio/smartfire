module Agents
  # Shared Fizzy reads for the REST agent Fizzy endpoints and the MCP
  # list_fizzy_boards / get_fizzy_board / search_fizzy_cards /
  # get_fizzy_card tools. Owns the workspace-wide fizzy capability
  # check, the agent-owner account resolution, id validation, and the
  # Fizzy failure mapping, so both surfaces read identically.
  class FizzyReads
    ID_FORMAT = /\A[A-Za-z0-9_-]+\z/

    def self.boards(agent:, account_id: nil)
      new(agent).boards(account_id: account_id)
    end

    def self.board(agent:, board_id:, account_id: nil)
      new(agent).board(board_id: board_id, account_id: account_id)
    end

    def self.search_cards(agent:, query:, account_id: nil)
      new(agent).search_cards(query: query, account_id: account_id)
    end

    def self.card(agent:, account_id:, number:)
      new(agent).card(account_id: account_id, number: number)
    end

    def initialize(agent)
      @agent = agent
    end

    def boards(account_id:)
      if (failure = access_failure)
        return failure
      end
      resolved = validated_id(account_id.presence || @account.fizzy_account_id)
      return resolved if resolved.is_a?(ServiceResult)

      ServiceResult.ok(client.boards(resolved))
    rescue ::Fizzy::Client::Error => error
      read_error(error)
    rescue ActiveRecord::Encryption::Errors::Decryption
      unreadable_token
    end

    def board(board_id:, account_id:)
      if (failure = access_failure)
        return failure
      end
      resolved_account = validated_id(account_id.presence || @account.fizzy_account_id)
      return resolved_account if resolved_account.is_a?(ServiceResult)
      resolved_board = validated_id(board_id)
      return resolved_board if resolved_board.is_a?(ServiceResult)

      ServiceResult.ok({
        "board" => client.board(resolved_account, resolved_board),
        "columns" => client.columns(resolved_account, resolved_board)
      })
    rescue ::Fizzy::Client::Error => error
      read_error(error)
    rescue ActiveRecord::Encryption::Errors::Decryption
      unreadable_token
    end

    def search_cards(query:, account_id:)
      if (failure = access_failure)
        return failure
      end
      if query.blank?
        return ServiceResult.fail("Missing query")
      end
      resolved = validated_id(account_id.presence || @account.fizzy_account_id)
      return resolved if resolved.is_a?(ServiceResult)

      ServiceResult.ok(client.search(resolved, query.to_s))
    rescue ::Fizzy::Client::Error => error
      read_error(error)
    rescue ActiveRecord::Encryption::Errors::Decryption
      unreadable_token
    end

    def card(account_id:, number:)
      if (failure = access_failure)
        return failure
      end
      unless number.to_s.match?(/\A\d+\z/)
        return ServiceResult.fail("Invalid card number", status: :not_found)
      end
      resolved = validated_id(account_id)
      return resolved if resolved.is_a?(ServiceResult)

      ServiceResult.ok(client.card(resolved, number.to_i))
    rescue ::Fizzy::Client::Error => error
      read_error(error)
    rescue ActiveRecord::Encryption::Errors::Decryption
      unreadable_token
    end

    private
      # Room-less reads follow the approvals rule: the grant must be
      # workspace-wide, since the Fizzy call concerns no room. Reads run
      # as the agent owner's own linked Fizzy account, so an agent only
      # ever sees what its owner can access in Fizzy.
      def access_failure
        unless @agent.can?(:fizzy, nil)
          return ServiceResult.fail("Forbidden: agent lacks fizzy capability", status: :forbidden)
        end

        owner = @agent.owner
        if owner.nil?
          return ServiceResult.fail("Agent has no owner recorded")
        end

        @account = owner.fizzy_connected_account
        unless @account&.usable?
          return ServiceResult.fail("Agent owner has no usable Fizzy account")
        end

        nil
      end

      def client
        ::Fizzy::Client.new(token: @account.access_token)
      end

      # Guards an agent-supplied Fizzy id before it is interpolated into
      # an API path. Returns the id, or a 404 failure when invalid.
      def validated_id(value)
        value = value.to_s
        return value if value.match?(ID_FORMAT)

        ServiceResult.fail("Not found in Fizzy", status: :not_found)
      end

      # Maps Fizzy failures on agent reads to agent-facing statuses. A
      # 401 disconnects the owner's account exactly like the member
      # path, so the profile offers a reconnect instead of failing
      # silently. A 403 reads as 404, like the member card frame's
      # minimal chip: the token cannot see the resource, and the
      # response must not say whether it exists.
      def read_error(error)
        case error
        when ::Fizzy::Client::Unauthorized
          @account.mark_disconnected!("Fizzy rejected the linked token (401)")
          ServiceResult.fail("Agent owner's Fizzy token was rejected")
        when ::Fizzy::Client::NotFound, ::Fizzy::Client::Forbidden
          ServiceResult.fail("Not found in Fizzy", status: :not_found)
        when ::Fizzy::Client::Refused
          ServiceResult.fail(error.message)
        else
          ServiceResult.fail(error.message, status: :bad_gateway)
        end
      end

      def unreadable_token
        @account.mark_disconnected!(FizzyConnectedAccount::UNREADABLE_TOKEN_REASON)
        ServiceResult.fail(FizzyConnectedAccount::UNREADABLE_TOKEN_REASON)
      end
  end
end
