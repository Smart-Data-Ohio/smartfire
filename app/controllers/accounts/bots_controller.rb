class Accounts::BotsController < ApplicationController
  before_action :ensure_can_administer, only: %i[ index new create destroy ]
  before_action :set_bot, only: %i[ edit update destroy ]
  before_action :ensure_can_manage_bot, only: %i[ edit update ]
  before_action :ensure_can_change_webhook, only: :update
  before_action :set_agent, only: %i[ edit update ]

  def index
    @bots = User.active_bots.ordered.includes(agent: :owner)
  end

  def new
    @bot = User.active_bots.new
  end

  def create
    bot = User.create_bot! bot_params
    bot.create_agent!(kind: :workspace, owner: Current.user)
    redirect_to account_bots_url
  rescue ActiveRecord::RecordInvalid => error
    @bot = error.record
    render :new, status: :unprocessable_entity
  end

  def edit
  end

  def update
    @agent&.assign_attributes(agent_params)

    if @agent&.invalid?
      render :edit, status: :unprocessable_entity
    elsif @bot.update_bot(bot_params)
      @agent&.save!
      redirect_to account_bots_url
    else
      render :edit, status: :unprocessable_entity
    end
  end

  def destroy
    @bot.deactivate
    redirect_to account_bots_url
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

    def bot_params
      permitted = %i[ name avatar icon_name ]
      permitted << :webhook_url if Current.user.administrator?
      params.require(:user).permit(*permitted)
    end

    def agent_params
      params.permit(agent: %i[ provider runtime description ])[:agent] || {}
    end
end
