class Accounts::UsersController < ApplicationController
  before_action :ensure_can_administer, :set_user, only: %i[ update destroy ]

  def index
    set_page_and_extract_portion_from User.active.ordered.without_bots.includes(:google_identity), per_page: 500
  end

  def update
    previous_role = @user.role
    @user.update(role_params)
    if previous_role != @user.role
      AuditLog.record!(action: "user.role.change", target: @user,
        changes: { role: [ previous_role, @user.role ] })
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
      changes: { status: [ previous_status, "deactivated" ] })
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
