require "test_helper"

class AgentWorkingPresenceTest < ActiveSupport::TestCase
  setup do
    @agent = agents(:bender_agent)
  end

  test "sets presence with a TTL" do
    @agent.set_working_presence!("Running tests…")

    assert_equal "Running tests…", @agent.reload.working_presence_text
    assert_in_delta Agent::WORKING_PRESENCE_TTL.from_now.to_i,
      @agent.working_presence_expires_at.to_i, 5
  end

  test "blank clears presence" do
    @agent.set_working_presence!("Thinking…")
    @agent.set_working_presence!("")

    assert_nil @agent.reload.working_presence_text
    assert_nil @agent.working_presence
  end

  test "expired presence reads as cleared" do
    @agent.set_working_presence!("Thinking…")

    travel (Agent::WORKING_PRESENCE_TTL + 1.minute) do
      assert_nil @agent.reload.working_presence_text
    end
  end

  test "presence is capped in length" do
    @agent.working_presence = "x" * (Agent::WORKING_PRESENCE_LIMIT + 1)

    assert_not @agent.valid?
  end

  test "shared service sets and clears" do
    result = Agents::WorkingPresence.set(agent: @agent, text: "Thinking…")

    assert result.ok?
    assert_equal "Thinking…", result.payload[:working_presence]

    cleared = Agents::WorkingPresence.set(agent: @agent, text: "")

    assert cleared.ok?
    assert_nil cleared.payload[:working_presence]
  end

  test "shared service rejects overlong text" do
    result = Agents::WorkingPresence.set(agent: @agent, text: "x" * (Agent::WORKING_PRESENCE_LIMIT + 1))

    assert_not result.ok?
  end
end
