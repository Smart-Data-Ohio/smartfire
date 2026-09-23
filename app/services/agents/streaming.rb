module Agents
  # Shared streaming messages for the REST streaming API and the MCP
  # start_stream / append_stream / finalize_stream tools. An agent posts
  # a message in a `streaming` state, appends to it or replaces its body
  # (Markdown only), then finalizes; every side effect fires exactly
  # once at finalize (see Message#finalize_stream!). The caller owns
  # room membership, the post_messages grant, and throttling for stream
  # creation; updates and finalizes own their own authorization (the
  # message must be the agent's own streaming message in one of its
  # rooms), so both surfaces decide identically.
  class Streaming
    def self.start(agent:, room:, thread_id: nil, attributes: {})
      if thread_id.present?
        start_thread_stream(agent: agent, room: room, thread_id: thread_id, attributes: attributes)
      else
        start_root_stream(agent: agent, room: room, attributes: attributes)
      end
    end

    # attributes carries symbol keys: markdown_source (required key, may
    # be blank for an incremental stream), client_message_id,
    # reply_to_message_id, reply_notify_author.
    def self.start_root_stream(agent:, room:, attributes:)
      attrs = normalize_root_attributes(room, attributes)

      if (duplicate = Message.find_duplicate(room: room, creator: agent.user, client_message_id: attrs[:client_message_id]))
        return ServiceResult.ok(duplicate, status: :created)
      end

      if (denial = Budgets.check(agent, :messages))
        return denial
      end

      message = room.root_messages.new(attrs.merge(streaming: true))
      message.save!
      message.broadcast_stream_start

      ServiceResult.ok(message, status: :created)
    rescue ActiveRecord::RecordInvalid => error
      ServiceResult.fail(error.record.errors.full_messages.to_sentence,
        payload: { errors: error.record.errors.to_hash })
    end

    def self.start_thread_stream(agent:, room:, thread_id:, attributes:)
      thread = room.channel_threads.find_by(id: thread_id)
      return ServiceResult.fail("Thread not found", status: :not_found) unless thread

      if thread.locked?
        return ServiceResult.fail("This thread is locked", status: :unprocessable_entity)
      end

      if (duplicate = Message.find_duplicate(room: room, creator: agent.user, client_message_id: attributes[:client_message_id] || attributes["client_message_id"]))
        return ServiceResult.ok(duplicate, status: :created)
      end

      if (denial = Budgets.check(agent, :messages))
        return denial
      end

      attrs = normalize_thread_attributes(attributes)
      message = thread.post_message!(creator: agent.user, attributes: attrs.merge(streaming: true))
      message.broadcast_stream_start

      ServiceResult.ok(message, status: :created)
    rescue ActiveRecord::RecordInvalid => error
      ServiceResult.fail(error.record.errors.full_messages.to_sentence,
        payload: { errors: error.record.errors.to_hash })
    rescue ChannelThread::LockedError => error
      ServiceResult.fail(error.message, status: :unprocessable_entity)
    end

    # Appends text to the stream, or replaces the whole body when append
    # is absent. Only Markdown streams exist, so both paths write
    # markdown_source. The save always lands; the broadcast is throttled
    # (see Message::Broadcasts#broadcast_stream_update).
    def self.update(agent:, id:, append: nil, markdown_source: nil)
      message, denial = find_own_stream(agent, id)
      return denial if denial

      if !append.nil?
        message.markdown_source = message.markdown_source.to_s + append.to_s
      elsif !markdown_source.nil?
        message.markdown_source = markdown_source.to_s
      else
        return ServiceResult.fail("append or markdown_source is required")
      end

      begin
        message.save!
      rescue ActiveRecord::RecordInvalid => error
        return ServiceResult.fail(error.record.errors.full_messages.to_sentence,
          payload: { errors: error.record.errors.to_hash })
      end
      message.broadcast_stream_update

      ServiceResult.ok(message)
    end

    # Idempotent: finalizing an already-final message succeeds without
    # repeating side effects.
    def self.finalize(agent:, id:)
      message, denial = find_own_stream(agent, id, streaming_only: false)
      return denial if denial

      message.finalize_stream! if message.streaming?

      ServiceResult.ok(message.reload)
    end

    # The agent's own streaming message in one of its rooms, or a denial.
    # Authorization first: anything the agent must not see (missing,
    # outside its rooms, another author's) is the same 404.
    def self.find_own_stream(agent, id, streaming_only: true)
      message = Message.find_by(id: id)
      if message.nil? || message.creator_id != agent.user_id ||
          !agent.user.rooms.where(id: message.room_id).exists?
        return [ nil, ServiceResult.fail("Message not found", status: :not_found) ]
      end

      unless agent.can?(:post_messages, message.room_id)
        return [ nil, ServiceResult.fail("Forbidden: agent lacks post_messages capability", status: :forbidden) ]
      end

      if streaming_only && !message.streaming?
        return [ nil, ServiceResult.fail("Message is not streaming", status: :unprocessable_entity) ]
      end

      # A locked thread is frozen: humans cannot post, edit, or delete in
      # it, and streams cannot start in it, so in-flight streams pause too.
      # Already-final messages still finalize idempotently.
      if message.streaming? && message.thread&.locked?
        return [ nil, ServiceResult.fail("This thread is locked", status: :unprocessable_entity) ]
      end

      [ message, nil ]
    end

    # Root streams resolve the reply target to a root message id up front:
    # an unknown id raises RecordNotFound, like the human endpoint.
    def self.normalize_root_attributes(room, attributes)
      attrs = attributes.to_h.symbolize_keys
      attrs.delete(:drive_file_ids)

      if attrs.key?(:reply_to_message_id)
        attrs[:reply_to_message_id] = if attrs[:reply_to_message_id].present?
          room.root_messages.find(attrs[:reply_to_message_id]).id
        end
      end
      if attrs.key?(:reply_notify_author)
        attrs[:reply_notify_author] = ActiveModel::Type::Boolean.new.cast(attrs[:reply_notify_author])
      end

      attrs
    end

    # A thread stream target stays a plain id: the model validates that
    # it lives in the same conversation.
    def self.normalize_thread_attributes(attributes)
      attrs = attributes.to_h.symbolize_keys
      attrs.delete(:drive_file_ids)
      attrs[:reply_to_message_id] = attrs[:reply_to_message_id].presence if attrs.key?(:reply_to_message_id)
      if attrs.key?(:reply_notify_author)
        attrs[:reply_notify_author] = ActiveModel::Type::Boolean.new.cast(attrs[:reply_notify_author])
      end

      attrs
    end
    private_class_method :start_root_stream, :start_thread_stream, :find_own_stream,
      :normalize_root_attributes, :normalize_thread_attributes
  end
end
