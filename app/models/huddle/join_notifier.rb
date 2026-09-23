class Huddle::JoinNotifier
  # A viewer whose ring is still live (or only just timed out) already has
  # the loudest possible notice, so a join inside this window stays quiet
  # for them. Older unhandled rings are stale — the banner hid itself at
  # the 45-second ring timeout — and a join notice is welcome again.
  RECENT_RING_WINDOW = 60.seconds

  class << self
    # Tells a room's members that the grant's user joined the call. Runs
    # from Huddle::JoinNoticeJob on the grant's first gateway sighting,
    # so it fires when the joiner actually connects, not when the token
    # is issued, and reconnects inside the liveness window stay silent.
    # Sightings that change no roster (a second device while another of
    # the joiner's grants is already listed) never enqueue the job.
    #
    # Members already in the call get an in-call toast payload in every
    # room kind. Members outside the call get the "in your huddle"
    # banner payload plus a throttled push, but only in one-to-one and
    # group DMs — channel and voice-room members outside the call opted
    # into a standing call and must not be nudged per join.
    def notify_join(grant)
      room = Room.alive.find_by(id: grant.room_id)
      joiner = grant.user
      return unless room && active_human?(joiner)
      return unless grant_still_in_call?(grant)

      in_call_ids = in_call_user_ids(room)
      memberships = room.memberships.includes(:user).where.not(user_id: joiner.id)
      return if memberships.empty?

      memberships.find_each do |membership|
        viewer = membership.user
        next unless active_human?(viewer)

        if in_call_ids.include?(viewer.id)
          broadcast_join(room:, joiner:, viewer:, in_call: true)
        elsif room.direct? && noticeable_outside_call?(room, membership)
          broadcast_join(room:, joiner:, viewer:, in_call: false)
          Huddle::JoinPusher.new(grant:, recipient: viewer, room_membership: membership).push
        end
      end
    end

    # Tells the members still in the call that the grant's user left it.
    # Runs inline from the leave report and revocation paths, in every
    # room kind, in-call viewers only. When the last participant drops
    # out of a DM call, the room's other members also get a dismissal so
    # their join banners clear immediately instead of waiting out the
    # presence poll.
    def notify_leave(grant)
      room = Room.alive.find_by(id: grant.room_id)
      leaver = grant.user
      return unless room && active_human?(leaver)
      return if HuddleGrant.active.in_call.where(room_id: room.id, user_id: leaver.id).where.not(id: grant.id).exists?

      in_call_ids = in_call_user_ids(room) - [ leaver.id ]
      if in_call_ids.any?
        User.active.without_bots.where(id: in_call_ids).find_each do |viewer|
          broadcast_leave(room:, leaver:, viewer:)
        end
      elsif room.direct?
        dismiss_outside_call(room:, leaver:)
      end
    end

    private
      # The job runs after the sighting, so a joiner who already left or
      # was revoked reads the live row, not the sighting: without this a
      # quick join-then-leave toasts "joined" for someone already gone,
      # after the "left" the leave path already sent inline.
      def grant_still_in_call?(grant)
        HuddleGrant.active.in_call.where(id: grant.id).exists?
      end

      def in_call_user_ids(room)
        HuddleGrant.active.in_call.where(room_id: room.id).distinct.pluck(:user_id).to_set
      end

      def active_human?(user)
        ActivityItem.active_human?(user)
      end

      # Outside-call notices mirror invitation scoping — switched-off and
      # hidden rooms stay silent — and skip a viewer whose ring for this
      # room is still live: ringing is already the notice.
      def noticeable_outside_call?(room, membership)
        return false if membership.involved_in_invisible? || membership.involved_in_nothing?

        !recently_rung?(room, membership.user_id)
      end

      def recently_rung?(room, user_id)
        ActivityItem
          .where(user_id:, source_type: HuddleGrant.polymorphic_name, event_type: "huddle_started", handled_at: nil)
          .where(created_at: RECENT_RING_WINDOW.ago..)
          .joins("INNER JOIN huddle_grants AS rung_grants ON rung_grants.id = activity_items.source_id")
          .where(rung_grants: { room_id: room.id })
          .exists?
      end

      def broadcast_join(room:, joiner:, viewer:, in_call:)
        ActionCable.server.broadcast HuddleNoticeChannel.stream_name_for(viewer.id), {
          huddleJoinNotice: {
            eventType: "huddle_joined",
            roomId: room.id,
            roomName: room_name_for(room, viewer),
            roomPath: room_path(room),
            joinerId: joiner.id,
            joinerName: joiner.name,
            inCall: in_call
          }
        }
      end

      def broadcast_leave(room:, leaver:, viewer:)
        ActionCable.server.broadcast HuddleNoticeChannel.stream_name_for(viewer.id), {
          huddleJoinNotice: {
            eventType: "huddle_left",
            roomId: room.id,
            roomName: room_name_for(room, viewer),
            joinerId: leaver.id,
            joinerName: leaver.name
          }
        }
      end

      # The last one out clears every other member's banner: only outsiders
      # ever held one, and nobody is left in the call to toast.
      def dismiss_outside_call(room:, leaver:)
        room.memberships.includes(:user).where.not(user_id: leaver.id).find_each do |membership|
          viewer = membership.user
          next unless active_human?(viewer)

          ActionCable.server.broadcast HuddleNoticeChannel.stream_name_for(viewer.id), {
            huddleJoinNotice: { eventType: "huddle_ended", roomId: room.id }
          }
        end
      end

      def room_name_for(room, viewer)
        if room.direct?
          room.direct_display_name(for_user: viewer)
        else
          room.name
        end
      end

      def room_path(room)
        Rails.application.routes.url_helpers.room_path(room)
      end
  end
end
