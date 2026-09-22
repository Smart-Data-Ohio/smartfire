class Accounts::Bots::CredentialsController < ApplicationController
  before_action :set_bot
  # The agent's owner may list and revoke its credentials; issuing a new
  # one needs a current administrator. Legacy bots without an agent row
  # have no owner, so only administrators reach them.
  before_action :ensure_can_manage_bot, only: %i[ index destroy ]
  before_action :ensure_can_administer, only: :create
  before_action :set_agent

  def index
    @credentials = @agent.agent_credentials.order(created_at: :desc)
    @credential = AgentCredential.new
  end

  def create
    @credential, @plain_secret = AgentCredential.create_with_secret!(
      agent: @agent,
      name: credential_params[:name],
      created_by: Current.user,
      expires_at: credential_params[:expires_at].presence
    )

    render :show, status: :created
  rescue ActiveRecord::RecordInvalid => error
    @credential = error.record
    @credentials = @agent.agent_credentials.order(created_at: :desc)
    render :index, status: :unprocessable_entity
  end

  def destroy
    @agent.agent_credentials.find(params[:id]).revoke!
    redirect_to account_bot_credentials_url(@bot)
  end

  private
    def set_bot
      @bot = User.active_bots.find(params[:bot_id])
    end

    def ensure_can_manage_bot
      head :forbidden unless Current.user.administrator? || @bot.agent&.owner == Current.user
    end

    def set_agent
      @agent = @bot.agent || @bot.create_agent!(kind: :workspace, owner: Current.user)
    end

    def credential_params
      params.require(:agent_credential).permit(:name, :expires_at)
    end
end
