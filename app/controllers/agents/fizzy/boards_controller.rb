class Agents::Fizzy::BoardsController < ApplicationController
  include AgentApiThrottle
  include FizzyAgentAuthentication

  allow_agent_access only: %i[ index show ]

  before_action :ensure_fizzy_agent_token, only: %i[ index show ]
  throttle_agent_api limit: 120, only: %i[ index show ]

  # GET /agents/fizzy/boards (Bearer-only, JSON). Lists the boards the
  # agent owner's Fizzy account can access. Requires the workspace-wide
  # fizzy capability. The lookup lives in Agents::FizzyReads, shared
  # with the MCP tools.
  def index
    no_store_response!

    render_fizzy_result Agents::FizzyReads.boards(agent: Current.agent, account_id: params[:account_id])
  end

  # GET /agents/fizzy/boards/:id (Bearer-only, JSON). Returns the board
  # with its columns, so an agent can resolve a column id before
  # requesting a move.
  def show
    no_store_response!

    render_fizzy_result Agents::FizzyReads.board(agent: Current.agent, board_id: params[:id], account_id: params[:account_id])
  end
end
