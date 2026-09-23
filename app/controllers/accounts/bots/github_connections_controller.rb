class Accounts::Bots::GithubConnectionsController < ApplicationController
  # Only a current administrator may link, relink, or unlink the agent's
  # GitHub identity: it decides whom approved write actions act as.
  before_action :ensure_can_administer
  before_action :set_bot

  # Links the agent's own fine-grained personal access token (for a GitHub
  # machine user dedicated to the agent) so approved write actions run as
  # the agent's GitHub identity — never the workspace token. When the
  # agent's owner has a usable GitHub App token it is used instead (see
  # Github::AgentIdentity). The pasted token is validated with GET /user
  # before anything is stored; it is never logged (filtered as :token) or
  # rendered back.
  def create
    token = params[:access_token].to_s.strip
    if token.blank?
      return redirect_to edit_account_bot_path(@bot), alert: "Paste a token to connect GitHub."
    end

    login = Github::WriteClient.authenticated_login(token)
    account = @bot.github_connected_account || @bot.build_github_connected_account
    account.assign_attributes(
      github_login: login, access_token: token, disconnected_reason: nil,
      token_source: "pat", refresh_token: nil, token_expires_at: nil, last_error: nil
    )
    account.save!

    redirect_to edit_account_bot_path(@bot), notice: "GitHub connected as #{login}."
  rescue Github::WriteClient::Unauthorized
    redirect_to edit_account_bot_path(@bot), alert: "GitHub rejected that token. Check it and try again."
  rescue Github::WriteClient::Error
    redirect_to edit_account_bot_path(@bot), alert: "Could not reach GitHub. Try again."
  end

  def destroy
    if (account = @bot.github_connected_account)
      account.revoke_remote_token!
      account.destroy!
    end
    redirect_to edit_account_bot_path(@bot), notice: "GitHub disconnected."
  end

  private
    def set_bot
      @bot = User.active_bots.find(params[:bot_id])
    end
end
