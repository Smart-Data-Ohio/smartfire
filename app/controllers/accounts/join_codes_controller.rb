class Accounts::JoinCodesController < ApplicationController
  before_action :ensure_can_administer
  before_action :require_sudo_mode

  def create
    Current.account.reset_join_code
    # The code itself is a credential: the row records that it was reset,
    # never what it changed to.
    AuditLog.record!(action: "account.join_code.reset", target: Current.account)
    redirect_to edit_account_url
  end
end
