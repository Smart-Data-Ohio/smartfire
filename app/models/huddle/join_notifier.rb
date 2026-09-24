class Huddle::JoinNotifier
  # A viewer whose ring is still live (or only just timed out) already has
  # the loudest possible notice, so a join inside this window stays quiet
  # for them. Older unhandled rings are stale — the banner hid itself at
  # the 45-second ring timeout — and a join notice is welcome again.
  RECENT_RING_WINDOW = 60.seconds

  # A join whose user had an in-call grant revoked inside this window is a
  # mute-cycle rejoin, not a genuine join: the revoke dropped them and the
  # client came straight back with a fresh grant. The window matches the
  # join-notice client's leave toast delay (leaveDelayValue, five seconds
  # in huddle_join_notice_controller.js): the revoke's leave broadcasts
  # inline and toasts once the delay passes, so a revoke older than that
  # means the viewer already saw "left" and the join must toast as
  # genuine. Keep the two in step.
  REJOIN_WINDOW = 5.seconds

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
      memberships = room.memberships.includes(:user).where.not(user_id: joiner.id).to_a
      return if memberships.empty?

      # Direct-room display names are per-viewer but computed from one
      # member list, and the recent-ring check runs once for the room:
      # the fan-out below stays flat in queries as the group grows.
      direct = room.direct?
      members = room.users.ordered.to_a if direct
      rung_ids = recently_rung_ids(room, memberships.map(&:user_id)) if direct
      rejoin = rejoin_after_revoke?(room, joiner, grant)

      memberships.each do |membership|
        viewer = membership.user
        next unless active_human?(viewer)

        if in_call_ids.include?(viewer.id)
          broadcast_join(room:, joiner:, viewer:, in_call: true, members:, rejoin:)
        elsif direct && noticeable_outside_call?(membership, rung_ids)
          broadcast_join(room:, joiner:, viewer:, in_call: false, members:, rejoin:)
          Huddle::JoinPusher.new(grant:, recipient: viewer, room_membership: membership).push
        end
      end
    end

    # Tells the members still in the call that the grant's user left it.
    # Runs inline from the leave report and revocation paths, in every
    # room kind, in-call viewers only. In DMs the members outside the
    # call hear about the leave too, so their join banners drop the
    # leaver while the others remain. When the last participant drops
    # out of a DM call, the room's other members get a dismissal instead
    # so their join banners clear immediately instead of waiting out the
    # presence poll.
    def notify_leave(grant)
      room = Room.alive.find_by(id: grant.room_id)
      leaver = grant.user
      return unless room && active_human?(leaver)
      return if HuddleGrant.active.in_call.where(room_id: room.id, user_id: leaver.id).where.not(id: grant.id).exists?

      in_call_ids = in_call_user_ids(room) - [ leaver.id ]
      if in_call_ids.any?
        members = room.users.ordered.to_a if room.direct?
        User.active.without_bots.where(id: in_call_ids).find_each do |viewer|
          broadcast_leave(room:, leaver:, viewer:, members:)
        end
        broadcast_leave_to_outsiders(room:, leaver:, in_call_ids:, members:) if room.direct?
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

      # True when the joiner's previous grant for the room was revoked out
      # of a live call inside the rejoin window: a server mute, role
      # change, or kick whose client came straight back with this grant.
      # The revoked grant's liveness sighting is the proof it was in the
      # call — nothing clears it on revoke, and the gateway's disconnect
      # report only posts after the three-second reconnect grace, while a
      # rejoined client sights its new grant sooner; by the time the
      # report could clear the sighting, the leave it follows is seconds
      # old and no delivery race can still flip the order. A revoke of a
      # quiet grant — muted while away, then joining — marks nothing, and
      # the genuine join toasts as usual.
      def rejoin_after_revoke?(room, joiner, grant)
        HuddleGrant.where(room_id: room.id, user_id: joiner.id)
          .where.not(id: grant.id)
          .where(revoked_at: REJOIN_WINDOW.ago..)
          .where(last_seen_at: HuddleGrant::IN_CALL_WINDOW.ago..)
          .exists?
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
      def noticeable_outside_call?(membership, rung_ids)
        return false if membership.involved_in_invisible? || membership.involved_in_nothing?

        !rung_ids.include?(membership.user_id)
      end

      def recently_rung_ids(room, user_ids)
        ActivityItem
          .where(user_id: user_ids, source_type: HuddleGrant.polymorphic_name, event_type: "huddle_started", handled_at: nil)
          .where(created_at: RECENT_RING_WINDOW.ago..)
          .joins("INNER JOIN huddle_grants AS rung_grants ON rung_grants.id = activity_items.source_id")
          .where(rung_grants: { room_id: room.id })
          .pluck(:user_id).to_set
      end

      def broadcast_join(room:, joiner:, viewer:, in_call:, members:, rejoin:)
        ActionCable.server.broadcast HuddleNoticeChannel.stream_name_for(viewer.id), {
          huddleJoinNotice: {
            eventType: "huddle_joined",
            roomId: room.id,
            roomName: room_name_for(room, viewer, members),
            roomPath: room_path(room),
            joinerId: joiner.id,
            joinerName: joiner.name,
            inCall: in_call,
            rejoin: rejoin
          }
        }
      end

      def broadcast_leave(room:, leaver:, viewer:, members:)
        ActionCable.server.broadcast HuddleNoticeChannel.stream_name_for(viewer.id), {
          huddleJoinNotice: {
            eventType: "huddle_left",
            roomId: room.id,
            roomName: room_name_for(room, viewer, members),
            joinerId: leaver.id,
            joinerName: leaver.name
          }
        }
      end

      # A leave while others remain in the call still reaches the DM's
      # outsiders: their banners drop the leaver from the roster instead
      # of listing someone already gone.
      def broadcast_leave_to_outsiders(room:, leaver:, in_call_ids:, members:)
        room.memberships.includes(:user).where.not(user_id: [ leaver.id, *in_call_ids ]).find_each do |membership|
          viewer = membership.user
          next unless active_human?(viewer)

          broadcast_leave(room:, leaver:, viewer:, members:)
        end
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

      def room_name_for(room, viewer, members)
        if room.direct?
          room.direct_display_name(for_user: viewer, members: members)
        else
          room.name
        end
      end

      def room_path(room)
        Rails.application.routes.url_helpers.room_path(room)
      end
  end
end
