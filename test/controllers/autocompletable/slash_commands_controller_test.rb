require "test_helper"

class Autocompletable::SlashCommandsControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
    @room = rooms(:watercooler)
  end

  test "lists built-ins with metadata" do
    get autocompletable_slash_commands_url(room_id: @room.id), as: :json

    assert_response :success
    names = response.parsed_body.map { |command| command["name"] }
    assert_equal %w[ huddle event poll remind status dnd ooo shrug me play ], names

    poll = response.parsed_body.find { |command| command["name"] == "poll" }
    assert_equal "poll", poll["value"]
    assert poll["description"].present?
    assert_nil poll["agent"]
  end

  test "exposes takes_arguments for immediate and argument commands" do
    get autocompletable_slash_commands_url(room_id: @room.id), as: :json

    assert_response :success
    by_name = response.parsed_body.index_by { |command| command["name"] }
    assert_equal false, by_name["poll"]["takes_arguments"]
    assert_equal false, by_name["event"]["takes_arguments"]
    assert_equal false, by_name["huddle"]["takes_arguments"]
    assert_equal true, by_name["shrug"]["takes_arguments"]
    assert_equal true, by_name["remind"]["takes_arguments"]
    assert_equal "<when> <text>", by_name["remind"]["arg_hint"]
  end

  test "includes the room's agent commands" do
    AgentSlashCommand.create!(agent: agents(:bender_agent), room: @room, name: "deploy", description: "Ship it")

    get autocompletable_slash_commands_url(room_id: @room.id), as: :json

    assert_response :success
    deploy = response.parsed_body.find { |command| command["name"] == "deploy" }
    assert_equal "Ship it", deploy["description"]
    assert_equal "Bender Bot", deploy["agent"]
    assert_equal true, deploy["takes_arguments"]
  end

  test "agent commands registered without arguments run immediately" do
    AgentSlashCommand.create!(agent: agents(:bender_agent), room: @room, name: "deploy", takes_arguments: false)

    get autocompletable_slash_commands_url(room_id: @room.id), as: :json

    assert_response :success
    deploy = response.parsed_body.find { |command| command["name"] == "deploy" }
    assert_equal false, deploy["takes_arguments"]
  end

  test "thread conversations hide root-only commands" do
    thread = ChannelThread.create!(room: @room, creator: users(:david), name: "Side chat")

    get autocompletable_slash_commands_url(room_id: @room.id, thread_id: thread.id), as: :json

    assert_response :success
    names = response.parsed_body.map { |command| command["name"] }
    assert_not_includes names, "poll"
    assert_includes names, "shrug"
  end

  test "filters by query" do
    get autocompletable_slash_commands_url(room_id: @room.id, query: "sta"), as: :json

    assert_response :success
    names = response.parsed_body.map { |command| command["name"] }
    assert_includes names, "status"
    assert_not_includes names, "poll"
  end

  test "non-members get 404" do
    private_room = Rooms::Closed.create!(name: "Private", creator: users(:jason))
    private_room.memberships.grant_to users(:jason)

    get autocompletable_slash_commands_url(room_id: private_room.id), as: :json

    assert_response :not_found
  end
end
