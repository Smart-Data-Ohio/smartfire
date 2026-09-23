class Users::BansController < ApplicationController
  before_action :ensure_can_administer
  before_action :set_user

  def create
    previous_status = @user.status
    @user.ban
    record_status_change("user.ban", previous_status: previous_status)
    redirect_to @user
  end

  def destroy
    previous_status = @user.status
    @user.unban
    record_status_change("user.unban", previous_status: previous_status)
    redirect_to @user
  end

  private
    # Records the actual transition: ban/unban accept users in any
    # status, and a replay that changes nothing writes no row.
    def record_status_change(action, previous_status:)
      if @user.status != previous_status
        AuditLog.record!(action: action, target: @user,
          changes: { status: AuditLog.pair(previous_status, @user.status) })
      end
    end

    def set_user
      @user = User.find(params[:user_id])
    end
end
