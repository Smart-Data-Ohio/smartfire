class UsersController < ApplicationController
  require_unauthenticated_access only: %i[ new create ]

  before_action :set_user, only: :show
  before_action :verify_join_code, only: %i[ new create ]

  # People directory: every active member except yourself, with presence.
  def index
    @users = User.active.includes(:agent).with_attached_avatar.ordered.where.not(id: Current.user.id)
    @online_ids = WorkspacePresenceLease.online_user_ids(@users.map(&:id)).to_set
  end

  def new
    @user = User.new
  end

  def create
    @user = User.create!(user_params)
    start_new_session_for @user
    redirect_to root_url
  rescue ActiveRecord::RecordNotUnique
    redirect_to new_session_url(email_address: user_params[:email_address])
  end

  def show
    @dnd_allowed = Current.user ? Current.user.dnd_allowed_users.exists?(allowed_user_id: @user.id) : false
  end

  private
    def set_user
      @user = User.find(params[:id])
    end

    def verify_join_code
      head :not_found if Current.account.join_code != params[:join_code]
    end

    def user_params
      params.require(:user).permit(:name, :avatar, :email_address, :password)
    end
end
