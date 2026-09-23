module Agents::StepsHelper
  # Compact duration for a step: milliseconds under a second, one decimal
  # of seconds above it.
  def agent_step_duration(duration_ms)
    if duration_ms < 1000
      "#{duration_ms}ms"
    else
      format("%.1fs", duration_ms / 1000.0)
    end
  end

  # Today's usage line for the agent page, e.g. "3/50 messages · 0/10
  # board posts · 1/5 external actions". Caps without a limit read as
  # bare counts.
  def agent_budget_usage_line(agent)
    usage = Agents::Budgets.usage(agent)

    [
      budget_usage_cell(usage[:messages], agent.daily_message_cap, "messages"),
      budget_usage_cell(usage[:board_posts], agent.daily_board_post_cap, "board posts"),
      budget_usage_cell(usage[:external_actions], agent.daily_external_action_cap, "external actions")
    ].join(" · ")
  end

  private
    def budget_usage_cell(used, limit, noun)
      limit ? "#{used}/#{limit} #{noun}" : "#{used} #{noun}"
    end
end
