module BoardAutomations
  # Fires board SLA nudges from Periodic::Runner. Each rule watches one
  # work status: a post sitting in it past nudge_after_minutes nudges its
  # owner, and past escalate_after_minutes escalates to the board's
  # creator. Each stage fires once per status crossing — the claim row
  # carries the status entry time, so a later status change is a new
  # claim — and claiming happens before notifying, so a crash or a
  # second runner never double-fires. One rule's failure never stops the
  # sweep. Pushes dedupe per recipient per sweep: when both stages reach
  # the same person (an unassigned post, or one owned by the board's
  # creator), each stage still claims and writes its inbox item, but
  # only the first push goes out.
  class SlaDispatcher
    class << self
      def dispatch_due!(now: Time.current)
        pushed_recipient_ids = Set.new
        BoardSlaRule.includes(:room).find_each do |rule|
          dispatch_rule!(rule, now: now, pushed_recipient_ids: pushed_recipient_ids)
        rescue => error
          Rails.logger.error "Board SLA sweep failed for rule #{rule.id}: #{error.class}: #{error.message}"
        end
      end

      private
        def dispatch_rule!(rule, now:, pushed_recipient_ids:)
          room = rule.room
          return unless room&.board? && room.deleted_at.nil?
          return if rule.work_status == "done" # Legacy rows: done takes no rule.

          due_threads(rule, now: now).find_each do |thread|
            dispatch_thread!(rule, thread, now: now, pushed_recipient_ids: pushed_recipient_ids)
          rescue => error
            Rails.logger.error "Board SLA nudge failed for thread #{thread.id}: #{error.class}: #{error.message}"
          end
        end

        def due_threads(rule, now:)
          rule.room.channel_threads.work
            .where(work_status: rule.work_status)
            .where("work_status_changed_at IS NOT NULL AND work_status_changed_at <= ?", now - rule.nudge_after_minutes.minutes)
        end

        # Fires every due stage independently, each with its own claim:
        # the nudge goes to the owner, the escalation to the board's
        # creator. The thread is re-read under no lock but before
        # claiming, so a status change racing the sweep simply misses its
        # thresholds instead of notifying for a status already left.
        def dispatch_thread!(rule, thread, now:, pushed_recipient_ids:)
          thread = ChannelThread.work.find_by(id: thread.id)
          return unless thread&.room_id == rule.room_id
          return unless thread.work_status == rule.work_status

          entered_at = thread.work_status_changed_at
          return if entered_at.nil?

          claimed = claimed_stages_for(rule, thread, entered_at)
          if !claimed.include?("nudge") && entered_at <= now - rule.nudge_after_minutes.minutes
            fire_stage!(rule, thread, "nudge", entered_at, now, pushed_recipient_ids)
          end
          if !claimed.include?("escalation") && entered_at <= now - rule.escalate_after_minutes.minutes
            fire_stage!(rule, thread, "escalation", entered_at, now, pushed_recipient_ids)
          end
        end

        # Stages already claimed for this status crossing, read before any
        # write so repeat sweeps never open a claim transaction for them.
        # A racing runner can still win between this read and the insert;
        # fire_stage! keeps its unique-violation rescue for that.
        def claimed_stages_for(rule, thread, entered_at)
          BoardSlaNudge.where(
            channel_thread_id: thread.id,
            work_status: rule.work_status,
            status_entered_at: entered_at
          ).pluck(:stage)
        end

        def fire_stage!(rule, thread, stage, entered_at, now, pushed_recipient_ids)
          recipient = recipient_for(rule, thread, stage)
          return unless recipient

          nudge = BoardSlaNudge.create!(
            room: rule.room,
            channel_thread: thread,
            work_status: rule.work_status,
            stage: stage,
            status_entered_at: entered_at,
            recipient: recipient
          )

          ActivityItems::Recorder.record!(recipient: recipient, source: nudge, event_type: "work_sla")
          BoardAutomations::NudgePushJob.perform_later(nudge) if pushed_recipient_ids.add?(recipient.id)
        rescue ActiveRecord::RecordNotUnique, ActiveRecord::RecordInvalid
          # A racing runner claimed this crossing between the pre-check
          # and the insert: exactly-once holds.
          nil
        end

        # The nudge goes to whoever owns the post: a human owner directly,
        # or the human behind an agent owner; an unassigned post nudges the
        # board's creator. The escalation always goes to the board's
        # creator. Every candidate must be an active human member of the
        # board; without one the stage stays unclaimed and retries on a
        # later sweep, when an assignment may have supplied a recipient.
        def recipient_for(rule, thread, stage)
          return board_creator_recipient(rule) if stage == "escalation"

          owner = thread.work_owner
          if owner && !owner.bot? && active_member?(rule.room, owner)
            return owner
          end

          if owner&.bot?
            agent = owner.agent || Agent.find_by(user_id: owner.id)
            human = agent&.owner
            return human if human && !human.bot? && active_member?(rule.room, human)
          end

          board_creator_recipient(rule)
        end

        def board_creator_recipient(rule)
          creator = rule.room.creator
          creator if creator && !creator.bot? && active_member?(rule.room, creator)
        end

        def active_member?(room, user)
          user.active? && room.memberships.exists?(user_id: user.id)
        end
    end
  end
end
