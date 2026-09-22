# Bot keys are "<id>-<token>". Only the token's SHA-256 digest is stored
# (bot_token_digest), as agent credentials store theirs, so the key can be
# shown only once: right after creation or a reset, while the plaintext is
# still in memory.
module User::Bot
  extend ActiveSupport::Concern

  # Stands in for the key wherever it has to appear but is no longer known,
  # for example the bot's own reply path in legacy webhook payloads.
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

      User.create!(**attributes, bot_token_digest: digest_bot_token(bot_token), role: :bot).tap do |user|
        user.plain_bot_token = bot_token
        user.create_webhook!(url: webhook_url) if webhook_url
      end
    end

    # Looks the bot up by id, then compares token digests in constant time.
    def authenticate_bot(bot_key)
      bot_id, bot_token = bot_key.to_s.split("-", 2)
      return if bot_id.blank? || bot_token.blank? || !bot_id.match?(/\A\d+\z/)

      bot = active_bots.find_by(id: bot_id)
      bot if bot&.bot_token_digest.present? &&
        ActiveSupport::SecurityUtils.secure_compare(bot.bot_token_digest, digest_bot_token(bot_token))
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

  # The full key while it is known, otherwise BOT_KEY_PLACEHOLDER: the
  # stored digest cannot be turned back into a key.
  def bot_key
    plain_bot_key || BOT_KEY_PLACEHOLDER
  end

  # Issues a new token, invalidating the old key, and returns the new key.
  def reset_bot_key
    token = self.class.generate_bot_token
    update! bot_token_digest: self.class.digest_bot_token(token)
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
