class Users::BansController < ApplicationController
  before_action :ensure_can_administer
  before_action :set_user

  def create
    @user.ban
    AuditLog.record!(action: "user.ban", target: @user, changes: { status: [ "active", "banned" ] })
    redirect_to @user
  end

  def destroy
    @user.unban
    AuditLog.record!(action: "user.unban", target: @user, changes: { status: [ "banned", "active" ] })
    redirect_to @user
  end

  private
    def set_user
      @user = User.find(params[:user_id])
    end
end
