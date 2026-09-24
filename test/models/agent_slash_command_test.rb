require "test_helper"

class AgentSlashCommandTest < ActiveSupport::TestCase
  setup do
    @agent = agents(:bender_agent)
    @room = rooms(:watercooler)
  end

  test "registers a command" do
    command = AgentSlashCommand.create!(agent: @agent, room: @room, name: "Deploy", description: " Ship it ")

    assert_equal "deploy", command.name
    assert_equal "Ship it", command.description
  end

  test "new commands take arguments by default" do
    command = AgentSlashCommand.create!(agent: @agent, room: @room, name: "deploy")

    assert_equal true, command.takes_arguments
  end

  test "names are unique per room across agents" do
    AgentSlashCommand.create!(agent: @agent, room: @room, name: "deploy")

    other = Agent.create!(user: User.create_bot!(name: "Other Bot"), owner: users(:david), kind: :workspace)
    duplicate = AgentSlashCommand.new(agent: other, room: @room, name: "deploy")

    assert_not duplicate.valid?
    assert_includes duplicate.errors[:name], "has already been taken"
  end

  test "names must be lowercase command words" do
    [ "Deploy!", "/deploy", "two words", "x" * 33 ].each do |name|
      command = AgentSlashCommand.new(agent: @agent, room: @room, name: name)
      assert_not command.valid?, "#{name.inspect} should be invalid"
    end
  end

  test "names cannot shadow built-ins" do
    command = AgentSlashCommand.new(agent: @agent, room: @room, name: "poll")

    assert_not command.valid?
    assert_match "built-in", command.errors[:name].first
  end

  test "descriptions are capped" do
    command = AgentSlashCommand.new(agent: @agent, room: @room, name: "deploy", description: "x" * 141)

    assert_not command.valid?
  end
end
