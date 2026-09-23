module Calendar
  # Minute-tick meeting boundary checks for the periodic runner: enqueues
  # interval refreshes for opted-in members whose cache went stale, and
  # broadcasts each member's status badge when their in-meeting state
  # flips. Flips are claimed with a conditional UPDATE, so concurrent
  # ticks announce each boundary exactly once; a tick that finds no flip
  # broadcasts nothing. Member panels and DM dots poll the same reader
  # on their own cadence, so they need no broadcast.
  class MeetingDispatcher
    REFRESH_STALE_AFTER = 15.minutes

    def self.dispatch_due!(now: Time.current)
      flips = []

      User.active.where(meeting_status_enabled: true).includes(:meeting_cache).find_each do |user|
        begin
          cache = user.meeting_cache

          if cache.nil? || cache.fetched_at.nil? || cache.fetched_at <= now - REFRESH_STALE_AFTER
            Calendar::MeetingRefreshJob.perform_later(user.id)
          end

          next if cache.nil?

          flips << user if cache.claim_broadcast!(cache.in_meeting?(now:))
        rescue => error
          Rails.logger.error "Calendar::MeetingDispatcher failed for user #{user.id}: #{error.class}"
        end
      end

      broadcast_flips!(flips) if flips.any?
    end

    def self.broadcast_flips!(users)
      lease_states = WorkspacePresenceLease.presence_by_user_id(users.map(&:id))

      users.each do |user|
        Turbo::StreamsChannel.broadcast_update_to([ user, :status ],
          target: ActionView::RecordIdentifier.dom_id(user, :status_badge),
          partial: "users/statuses/badge",
          locals: { user:, presence: user.effective_presence(lease_states[user.id] || :offline) })
      end
    end
    private_class_method :broadcast_flips!
  end
end
