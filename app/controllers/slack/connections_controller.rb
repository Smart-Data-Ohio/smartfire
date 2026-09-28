module Slack
  # Disconnects the member's own Slack connection. The token is revoked
  # remotely first (best effort) so no live grant survives the delete.
  # Blocked while any of the member's runs is still active.
  class ConnectionsController < ApplicationController
    before_action :require_sudo_mode

    def destroy
      if Current.user.slack_imports.active.exists?
        return redirect_to return_path, alert: "Finish or cancel your running Slack import first."
      end

      if (connection = Current.user.slack_connection)
        if (token = readable_token(connection)).present?
          Slack::OAuth.revoke(token)
        end
        slack_user_id = connection.slack_user_id
        connection.destroy!
        AuditLog.record!(action: "slack.account.disconnect", actor: Current.user, target: Current.user,
          changes: { slack_user_id: })
      end

      redirect_to return_path, notice: "Slack disconnected."
    end

    private
      # An undecryptable token (rotated encryption key) still
      # disconnects locally; there is nothing readable to revoke.
      def readable_token(connection)
        connection.access_token
      rescue ActiveRecord::Encryption::Errors::Decryption
        nil
      end

      def return_path
        candidate = params[:return_to].to_s
        if OAuthController::RETURN_PATHS.include?(candidate)
          candidate
        elsif Current.user.can_administer?
          account_slack_import_path
        else
          slack_imports_path
        end
      end
  end
end
