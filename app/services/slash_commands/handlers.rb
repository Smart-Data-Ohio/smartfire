module SlashCommands
  # One method per built-in command, named by the Registry entry. Each
  # receives a Registry::Context and returns a Registry::Result. Posting
  # handlers run the same create/broadcast/deliver path as the message
  # endpoints, so slash-posted messages notify, deliver to agents, and
  # index exactly like typed ones.
  module Handlers
    DND_DURATION_PATTERN = /\A(?<amount>\d+)\s*(?<unit>m(?:ins?)?|minutes?|h(?:rs?)?|hours?|d(?:ays?)?)\z/i
    OOO_DURATION_PATTERN = /\A(?<amount>\d+)\s*(?<unit>w(?:eeks?)?|m(?:ins?)?|minutes?|h(?:rs?)?|hours?|d(?:ays?)?)\b(?<rest>.*)\z/im
    OOO_DAY_PATTERN = /\A(?<token>today|tomorrow|(?:next\s+)?(?:sunday|monday|tuesday|wednesday|thursday|friday|saturday))(?<rest>\s+.*|\z)/im
    OOO_ISO_DATE_PATTERN = /\A(?<date>\d{4}-\d{2}-\d{2})(?<rest>\s+.*|\z)/m
    OOO_MONTH_PATTERN = /\A(?<month>jan(?:uary)?|feb(?:ruary)?|mar(?:ch)?|apr(?:il)?|may|jun(?:e)?|jul(?:y)?|aug(?:ust)?|sep(?:t(?:ember)?)?|oct(?:ober)?|nov(?:ember)?|dec(?:ember)?)\s+(?<day>\d{1,2})(?:st|nd|rd|th)?(?<rest>\s+.*|\z)/im
    OOO_MONTHS = {
      "jan" => 1, "feb" => 2, "mar" => 3, "apr" => 4, "may" => 5, "jun" => 6,
      "jul" => 7, "aug" => 8, "sep" => 9, "oct" => 10, "nov" => 11, "dec" => 12
    }.freeze
    # A rest starting with a time-of-day belongs to the time language
    # ("friday 5pm" keeps 5pm); anything else is the note on a bare day.
    OOO_TIME_LEAD_PATTERN = /\A(?:at\s+)?\d{1,2}(?::\d{2})?/i

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

      def handle_ooo(context)
        user = context.user
        normalized = context.args.to_s.strip

        if normalized.casecmp("off").zero?
          user.update!(ooo_until: nil, ooo_note: nil)
          user.claim_ooo_broadcast!(user.out_of_office?)
          Calendar::OooDispatcher.broadcast_ooo_for(user)

          if user.calendar_ooo_active?
            return Registry::Result.ephemeral(
              "Manual out of office is off. Your calendar still shows you out until #{user.ooo_until_date}.")
          end

          return Registry::Result.ephemeral("Out of office is off.")
        end

        time, note = ooo_time_and_note(normalized, zone: user.time_zone_or_default)

        if time.nil?
          return Registry::Result.error(
            "Usage: /ooo <when> [note] — for example “/ooo tomorrow Back soon”, “/ooo friday”, “/ooo 2026-10-05”, or “/ooo 3d”. Bare days and dates run to the end of the day; “/ooo friday 5pm” keeps the time. “/ooo off” clears it.")
        end
        if time <= Time.current
          return Registry::Result.error("“#{time.in_time_zone(user.time_zone_or_default).to_fs(:long)}” is in the past.")
        end

        user.update!(ooo_until: time, ooo_note: note.presence)
        user.claim_ooo_broadcast!(true)
        Calendar::OooDispatcher.broadcast_ooo_for(user)

        message = "Out of office until #{user.ooo_until_date}."
        message += " Note: “#{user.ooo_note}”." if user.ooo_note.present?
        Registry::Result.ephemeral(message)
      rescue ActiveRecord::RecordInvalid => error
        Registry::Result.error(error.record.errors.full_messages.to_sentence)
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
            # post_message! already processes attachments, and the thread
            # path never fans out to legacy webhooks (see
            # ChannelThreadMessagesController#create).
            context.thread.post_message!(
              creator: context.user,
              attributes: { markdown_source:, action: }
            )
          else
            context.room.root_messages.create!(creator: context.user, markdown_source:, action:)
              .tap(&:process_attachment)
          end

          message.broadcast_create
          Message::BotWebhookFanout.deliver_for(message) unless context.thread
          message
        end

        # Splits "/ooo <when> [note]": a leading duration ("30m", "2h",
        # "3d", "1 week"), a bare day or date ("tomorrow", "friday",
        # "2026-10-05", "oct 5") running to the end of that day in the
        # member's zone like the form presets, or a time with an explicit
        # clock time ("friday 5pm", "2026-10-01 15:00"), plus the trailing
        # note. Returns [ time, note ] or nil when no leading time
        # parses.
        def ooo_time_and_note(args, zone:)
          if (match = args.match(OOO_DURATION_PATTERN))
            amount = match[:amount].to_i
            duration = case match[:unit].downcase
            when /\Am/ then amount.minutes
            when /\Ah/ then amount.hours
            when /\Ad/ then amount.days
            else amount.weeks
            end
            return nil unless duration&.positive?

            [ Time.current + duration, match[:rest].to_s.strip.presence ]
          else
            ooo_bare_day(args, zone:) || TimeParser.split_leading_time(args, zone:)
          end
        end

        # A bare day or date leads the args and means the end of that day,
        # matching the status form presets. Returns [ time, note ] or nil
        # when the args lead with an explicit clock time instead (which
        # the time language keeps) or with no day or date at all.
        def ooo_bare_day(args, zone:)
          time_zone = ActiveSupport::TimeZone[zone] || Time.zone

          if (match = args.match(OOO_DAY_PATTERN)) && !ooo_time_led?(match[:rest])
            time = ooo_day_end(match[:token], zone: time_zone)
            [ time, match[:rest].to_s.strip.presence ] if time
          elsif (match = args.match(OOO_ISO_DATE_PATTERN)) && !ooo_time_led?(match[:rest])
            time = ooo_iso_day_end(match[:date], zone: time_zone)
            [ time, match[:rest].to_s.strip.presence ] if time
          elsif (match = args.match(OOO_MONTH_PATTERN)) && !ooo_time_led?(match[:rest])
            time = ooo_month_day_end(match[:month], match[:day], zone: time_zone)
            [ time, match[:rest].to_s.strip.presence ] if time
          end
        end

        def ooo_time_led?(rest)
          rest.to_s.strip.match?(OOO_TIME_LEAD_PATTERN)
        end

        # End of the named day in the member's zone. Bare weekdays resolve
        # like the form's Monday preset: the next one, a week out on the
        # same weekday; "next <weekday>" is the one after that.
        def ooo_day_end(token, zone:)
          zoned = Time.current.in_time_zone(zone)
          normalized = token.to_s.strip.downcase.gsub(/\s+/, " ")

          date = case normalized
          when "today" then zoned.to_date
          when "tomorrow" then zoned.to_date + 1
          else
            next_prefix = normalized.start_with?("next ")
            weekday = next_prefix ? normalized.sub(/\Anext\s+/, "") : normalized
            target = TimeParser::WEEKDAYS.index(weekday)
            return nil unless target

            delta = (target - zoned.wday) % 7
            delta = 7 if delta.zero?
            delta += 7 if next_prefix
            zoned.to_date + delta
          end

          zone.parse(date.to_s)&.end_of_day
        end

        def ooo_iso_day_end(date, zone:)
          zone.parse(date.to_s)&.end_of_day
        rescue ArgumentError, TypeError
          nil
        end

        # End of the named month day in the member's zone: this year's
        # when it is still ahead, else next year's.
        def ooo_month_day_end(month_name, day, zone:)
          month = OOO_MONTHS[month_name.to_s.strip.downcase.first(3)]
          day_number = day.to_i
          return nil unless month && day_number.between?(1, 31)

          begin
            date = Date.new(Time.current.in_time_zone(zone).year, month, day_number)
          rescue Date::Error
            return nil
          end

          time = zone.parse(date.to_s)&.end_of_day
          return time if time && time > Time.current

          begin
            rolled = Date.new(date.year + 1, month, day_number)
          rescue Date::Error
            return nil
          end
          zone.parse(rolled.to_s)&.end_of_day
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
