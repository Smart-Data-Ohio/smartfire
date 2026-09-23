class Accounts::Bots::KeysController < ApplicationController
  before_action :ensure_can_administer
  before_action :require_sudo_mode

  # Issues a new key and shows it once: only its digest is stored.
  def update
    @bot = User.active_bots.find(params[:bot_id])
    @bot_key = @bot.reset_bot_key
    # The new key never reaches the log: only that a rotation happened.
    AuditLog.record!(action: "agent.credential.reset", target: @bot.agent || @bot)

    no_store_response!
    render :show
  end
end
