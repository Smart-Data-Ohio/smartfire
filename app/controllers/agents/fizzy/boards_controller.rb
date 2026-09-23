class Agents::Fizzy::BoardsController < ApplicationController
  include AgentApiThrottle
  include FizzyAgentAuthentication

  allow_agent_access only: %i[ index show ]

  before_action :ensure_fizzy_agent_token, only: %i[ index show ]
  before_action :ensure_fizzy_capability, only: %i[ index show ]
  before_action :ensure_owner_fizzy_account, only: %i[ index show ]
  throttle_agent_api limit: 120, only: %i[ index show ]

  # GET /agents/fizzy/boards (Bearer-only, JSON). Lists the boards the
  # agent owner's Fizzy account can access. Requires the workspace-wide
  # fizzy capability.
  def index
    no_store_response!

    account_id = validated_fizzy_id(resolved_fizzy_account_id)
    return if account_id.nil?

    render json: fizzy_client.boards(account_id)
  rescue Fizzy::Client::Error => error
    render_fizzy_read_error(error)
  rescue ActiveRecord::Encryption::Errors::Decryption
    @fizzy_account.mark_disconnected!(FizzyConnectedAccount::UNREADABLE_TOKEN_REASON)
    render json: { error: FizzyConnectedAccount::UNREADABLE_TOKEN_REASON }, status: :unprocessable_entity
  end

  # GET /agents/fizzy/boards/:id (Bearer-only, JSON). Returns the board
  # with its columns, so an agent can resolve a column id before
  # requesting a move.
  def show
    no_store_response!

    account_id = validated_fizzy_id(resolved_fizzy_account_id)
    return if account_id.nil?
    board_id = validated_fizzy_id(params[:id])
    return if board_id.nil?

    render json: {
      board: fizzy_client.board(account_id, board_id),
      columns: fizzy_client.columns(account_id, board_id)
    }
  rescue Fizzy::Client::Error => error
    render_fizzy_read_error(error)
  rescue ActiveRecord::Encryption::Errors::Decryption
    @fizzy_account.mark_disconnected!(FizzyConnectedAccount::UNREADABLE_TOKEN_REASON)
    render json: { error: FizzyConnectedAccount::UNREADABLE_TOKEN_REASON }, status: :unprocessable_entity
  end
end
