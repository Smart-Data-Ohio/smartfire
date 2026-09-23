module Agents
  # Shared board posts for the REST agent posts API and the MCP
  # list_board_posts / create_board_post tools. Owns the list filters and
  # the creation path; the callers own room membership, the
  # read/post/manage_threads grants, and throttling.
  class BoardPosts
    LIST_MAX_LIMIT = 100
    LIST_STATUSES = %w[ planned in_progress blocked done open all ].freeze
    LIST_OWNER_FILTERS = %w[ me agents ].freeze

    def self.list(agent:, room:, status: nil, owner: nil, tag: nil)
      status_filter = status.to_s.strip.presence || "open"
      unless LIST_STATUSES.include?(status_filter)
        return ServiceResult.fail("Status must be one of #{LIST_STATUSES.join(", ")}")
      end

      owner_filter = owner.to_s.strip
      unless owner_filter.blank? || LIST_OWNER_FILTERS.include?(owner_filter) || owner_filter.match?(/\A\d+\z/)
        return ServiceResult.fail("Owner must be a user id, me, or agents")
      end

      scope = room.channel_threads.ordered
        .includes(:room, :work_owner, :tags, work_thread_links: %i[ github_pull_request event ])

      scope = case status_filter
      when "done"
        scope.work.where(work_status: "done")
      when "all"
        scope
      when "open"
        scope.work.where.not(work_status: "done")
      else
        scope.work.where(work_status: status_filter)
      end

      scope = case owner_filter
      when "me"
        scope.where(work_owner_id: agent.user_id)
      when "agents"
        scope.where(work_owner_id: Agent.select(:user_id))
      when /\A\d+\z/
        scope.where(work_owner_id: owner_filter.to_i)
      else
        scope
      end

      tag_filter = tag.to_s.strip.downcase
      if tag_filter.present?
        scope = scope.where(id: ThreadTag.where(name: tag_filter).select(:channel_thread_id))
      end

      ServiceResult.ok(scope.limit(LIST_MAX_LIMIT).to_a)
    end

    def self.create(agent:, room:, title:, body: nil, tags: nil, work_status: nil, run_url: nil, owner_id: nil)
      if (denial = Budgets.check(agent, :board_posts))
        return denial
      end

      thread = ChannelThread.create_board_post!(
        room: room,
        creator: agent.user,
        name: title,
        work_status: work_status.presence || "in_progress",
        owner_id: owner_id.presence || agent.user_id,
        tags: tags,
        run_url: run_url,
        first_message: body
      )

      ServiceResult.ok(thread, status: :created)
    rescue ActiveRecord::RecordNotFound
      ServiceResult.fail("Agent is not a member of the room", status: :not_found)
    rescue ActiveRecord::RecordInvalid => error
      ServiceResult.fail(error.record.errors.full_messages.to_sentence)
    end
  end
end
