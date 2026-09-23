class Room < ApplicationRecord
  has_many :memberships, dependent: :delete_all do
    def grant_to(users)
      room = proxy_association.owner
      Membership.insert_all(Array(users).collect { |user| { room_id: room.id, user_id: user.id, involvement: room.default_involvement } })
    end

    def revoke_from(users)
      destroy_by user: users
    end

    def revise(granted: [], revoked: [])
      transaction do
        grant_to(granted) if granted.present?
        revoke_from(revoked) if revoked.present?
      end
    end
  end

  has_many :users, through: :memberships
  has_many :messages, dependent: :destroy
  has_many :message_pins, dependent: :delete_all
  has_many :root_messages, -> { where(thread_id: nil) }, class_name: "Message", foreign_key: :room_id
  has_many :channel_threads, dependent: :destroy
  has_many :events, dependent: :destroy
  has_many :hosted_events, class_name: "Event", foreign_key: :venue_room_id, dependent: :nullify
  has_many :github_repository_subscriptions, class_name: "Github::RepositorySubscription", dependent: :destroy
  # The agent ledger outlives the room; only the room link is cleared.
  has_many :agent_events, dependent: :nullify
  has_many :agent_approvals, dependent: :nullify

  belongs_to :creator, class_name: "User", default: -> { Current.user }

  before_destroy -> { HuddleGrant.revoke_for_room!(self) }
  before_destroy -> { AgentGrant.revoke_for_room!(self) }
  validate :direct_rooms_keep_their_type, on: :update

  normalizes :icon_name, with: ->(name) { Icons.normalize_name(name) }
  validate :icon_name_must_resolve, if: :icon_name_changed?

  scope :opens,           -> { where(type: "Rooms::Open") }
  scope :closeds,         -> { where(type: "Rooms::Closed") }
  scope :directs,         -> { where(type: "Rooms::Direct") }
  scope :voices,          -> { where(type: "Rooms::Voice") }
  scope :boards,          -> { where(type: "Rooms::Board") }
  scope :without_directs, -> { where.not(type: "Rooms::Direct") }

  scope :ordered, -> { order("LOWER(name)") }

  # Soft-deleted rooms grant nothing: every access path reads through alive.
  scope :alive, -> { where(deleted_at: nil) }
  scope :deleted, -> { where.not(deleted_at: nil) }
  # Destroys never enqueued, or enqueued longer ago than the cutoff: the
  # stuck-room sweep's claim candidates.
  scope :destroy_unclaimed_before, ->(cutoff) { where("destroy_enqueued_at IS NULL OR destroy_enqueued_at < ?", cutoff) }

  class << self
    def create_for(attributes, users:)
      transaction do
        create!(attributes).tap do |room|
          room.memberships.grant_to users
        end
      end
    end

    def original
      order(:created_at).first
    end
  end

  def receive(message)
    return if message.system_note?

    unread_memberships(message)
    push_later(message)
  end

  def deleted?
    deleted_at.present?
  end

  # First half of asynchronous room deletion (see RoomsController#destroy).
  # Synchronously removes the room from everyone — with no memberships the
  # room disappears and every membership-based access check fails — revokes
  # huddle and agent access, ends live streams, and marks the room for
  # Room::DestroyJob, which removes the remaining content in batches.
  def begin_destroy!
    transaction do
      update!(deleted_at: Time.current)
      memberships.delete_all
      HuddleGrant.revoke_for_room!(self)
      AgentGrant.revoke_for_room!(self)
      Stream.live.where(room_id: id).find_each(&:end!)
    end
  end

  def open?
    is_a?(Rooms::Open)
  end

  def closed?
    is_a?(Rooms::Closed)
  end

  def direct?
    is_a?(Rooms::Direct)
  end

  def voice?
    is_a?(Rooms::Voice)
  end

  def stage?
    is_a?(Rooms::Stage)
  end

  def board?
    is_a?(Rooms::Board)
  end

  def default_involvement
    "mentions"
  end

  private
    def icon_name_must_resolve
      if icon_name.present? && Icons.find(icon_name).nil?
        errors.add :icon_name, "is not a known icon"
      end
    end

    # Open and closed rooms convert into each other freely. A direct room can't become
    # either: its participants agreed to a private conversation, not to one whose
    # audience someone else gets to widen afterwards.
    def direct_rooms_keep_their_type
      if type_changed? && type_was == "Rooms::Direct"
        errors.add :type, "can't be changed for a direct room"
      end
    end

    def unread_memberships(message)
      memberships.visible.disconnected.where.not(user: message.creator).update_all(unread_at: message.created_at, updated_at: Time.current)
    end

    def push_later(message)
      Room::PushMessageJob.perform_later(self, message)
    end
end
