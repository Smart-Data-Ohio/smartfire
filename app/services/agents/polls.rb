module Agents
  # Agent polls for the REST endpoints and the MCP create_poll/get_poll
  # tools. Both creation and result reads require post_messages in the
  # room, the same grant as posting: a poll is a message with ballots
  # attached. Membership misses answer 404; present-but-ungranted
  # answers 403, matching the message endpoints.
  class Polls
    def self.create(agent:, room_id:, question:, options:, multiple: false, anonymous: false, closes_at: nil)
      room = agent.user.rooms.find_by(id: room_id)
      return ServiceResult.fail("Room not found", status: :not_found) unless room
      unless agent.can?(:post_messages, room)
        return ServiceResult.fail("Forbidden: agent lacks post_messages capability", status: :forbidden)
      end

      closes_at, failure = parse_closes_at(closes_at)
      return failure if failure
      if closes_at && closes_at <= Time.current
        return ServiceResult.fail("Closes at must be in the future")
      end

      labels = Poll.normalize_labels(options)
      unless labels.size.between?(Poll::MIN_OPTIONS, Poll::MAX_OPTIONS)
        return ServiceResult.fail("Poll needs between #{Poll::MIN_OPTIONS} and #{Poll::MAX_OPTIONS} options")
      end
      if labels.any? { |label| label.length > PollOption::LABEL_LIMIT }
        return ServiceResult.fail("Options are limited to #{PollOption::LABEL_LIMIT} characters")
      end
      if question.to_s.strip.blank?
        return ServiceResult.fail("Question can't be blank")
      end

      # Everything is pre-validated above, so the poll creation below
      # cannot fail validation: the message post (which broadcasts and
      # delivers) never needs retracting.
      posted = Posting.post(
        agent: agent, room: room,
        attributes: { markdown_source: question.to_s.strip, client_message_id: SecureRandom.uuid },
        drive_file_ids: :absent
      )
      return posted unless posted.ok?

      poll = Poll.create_for_message!(
        message: posted.payload, labels: labels,
        multiple: ActiveModel::Type::Boolean.new.cast(multiple),
        anonymous: ActiveModel::Type::Boolean.new.cast(anonymous),
        closes_at: closes_at
      )

      ServiceResult.ok(poll.results_payload(viewer: agent.user), status: :created)
    rescue ActiveRecord::RecordInvalid => error
      ServiceResult.fail(error.record.errors.full_messages.to_sentence)
    end

    def self.show(agent:, room_id:, poll_id:)
      room = agent.user.rooms.find_by(id: room_id)
      return ServiceResult.fail("Poll not found", status: :not_found) unless room

      poll = Poll.joins(:message).where(messages: { room_id: room.id }).find_by(id: poll_id)
      return ServiceResult.fail("Poll not found", status: :not_found) unless poll
      unless agent.can?(:post_messages, room)
        return ServiceResult.fail("Forbidden: agent lacks post_messages capability", status: :forbidden)
      end

      ActiveRecord::Associations::Preloader.new(records: [ poll ], associations: [ :poll_options, { poll_votes: :user } ]).call
      ServiceResult.ok(poll.results_payload(viewer: agent.user))
    end

    def self.parse_closes_at(value)
      return [ nil, nil ] if value.blank?
      return [ value, nil ] if value.is_a?(Time) || value.is_a?(DateTime) || value.is_a?(ActiveSupport::TimeWithZone)

      parsed = Time.zone.parse(value.to_s)
      return [ nil, ServiceResult.fail("closes_at is invalid") ] if parsed.nil?

      [ parsed, nil ]
    rescue ArgumentError, TypeError
      [ nil, ServiceResult.fail("closes_at is invalid") ]
    end
    private_class_method :parse_closes_at
  end
end
