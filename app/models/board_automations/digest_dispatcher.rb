module BoardAutomations
  # Posts each board's daily stale-work digest from Periodic::Runner: the
  # open posts sitting in a ruled status past the rule's nudge threshold,
  # stalest first. The digest goes out as one quiet system note
  # (messages.system_note) — never a normal message — so it renders in
  # the timeline without unread, push, agent delivery, inbox, or search.
  # Claimed per board and day before posting, so a digest goes out at
  # most once a day even across runner restarts. Boards with no stale
  # posts post nothing.
  class DigestDispatcher
    MAX_LISTED_POSTS = 20

    class << self
      def dispatch_due!(now: Time.current)
        Room.alive.boards.where(id: BoardSlaRule.select(:room_id)).find_each do |board|
          dispatch_board!(board, now: now)
        rescue => error
          Rails.logger.error "Board stale digest failed for room #{board.id}: #{error.class}: #{error.message}"
        end
      end

      private
        def dispatch_board!(board, now:)
          stale = stale_posts_for(board, now: now)
          return if stale.empty?

          digest = BoardStaleDigest.create_or_find_by!(room: board, digest_on: now.to_date)
          return unless digest.previously_new_record?

          message = post_digest_note!(board, stale, now: now)
          digest.update!(message: message)
        end

        # Open posts in a ruled status whose entry time passed the rule's
        # nudge threshold, stalest first. Done posts are never stale, even
        # with a rule covering done.
        def stale_posts_for(board, now:)
          rules = board.board_sla_rules.index_by(&:work_status)
          return [] if rules.empty?

          board.channel_threads.work
            .where(work_status: rules.keys - [ "done" ])
            .where.not(work_status_changed_at: nil)
            .includes(:work_owner)
            .order(work_status_changed_at: :asc, id: :asc)
            .select do |thread|
              rule = rules[thread.work_status]
              rule && thread.work_status_changed_at <= now - rule.nudge_after_minutes.minutes
            end
        end

        # Plain Action Text like group-DM membership notes: the digest
        # lines are escaped on the way in so post titles and owner names
        # render literally instead of being parsed as formatting.
        def post_digest_note!(board, stale, now:)
          board.messages.create!(
            creator: board.creator,
            system_note: true,
            body: ERB::Util.h(digest_text(stale, now: now))
          ).tap(&:broadcast_create)
        end

        def digest_text(stale, now:)
          lines = stale.first(MAX_LISTED_POSTS).map do |thread|
            age = age_phrase(thread.work_status_changed_at, now)
            owner = thread.work_owner&.name || "Unassigned"
            status = ChannelThread::WORK_STATUS_LABELS.fetch(thread.work_status, thread.work_status.to_s.humanize)
            "#{thread.name} — #{status} · #{owner} · #{age}"
          end
          lines << "and #{stale.size - MAX_LISTED_POSTS} more" if stale.size > MAX_LISTED_POSTS

          "Stale work digest: #{stale.size} #{'post'.pluralize(stale.size)} past #{"its".pluralize(stale.size)} SLA\n#{lines.join("\n")}"
        end

        def age_phrase(since, now)
          minutes = ((now - since) / 60).floor.clamp(0..)

          if minutes < 60
            "#{minutes} #{'minute'.pluralize(minutes)}"
          elsif minutes < 24 * 60
            hours = (minutes / 60.0).round(1)
            "#{hours} #{'hour'.pluralize(hours)}"
          else
            days = (minutes / 1440.0).round(1)
            "#{days} #{'day'.pluralize(days)}"
          end
        end
    end
  end
end
