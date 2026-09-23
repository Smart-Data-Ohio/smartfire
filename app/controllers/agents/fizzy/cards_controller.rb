class Agents::Fizzy::CardsController < ApplicationController
  include AgentApiThrottle
  include FizzyAgentAuthentication

  allow_agent_access only: %i[ search show ]

  before_action :ensure_fizzy_agent_token, only: %i[ search show ]
  before_action :ensure_fizzy_capability, only: %i[ search show ]
  before_action :ensure_owner_fizzy_account, only: %i[ search show ]
  throttle_agent_api limit: 120, only: %i[ search show ]

  # GET /agents/fizzy/cards/search?q= (Bearer-only, JSON). Full-text
  # card search through the agent owner's Fizzy account.
  def search
    no_store_response!

    if params[:q].blank?
      render json: { error: "Missing query" }, status: :unprocessable_entity
      return
    end

    account_id = validated_fizzy_id(resolved_fizzy_account_id)
    return if account_id.nil?

    render json: fizzy_client.search(account_id, params[:q].to_s)
  rescue Fizzy::Client::Error => error
    render_fizzy_read_error(error)
  rescue ActiveRecord::Encryption::Errors::Decryption
    @fizzy_account.mark_disconnected!(FizzyConnectedAccount::UNREADABLE_TOKEN_REASON)
    render json: { error: FizzyConnectedAccount::UNREADABLE_TOKEN_REASON }, status: :unprocessable_entity
  end

  # GET /agents/fizzy/cards/:account_id/:number (Bearer-only, JSON).
  # Shows one card, including its steps, through the agent owner's
  # Fizzy account.
  def show
    no_store_response!

    number = params[:number].to_s
    unless number.match?(/\A\d+\z/)
      render json: { error: "Invalid card number" }, status: :not_found
      return
    end
    account_id = validated_fizzy_id(params[:account_id])
    return if account_id.nil?

    render json: fizzy_client.card(account_id, number.to_i)
  rescue Fizzy::Client::Error => error
    render_fizzy_read_error(error)
  rescue ActiveRecord::Encryption::Errors::Decryption
    @fizzy_account.mark_disconnected!(FizzyConnectedAccount::UNREADABLE_TOKEN_REASON)
    render json: { error: FizzyConnectedAccount::UNREADABLE_TOKEN_REASON }, status: :unprocessable_entity
  end
end
