require "test_helper"

class Rooms::SlashCommandsControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
    @room = rooms(:watercooler)
    users(:david).update!(time_zone: "America/New_York")
  end

  test "shrug posts through the dispatcher" do
    assert_difference -> { @room.messages.count }, 1 do
      post room_slash_commands_url(@room), params: { text: "/shrug ship it" }, as: :json
    end

    assert_response :success
    assert_equal "posted", response.parsed_body["status"]
    assert_equal @room.messages.ordered.last.id, response.parsed_body["message_id"]
  end

  test "unknown commands answer an error without posting" do
    assert_no_difference -> { Message.count } do
      post room_slash_commands_url(@room), params: { text: "/frobnicate" }, as: :json
    end

    assert_response :success
    assert_equal "error", response.parsed_body["status"]
    assert_match "Unknown command", response.parsed_body["message"]
  end

  test "status answers ephemeral confirmation" do
    post room_slash_commands_url(@room), params: { text: "/status 🚂 On a train" }, as: :json

    assert_response :success
    assert_equal "ephemeral", response.parsed_body["status"]
    assert_match "On a train", response.parsed_body["message"]
    assert_equal "🚂", users(:david).reload.custom_status_emoji
  end

  test "event answers an open_url" do
    post room_slash_commands_url(@room), params: { text: "/event Launch party" }, as: :json

    assert_response :success
    assert_equal "open_url", response.parsed_body["status"]
    assert_match "Launch", response.parsed_body["url"]
  end

  test "bare event opens the blank form" do
    post room_slash_commands_url(@room), params: { text: "/event" }, as: :json

    assert_response :success
    assert_equal "open_url", response.parsed_body["status"]
    assert_equal new_room_event_path(@room), URI.parse(response.parsed_body["url"]).path
  end

  test "poll answers open_poll" do
    post room_slash_commands_url(@room), params: { text: "/poll" }, as: :json

    assert_response :success
    assert_equal "open_poll", response.parsed_body["status"]
  end

  test "agent commands invoke through the room" do
    AgentSlashCommand.create!(agent: agents(:bender_agent), room: @room, name: "deploy")

    post room_slash_commands_url(@room), params: { text: "/deploy staging" }, as: :json

    assert_response :success
    assert_equal "ephemeral", response.parsed_body["status"]
    assert_equal "Sent to Bender Bot", response.parsed_body["message"]
  end

  test "thread commands dispatch with the thread" do
    thread = ChannelThread.create!(room: @room, creator: users(:david), name: "Side chat")

    post room_slash_commands_url(@room), params: { text: "/poll", thread_id: thread.id }, as: :json

    assert_response :success
    assert_equal "error", response.parsed_body["status"]
    assert_match "not in threads", response.parsed_body["message"]
  end

  test "non-members get 404" do
    private_room = Rooms::Closed.create!(name: "Private", creator: users(:jason))
    private_room.memberships.grant_to users(:jason)

    post room_slash_commands_url(private_room), params: { text: "/shrug hi" }, as: :json

    assert_response :not_found
  end

  test "bots are forbidden" do
    delete session_url

    bot = users(:bender)
    bot.update!(email_address: "bender@example.test", password: "secret123456")
    sign_in bot

    post room_slash_commands_url(@room), params: { text: "/shrug hi" }, as: :json

    assert_response :forbidden
  end
end
