class Agents::Fizzy::CardActionsController < ApplicationController
  include AgentApiThrottle
  include FizzyAgentAuthentication

  allow_agent_access only: :create

  before_action :ensure_fizzy_agent_token, only: :create
  before_action :ensure_fizzy_external_action, only: :create
  before_action :ensure_owner_fizzy_account, only: :create
  throttle_agent_api limit: 60, only: :create

  # POST /agents/fizzy/card_actions (Bearer-only, JSON). Requests a Fizzy
  # write action — create, comment, move, close, or reopen — as the agent
  # owner's own linked Fizzy account. Never calls Fizzy: it creates an
  # AgentApproval for a human decider, and the action only runs when the
  # request is approved. A repeated external_id returns the existing row
  # with 200, like POST /agents/approvals.
  def create
    no_store_response!

    agent = Current.agent

    if params[:external_id].present?
      existing = AgentApproval.where(agent_id: agent.id, external_id: params[:external_id].to_s).first
      if existing
        existing.expire_if_due!
        render json: approval_created_payload(existing), status: :ok
        return
      end
    end

    action = Fizzy::AgentCardAction.new(
      account_id: params[:account_id].presence || @fizzy_account.fizzy_account_id,
      kind: params[:kind].to_s,
      board_id: params[:board_id],
      number: params[:number],
      column_id: params[:column_id],
      title: params[:title],
      description: params[:description],
      body: params[:body]
    )
    unless action.valid?
      render json: { errors: action.errors.to_hash }, status: :unprocessable_entity
      return
    end

    approval = AgentApproval.new(
      agent: agent,
      agent_credential: current_credential,
      action: action.action_name,
      summary: action.summary,
      payload: action.payload_json,
      external_id: params[:external_id].presence,
      fizzy_connected_account_id: @fizzy_account.id,
      fizzy_user_id: @fizzy_account.fizzy_user_id,
      fizzy_user_name: @fizzy_account.fizzy_user_name
    )

    if approval.save
      render json: approval_created_payload(approval), status: :accepted
    else
      render json: { error: approval.errors.full_messages.to_sentence }, status: :unprocessable_entity
    end
  rescue ActiveRecord::RecordNotUnique
    # Two identical requests raced past the replay lookup; the loser
    # answers with the winner's row exactly like a replay.
    existing = AgentApproval.find_by!(agent_id: agent.id, external_id: params[:external_id].to_s)
    existing.expire_if_due!
    render json: approval_created_payload(existing), status: :ok
  end

  private
    def current_credential
      scheme, token = request.authorization.to_s.split(" ", 2)
      return nil unless scheme&.casecmp?("Bearer") && token.present?

      AgentCredential.find_by(token_digest: AgentCredential.digest(token.strip))
    end

    def approval_created_payload(approval)
      {
        id: approval.id,
        status: approval.effective_status,
        expires_at: approval.expires_at&.utc
      }.compact
    end
end
