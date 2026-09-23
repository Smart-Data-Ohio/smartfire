module Github
  # Links a member's own fine-grained personal access token so PR write
  # actions run as their GitHub user. The pasted token is validated with
  # GET /user before anything is stored; it is never logged (filtered as
  # :token) or rendered back.
  class ConnectionsController < ApplicationController
    def create
      token = params[:access_token].to_s.strip
      if token.blank?
        return redirect_to user_profile_path, alert: "Paste a token to connect GitHub."
      end

      login = WriteClient.authenticated_login(token)
      account = Current.user.github_connected_account || Current.user.build_github_connected_account
      account.assign_attributes(github_login: login, access_token: token, disconnected_reason: nil)
      account.save!
      # The repo-access cache key carries updated_at: bump it even when the
      # token is unchanged so a cached denial never survives a relink.
      account.touch
      # The pasted token never reaches the log: only the login GitHub confirmed.
      AuditLog.record!(action: "github.account.connect", actor: Current.user, target: Current.user,
        changes: { github_login: login })

      redirect_to user_profile_path, notice: link_notice(login)
    rescue WriteClient::Unauthorized
      redirect_to user_profile_path, alert: "GitHub rejected that token. Check it and try again."
    rescue WriteClient::Error
      redirect_to user_profile_path, alert: "Could not reach GitHub. Try again."
    end

    def destroy
      login = Current.user.github_connected_account&.github_login
      Current.user.github_connected_account&.destroy!
      AuditLog.record!(action: "github.account.disconnect", actor: Current.user, target: Current.user,
        changes: { github_login: login })
      redirect_to user_profile_path, notice: "GitHub disconnected."
    end

    private
      # Linking sets the profile login from the one GitHub confirmed (see
      # GithubConnectedAccount#claim_verified_login), unless another member's
      # own linked token already confirms it.
      def link_notice(login)
        if Current.user.reload.github_login == login.to_s.downcase
          "GitHub connected as #{login}."
        else
          "GitHub connected as #{login}. Another member's linked GitHub account already uses that username, so your profile username was left unchanged."
        end
      end
  end
end
