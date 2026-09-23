class Agents::Fizzy::CardActionsController < ApplicationController
  include AgentApiThrottle
  include FizzyAgentAuthentication

  allow_agent_access only: :create

  before_action :ensure_fizzy_agent_token, only: :create
  throttle_agent_api limit: 60, only: :create

  # POST /agents/fizzy/card_actions (Bearer-only, JSON). Requests a Fizzy
  # write action — create, comment, move, close, or reopen — as the agent
  # owner's own linked Fizzy account. Never calls Fizzy: it creates an
  # AgentApproval for a human decider, and the action only runs when the
  # request is approved. A repeated external_id returns the existing row
  # with 200, like POST /agents/approvals. The request lives in
  # Agents::FizzyCardActions, shared with the MCP tools.
  def create
    no_store_response!

    result = Agents::FizzyCardActions.create(agent: Current.agent, fields: card_action_fields, credential: current_credential)

    if result.ok?
      render json: result.payload, status: result.status
    else
      render json: result.failure_body, status: result.status
    end
  end

  private
    def card_action_fields
      {
        "account_id" => params[:account_id],
        "kind" => params[:kind],
        "board_id" => params[:board_id],
        "number" => params[:number],
        "column_id" => params[:column_id],
        "title" => params[:title],
        "description" => params[:description],
        "body" => params[:body],
        "external_id" => params[:external_id]
      }
    end

    def current_credential
      scheme, token = request.authorization.to_s.split(" ", 2)
      return nil unless scheme&.casecmp?("Bearer") && token.present?

      AgentCredential.find_by(token_digest: AgentCredential.digest(token.strip))
    end
end
