module Calendar
  # One Google Calendar push channel (events.watch) for one connected
  # member, backing two-way RSVP sync: changes the member makes in Google
  # update Smartfire attendance. The raw channel token is shown to Google
  # once at watch time; only its SHA-256 digest is stored, and
  # notifications authenticate by echoing it back. Jobs take the channel
  # id, never the token.
  class PushChannel < ApplicationRecord
    self.table_name = "calendar_push_channels"

    RENEW_WITHIN = 24.hours
    RENEW_INTERVAL = 1.hour

    belongs_to :user

    validates :user_id, uniqueness: true
    validates :channel_id, :token_digest, presence: true

    class << self
      # Push needs Google OAuth plus a publicly reachable callback URL
      # from config; without either, watching stays disabled with no
      # errors, and the health page says what to set.
      def watching_enabled?
        Google::Client.configured? && callback_url.present?
      end

      def callback_url
        ENV["GOOGLE_CALENDAR_WEBHOOK_URL"].presence
      end

      # Opens (or re-opens) the member's channel. No-op unless watching
      # is enabled and the account is usable. Returns the channel or nil.
      # The new channel is watched before the old one is stopped, so a
      # failed watch leaves the old channel alive with no gap: the row
      # keeps its old identity (with last_error) and Google's deliveries
      # keep authenticating until expiry.
      def watch_for!(user)
        return nil unless watching_enabled?

        account = user.google_account
        return nil unless account&.usable? && account.calendar?

        existing = find_by(user_id: user.id)

        channel_id = SecureRandom.uuid
        token = SecureRandom.hex(32)
        response = Google::Client.new(account).watch_events(
          channel_id:, token:, address: callback_url
        )

        # The new channel is live: retire the old one remotely, then
        # swap the row to the new identity.
        existing&.stop_remote!
        record = existing || new(user_id: user.id)
        record.assign_attributes(
          channel_id:, resource_id: response["resourceId"],
          token_digest: digest(token),
          expires_at: parse_expiration(response["expiration"]),
          last_message_number: 0, last_error: nil
        )
        record.save!
        record
      rescue Google::Client::Unavailable
        raise
      rescue Google::Client::Error => error
        existing&.update_column(:last_error, error_summary(error))
        nil
      end

      # Renews channels expiring soon, opens channels for connected
      # accounts that have none, and drops channels whose account went
      # away. Runs from the periodic runner every hour; never raises.
      def renew_expiring!(now: Time.current)
        return unless watching_enabled?

        includes(user: :google_account).find_each do |channel|
          begin
            account = channel.user.google_account
            if account&.usable? && account.calendar?
              channel.renew! if channel.expires_at.nil? || channel.expires_at <= now + RENEW_WITHIN
            else
              channel.stop_remote!
              channel.destroy!
            end
          rescue => error
            Rails.logger.error "Calendar::PushChannel renewal failed for user #{channel.user_id}: #{error.class}"
          end
        end

        heal_missing_channels!
      end

      # A first watch that failed permanently — or a re-watch after Google
      # dropped the channel — leaves no row behind, so the renewal loop
      # above would never retry it. Open one channel per connected
      # calendar account that is missing one; failures log and retry on
      # the next sweep.
      def heal_missing_channels!
        GoogleAccount.where(disconnected_reason: nil).find_each do |account|
          next if exists?(user_id: account.user_id)

          begin
            watch_for!(account.user)
          rescue => error
            Rails.logger.error "Calendar::PushChannel watch failed for user #{account.user_id}: #{error.class}"
          end
        end
      end

      def digest(token)
        Digest::SHA256.hexdigest(token.to_s)
      end

      private
        def parse_expiration(milliseconds)
          ms = milliseconds.to_i
          ms.positive? ? Time.zone.at(ms / 1000.0) : nil
        end

        def error_summary(error)
          "#{error.class.name.demodulize}: #{error.message}".truncate(250)
        end
    end

    # True when the presented channel token matches the stored digest.
    def token_matches?(presented)
      return false if presented.blank? || token_digest.blank?

      ActiveSupport::SecurityUtils.secure_compare(
        self.class.digest(presented), token_digest
      )
    rescue ArgumentError # digests of different lengths never match
      false
    end

    # Claims a notification by its message number: Google may redeliver,
    # so only a number above the last seen one proceeds. The claim is one
    # conditional UPDATE, so concurrent deliveries cannot both win.
    def claim_notification!(message_number)
      number = message_number.to_i
      return false unless number.positive?

      self.class.where(id:).where("last_message_number < ?", number)
        .update_all(last_message_number: number, last_notification_at: Time.current) == 1
    end

    def expired?(now: Time.current)
      expires_at.present? && expires_at <= now
    end

    # Opens a fresh channel and retires this one. The new channel is
    # watched before the old one is stopped, so a failed re-watch keeps
    # this row and the old channel alive with no gap: Google's
    # deliveries keep authenticating until expiry, and the next sweep
    # retries.
    def renew!
      self.class.watch_for!(user) || self
    end

    # Best-effort remote stop; never raises.
    def stop_remote!
      account = user.google_account
      return if resource_id.blank? || !(account&.usable? && account.calendar?)

      Google::Client.new(account).stop_channel(channel_id:, resource_id:)
    rescue Google::Client::Error
      nil
    end
  end
end
