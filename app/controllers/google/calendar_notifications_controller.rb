module Google
  # Receives Google Calendar push notifications (events.watch) for
  # two-way RSVP sync. An API controller: Google authenticates each
  # delivery by echoing the channel's secret token, and there is no
  # session, so forgery protection is not loaded rather than skipped.
  #
  # The sync handshake (state "sync") is acknowledged without work;
  # change notifications (state "exists") are deduplicated by message
  # number and enqueue a re-read; "not_exists" means Google dropped the
  # channel, so the row is removed and a re-watch enqueued. Authenticated
  # deliveries always answer 200 so Google does not retry them.
  class CalendarNotificationsController < ActionController::API
    def create
      channel = Calendar::PushChannel.find_by(channel_id: request.headers["X-Goog-Channel-Id"].to_s)
      return head(:not_found) if channel.nil?
      return head(:forbidden) unless channel.token_matches?(request.headers["X-Goog-Channel-Token"].to_s)

      case request.headers["X-Goog-Resource-State"].to_s
      when "sync"
        head :ok
      when "exists"
        if channel.claim_notification!(request.headers["X-Goog-Message-Number"])
          Calendar::InboundSyncJob.perform_later(channel.user_id)
        end
        head :ok
      when "not_exists"
        channel.destroy!
        Calendar::WatchChannelJob.perform_later(channel.user_id)
        head :ok
      else
        head :ok
      end
    end
  end
end
