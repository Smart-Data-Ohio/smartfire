class HuddleGrant < ApplicationRecord
  class Ineligible < StandardError; end

  INVITATION_DEDUP_WINDOW = 2.minutes
  # A retry belongs to the same attempt while the attempt's item is this
  # young; older unhandled items are left alone and the retry opens a new
  # one.
  SAME_ATTEMPT_WINDOW = 10.minutes
  IN_CALL_WINDOW = 20.seconds
  SEEN_TOUCH_INTERVAL = 10.seconds
  # A banner-only ring lives at most the client's ring timeout, so a leave
  # later than this after issuance has no banner to stop.
  CALL_ENDED_WINDOW = 1.minute

  belongs_to :session, optional: true
  belongs_to :user, optional: true
  belongs_to :membership, optional: true
  belongs_to :room, optional: true

  has_many :huddle_cleanups, dependent: :restrict_with_exception
  has_many :activity_items, as: :source, dependent: :destroy, inverse_of: :source

  scope :active, -> { where(revoked_at: nil) }
  scope :in_call, -> { where("last_seen_at > ?", IN_CALL_WINDOW.ago) }

  validates :identity, :room_name, presence: true
  validates :identity, uniqueness: true

  after_update_commit :broadcast_voice_presence, if: :saved_change_to_revoked_at?
  after_update_commit :broadcast_call_ended_to_invitee, if: :revoked_while_in_call?

  class << self
    def issue!(session:, membership:)
      attempts = 0

      begin
        grant = transaction do
          user = User.active.where.not(role: :bot).lock.find_by(id: session.user_id)
          current_session = Session.lock.find_by(id: session.id, user_id: user&.id)
          current_membership = Membership.lock.find_by(id: membership.id, user_id: user&.id, room_id: membership.room_id)
          current_room = Room.lock.find_by(id: current_membership&.room_id)

          raise Ineligible unless user && current_session && current_membership && current_room

          revoke_scope! active.where(session_id: current_session.id, room_id: current_room.id)
            .where.not(membership_id: current_membership.id)

          # One active call per session: joining room B ends this session's
          # publish in room A, so two tabs cannot publish in two rooms. Only
          # grants the gateway still sees in the call are revoked; quiet
          # ones simply stay out of the call.
          revoke_scope! active.in_call.where(session_id: current_session.id)
            .where.not(room_id: current_room.id)

          stage_role = current_room.stage? ? current_membership.stage_role : nil

          existing = active.find_by(session_id: current_session.id, membership_id: current_membership.id)
          if existing && existing.stage_role == stage_role
            existing.update!(last_issued_at: Time.current)
            existing
          else
            existing&.revoke!
            create!(
              identity: "campfire-participant-#{SecureRandom.hex(32)}",
              room_name: Huddle.room_name(current_room.id),
              session_id: current_session.id,
              user_id: user.id,
              membership_id: current_membership.id,
              room_id: current_room.id,
              stage_role: stage_role,
              last_issued_at: Time.current
            )
          end
        end

        grant.after_issued!
        grant
      rescue ActiveRecord::RecordNotUnique
        attempts += 1
        retry if attempts < 3

        raise
      end
    end

    def revoke_for_membership!(membership)
      revoke_scope! active.where(membership_id: membership.id)
    end

    # Records a host↔speaker change on the member's active grants without
    # revoking them: the publish permission is unchanged, so the call and
    # any live stream continue on the same participant identity.
    def update_role_for_membership!(membership)
      active.where(membership_id: membership.id).update_all(stage_role: membership.stage_role)
    end

    def revoke_for_session!(session)
      revoke_scope! active.where(session_id: session.id)
    end

    def revoke_for_user!(user)
      revoke_scope! active.where(user_id: user.id)
    end

    def revoke_for_room!(room)
      transaction do
        room_name = where(room_id: room.id).pick(:room_name)
        room_name ||= Huddle.room_name(room.id) if Huddle.token_signing_configured?

        revoke_scope! active.where(room_id: room.id), create_cleanup: false
        HuddleCleanup.create_room_deletion!(room_name) if room_name.present?
      end
    end

    # Who is currently in the room's huddle: distinct users with an active
    # grant the gateway has seen within the in-call window, ordered by name.
    def participants_for(room)
      active.in_call.where(room_id: room.id).includes(:user).filter_map(&:user).uniq
        .sort_by { |user| user.name.downcase }
    end

    private
      def revoke_scope!(scope, create_cleanup: true)
        scope.find_each { |grant| grant.revoke!(create_cleanup: create_cleanup) }
      end
  end

  def authorized?
    return false if revoked?

    return false unless User.active.where.not(role: :bot).exists?(id: user_id) &&
      Session.exists?(id: session_id, user_id: user_id) &&
      Room.exists?(id: room_id)

    membership = Membership.find_by(id: membership_id, user_id: user_id, room_id: room_id)
    return false unless membership

    # A stage grant is only valid for the role it was issued for. The
    # gateway's per-second check revokes through here, so a demoted speaker
    # whose grant somehow survived the role-change revocation still loses the
    # call on the next check.
    !room.stage? || membership.stage_role == stage_role
  end

  # The gateway calls this about once per second per connected participant,
  # so the steady-state authorized check runs without a lock: only a grant
  # that looks revoked takes the write lock, and rechecks inside it before
  # revoking.
  def authorize_or_revoke!
    return true if authorized?

    with_lock do
      return true if authorized?

      revoke!
      false
    end
  end

  def revoke!(create_cleanup: true)
    return if revoked?

    transaction do
      update!(revoked_at: Time.current)
      HuddleCleanup.create_participant_removal!(self) if create_cleanup
      Stream.end_when_last_grant_revoked(self)
    end
  end

  def revoked?
    revoked_at.present?
  end

  def in_call?
    last_seen_at.present? && last_seen_at > IN_CALL_WINDOW.ago
  end

  # Drops the grant from the call without revoking it: leaving the call does
  # not end the session's authorization, it only stops counting as present.
  # Called from the explicit leave endpoint and from the gateway's
  # disconnect event, both of which broadcast fresh presence. The optional
  # floor keeps a stale disconnect event from clobbering a liveness
  # sighting that landed after the disconnect (quick rejoin).
  def mark_out_of_call!(seen_after: nil)
    return false if last_seen_at.nil?
    return false if seen_after.present? && last_seen_at > seen_after

    was_in_call = in_call?
    update_columns(last_seen_at: nil)
    broadcast_voice_presence
    broadcast_call_ended_to_invitee if was_in_call
    true
  end

  # The gateway checks every connected participant about once per second, so
  # liveness is persisted at most every SEEN_TOUCH_INTERVAL, and the
  # first-sighting presence refresh goes through a job instead of rendering
  # and broadcasting synchronously inside the check.
  def record_seen!
    return if last_seen_at.present? && last_seen_at > SEEN_TOUCH_INTERVAL.ago

    first_seen = !in_call?
    update_columns(last_seen_at: Time.current)
    Huddle::BroadcastPresenceJob.perform_later(id) if first_seen
  end

  # Post-commit work for every issuance, created or reused: obtaining a grant
  # means joining the room's call, so the issuer's own open invitations for
  # the room are handled and the DM peer rings for a new call.
  def after_issued!
    clear_open_invitations!
    invite_direct_participant
    broadcast_voice_presence
  end

  def authorization_payload
    { grant_id: id, room_name: room_name, identity: identity }
  end

  # Contract consumed by ActivityItems::Recorder. Only the other human in a
  # one-to-one DM can receive a huddle invitation from this grant.
  def activity_recipient_ids
    [ direct_huddle_recipient&.id ].compact
  end

  # Refresh the presence stacks in the room members' sidebars and in the
  # room header, for every room kind. The sidebar partial renders once and
  # the same HTML goes to each member's rooms stream, so a 200-member
  # channel does not render 200 times per join.
  def broadcast_voice_presence
    return unless Huddle.configured?

    huddle_room = Room.find_by(id: room_id)
    return unless huddle_room

    participants = self.class.participants_for(huddle_room)
    sidebar_html = ApplicationController.render(
      partial: "rooms/huddles/participants",
      locals: { room: huddle_room, placement: :sidebar, participants: participants }
    )

    huddle_room.memberships.includes(:user).find_each do |membership|
      broadcast_replace_to membership.user, :rooms,
        target: [ huddle_room, :sidebar_voice_participants ],
        html: sidebar_html
    end

    broadcast_replace_to huddle_room, :messages,
      target: [ huddle_room, :header_voice_participants ],
      partial: "rooms/huddles/participants",
      locals: { room: huddle_room, placement: :header, participants: participants }
  end

  private
    # Joining late clears even a missed item: any open invitation for this
    # room and recipient is handled as soon as they obtain a grant here.
    def clear_open_invitations!
      open_invitations_for(user_id).find_each(&:mark_handled!)
    end

    # A huddle "starts" for a DM when a grant is issued while the other
    # participant is not in the call. Grants persist per session, so issuance
    # (created or reused) drives the ring and the dedup window guards it: any
    # invitation or missed item from the last two minutes, handled or not,
    # keeps reconnects and rejoins silent.
    def invite_direct_participant
      # Voice and stage channels are standing calls that members join at
      # will: nobody is ever invited, rung, or marked as missing the call.
      return if room.is_a?(Rooms::Voice) || room.is_a?(Rooms::Stage)

      recipient = direct_huddle_recipient
      return unless recipient
      return if HuddleGrant.in_call.where(room_id: room_id, user_id: recipient.id).exists?
      return if recent_invitation?(recipient)
      return if recent_grant_issuance?

      unless recipient.inbox_preferences.huddle_invitations
        broadcast_suppressed_invitation!(recipient)
        return
      end

      # The retrying grant re-rings through the row it already owns for
      # the recipient when one exists, handled or not: repointing another
      # attempt's row at this grant instead would collide with that owned
      # row on the unique recipient-plus-source index, and the recipient
      # would never be rung. With the owned row claimed here, the rows the
      # same-attempt lookup below can return never collide.
      if (owned = owned_invitation_item(recipient))
        return unless refresh_invitation!(owned)

        Huddle::PushInvitationJob.perform_later(owned.id)
        return
      end

      # The same attempt re-rings through its own row even when the retry
      # comes from another session with a brand-new grant: same room, same
      # starter, same recipient, and the previous item still unhandled. A
      # handled item proves the recipient joined, closing the attempt, so
      # the next invite opens a new one with its own item.
      if (attempt = same_attempt_item(recipient))
        return unless refresh_attempt!(attempt)

        Huddle::PushInvitationJob.perform_later(attempt.id)
        return
      end

      item = ActivityItems::Recorder.record!(recipient:, source: self, event_type: "huddle_started")
      return unless item
      return unless item.previously_new_record? || refresh_invitation!(item)

      Huddle::PushInvitationJob.perform_later(item.id)
    end

    # The row this grant already owns for the recipient, if any.
    def owned_invitation_item(recipient)
      ActivityItem.find_by(user_id: recipient.id, source_type: HuddleGrant.polymorphic_name, source_id: id)
    end

    # The recipient's unhandled huddle item for this room from this starter,
    # if one is still young enough to be the same attempt: a retry of the
    # attempt it belongs to reuses it (repointed at the new grant) instead
    # of stacking a second missed-call item beside it. Only unhandled rows
    # qualify — ringing, missed, or dismissed — so a recipient who joined
    # keeps their history and the retry reads as new, and only rows from
    # the last ten minutes qualify, so a retry after a long silence opens
    # a new item instead of resurrecting a stale one.
    def same_attempt_item(recipient)
      invitations_for(recipient.id)
        .where(handled_at: nil, invitation_grants: { user_id: user_id })
        .where(activity_items: { created_at: SAME_ATTEMPT_WINDOW.ago.. })
        .order(created_at: :desc)
        .first
    end

    # Resets the same attempt's row in place under a lock, repointed at the
    # retrying grant so the banner and the push describe the live call. A
    # row the recipient handled concurrently — they joined while the retry
    # was in flight — is left alone and the retry stays silent.
    def refresh_attempt!(item)
      refreshed = false

      item.with_lock do
        unless item.handled? || item.created_at >= INVITATION_DEDUP_WINDOW.ago
          item.update!(source: self, event_type: "huddle_started", read_at: nil, handled_at: nil, created_at: Time.current)
          refreshed = true
        end
      end

      refreshed
    end

    # Inbox identity is recipient + source, so a reused grant re-rings through
    # the same row. The row is reset in place under a lock so the recipient's
    # existing links stay valid and two concurrent issuances cannot both ring;
    # a row another issuance refreshed inside the window stands as is.
    def refresh_invitation!(item)
      refreshed = false

      item.with_lock do
        if stale_invitation?(item)
          item.update!(event_type: "huddle_started", read_at: nil, handled_at: nil, created_at: Time.current)
          refreshed = true
        end
      end

      refreshed
    end

    def direct_huddle_recipient
      return unless room.is_a?(Rooms::Direct)

      member_ids = room.memberships.pluck(:user_id)
      return unless member_ids.size == 2
      return unless User.active.without_bots.where(id: member_ids).count == 2

      other_id = (member_ids - [ user_id ]).first
      return unless other_id

      recipient = User.active.without_bots.find_by(id: other_id)
      return unless recipient
      return if room.memberships.where(user_id: other_id, involvement: %w[ nothing invisible ]).exists?

      recipient
    end

    def recent_invitation?(recipient)
      invitations_for(recipient.id)
        .where(activity_items: { created_at: INVITATION_DEDUP_WINDOW.ago.. })
        .exists?
    end

    # The item query above cannot throttle the suppressed path, which never
    # creates an item, so a second grant for this room and starter inside
    # the same window also stays silent: rejoins and reconnects ring once.
    def recent_grant_issuance?
      previous_issue = last_issued_at_previously_was
      return true if previous_issue && previous_issue >= INVITATION_DEDUP_WINDOW.ago

      HuddleGrant.where(room_id: room_id, user_id: user_id)
        .where(created_at: INVITATION_DEDUP_WINDOW.ago..)
        .where.not(id: id)
        .exists?
    end

    def open_invitations_for(user_id)
      invitations_for(user_id).where(handled_at: nil)
    end

    def invitations_for(user_id)
      ActivityItem
        .where(user_id:, source_type: HuddleGrant.polymorphic_name, event_type: ActivityItem::HUDDLE_EVENT_TYPES)
        .joins("INNER JOIN huddle_grants AS invitation_grants ON invitation_grants.id = activity_items.source_id")
        .where(invitation_grants: { room_id: room_id })
    end

    def stale_invitation?(item)
      item.handled? || item.event_type != "huddle_started" || item.created_at < INVITATION_DEDUP_WINDOW.ago
    end

    # Only a revocation that actually ends a call rings the ended bell: a
    # grant that was already quiet had no live banner to stop.
    def revoked_while_in_call?
      saved_change_to_revoked_at? && in_call?
    end

    # Tells the DM peer's banner the call ended: the starter hung up, was
    # removed, or joined another room. Open invitations for this grant flip
    # the banner to a "caller left" state instead of ringing until the
    # missed-call resolution; a banner-only ring (items switched off) gets
    # the same payload with a zero id. The items themselves still resolve
    # to missed calls on their own schedule.
    def broadcast_call_ended_to_invitee
      return unless room && user

      delivered = false
      open_call_items.find_each do |item|
        ActionCable.server.broadcast ActivityChannel.stream_name_for(item.user_id), call_ended_payload(item)
        delivered = true
      end
      broadcast_suppressed_call_ended! unless delivered
    end

    def open_call_items
      ActivityItem.where(
        source_type: HuddleGrant.polymorphic_name, source_id: id,
        event_type: "huddle_started", handled_at: nil
      )
    end

    def call_ended_payload(item)
      routes = Rails.application.routes.url_helpers
      {
        activityItemId: item.id,
        huddleInvitation: {
          activityItemId: item.id,
          eventType: "huddle_ended",
          state: item.state,
          roomId: room.id,
          roomName: suppressed_invitation_room_name(item.user),
          roomPath: routes.room_path(room),
          callerName: user.name,
          readPath: routes.read_activity_item_path(item, state: "read"),
          handledPath: routes.handled_activity_item_path(item, state: "handled")
        }
      }
    end

    def broadcast_suppressed_call_ended!
      recipient = direct_huddle_recipient
      return unless recipient
      return if recipient.inbox_preferences.huddle_invitations
      return unless last_issued_at.present? && last_issued_at > CALL_ENDED_WINDOW.ago
      return unless ActivityItem.active_human?(recipient)

      routes = Rails.application.routes.url_helpers
      ActionCable.server.broadcast ActivityChannel.stream_name_for(recipient.id), {
        activityItemId: 0,
        huddleInvitation: {
          activityItemId: 0,
          eventType: "huddle_ended",
          state: "unread",
          roomId: room.id,
          roomName: suppressed_invitation_room_name(recipient),
          roomPath: routes.room_path(room),
          callerName: user&.name || "Someone",
          readPath: "",
          handledPath: ""
        }
      }
    end

    # With huddle inbox items switched off, the call still rings in-app
    # through a banner payload without an item. Join and Dismiss skip the
    # read/handled round-trip that needs an item id. The payload carries no
    # nulls: empty paths and a zero id read as "no item" in the Stimulus
    # guards, where nil would arrive as the string "null" and NaN.
    def broadcast_suppressed_invitation!(recipient)
      return unless ActivityItem.active_human?(recipient)

      routes = Rails.application.routes.url_helpers
      ActionCable.server.broadcast ActivityChannel.stream_name_for(recipient.id), {
        activityItemId: 0,
        huddleInvitation: {
          activityItemId: 0,
          eventType: "huddle_started",
          state: "unread",
          roomId: room.id,
          roomName: suppressed_invitation_room_name(recipient),
          roomPath: routes.room_path(room),
          callerName: user&.name || "Someone",
          readPath: "",
          handledPath: ""
        }
      }
    end

    def suppressed_invitation_room_name(recipient)
      if room.direct?
        room.users.without(recipient).pluck(:name).to_sentence.presence || recipient.name
      else
        room.name
      end
    end
end
