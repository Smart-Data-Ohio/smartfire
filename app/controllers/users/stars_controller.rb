# Stars are a human preference: the signed-in human stars and unstars
# other people (bots and agents included) so they float to the top of
# the member panel, the people directory, and the new-DM picker. Every
# action touches only the current user's own rows, and the starred
# person is never told. The profile card drives the turbo-stream
# responses; the member-row menu drives the JSON ones.
class Users::StarsController < ApplicationController
  rescue_from ActiveRecord::RecordNotFound, with: -> { head :not_found }

  before_action :set_starred_user
  before_action :ensure_human_starrer

  def create
    Current.user.user_stars.find_or_create_by!(starred_user: @starred_user)
    respond_with_star(starred: true)
  rescue ActiveRecord::RecordNotUnique, ActiveRecord::RecordInvalid
    # A concurrent star won first (at the unique index or the uniqueness
    # validation); the star the member wanted already exists.
    respond_with_star(starred: true)
  end

  def destroy
    Current.user.user_stars.where(starred_user: @starred_user).delete_all
    respond_with_star(starred: false)
  end

  private
    def request_authentication
      request.format.json? ? head(:unauthorized) : super
    end

    def set_starred_user
      @starred_user = User.find(params[:user_id])
      head :unprocessable_entity if @starred_user == Current.user
    end

    def ensure_human_starrer
      head :forbidden unless Current.user&.active? && !Current.user.bot?
    end

    def respond_with_star(starred:)
      respond_to do |format|
        format.turbo_stream do
          render turbo_stream: turbo_stream.replace(
            ActionView::RecordIdentifier.dom_id(@starred_user, :star),
            partial: "users/stars/toggle",
            locals: { user: @starred_user, starred: starred }
          )
        end
        format.json { render json: { starred: starred } }
        format.html { redirect_back fallback_location: user_path(@starred_user) }
      end
    end
end
