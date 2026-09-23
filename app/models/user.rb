class User < ApplicationRecord
  include Avatar, Bannable, Bot, Mentionable, Role, StatusSettings, Transferable

  has_many :memberships, dependent: :delete_all
  # Listings, room scopes, and reachable messages all read through here, so
  # soft-deleted rooms disappear from every one of them at once.
  has_many :rooms, -> { alive }, through: :memberships

  has_many :reachable_messages, through: :rooms, source: :messages
  has_many :messages, dependent: :destroy, foreign_key: :creator_id
  has_many :channel_threads, dependent: :destroy, foreign_key: :creator_id
  has_many :thread_memberships, class_name: "ThreadMembership", dependent: :destroy
  has_many :followed_threads, through: :thread_memberships, source: :thread

  has_many :push_subscriptions, class_name: "Push::Subscription", dependent: :delete_all

  has_one :google_account, dependent: :destroy
  has_one :google_identity, dependent: :destroy
  has_one :github_connected_account, dependent: :destroy
  has_one :fizzy_connected_account, dependent: :destroy
  has_many :event_calendar_entries, dependent: :destroy

  has_many :boosts, dependent: :destroy, foreign_key: :booster_id
  # Destroyed (not deleted) so each removed pin broadcasts its badge,
  # count, and panel updates through the pin commit callbacks. The
  # message cascade above runs first, so pins on the user's own messages
  # are already gone when this association loads: no pin broadcasts twice.
  has_many :message_pins, dependent: :destroy, foreign_key: :pinner_id
  has_many :saved_items, dependent: :delete_all
  has_many :searches, dependent: :delete_all
  has_many :room_categories, dependent: :destroy

  has_many :sessions, dependent: :destroy
  has_many :workspace_presence_leases, dependent: :delete_all
  has_many :bans, dependent: :destroy

  enum :status, %i[ active deactivated banned ], default: :active

  VOICE_MODES = %w[ voice_activity push_to_talk ].freeze
  DEFAULT_PUSH_TO_TALK_KEY = "`".freeze

  normalizes :github_login, with: ->(login) { login.to_s.strip.downcase.presence }

  validates :github_login, uniqueness: { case_sensitive: false, message: "is already linked to another user" }, allow_nil: true
  validate :github_login_must_match_verified_account, if: :will_save_change_to_github_login?
  validate :inbox_preferences_must_be_boolean
  validate :voice_settings_must_be_valid

  normalizes :push_to_talk_key, with: ->(key) { key.to_s.strip.presence }

  normalizes :icon_name, with: ->(name) { Icons.normalize_name(name) }
  validate :icon_name_must_resolve, if: :icon_name_changed?

  before_update -> { HuddleGrant.revoke_for_user!(self) }, if: -> { will_save_change_to_status? && !active? }
  before_destroy -> { HuddleGrant.revoke_for_user!(self) }, prepend: true
  before_update -> { AgentGrant.revoke_for_user!(self) }, if: -> { will_save_change_to_status? && !active? }
  before_destroy -> { AgentGrant.revoke_for_user!(self) }, prepend: true

  has_secure_password validations: false

  # Users whose memberships are managed explicitly (like the GitHub bot)
  # skip the automatic open-room grant at creation.
  attr_accessor :skip_open_room_grant

  after_create_commit :grant_membership_to_open_rooms, unless: :skip_open_room_grant

  scope :ordered, -> { order("LOWER(name)") }
  scope :filtered_by, ->(query) { where("name like ?", "%#{query}%") }

  # Live presence for cards, the member panel, and the people directory.
  # Bots hold no presence lease, so a bot with an agent reads live from the
  # agent instead, like the agent profiles do: not suspended, and checked
  # in at least once. Pass preloaded lease ids to avoid a query per user.
  def online_now?(online_ids = nil)
    return false unless active?

    if bot? && agent
      agent.suspended_at.nil? && agent.last_seen_at.present?
    else
      (online_ids || WorkspacePresenceLease.online_user_ids([ id ])).include?(id)
    end
  end

  # Per-integration inbox switches. Missing keys read as true so existing
  # users keep today's behavior; only explicit false suppresses an item.
  def inbox_preferences
    User::InboxPreferences.new(self[:inbox_preferences])
  end

  def inbox_preferences=(value)
    hash = value.is_a?(ActionController::Parameters) ? value.to_unsafe_h : value
    unless hash.nil? || hash.is_a?(Hash)
      self[:inbox_preferences] = value
      return
    end

    existing = self[:inbox_preferences]
    existing = {} unless existing.is_a?(Hash)
    self[:inbox_preferences] = existing.merge((hash || {}).stringify_keys.slice(*User::InboxPreferences::KEYS))
  end

  # True while a connected GitHub account vouches for the login: GitHub
  # confirmed it when the token was linked, so the profile cannot edit it.
  def github_login_verified?
    github_connected_account&.connected? || false
  end

  # How the microphone opens in calls: always live, or only while the
  # push-to-talk key is held. Nil reads as voice activity, so existing users
  # keep today's behavior until they switch.
  def voice_mode
    self[:voice_mode].presence_in(VOICE_MODES) || "voice_activity"
  end

  def push_to_talk?
    voice_mode == "push_to_talk"
  end

  # The KeyboardEvent.key held to talk in push-to-talk mode: a character key
  # like "`" or a named key like "CapsLock". Nil reads as the backtick.
  def push_to_talk_key
    self[:push_to_talk_key].presence || DEFAULT_PUSH_TO_TALK_KEY
  end

  def initials
    name.scan(/\b\w/).join
  end

  def title
    [ name, bio ].compact_blank.join(" – ")
  end

  def deactivate
    calendar_event_ids = nil

    # The push channel stops remotely before the transaction: the HTTP
    # call must not hold the database write lock, and the Google account
    # is still usable here. The row itself is destroyed inside, so a
    # failed stop still removes the local channel.
    push_channel = Calendar::PushChannel.find_by(user_id: id)
    push_channel&.stop_remote!

    transaction do
      close_remote_connections

      # A sole stage host would otherwise vanish with every other membership
      # below, bypassing the sole-host check and leaving listeners with no
      # manager. Promote a successor first, in this same transaction.
      promote_replacement_stage_hosts

      # Grant revocation below ends streams only when a grant exists; a
      # presenter who never joined still holds a live stream, so end those
      # explicitly in this same transaction.
      Stream.end_live_for_user!(self)

      # delete_all skips the membership hook, so capture the entries now
      # for cleanup syncs after commit.
      calendar_event_ids = event_calendar_entries.pluck(:event_id)
      memberships.without_direct_rooms.delete_all
      push_subscriptions.delete_all
      searches.delete_all
      sessions.delete_all
      Calendar::DisconnectCleanupJob.perform_later([], google_account.cleanup_snapshot, google_account.id) if google_account&.usable?
      push_channel&.destroy!
      google_account&.mark_disconnected!("Account deactivated")
      github_connected_account&.mark_disconnected!("Account deactivated")
      fizzy_connected_account&.mark_disconnected!("Account deactivated")
      # Agents this person owns stop with them: suspension revokes their
      # grants, suspended agents' Bearer tokens are refused (401), and their
      # bot keys fail every capability check (403).
      Agent.where(owner_id: id).find_each(&:suspend!)

      update! status: :deactivated, email_address: deactived_email_address
    end

    calendar_event_ids.each { |event_id| Calendar::SyncEntryJob.perform_later(event_id, id) }
  end

  def reset_remote_connections
    close_remote_connections reconnect: true
  end

  private
    def icon_name_must_resolve
      if icon_name.present? && Icons.find(icon_name).nil?
        errors.add :icon_name, "is not a known icon"
      end
    end

    # For every stage room where this user is the only host and other members
    # remain, promote one remaining member to host before the memberships are
    # deleted: an active administrator member is preferred, otherwise the
    # earliest-created remaining membership. Rooms with another host already,
    # and rooms left empty by the deactivation, are left alone.
    def promote_replacement_stage_hosts
      memberships.includes(:room).where(stage_role: :host).select { |membership| membership.room.stage? }.each do |host_membership|
        remaining = host_membership.room.memberships.includes(:user).where.not(user_id: id).order(:created_at).to_a
        next if remaining.empty?
        next if remaining.any?(&:host?)

        successor = remaining.find { |membership| membership.user.active? && membership.user.administrator? } || remaining.first
        successor.change_stage_role!("host")
      end
    end

    def github_login_must_match_verified_account
      return unless github_login_verified?
      return if github_login == github_connected_account.github_login.to_s.strip.downcase

      errors.add(:github_login, "is set by your linked GitHub account")
    end

    def inbox_preferences_must_be_boolean
      raw = self[:inbox_preferences]
      unless raw.nil? || raw.is_a?(Hash)
        errors.add(:inbox_preferences, "is invalid")
        return
      end

      (raw || {}).each do |key, value|
        unless User::InboxPreferences.boolean_value?(value)
          errors.add(:"inbox_preferences.#{key}", "must be true or false")
        end
      end
    end

    # Reads the raw columns: the voice_mode and push_to_talk_key readers
    # normalize nil to their defaults, which would mask invalid values from
    # a plain inclusion check.
    def voice_settings_must_be_valid
      if self[:voice_mode].present? && !self[:voice_mode].in?(VOICE_MODES)
        errors.add(:voice_mode, "is invalid")
      end

      if self[:push_to_talk_key].present? && self[:push_to_talk_key].length > 20
        errors.add(:push_to_talk_key, "is too long")
      end
    end

    def grant_membership_to_open_rooms
      Membership.insert_all(Rooms::Open.alive.pluck(:id).collect { |room_id| { room_id: room_id, user_id: id } })
    end

    def deactived_email_address
      email_address&.gsub(/@/, "-deactivated-#{SecureRandom.uuid}@")
    end

    def close_remote_connections(reconnect: false)
      ActionCable.server.remote_connections.where(current_user: self).disconnect reconnect: reconnect
    end
end
