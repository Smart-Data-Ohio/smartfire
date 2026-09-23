module Fizzy
  # Links a member's own Fizzy personal access token so card previews
  # and card creation run as their Fizzy user. The pasted token is
  # validated with GET /my/identity before anything is stored; it is
  # never logged (filtered as :token) or rendered back.
  class ConnectionsController < ApplicationController
    def create
      token = params[:access_token].to_s.strip
      if token.blank?
        return redirect_to user_profile_path, alert: "Paste a token to connect Fizzy."
      end

      identity = Client.identity_for(token)
      account = Array(identity["accounts"]).first
      if account.blank? || account["slug"].blank?
        return redirect_to user_profile_path, alert: "That token has no Fizzy account to use."
      end

      connected = Current.user.fizzy_connected_account || Current.user.build_fizzy_connected_account
      connected.assign_attributes(
        fizzy_account_id: account["slug"].to_s.delete_prefix("/"),
        fizzy_account_name: account["name"],
        fizzy_user_id: account.dig("user", "id"),
        fizzy_user_name: account.dig("user", "name"),
        access_token: token,
        disconnected_reason: nil
      )
      connected.save!
      # The pasted token never reaches the log: only the identity Fizzy confirmed.
      AuditLog.record!(action: "fizzy.account.connect", actor: Current.user, target: Current.user,
        changes: { fizzy_user_name: connected.fizzy_user_name, fizzy_account_name: connected.fizzy_account_name })

      redirect_to user_profile_path, notice: "Fizzy connected as #{connected.fizzy_user_name} (#{connected.fizzy_account_name})."
    rescue Client::Unauthorized
      redirect_to user_profile_path, alert: "Fizzy rejected that token. Check it and try again."
    rescue Client::Error
      redirect_to user_profile_path, alert: "Could not reach Fizzy. Try again."
    end

    def destroy
      account = Current.user.fizzy_connected_account
      Fizzy::CardCache.where(user_id: Current.user.id).delete_all
      # Disconnecting with no link clears caches only and writes no row.
      if account
        account.destroy!
        AuditLog.record!(action: "fizzy.account.disconnect", actor: Current.user, target: Current.user,
          changes: { fizzy_user_name: account.fizzy_user_name, fizzy_account_name: account.fizzy_account_name })
      end
      redirect_to user_profile_path, notice: "Fizzy disconnected."
    end
  end
end
