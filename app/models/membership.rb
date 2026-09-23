class Membership < ApplicationRecord
  include Connectable

  belongs_to :room
  belongs_to :user
  belongs_to :room_category, optional: true

  before_destroy -> { HuddleGrant.revoke_for_membership!(self) }
  before_destroy -> { AgentGrant.revoke_for_membership!(self) }
  before_destroy -> { Stream.end_live_for_membership!(self) }
  # The removal notice goes out before the connection reset below: once the
  # client processes the disconnect, broadcasts queued behind it are dropped.
  after_destroy_commit :broadcast_room_removal_to_user
  after_destroy_commit :reset_user_remote_connections
  after_destroy_commit :remove_thread_membership
  after_destroy_commit :sync_removed_room_calendar_entries

  enum :involvement, %w[ invisible nothing muted mentions everything ].index_by(&:itself), prefix: :involved_in

  # Stage roles only exist on stage-room memberships; every other room leaves
  # both stage columns nil. New stage members start as listeners.
  enum :stage_role, %w[ listener speaker host ].index_by(&:itself)

  before_validation :default_stage_role, on: :create
  before_update :sync_huddle_grants_on_stage_role_change, if: :stage_role_changed?

  validate :stage_attributes_only_for_stage_rooms
  validate :raised_hands_only_for_listeners
  validate :at_least_one_host_remains, on: :update, if: :stage_role_changed?
  validate :room_category_belongs_to_user

  scope :with_ordered_room, -> { includes(:room).joins(:room).order("LOWER(rooms.name)") }
  scope :without_direct_rooms, -> { joins(:room).where.not(room: { type: "Rooms::Direct" }) }

  scope :visible, -> { where.not(involvement: :invisible) }
  scope :unread,  -> { where.not(unread_at: nil) }
  scope :favorites, -> { where.not(favorite_position: nil).order(:favorite_position, :id) }

  # Reading a room advances the unread pointer to its newest root
  # message, so the next visit finds no "New messages" divider.
  def read
    update!(unread_at: nil, last_read_message_id: latest_root_message_id)
  end

  def unread?
    unread_at.present?
  end

  # The first root message after the unread pointer, if any. Rows that
  # predate the pointer (unread_at set, no pointer) fall back to the
  # first message at or after the unread stamp.
  def first_unread_message
    if last_read_message_id.present? && (reference = room.root_messages.find_by(id: last_read_message_id))
      room.root_messages.after(reference).ordered.first
    elsif unread_at.present?
      room.root_messages.where("messages.created_at >= ?", unread_at).ordered.first
    end
  end

  def unread_count
    boundary = first_unread_message
    return 0 if boundary.nil?

    unread_count_from(boundary)
  end

  def unread_count_from(boundary)
    room.root_messages.where("(messages.created_at, messages.id) >= (?, ?)", boundary.created_at, boundary.id).count
  end

  # Mark the room unread starting at the given root message: the
  # pointer moves to just before it, so the divider lands above it.
  def mark_unread_before(message)
    previous_id = room.root_messages.before(message).order(created_at: :desc, id: :desc).pick(:id)
    update!(unread_at: message.created_at, last_read_message_id: previous_id)
  end

  def favorited?
    favorite_position.present?
  end

  def favorite!
    return if favorited?

    next_position = (self.class.where(user_id: user_id).where.not(favorite_position: nil).maximum(:favorite_position) || -1) + 1
    update!(favorite_position: next_position)
  end

  def unfavorite!
    update!(favorite_position: nil)
  end

  # Move this favourite to the given 0-based position, compacting the
  # user's other favourites around it. Out-of-range positions clamp.
  def move_favorite_to(position)
    return unless favorited?

    ordered_ids = self.class.where(user_id: user_id).where.not(favorite_position: nil)
      .order(:favorite_position, :id).pluck(:id) - [ id ]
    ordered_ids.insert(position.to_i.clamp(0, ordered_ids.size), id)

    self.class.transaction do
      ordered_ids.each_with_index do |membership_id, index|
        self.class.where(id: membership_id).update_all(favorite_position: index, updated_at: Time.current)
      end
    end
  end

  def latest_root_message_id
    Message.where(room_id: room_id, thread_id: nil).order(created_at: :desc, id: :desc).pick(:id)
  end

  def raise_hand!
    update!(hand_raised_at: Time.current)
  end

  def lower_hand!
    update!(hand_raised_at: nil)
  end

  def hand_raised?
    hand_raised_at.present?
  end

  # Any role change clears a raised hand. A change that crosses the publish
  # boundary (to or from listener) revokes the member's active huddle grants
  # in the same transaction, so the gateway removes a demoted speaker and
  # the client rejoins with a fresh token for the new role; a host↔speaker
  # change keeps the same publish permission, so the grants keep their
  # identity and just record the new role.
  def change_stage_role!(new_role)
    update!(stage_role: new_role, hand_raised_at: nil)
  end

  private
    def default_stage_role
      self.stage_role ||= :listener if room&.stage?
    end

    def sync_huddle_grants_on_stage_role_change
      if (stage_role_was == "listener") != (stage_role == "listener")
        HuddleGrant.revoke_for_membership!(self)
      else
        HuddleGrant.update_role_for_membership!(self)
      end
    end

    def stage_attributes_only_for_stage_rooms
      return if room&.stage?

      errors.add(:stage_role, "only exists on stage rooms") if stage_role.present?
      errors.add(:hand_raised_at, "only exists on stage rooms") if hand_raised_at.present?
    end

    def raised_hands_only_for_listeners
      if hand_raised_at.present? && stage_role != "listener"
        errors.add(:hand_raised_at, "can only be raised by a listener")
      end
    end

    def room_category_belongs_to_user
      if room_category.present? && room_category.user_id != user_id
        errors.add(:room_category, "must belong to the member")
      end
    end

    def at_least_one_host_remains
      return unless stage_role_was == "host" && stage_role != "host"

      # The check runs inside the update transaction after locking the room:
      # without it, two concurrent demotions of the last two hosts could both
      # pass and strand the room. SQLite's immediate transaction mode
      # serializes writers, so the transaction plus a re-read inside it is
      # sufficient.
      room.lock!
      return if room.memberships.where(stage_role: :host).where.not(id: id).exists?

      errors.add(:stage_role, "can't demote the last host")
    end
    # Drop the removed member's sidebar row over their existing rooms stream,
    # the same stream the involvement toggle uses. Every room kind also drops
    # the header presence stack first: unlike the row, nothing else refreshes
    # it. Without huddle configuration no stacks exist, so there is nothing
    # to drop.
    # A failed broadcast (cable adapter outage) must never stop the
    # connection reset that follows: report it and let the callbacks run on.
    def broadcast_room_removal_to_user
      broadcast_remove_to user, :rooms, target: [ room, :header_voice_participants ] if Huddle.configured?
      broadcast_remove_to user, :rooms, target: [ room, :list ]
    rescue StandardError => error
      Rails.error.report(error, handled: true, severity: :warning, context: { membership_id: id, room_id: room_id, user_id: user_id })
    end

    def reset_user_remote_connections
      user.reset_remote_connections
    end

    def remove_thread_membership
      ThreadMembership
        .joins(:thread)
        .where(user_id: user_id, channel_threads: { room_id: room_id })
        .delete_all
    end

    def sync_removed_room_calendar_entries
      EventCalendarEntry.where(user_id: user_id).joins(:event)
        .where(events: { room_id: room_id }).pluck(:event_id)
        .each { |event_id| Calendar::SyncEntryJob.perform_later(event_id, user_id) }
    end
end
