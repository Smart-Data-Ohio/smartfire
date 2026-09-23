module BoardAutomations
  class NudgePushJob < ApplicationJob
    def perform(nudge)
      BoardAutomations::NudgePusher.new(nudge: nudge).push
    end
  end
end
