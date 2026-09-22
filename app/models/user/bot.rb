# Bot keys are "<id>-<token>". Authentication compares the token's SHA-256
# digest (bot_token_digest), as agent credentials do, and the UI shows a key
# only once: right after creation or a reset.
#
# TRANSITIONAL: the plaintext bot_token column is still written and kept
# this release, so a rolled-back previous container still authenticates and
# legacy webhook payloads (Webhook#room_bot_messages_path) keep a working
# room.path. A follow-up release drops bot_token; after that bot_key falls
# back to BOT_KEY_PLACEHOLDER for stored bots.
module User::Bot
  extend ActiveSupport::Concern

  # Stands in for the key wherever it has to appear but is not shown, such
  # as the curl examples on the bots page.
  BOT_KEY_PLACEHOLDER = "BOT_KEY"

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

      User.create!(**attributes, bot_token: bot_token, bot_token_digest: digest_bot_token(bot_token), role: :bot).tap do |user|
        user.plain_bot_token = bot_token
        user.create_webhook!(url: webhook_url) if webhook_url
      end
    end

    # Looks the bot up by id, then compares token digests in constant time.
    # A row without a digest (written by a rolled-back previous release)
    # is checked against its plaintext once and gets its digest backfilled.
    def authenticate_bot(bot_key)
      bot_id, bot_token = bot_key.to_s.split("-", 2)
      return if bot_id.blank? || bot_token.blank? || !bot_id.match?(/\A\d+\z/)

      bot = active_bots.find_by(id: bot_id)
      return if bot.nil?

      if bot.bot_token_digest.present?
        bot if ActiveSupport::SecurityUtils.secure_compare(bot.bot_token_digest, digest_bot_token(bot_token))
      elsif bot.bot_token.present? && ActiveSupport::SecurityUtils.secure_compare(bot.bot_token, bot_token)
        bot.update_columns(bot_token_digest: digest_bot_token(bot_token))
        bot
      end
    end

    def generate_bot_token
      SecureRandom.alphanumeric(12)
    end

    def digest_bot_token(token)
      Digest::SHA256.hexdigest(token.to_s)
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

  # The full key. TRANSITIONAL: read from the plaintext column while it
  # exists; once it is dropped, only a just-created or just-reset bot knows
  # its key and stored bots answer BOT_KEY_PLACEHOLDER.
  def bot_key
    plain_bot_key || (has_attribute?(:bot_token) && bot_token.present? ? "#{id}-#{bot_token}" : BOT_KEY_PLACEHOLDER)
  end

  # Issues a new token, invalidating the old key, and returns the new key.
  # TRANSITIONAL: writes the plaintext alongside the digest (see above).
  def reset_bot_key
    token = self.class.generate_bot_token
    update! bot_token: token, bot_token_digest: self.class.digest_bot_token(token)
    self.plain_bot_token = token
    plain_bot_key
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
