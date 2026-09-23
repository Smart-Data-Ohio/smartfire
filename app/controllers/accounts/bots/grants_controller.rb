class Accounts::Bots::GrantsController < ApplicationController
  before_action :set_bot
  before_action :ensure_can_manage_bot
  # Owners keep a read-only view and may revoke; widening what an agent can
  # do needs a current administrator, so a demoted owner cannot keep
  # granting capabilities (workspace-wide or external_action included).
  before_action :ensure_can_administer, only: :create
  before_action :set_agent

  def index
    @grants = ordered_grants
    @grant = AgentGrant.new
  end

  def create
    # SQLite opens write transactions in immediate mode, so concurrent creates
    # serialize here and the second one finds the first one's row instead of
    # tripping the unique index. Granting an already-granted capability is
    # therefore idempotent, with the index rescue kept for other databases.
    @grant = AgentGrant.transaction do
      @agent.agent_grants.active.find_by(capability: grant_params[:capability], room_id: grant_params[:room_id]) ||
        @agent.agent_grants.create(grant_params.merge(granted_by: Current.user))
    end

    if @grant.persisted?
      if @grant.previously_new_record?
        AuditLog.record!(action: "agent.grant.create", target: @grant,
          changes: { capability: @grant.capability, room: @grant.room&.name })
      end
      redirect_to account_bot_grants_url(@bot)
    else
      @grants = ordered_grants
      render :index, status: :unprocessable_entity
    end
  rescue ActiveRecord::RecordNotUnique
    redirect_to account_bot_grants_url(@bot)
  end

  def destroy
    grant = @agent.agent_grants.find(params[:id])
    # revoke! is idempotent: re-revoking changes nothing and writes no row.
    unless grant.revoked?
      grant.revoke!
      AuditLog.record!(action: "agent.grant.revoke", target: grant,
        changes: { capability: grant.capability, room: grant.room&.name })
    end
    redirect_to account_bot_grants_url(@bot)
  end

  private
    def set_bot
      @bot = User.active_bots.find(params[:bot_id])
    end

    def ensure_can_manage_bot
      head :forbidden unless Current.user.administrator? || @bot.agent&.owner == Current.user
    end

    def set_agent
      @agent = @bot.agent
      return if @agent

      # A legacy bot gains its agent row on first visit; that creation is
      # audited like one from the bots page.
      @agent = @bot.create_agent!(kind: :workspace, owner: Current.user)
      AuditLog.record!(action: "agent.create", target: @agent,
        changes: { name: @bot.name, kind: "workspace" })
    end

    def ordered_grants
      @agent.agent_grants.includes(:room, :granted_by).order(:revoked_at, :capability, :room_id)
    end

    def grant_params
      params.require(:agent_grant).permit(:capability, :room_id).tap do |grant|
        grant[:room_id] = nil if grant[:room_id].blank?
      end
    end
end
