module Calendar
  # Applies one push notification: re-reads the member's synced Google
  # copies and carries RSVP-shaped changes back into Smartfire
  # attendance. A copy the member cancelled or deleted in Google declines
  # the event locally; a confirmed copy never overrides a local
  # response. Other edits (time, title) stay one-way: Smartfire remains
  # the source of truth for those, and the outbound sync already
  # converges them.
  #
  # The mapping is convergent, not flapping: declining locally deletes
  # the remote copy, which reads back as declined (no change), and a
  # local decline always wins over a confirmed remote copy — the member
  # re-accepts in Smartfire if they change their mind.
  class InboundSync
    # One push covers the whole calendar; bound the re-read to the
    # member's upcoming synced entries.
    MAX_ENTRIES = 50

    def self.sync(user_id)
      user = User.find_by(id: user_id)
      new(user).sync! if user
    end

    def initialize(user)
      @user = user
    end

    def sync!
      account = @user.google_account
      return unless account&.usable? && account.calendar?

      client = Google::Client.new(account)
      entries.find_each do |entry|
        sync_entry!(client, entry)
      end
    rescue Google::Client::Unavailable
      raise
    rescue Google::Client::Error => error
      channel&.update_column(:last_error, "#{error.class.name.demodulize}: #{error.message}".truncate(250))
      Rails.logger.warn "Calendar::InboundSync failed for user #{@user.id}: #{error.class}"
    end

    private
      def entries
        @user.event_calendar_entries.joins(:event).merge(Event.upcoming).order(:event_id).limit(MAX_ENTRIES)
          .includes(event: :room)
      end

      def channel
        @channel ||= Calendar::PushChannel.find_by(user_id: @user.id)
      end

      def sync_entry!(client, entry)
        event = entry.event
        return unless event.respondable_by?(@user)

        local = event.response_for(@user)
        remote = remote_status(client, entry.google_event_id)

        # Inbound sync only ever declines. A confirmed remote copy —
        # including a restored one — never flips a local decline back to
        # going: local declines win.
        if remote.in?([ :cancelled, :deleted ])
          event.respond!(@user, "declined") if local.in?(Event::NOTIFYING_RESPONSES)
        end
      rescue Google::Client::Unavailable, Google::Client::Unauthorized
        # A revoked grant fails every entry identically, so abort the
        # sweep instead of burning one refresh per entry. The outer
        # rescue records it on the channel; the renewal sweep later
        # drops the channel once the account reads as disconnected.
        raise
      rescue Google::Client::Error => error
        Rails.logger.warn "Calendar::InboundSync entry failed for event #{event.id}: #{error.class}"
      end

      def remote_status(client, google_event_id)
        remote = client.get_event(google_event_id)
        remote.is_a?(Hash) && remote["status"] == "cancelled" ? :cancelled : :confirmed
      rescue Google::Client::NotFound
        :deleted
      end
  end
end
