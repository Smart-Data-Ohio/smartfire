class Accounts::Bots::KeysController < ApplicationController
  before_action :ensure_can_administer

  # Issues a new key and shows it once: only its digest is stored.
  def update
    @bot = User.active_bots.find(params[:bot_id])
    @bot_key = @bot.reset_bot_key

    no_store_response!
    render :show
  end
end
