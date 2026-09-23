module SlashCommands
  # One method per built-in command, named by the Registry entry. Each
  # receives a Registry::Context and returns a Registry::Result. Posting
  # handlers run the same create/broadcast/deliver path as the message
  # endpoints, so slash-posted messages notify, deliver to agents, and
  # index exactly like typed ones.
  module Handlers
    DND_DURATION_PATTERN = /\A(?<amount>\d+)\s*(?<unit>m(?:ins?)?|minutes?|h(?:rs?)?|hours?|d(?:ays?)?)\z/i

    class << self
      def handle_huddle(context)
        unless Huddle.configured?
          return Registry::Result.error("Huddles are not configured in this workspace.")
        end

        Registry::Result.start_huddle(context.room)
      end

      def handle_event(context)
        if context.args.blank?
          return Registry::Result.error("Usage: /event <title> <when> — for example “/event Launch party friday 5pm”.")
        end

        title, time = TimeParser.split_trailing_time(context.args, zone: context.user.time_zone_or_default)
        if title.blank?
          return Registry::Result.error("Usage: /event <title> <when> — for example “/event Launch party friday 5pm”.")
        end
        if time && time <= Time.current
          return Registry::Result.error("“#{time.in_time_zone(context.user.time_zone_or_default).to_fs(:long)}” is in the past.")
        end

        prefill = { title:, time_zone: context.user.time_zone_or_default }
        prefill[:starts_at] = time.utc.iso8601 if time
        Registry::Result.open_url(context.routes.new_room_event_path(context.room, event: prefill))
      end

      def handle_poll(_context)
        Registry::Result.open_poll
      end

      def handle_remind(context)
        if context.args.blank?
          return Registry::Result.error("Usage: /remind <when> <text> — for example “/remind in 20 minutes review the deploy”.")
        end

        time, text = TimeParser.split_leading_time(context.args, zone: context.user.time_zone_or_default)
        if time.nil? || text.blank?
          return Registry::Result.error("Usage: /remind <when> <text> — for example “/remind tomorrow 9am file expenses”.")
        end
        if time <= Time.current
          return Registry::Result.error("“#{time.in_time_zone(context.user.time_zone_or_default).to_fs(:long)}” is in the past.")
        end

        message = post_message(context, text)
        saved_item = context.user.saved_items.find_or_initialize_by(message:)
        saved_item.remind_at = time
        saved_item.save!

        notice = "Reminder set for #{time.in_time_zone(context.user.time_zone_or_default).to_fs(:long)}."
        Registry::Result.posted(message, notice:)
      rescue ActiveRecord::RecordInvalid => error
        Registry::Result.error(error.record.errors.full_messages.to_sentence)
      end

      def handle_status(context)
        emoji, text = context.args.split(/\s+/, 2)
        if emoji.blank? || text.blank?
          return Registry::Result.error("Usage: /status <emoji> <text> — for example “/status 🚂 On a train”.")
        end

        context.user.update!(
          custom_status_emoji: emoji,
          custom_status_text: text,
          custom_status_expires_at: Time.current.in_time_zone(context.user.time_zone_or_default).end_of_day
        )
        Registry::Result.ephemeral("Status set to “#{context.user.custom_status_display}”.")
      rescue ActiveRecord::RecordInvalid => error
        Registry::Result.error(error.record.errors.full_messages.to_sentence)
      end

      def handle_dnd(context)
        user = context.user

        case dnd_action(context.args, zone: user.time_zone_or_default)
        in [ :off, ]
          user.update!(dnd_enabled: false, dnd_until: nil)
          Registry::Result.ephemeral("Do Not Disturb is off.")
        in [ :on, nil ]
          user.update!(dnd_enabled: true, dnd_until: nil)
          Registry::Result.ephemeral("Do Not Disturb is on.")
        in [ :on, time ]
          user.update!(dnd_enabled: true, dnd_until: time)
          Registry::Result.ephemeral("Do Not Disturb is on until #{time.in_time_zone(user.time_zone_or_default).to_fs(:long)}.")
        in [ :toggle, ]
          if user.manual_dnd_active?
            user.update!(dnd_enabled: false, dnd_until: nil)
            Registry::Result.ephemeral("Do Not Disturb is off.")
          else
            user.update!(dnd_enabled: true, dnd_until: nil)
            Registry::Result.ephemeral("Do Not Disturb is on.")
          end
        in nil
          Registry::Result.error("Usage: /dnd [30m|2h|until 5pm|off] — bare /dnd toggles.")
        end
      end

      def handle_shrug(context)
        body = context.args.blank? ? Dispatcher::SHRUG : "#{context.args} #{Dispatcher::SHRUG}"
        Registry::Result.posted(post_message(context, body))
      rescue ActiveRecord::RecordInvalid => error
        Registry::Result.error(error.record.errors.full_messages.to_sentence)
      end

      def handle_me(context)
        if context.args.blank?
          return Registry::Result.error("Usage: /me <action> — for example “/me is reviewing the deploy”.")
        end

        Registry::Result.posted(post_message(context, context.args, action: true))
      rescue ActiveRecord::RecordInvalid => error
        Registry::Result.error(error.record.errors.full_messages.to_sentence)
      end

      def handle_play(context)
        Registry::Result.posted(post_message(context, "/play #{context.args}".strip))
      rescue ActiveRecord::RecordInvalid => error
        Registry::Result.error(error.record.errors.full_messages.to_sentence)
      end

      private
        def post_message(context, markdown_source, action: false)
          message = if context.thread
            context.thread.post_message!(
              creator: context.user,
              attributes: { markdown_source:, action: }
            )
          else
            context.room.root_messages.create!(creator: context.user, markdown_source:, action:)
          end

          message.process_attachment
          message.broadcast_create
          Message::BotWebhookFanout.deliver_for(message)
          message
        end

        # Returns [ :on|:off|:toggle, time-or-nil ], or nil when the
        # argument parses as neither.
        def dnd_action(args, zone:)
          normalized = args.to_s.strip
          return [ :toggle, nil ] if normalized.blank?
          return [ :off, nil ] if normalized.casecmp("off").zero?
          return [ :on, nil ] if normalized.casecmp("on").zero?

          if (match = normalized.match(DND_DURATION_PATTERN))
            amount = match[:amount].to_i
            duration = case match[:unit].downcase
            when /\Am/ then amount.minutes
            when /\Ah/ then amount.hours
            else amount.days
            end
            return [ :on, Time.current + duration ] if duration.positive?
            return nil
          end

          text = normalized.sub(/\Auntil\s+/i, "")
          time = TimeParser.parse(text, zone:)
          [ :on, time ] if time&.future?
        end
    end
  end
end
