module Agents
  # Shared work threads for the REST agent work API and the MCP
  # list_work / update_work / update_board_post / set_result tools. Owns
  # the ownership rule (the agent owns the thread, still belongs to its
  # room, and still holds read_messages there) and the manage_threads
  # gate for writes, so both surfaces enforce the same access.
  class WorkThreads
    LIST_MAX_LIMIT = 100

    def self.list(agent:)
      threads = ChannelThread.work.where(work_owner_id: agent.user_id)
        .where(room_id: Membership.where(user_id: agent.user_id).select(:room_id))
        .includes(:room, :work_owner, :tags, work_thread_links: %i[ github_pull_request event ]).order(updated_at: :desc, id: :desc).to_a
      threads.select! { |thread| agent.can?(:read_messages, thread.room) }

      ServiceResult.ok(threads.first(LIST_MAX_LIMIT))
    end

    def self.show(agent:, id:)
      thread = find_owned(agent, id)
      return ServiceResult.fail("Work not found", status: :not_found) unless thread

      ServiceResult.ok(thread)
    end

    # Each field updates only when its key is given; pass
    # ChannelThread::UNSET_WORK_VALUE for omitted fields so the model can
    # tell them from explicit blanks.
    def self.update(agent:, id:, work_status: ChannelThread::UNSET_WORK_VALUE, note: nil,
        tags: ChannelThread::UNSET_WORK_VALUE, run_url: ChannelThread::UNSET_WORK_VALUE)
      thread = find_owned(agent, id)
      return ServiceResult.fail("Work not found", status: :not_found) unless thread

      unless agent.can?(:manage_threads, thread.room)
        return ServiceResult.fail("Forbidden: agent lacks manage_threads capability", status: :forbidden)
      end

      begin
        thread.update_work_by_agent!(agent: agent, work_status: work_status, note: note, tags: tags, run_url: run_url)
      rescue ActiveRecord::RecordInvalid => error
        return ServiceResult.fail(error.record.errors.full_messages.to_sentence)
      end

      ServiceResult.ok(thread.reload)
    end

    def self.set_result(agent:, id:, markdown:)
      thread = find_owned(agent, id)
      return ServiceResult.fail("Work not found", status: :not_found) unless thread

      unless agent.can?(:manage_threads, thread.room)
        return ServiceResult.fail("Forbidden: agent lacks manage_threads capability", status: :forbidden)
      end

      begin
        thread.update_result_by_agent!(agent: agent, markdown: markdown)
      rescue ActiveRecord::RecordInvalid => error
        return ServiceResult.fail(error.record.errors.full_messages.to_sentence)
      end

      ServiceResult.ok(thread.reload)
    end

    # Ownership, current room membership, and read_messages in the room.
    def self.find_owned(agent, id)
      thread = ChannelThread.work.where(work_owner_id: agent.user_id)
        .includes(:room, :work_owner, :tags, work_thread_links: %i[ github_pull_request event ]).find_by(id: id)
      return nil unless thread
      return nil unless thread.room.memberships.exists?(user_id: agent.user_id)
      return nil unless agent.can?(:read_messages, thread.room)

      thread
    end
    private_class_method :find_owned
  end
end
