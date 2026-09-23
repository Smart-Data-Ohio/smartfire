# Bot keys are "<id>-<token>". Authentication compares the token's SHA-256
# digest (bot_token_digest), as agent credentials do, and the UI shows a key
# only once: right after creation or a reset.
#
# The plaintext bot_token column is retired: it is never written and
# never read for authentication. It cannot be dropped because production
# migrations must stay strictly additive, so a runtime task
# (Bots::ClearPlaintextTokens, via
# `bin/rails bots:clear_plaintext_tokens` and a one-time Periodic::Runner
# task) recomputes each leftover row's digest from its plaintext, then
# nulls the plaintext. New and reset bots store the digest alone, and
# stored bots answer bot_key with BOT_KEY_PLACEHOLDER.
module User::Bot
  extend ActiveSupport::Concern

  # Stands in for the key wherever it has to appear but is not shown, such
  # as the curl examples on the bots page.
  BOT_KEY_PLACEHOLDER = "BOT_KEY"

  # How long a webhook reply URL stays usable. Each delivery mints a fresh
  # one, so the window only needs to cover the receiver's prompt POST back.
  REPLY_URL_EXPIRY = 15.minutes

  included do
    scope :active_bots, -> { active.where(role: :bot) }
    scope :without_bots, -> { where.not(role: :bot) }
    has_one :webhook, dependent: :delete
    has_one :agent, dependent: :delete
  end

  module ClassMethods
    def create_bot!(attributes)
      bot_token = generate_bot_token
      webhook_url = attributes.delete(:webhook_url)

      User.create!(**attributes, bot_token: nil, bot_token_digest: digest_bot_token(bot_token), role: :bot).tap do |user|
        user.plain_bot_token = bot_token
        user.create_webhook!(url: webhook_url) if webhook_url
      end
    end

    # Looks the bot up by id and compares digests in constant time. The
    # plaintext column is never consulted: a row without a digest (written
    # by a release before digests) cannot authenticate until its key is
    # reset.
    def authenticate_bot(bot_key)
      bot_id, bot_token = bot_key.to_s.split("-", 2)
      return if bot_id.blank? || bot_token.blank? || !bot_id.match?(/\A\d+\z/)

      bot = active_bots.find_by(id: bot_id)
      return if bot.nil? || bot.bot_token_digest.blank?

      bot if ActiveSupport::SecurityUtils.secure_compare(bot.bot_token_digest, digest_bot_token(bot_token))
    end

    # Verifies a signed webhook reply token minted by #reply_token_for.
    # The token binds the bot to one room and expires; the bot must still
    # be active and a member of that room.
    def authenticate_bot_reply_token(token, room_id:)
      data = reply_verifier.verified(token.to_s.strip)
      return unless data.is_a?(Hash) && data["room_id"].to_s == room_id.to_s

      bot = active_bots.find_by(id: data["bot_id"])
      return if bot.nil?
      return unless bot.rooms.exists?(room_id)

      bot
    end

    def generate_bot_token
      SecureRandom.alphanumeric(12)
    end

    def digest_bot_token(token)
      Digest::SHA256.hexdigest(token.to_s)
    end

    def reply_verifier
      Rails.application.message_verifier("bot_reply")
    end
  end

  def update_bot!(attributes)
    update_bot(attributes) || raise(ActiveRecord::RecordInvalid, self)
  end

  def update_bot(attributes)
    success = false

    transaction do
      # Only a submitted webhook_url (blank included) changes the webhook; an
      # edit that leaves the key out keeps it.
      update_webhook_url!(attributes.delete(:webhook_url)) if attributes.key?(:webhook_url)
      success = update(attributes)
      raise ActiveRecord::Rollback unless success
    end

    success
  end


  # The plaintext token, known only in the object that just created or
  # reset it. Never persisted.
  attr_accessor :plain_bot_token

  # The full key while it is still known, otherwise nil.
  def plain_bot_key
    "#{id}-#{plain_bot_token}" if plain_bot_token.present?
  end

  # The full key while it is still known (just created or reset),
  # otherwise the placeholder. Stored bots never reveal their key: the
  # plaintext column is retired and unread.
  def bot_key
    plain_bot_key || BOT_KEY_PLACEHOLDER
  end

  # Issues a new token, invalidating the old key, and returns the new key.
  # Stores the digest alone; any retired plaintext is cleared with it.
  def reset_bot_key
    token = self.class.generate_bot_token
    update! bot_token: nil, bot_token_digest: self.class.digest_bot_token(token)
    self.plain_bot_token = token
    plain_bot_key
  end

  # A signed, expiring token authenticating this bot for one room's bot
  # posting endpoint (create only). Carried in legacy webhook payloads as
  # reply_url so receivers can post back without a long-lived key.
  def reply_token_for(room, expires_in: REPLY_URL_EXPIRY)
    self.class.reply_verifier.generate({ bot_id: id, room_id: room.id }, expires_in: expires_in)
  end


  def webhook_url
    webhook&.url
  end

  def deliver_webhook_later(message)
    Bot::WebhookJob.perform_later(self, message) if webhook
  end

  def deliver_webhook(message)
    webhook.deliver(message)
  end


  private
    def update_webhook_url!(url)
      if url.present?
        webhook&.update!(url: url) || create_webhook!(url: url)
      else
        webhook&.destroy
      end
    end
end
