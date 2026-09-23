class Accounts::Bots::WebhookSecretsController < ApplicationController
  before_action :set_bot
  before_action :ensure_can_manage_bot
  before_action :require_sudo_mode

  # Regenerates the secret signing this bot's webhook deliveries: the
  # agent secret for agent-backed bots, the webhook secret for legacy
  # bots. Legacy bots without a webhook URL have nothing to sign with.
  def create
    if @bot.agent
      @bot.agent.reset_webhook_signing_secret!
      # The secret itself never reaches the log: only that it was reset.
      AuditLog.record!(action: "agent.webhook_secret.reset", target: @bot.agent)
      redirect_to edit_account_bot_url(@bot), notice: "Signing secret reset. Update the receiving service with the new secret."
    elsif @bot.webhook
      @bot.webhook.reset_signing_secret!
      AuditLog.record!(action: "agent.webhook_secret.reset", target: @bot)
      redirect_to edit_account_bot_url(@bot), notice: "Signing secret reset. Update the receiving service with the new secret."
    else
      redirect_to edit_account_bot_url(@bot), alert: "Set a webhook URL before generating a signing secret."
    end
  end

  private
    def set_bot
      @bot = User.active_bots.find(params[:bot_id])
    end

    def ensure_can_manage_bot
      head :forbidden unless Current.user.administrator? || @bot.agent&.owner == Current.user
    end
end
