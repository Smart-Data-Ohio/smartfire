class Accounts::BotsController < ApplicationController
  before_action :ensure_can_administer, only: %i[ index new create destroy ]
  before_action :set_bot, only: %i[ edit update destroy kill_switch ]
  before_action :ensure_can_manage_bot, only: %i[ edit update kill_switch ]
  before_action :ensure_can_change_webhook, only: :update
  before_action :set_agent, only: %i[ edit update kill_switch ]

  def index
    @bots = User.active_bots.ordered.includes(agent: :owner)
  end

  def new
    @bot = User.active_bots.new
  end

  # Shows the new bot's key once: only its digest is stored.
  def create
    @bot = User.create_bot! bot_params
    @bot.create_agent!(kind: :workspace, owner: Current.user)
    @bot_key = @bot.plain_bot_key
    # The key itself never reaches the log: only that an agent was created.
    AuditLog.record!(action: "agent.create", target: @bot.agent,
      changes: { name: @bot.name, kind: "workspace" })

    no_store_response!
    render "accounts/bots/keys/show", status: :created
  rescue ActiveRecord::RecordInvalid => error
    @bot = error.record
    render :new, status: :unprocessable_entity
  end

  def edit
  end

  def update
    @agent&.assign_attributes(agent_params)
    previous_webhook_url = @bot.webhook_url

    if @agent&.invalid?
      render :edit, status: :unprocessable_entity
    elsif @bot.update_bot(bot_params)
      @agent&.save!
      record_bot_changes(previous_webhook_url: previous_webhook_url)
      redirect_to account_bots_url
    else
      render :edit, status: :unprocessable_entity
    end
  end

  def destroy
    agent = @bot.agent
    @bot.deactivate
    AuditLog.record!(action: "agent.suspend", target: agent || @bot,
      changes: { by: "bot removed" })
    redirect_to account_bots_url
  end

  # POST /account/bots/:id/kill_switch. Suspends the agent, cancels its
  # pending approvals, and records `agent.kill_switch`. Administrators
  # and the agent's owner.
  def kill_switch
    unless @agent
      head :not_found
      return
    end

    cancelled = @agent.kill_switch!

    redirect_to edit_account_bot_url(@bot),
      notice: "Agent suspended; #{cancelled} #{"approval".pluralize(cancelled)} cancelled."
  end

  private
    def set_bot
      @bot = User.active_bots.find(params[:id])
    end

    def ensure_can_manage_bot
      head :forbidden unless Current.user.administrator? || @bot.agent&.owner == Current.user
    end

    # The webhook URL decides where the app posts room messages and where it
    # makes server-side requests, so only a current administrator may change
    # it. An owner who is no longer (or never was) an administrator can still
    # submit the edit form unchanged.
    def ensure_can_change_webhook
      return if Current.user.administrator?
      return unless params[:user].respond_to?(:key?) && params[:user].key?(:webhook_url)

      head :forbidden if params[:user][:webhook_url].to_s.strip != @bot.webhook_url.to_s
    end

    # A legacy bot without an agent row stays that way: reading or editing
    # its page must not silently convert it into an agent.
    def set_agent
      @agent = @bot.agent
    end

    # Runs after both saves so previous_changes reflects what persisted. A
    # webhook URL change gets its own row; everything else shares one edit row.
    def record_bot_changes(previous_webhook_url:)
      # update_bot may have destroyed the webhook through the cached
      # association, which would still answer the old URL.
      @bot.association(:webhook).reload

      if @bot.webhook_url != previous_webhook_url
        before = AuditLog.webhook_origin_summary(previous_webhook_url)
        after = AuditLog.webhook_origin_summary(@bot.webhook_url)
        AuditLog.record!(action: "agent.webhook_url.change", target: @agent || @bot,
          changes: { webhook_url: AuditLog.pair(before, after) })
      end

      bot_changes = @bot.previous_changes.slice("name", "icon_name")
      agent_changes = @agent ? @agent.previous_changes.slice("provider", "runtime", "description",
        "daily_message_cap", "daily_board_post_cap", "daily_external_action_cap") : {}
      if bot_changes.present? || agent_changes.present?
        pairs = bot_changes.merge(agent_changes).transform_values { |change| AuditLog.pair(*change) }
        AuditLog.record!(action: "agent.update", target: @agent || @bot, changes: pairs)
      end
    end

    def bot_params
      permitted = %i[ name avatar icon_name ]
      permitted << :webhook_url if Current.user.administrator?
      params.require(:user).permit(*permitted)
    end

    def agent_params
      params.permit(agent: %i[ provider runtime description daily_message_cap daily_board_post_cap daily_external_action_cap ])[:agent] || {}
    end
end
