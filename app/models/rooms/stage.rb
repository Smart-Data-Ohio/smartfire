# Rooms with a standing stage huddle: a few members speak while everyone else
# listens. Hosts run the stage, speakers publish audio/video, and listeners
# can only subscribe. Membership works like Rooms::Voice, with explicit
# members chosen by an administrator or the creator, plus a per-member stage
# role. The room creator becomes the first host.
class Rooms::Stage < Room
  has_many :streams, foreign_key: :room_id, dependent: :destroy
  has_many :live_streams, -> { live }, class_name: "Stream", foreign_key: :room_id

  # The room's current live stream, if any. Queried fresh unless the caller
  # preloaded `live_streams` (at most one row per room), in which case the
  # preloaded record is read so list pages render the live dot without a
  # query per room. Ended rows are history and are never loaded for this.
  def live_stream
    if association(:live_streams).loaded?
      live_streams.first
    else
      streams.live.first
    end
  end

  # Ends the room's live session once no host membership remains: every
  # live stream ends, with the usual ended broadcasts, and every active
  # huddle grant in the room is revoked, dropping the remaining speakers
  # and listeners from the call. The room, its members, and its history
  # stay; an administrator promotes a new host afterwards if the stage
  # should go live again. A quiet timeline note records the end.
  #
  # Called when a host membership is destroyed and from user
  # deactivation, which deletes memberships without callbacks. The host
  # check runs after locking the room, like the last-host demotion
  # guard: without it, two concurrent departures of the last two hosts
  # could both pass and leave the session live with no host.
  def end_live_session_if_hostless!(departed_host:)
    transaction do
      lock!
      return if memberships.where(stage_role: :host).exists?

      Stream.live.where(room_id: id).each(&:end!)
      HuddleGrant.revoke_for_room!(self)
      post_stage_ended_note!(departed_host: departed_host)
    end
  end

  class << self
    def create_for(attributes, users:)
      super.tap do |room|
        room.memberships.where(stage_role: nil).update_all(stage_role: :listener)
        room.memberships.find_or_create_by!(user: room.creator).update!(stage_role: :host)
      end
    end
  end

  private
    # A quiet timeline note, like the group-DM membership notes: it renders
    # in the timeline but marks nothing unread and pushes nothing. Plain
    # Action Text, never Markdown, like the group-DM notes.
    def post_stage_ended_note!(departed_host:)
      messages.create!(creator: departed_host, system_note: true,
        body: ERB::Util.h("The stage ended because the last host left.")).tap(&:broadcast_create)
    end
end
