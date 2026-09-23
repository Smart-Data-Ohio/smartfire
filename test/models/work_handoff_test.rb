require "test_helper"

class WorkHandoffTest < ActiveSupport::TestCase
  setup do
    @board = Rooms::Board.create_for({ name: "Launch", creator: users(:david) },
      users: [ users(:david), users(:jz), users(:bender) ])
    @agent = agents(:bender_agent)
    grant!(@agent, "post_messages")
    grant!(@agent, "manage_threads")
    @thread = ChannelThread.create_board_post!(room: @board, creator: users(:david),
      name: "Ship it", work_status: "in_progress", owner_id: users(:david).id)
  end

  test "normalises link and question collections" do
    handoff = WorkHandoff.create!(
      channel_thread: @thread, sender: users(:david), receiver_agent: @agent,
      summary: "Halfway there",
      links: "https://example.com/a\nhttps://example.com/a\n  \nhttps://example.com/b",
      open_questions: [ "First?", "", "  ", "Second?" ]
    )

    assert_equal %w[ https://example.com/a https://example.com/b ], handoff.links
    assert_equal [ "First?", "Second?" ], handoff.open_questions
  end

  test "requires a summary within the cap" do
    blank = WorkHandoff.new(channel_thread: @thread, sender: users(:david), receiver_agent: @agent, summary: "  ")
    assert_not blank.valid?
    assert blank.errors[:summary].any?

    long = WorkHandoff.new(channel_thread: @thread, sender: users(:david), receiver_agent: @agent,
      summary: "x" * (WorkHandoff::SUMMARY_LIMIT + 1))
    assert_not long.valid?
    assert blank.errors[:summary].any? || long.errors[:summary].any?
  end

  test "caps links and requires http urls" do
    too_many = WorkHandoff.new(channel_thread: @thread, sender: users(:david), receiver_agent: @agent,
      summary: "Hi", links: (1..11).map { |i| "https://example.com/#{i}" })
    assert_not too_many.valid?
    assert_includes too_many.errors[:links], "are limited to #{WorkHandoff::LINKS_LIMIT} per handoff"

    bad_scheme = WorkHandoff.new(channel_thread: @thread, sender: users(:david), receiver_agent: @agent,
      summary: "Hi", links: [ "ftp://example.com/x" ])
    assert_not bad_scheme.valid?
    assert_includes bad_scheme.errors[:links], "must be http(s) URLs"

    too_long = WorkHandoff.new(channel_thread: @thread, sender: users(:david), receiver_agent: @agent,
      summary: "Hi", links: [ "https://example.com/#{"x" * WorkHandoff::LINK_LIMIT}" ])
    assert_not too_long.valid?
    assert_includes too_long.errors[:links], "must be at most #{WorkHandoff::LINK_LIMIT} characters each"
  end

  test "caps open questions" do
    too_many = WorkHandoff.new(channel_thread: @thread, sender: users(:david), receiver_agent: @agent,
      summary: "Hi", open_questions: (1..11).map { |i| "Question #{i}?" })
    assert_not too_many.valid?
    assert_includes too_many.errors[:open_questions], "are limited to #{WorkHandoff::OPEN_QUESTIONS_LIMIT} per handoff"

    too_long = WorkHandoff.new(channel_thread: @thread, sender: users(:david), receiver_agent: @agent,
      summary: "Hi", open_questions: [ "x" * (WorkHandoff::OPEN_QUESTION_LIMIT + 1) ])
    assert_not too_long.valid?
    assert_includes too_long.errors[:open_questions], "must be at most #{WorkHandoff::OPEN_QUESTION_LIMIT} characters each"
  end

  test "receiver_error accepts an eligible agent" do
    assert_nil WorkHandoff.receiver_error(@thread, @agent)
  end

  test "receiver_error rejects non-agents and unknown agents" do
    assert_equal "Receiver must be an active agent member of this room with permission to post",
      WorkHandoff.receiver_error(@thread, nil)
    assert_equal "Receiver must be an active agent member of this room with permission to post",
      WorkHandoff.receiver_error(@thread, Agent.new)
  end

  test "receiver_error rejects an agent outside the room" do
    outsider = Agent.create!(user: users(:kevin), owner: users(:david))

    assert_equal "Receiver must be an active agent member of this room with permission to post",
      WorkHandoff.receiver_error(@thread, outsider)
  end

  test "receiver_error rejects a suspended agent" do
    @agent.update!(suspended_at: Time.current)

    assert_equal "Receiver must be an active agent member of this room with permission to post",
      WorkHandoff.receiver_error(@thread, @agent)
  end

  test "receiver_error rejects an agent missing post_messages" do
    AgentGrant.where(agent: @agent, capability: "post_messages").update_all(revoked_at: Time.current)

    assert_equal "Receiver must be an active agent member of this room with permission to post",
      WorkHandoff.receiver_error(@thread, @agent)
  end

  test "receiver_error rejects an agent missing manage_threads" do
    AgentGrant.where(agent: @agent, capability: "manage_threads").update_all(revoked_at: Time.current)

    assert_equal "Receiver must hold the manage_threads capability in this room",
      WorkHandoff.receiver_error(@thread, @agent)
  end

  test "receiver_error rejects the current owner" do
    @thread.update_work!(actor: users(:david), work_owner_id: users(:bender).id)

    assert_equal "Receiver is already the owner of this work",
      WorkHandoff.receiver_error(@thread, @agent)
  end

  test "payload carries the context package" do
    handoff = WorkHandoff.create!(
      channel_thread: @thread, sender: users(:david), receiver_agent: @agent,
      summary: "Halfway", links: [ "https://example.com/a" ], open_questions: [ "Why?" ]
    )

    assert_equal({
      "id" => handoff.id,
      "summary" => "Halfway",
      "links" => [ "https://example.com/a" ],
      "open_questions" => [ "Why?" ],
      "sender_name" => "David",
      "receiver_agent_id" => @agent.id
    }, handoff.payload)
  end

  private
    def grant!(agent, capability)
      AgentGrant.create!(agent: agent, room: @board, granted_by: users(:david), capability: capability)
    end
end
