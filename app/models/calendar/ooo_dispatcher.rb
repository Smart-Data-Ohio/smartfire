module Calendar
  # Minute-tick out-of-office boundary checks for the periodic runner:
  # enqueues interval refreshes for calendar-OOO members whose cache went
  # stale, and broadcasts each member's status badge and DM notices when
  # their OOO state flips. Flips are claimed with a conditional UPDATE on
  # the member (User#claim_ooo_broadcast!), so concurrent ticks announce
  # each boundary exactly once; a tick that finds no flip broadcasts
  # nothing. Claiming an end also clears the expired manual columns. The
  # member panel and DM dots poll the same reader on their own cadence,
  # so they need no broadcast.
  class OooDispatcher
    REFRESH_STALE_AFTER = 15.minutes

    def self.dispatch_due!(now: Time.current)
      flips = []

      User.active.where("ooo_until IS NOT NULL OR ooo_calendar_enabled = ?", true)
        .includes(:meeting_cache).find_each do |user|
        begin
          cache = user.meeting_cache

          # Members with both opt-ins refresh through the meeting
          # dispatcher; only OOO-only members refresh here, so one tick
          # never enqueues two refreshes for the same member.
          if user.ooo_calendar_enabled? && !user.meeting_status_enabled? &&
              (cache.nil? || cache.fetched_at.nil? || cache.fetched_at <= now - REFRESH_STALE_AFTER)
            Calendar::MeetingRefreshJob.perform_later(user.id)
          end

          # No manual end, no cached intervals, and no prior flip: there is
          # no state to announce, so skip the claim instead of broadcasting
          # an empty first-false for a member the refresh above has not
          # seeded yet.
          next if user.ooo_until.nil? && cache.nil? && !user.ooo_broadcast?

          flips << user if user.claim_ooo_broadcast!(user.out_of_office?(now:), now:)
        rescue => error
          Rails.logger.error "Calendar::OooDispatcher failed for user #{user.id}: #{error.class}"
        end
      end

      broadcast_ooo_for(flips) if flips.any?
    end

    # Re-renders the status badge plus every DM notice line for members
    # whose OOO state changed outside the sweep (setting or clearing a
    # manual OOO, opting out of calendar OOO): open profile pages, cards,
    # and DM rooms subscribed to the member's streams update at once
    # instead of keeping the old state until a navigation. The notice
    # content names only the member's return date and own note — the same
    # string for every viewer — so one broadcast serves all subscribers.
    def self.broadcast_ooo_for(users)
      users = Array(users)
      return if users.empty?

      Calendar::MeetingDispatcher.broadcast_badges_for(users)

      users.each do |user|
        Turbo::StreamsChannel.broadcast_update_to([ user, :ooo_notice ],
          target: ActionView::RecordIdentifier.dom_id(user, :ooo_notice),
          partial: "rooms/show/ooo_notice_line",
          locals: { user: })
      end
    end
  end
end
