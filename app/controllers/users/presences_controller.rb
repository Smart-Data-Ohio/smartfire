class Users::PresencesController < ApplicationController
  MAX_IDS = 100

  # Live presence plus custom status for a set of workspace users, so
  # cached DM rows can paint dots without busting their cache keys.
  def show
    ids = Array(params[:ids]).filter_map { |id| Integer(id, exception: false) }.first(MAX_IDS)
    users = User.active.without_bots.where(id: ids).includes(:meeting_cache).index_by(&:id)
    lease_states = WorkspacePresenceLease.presence_by_user_id(users.keys)

    render json: {
      presences: users.map do |id, user|
        {
          id:,
          presence: user.effective_presence(lease_states[id] || :offline).to_s,
          status: user.status_text_display
        }
      end
    }
  end
end
