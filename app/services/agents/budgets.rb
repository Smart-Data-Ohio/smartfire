module Agents
  # Per-agent daily budgets, enforced inside the shared services REST and
  # MCP both use (Posting, Streaming, Polls, BoardPosts, Approvals,
  # FizzyCardActions, and the GitHub pull-request actions endpoint), so
  # switching surfaces cannot dodge a cap. Owners and administrators set
  # the caps on the bot edit page; nil means unlimited.
  #
  # Usage counts come from the rows those paths already write (messages,
  # board posts, approval requests), so no counter can drift from what
  # actually happened. When a cap is hit the agent gets 429, and the
  # owner gets one inbox item per cap per day (see AgentBudgetNotice),
  # however many requests overflow.
  class Budgets
    CAP_COLUMNS = {
      "messages" => :daily_message_cap,
      "board_posts" => :daily_board_post_cap,
      "external_actions" => :daily_external_action_cap
    }.freeze
    CAP_NOUNS = {
      "messages" => "message",
      "board_posts" => "board post",
      "external_actions" => "external action"
    }.freeze

    def self.limit_column(cap)
      CAP_COLUMNS.fetch(cap.to_s)
    end

    # Today's usage for the agent page: { messages:, board_posts:,
    # external_actions: }. A board post's opening message counts only
    # toward the board-post cap, never the message cap.
    def self.usage(agent)
      range = Date.current.all_day
      {
        messages: Message.where(creator_id: agent.user_id, created_at: range).where(board_post_opener: false).count,
        board_posts: ChannelThread.where(creator_id: agent.user_id, created_at: range)
          .where(room_id: Room.boards.select(:id)).count,
        external_actions: AgentApproval.where(agent_id: agent.id, created_at: range).count
      }
    end

    # Returns nil when the agent may act, or a 429 ServiceResult when the
    # cap is hit (recording the once-a-day notice as a side effect).
    # Callers check grants and idempotency replays first, so a forbidden
    # caller sees 403 and a retry never burns budget.
    def self.check(agent, cap)
      cap = cap.to_s
      limit = agent.public_send(limit_column(cap))
      return nil if limit.nil?

      count = usage(agent)[cap.to_sym]
      return nil if count < limit

      AgentBudgetNotice.record_for!(agent, cap)

      error = "Daily #{CAP_NOUNS.fetch(cap)} budget exceeded (#{limit}/day)"
      ServiceResult.fail(error, status: :too_many_requests,
        payload: { error: error, cap: cap, limit: limit, retry_after: seconds_until_reset })
    end

    def self.seconds_until_reset(now: Time.current)
      [ (now.end_of_day - now).to_i, 1 ].max
    end
  end
end
