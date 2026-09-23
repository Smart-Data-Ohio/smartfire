require "test_helper"

class Agents::PollsControllerTest < ActionDispatch::IntegrationTest
  setup do
    @room = rooms(:watercooler)
    @bot = users(:bender)
    @agent = agents(:bender_agent)
    @secret = "bender-test-secret-1234"
  end

  test "creates a poll with post_messages" do
    grant!(capability: "post_messages", room: @room)

    assert_difference -> { Poll.count }, 1 do
      post "/rooms/#{@room.id}/agents/polls", params: {
        question: "Lunch?", options: [ "Tacos", "Pizza" ], multiple: true
      }.to_json, headers: bearer_headers
    end

    assert_response :created
    assert_equal "Lunch?", response.parsed_body["question"]
    assert_equal true, response.parsed_body["multiple"]
    assert_equal 2, response.parsed_body["options"].size

    poll = Poll.order(:id).last
    assert_equal @bot, poll.message.creator
  end

  test "a legacy agent without any grants can create" do
    post "/rooms/#{@room.id}/agents/polls", params: {
      question: "Lunch?", options: [ "Tacos", "Pizza" ]
    }.to_json, headers: bearer_headers

    assert_response :created
  end

  test "creation requires post_messages" do
    AgentGrant.create!(agent: @agent, room: @room, granted_by: users(:david), capability: "read_messages")

    post "/rooms/#{@room.id}/agents/polls", params: {
      question: "Lunch?", options: [ "Tacos", "Pizza" ]
    }.to_json, headers: bearer_headers

    assert_response :forbidden
  end

  test "creation counts against the message budget" do
    grant!(capability: "post_messages", room: @room)
    @agent.update!(daily_message_cap: 1)
    @room.root_messages.create!(creator: @bot,
      markdown_source: "Spent", client_message_id: "poll-budget-spent")

    assert_no_difference -> { Message.count } do
      post "/rooms/#{@room.id}/agents/polls", params: {
        question: "Lunch?", options: [ "Tacos", "Pizza" ]
      }.to_json, headers: bearer_headers
    end

    assert_response :too_many_requests
    assert_equal "Daily message budget exceeded (1/day)", response.parsed_body["error"]
  end

  test "creation validates without posting" do
    grant!(capability: "post_messages", room: @room)

    assert_no_difference -> { Message.count } do
      post "/rooms/#{@room.id}/agents/polls", params: {
        question: "Lunch?", options: [ "Only" ]
      }.to_json, headers: bearer_headers
    end

    assert_response :unprocessable_entity
  end

  test "creation broadcasts the message with its poll card" do
    grant!(capability: "post_messages", room: @room)

    post "/rooms/#{@room.id}/agents/polls", params: {
      question: "Lunch?", options: [ "Tacos", "Pizza" ]
    }.to_json, headers: bearer_headers

    assert_response :created
    poll = Poll.order(:id).last
    streams = capture_turbo_stream_broadcasts([ @room, :messages ])
    append = streams.find { |stream| stream["action"] == "append" }
    assert append, "expected an append broadcast, got: #{streams.map { |stream| stream["action"] }.inspect}"
    assert_includes append.to_html, ActionView::RecordIdentifier.dom_id(poll, :card)
  end

  test "reads results with post_messages" do
    grant!(capability: "post_messages", room: @room)
    poll = create_poll
    poll.cast_vote!(users(:david), [ poll.poll_options.first.id ])

    get "/rooms/#{@room.id}/agents/polls/#{poll.id}", headers: bearer_headers

    assert_response :success
    assert_equal "Lunch?", response.parsed_body["question"]
    assert_equal 1, response.parsed_body["total_votes"]
    assert_equal [ "David" ], response.parsed_body["options"].first["voters"]
  end

  test "reads hide voters for anonymous polls" do
    grant!(capability: "post_messages", room: @room)
    poll = create_poll(anonymous: true)
    poll.cast_vote!(users(:david), [ poll.poll_options.first.id ])

    get "/rooms/#{@room.id}/agents/polls/#{poll.id}", headers: bearer_headers

    assert_response :success
    assert_equal 1, response.parsed_body["options"].first["votes"]
    assert_nil response.parsed_body["options"].first["voters"]
  end

  test "reads require post_messages" do
    grant!(capability: "read_messages", room: @room)
    poll = create_poll

    get "/rooms/#{@room.id}/agents/polls/#{poll.id}", headers: bearer_headers

    assert_response :forbidden
  end

  test "reads are 404 for polls outside the room" do
    grant!(capability: "post_messages", room: @room)
    other_room = rooms(:designers)
    message = other_room.root_messages.create!(creator: users(:david), markdown_source: "Secret?")
    poll = Poll.create_for_message!(message: message, labels: [ "Yes", "No" ])

    get "/rooms/#{@room.id}/agents/polls/#{poll.id}", headers: bearer_headers

    assert_response :not_found
  end

  test "rejects session requests" do
    sign_in :david

    get "/rooms/#{@room.id}/agents/polls/1", headers: { "Content-Type" => "application/json" }

    assert_response :forbidden
  end

  private
    def create_poll(**options)
      message = @room.root_messages.create!(creator: users(:david), markdown_source: "Lunch?")
      Poll.create_for_message!(message: message, labels: [ "Tacos", "Pizza" ], **options)
    end

    def grant!(capability:, room: nil)
      AgentGrant.create!(agent: @agent, room: room, granted_by: users(:david), capability: capability)
    end

    def bearer_headers
      { "Authorization" => "Bearer #{@secret}", "Content-Type" => "application/json" }
    end
end
