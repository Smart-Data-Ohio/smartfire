require "test_helper"

class AgentStepTest < ActiveSupport::TestCase
  setup do
    @room = rooms(:watercooler)
    @agent = agents(:bender_agent)
    @message = @room.root_messages.create!(creator: @agent.user,
      markdown_source: "Working on it", client_message_id: "step-parent")
  end

  test "attaches to the agent's own message" do
    step = AgentStep.create!(agent: @agent, message: @message, name: "Run tests",
      status: "running", input_summary: "bundle exec rails test", duration_ms: 1200)

    assert_equal @message, step.parent
    assert_equal 0, step.position
    assert_equal "running", step.status
  end

  test "attaches to a work thread the agent owns" do
    thread = ChannelThread.create!(room: @room, creator: users(:david), name: "Fix it",
      work_status: "in_progress", work_owner: @agent.user)

    step = AgentStep.create!(agent: @agent, channel_thread: thread, name: "Reproduce")

    assert_equal thread, step.parent
  end

  test "requires exactly one parent" do
    thread = ChannelThread.create!(room: @room, creator: users(:david), name: "Fix it",
      work_status: "in_progress", work_owner: @agent.user)

    parentless = AgentStep.new(agent: @agent, name: "Nowhere")
    assert_not parentless.valid?

    double = AgentStep.new(agent: @agent, message: @message, channel_thread: thread, name: "Both")
    assert_not double.valid?
  end

  test "rejects another author's message" do
    foreign = @room.root_messages.create!(creator: users(:david),
      markdown_source: "Mine", client_message_id: "step-foreign")

    step = AgentStep.new(agent: @agent, message: foreign, name: "Hijack")

    assert_not step.valid?
    assert_includes step.errors[:message], "must be the agent's own message"
  end

  test "rejects threads the agent does not own" do
    thread = ChannelThread.create!(room: @room, creator: users(:david), name: "Fix it",
      work_status: "in_progress", work_owner: users(:david))

    step = AgentStep.new(agent: @agent, channel_thread: thread, name: "Hijack")

    assert_not step.valid?
  end

  test "rejects non-work threads" do
    thread = ChannelThread.create!(room: @room, creator: users(:david), name: "Chat")

    step = AgentStep.new(agent: @agent, channel_thread: thread, name: "Chat step")

    assert_not step.valid?
  end

  test "caps the count per parent" do
    AgentStep::MAX_PER_PARENT.times do |n|
      AgentStep.create!(agent: @agent, message: @message, name: "Step #{n}")
    end

    overflow = AgentStep.new(agent: @agent, message: @message, name: "One too many")

    assert_not overflow.valid?
    assert_match(/limited to/, overflow.errors[:base].sole)
  end

  test "caps field sizes" do
    step = AgentStep.new(agent: @agent, message: @message,
      name: "x" * (AgentStep::NAME_LIMIT + 1),
      input_summary: "x" * (AgentStep::SUMMARY_LIMIT + 1),
      output_summary: "x" * (AgentStep::SUMMARY_LIMIT + 1),
      duration_ms: -1, status: "exploding")

    assert_not step.valid?
    assert step.errors[:name].any?
    assert step.errors[:input_summary].any?
    assert step.errors[:output_summary].any?
    assert step.errors[:duration_ms].any?
    assert step.errors[:status].any?
  end

  test "positions follow creation order" do
    first = AgentStep.create!(agent: @agent, message: @message, name: "First")
    second = AgentStep.create!(agent: @agent, message: @message, name: "Second")

    assert_equal [ first, second ], @message.agent_steps.to_a
    assert_equal [ 0, 1 ], [ first.position, second.position ]
  end

  test "steps die with their message and thread" do
    thread = ChannelThread.create!(room: @room, creator: users(:david), name: "Fix it",
      work_status: "in_progress", work_owner: @agent.user)
    AgentStep.create!(agent: @agent, message: @message, name: "On message")
    AgentStep.create!(agent: @agent, channel_thread: thread, name: "On thread")

    assert_difference "AgentStep.count", -1 do
      @message.destroy!
    end

    assert_difference "AgentStep.count", -1 do
      thread.destroy!
    end
  end
end
