module Agents
  # Shared agent posting for POST /rooms/:room_id/agents/messages and the
  # MCP post_message tool. Posts a root message, or — with a thread_id — a
  # reply inside that thread through ChannelThread#post_message!. The
  # caller owns room membership, the post_messages grant, and throttling;
  # this owns duplicate suppression, Drive id validation, broadcasts, and
  # delivery fan-out, so both surfaces post identically.
  class Posting
    # attributes carries symbol keys: body, attachment, client_message_id,
    # markdown_source, reply_to_message_id, reply_notify_author.
    # drive_file_ids is :absent when the caller sent no Drive key, nil
    # when the key held no usable array, or the id array.
    def self.post(agent:, room:, thread_id: nil, attributes: {}, drive_file_ids: :absent)
      if thread_id.present?
        post_thread_reply(agent: agent, room: room, thread_id: thread_id, attributes: attributes, drive_file_ids: drive_file_ids)
      else
        post_root(agent: agent, room: room, attributes: attributes, drive_file_ids: drive_file_ids)
      end
    end

    def self.post_root(agent:, room:, attributes:, drive_file_ids:)
      attrs = normalize_root_attributes(room, attributes)

      if (duplicate = Message.find_duplicate(room: room, creator: agent.user, client_message_id: attrs[:client_message_id]))
        # A retried create: the original request already saved, broadcast,
        # and delivered this message, so return it without repeating side
        # effects.
        return ServiceResult.ok(duplicate)
      end

      if (denial = Budgets.check(agent, :messages))
        return denial
      end

      message = room.root_messages.new(attrs)
      if (failure = apply_drive_file_ids(message, drive_file_ids))
        return failure
      end

      message.save!
      message.process_attachment
      message.broadcast_create
      Message::BotWebhookFanout.deliver_for(message)

      ServiceResult.ok(message)
    rescue ActiveRecord::RecordInvalid => error
      ServiceResult.fail(error.record.errors.full_messages.to_sentence,
        payload: { errors: error.record.errors.to_hash })
    end

    def self.post_thread_reply(agent:, room:, thread_id:, attributes:, drive_file_ids:)
      thread = room.channel_threads.find_by(id: thread_id)
      return ServiceResult.fail("Thread not found", status: :not_found) unless thread

      if thread.locked?
        return ServiceResult.fail("This thread is locked", status: :unprocessable_entity)
      end

      if (duplicate = Message.find_duplicate(room: room, creator: agent.user, client_message_id: attributes[:client_message_id] || attributes["client_message_id"]))
        # A retried create: return the original without re-posting.
        return ServiceResult.ok(duplicate)
      end

      if (denial = Budgets.check(agent, :messages))
        return denial
      end

      ids = validated_drive_file_ids!(drive_file_ids)
      attrs = normalize_thread_attributes(attributes)
      message = thread.post_message!(creator: agent.user, attributes: attrs, drive_file_ids: ids)
      message.broadcast_create

      ServiceResult.ok(message)
    rescue ActiveRecord::RecordInvalid => error
      ServiceResult.fail(error.record.errors.full_messages.to_sentence,
        payload: { errors: error.record.errors.to_hash })
    rescue ChannelThread::LockedError => error
      ServiceResult.fail(error.message, status: :unprocessable_entity)
    end

    # Root posts resolve the reply target to a root message id up front:
    # an unknown id raises RecordNotFound, like the human endpoint.
    def self.normalize_root_attributes(room, attributes)
      attrs = attributes.to_h.symbolize_keys
      attrs.delete(:drive_file_ids)
      attrs.delete(:body) if attrs.key?(:markdown_source) && !attrs[:markdown_source].nil?

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

    # A thread reply target stays a plain id: the model validates that it
    # lives in the same conversation.
    def self.normalize_thread_attributes(attributes)
      attrs = attributes.to_h.symbolize_keys
      attrs.delete(:drive_file_ids)
      attrs.delete(:body) if attrs.key?(:markdown_source) && !attrs[:markdown_source].nil?
      attrs[:reply_to_message_id] = attrs[:reply_to_message_id].presence if attrs.key?(:reply_to_message_id)
      if attrs.key?(:reply_notify_author)
        attrs[:reply_notify_author] = ActiveModel::Type::Boolean.new.cast(attrs[:reply_notify_author])
      end

      attrs
    end

    # Replaces the message's stored Drive set with the submitted ids, in
    # memory so the message and its attachments save in one transaction.
    # Returns a failure result for an unusable or invalid id set, nil when
    # the ids applied (or no Drive key was sent).
    def self.apply_drive_file_ids(message, drive_file_ids)
      return nil if drive_file_ids == :absent

      ids = drive_file_ids if drive_file_ids.is_a?(Array)
      if ids.nil? || ids.reject { |id| Google::DriveLink.valid_id?(id) }.any?
        message.errors.add :drive_attachments, "includes an invalid file id"
        return ServiceResult.fail(message.errors.full_messages.to_sentence,
          payload: { errors: message.errors.to_hash })
      end

      current = message.drive_attachments.to_a
      current.each do |attachment|
        attachment.mark_for_destruction unless ids.include?(attachment.file_id)
      end
      (ids - current.map(&:file_id)).each do |file_id|
        message.drive_attachments.build(file_id: file_id)
      end

      nil
    end

    # The submitted id set for a thread post, which goes through
    # ChannelThread#post_message! and needs plain ids rather than an
    # unsaved message to build on. Raises RecordInvalid exactly like
    # apply_drive_file_ids.
    def self.validated_drive_file_ids!(drive_file_ids)
      return nil if drive_file_ids == :absent

      probe = Message.new
      if (failure = apply_drive_file_ids(probe, drive_file_ids))
        raise ActiveRecord::RecordInvalid, probe
      end
      probe.drive_attachments.map(&:file_id)
    end
    private_class_method :post_root, :post_thread_reply, :normalize_root_attributes,
      :normalize_thread_attributes, :apply_drive_file_ids, :validated_drive_file_ids!
  end
end
