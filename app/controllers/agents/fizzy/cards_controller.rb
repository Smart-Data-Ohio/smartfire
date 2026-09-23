class Agents::Fizzy::CardsController < ApplicationController
  include AgentApiThrottle
  include FizzyAgentAuthentication

  allow_agent_access only: %i[ search show ]

  before_action :ensure_fizzy_agent_token, only: %i[ search show ]
  throttle_agent_api limit: 120, only: %i[ search show ]

  # GET /agents/fizzy/cards/search?q= (Bearer-only, JSON). Full-text
  # card search through the agent owner's Fizzy account. The lookup
  # lives in Agents::FizzyReads, shared with the MCP tools.
  def search
    no_store_response!

    render_fizzy_result Agents::FizzyReads.search_cards(agent: Current.agent, query: params[:q], account_id: params[:account_id])
  end

  # GET /agents/fizzy/cards/:account_id/:number (Bearer-only, JSON).
  # Shows one card, including its steps, through the agent owner's
  # Fizzy account.
  def show
    no_store_response!

    render_fizzy_result Agents::FizzyReads.card(agent: Current.agent, account_id: params[:account_id], number: params[:number])
  end
end
