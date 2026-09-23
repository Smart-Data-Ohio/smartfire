class Accounts::UsersController < ApplicationController
  before_action :ensure_can_administer, :set_user, only: %i[ update destroy ]
  before_action :require_sudo_mode, only: %i[ update destroy ]

  def index
    set_page_and_extract_portion_from User.active.ordered.without_bots.includes(:google_identity, :two_factor_credential), per_page: 500
  end

  def update
    @user.update(role_params)
    # previous_changes is only populated by a successful save: comparing
    # against the in-memory role would log a row for a failed update.
    if (role_change = @user.previous_changes["role"])
      AuditLog.record!(action: "user.role.change", target: @user,
        changes: { role: AuditLog.pair(*role_change) })
    end
    redirect_to edit_account_url
  end

  def destroy
    previous_status = @user.status
    # Deactivation rewrites the email in place: snapshot the label first
    # so the row keeps the address the member actually used.
    user_label = AuditLog.label_for(@user)
    @user.deactivate
    AuditLog.record!(action: "user.deactivate", target: @user, target_label: user_label,
      changes: { status: AuditLog.pair(previous_status, "deactivated") })
    redirect_to edit_account_url
  end

  private
    def set_user
      @user = User.active.find(params[:user_id] || params[:id])
    end

    def role_params
      { role: params.require(:user)[:role].presence_in(%w[ member administrator ]) || "member" }
    end
end
