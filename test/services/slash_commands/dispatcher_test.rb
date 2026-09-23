require "test_helper"

class SlashCommands::DispatcherTest < ActiveSupport::TestCase
  setup do
    @user = users(:david)
    @user.update!(time_zone: "America/New_York")
    @room = rooms(:watercooler)
    travel_to Time.zone.local(2026, 9, 23, 12, 0, 0)
  end

  teardown do
    travel_back
  end

  test "registry holds every shipped command with metadata" do
    names = SlashCommands::Registry.all.map(&:name)

    assert_equal %w[ huddle event poll remind status dnd shrug me play ], names
    SlashCommands::Registry.all.each do |command|
      assert command.description.present?, "#{command.name} needs a description"
      assert_respond_to SlashCommands::Handlers, command.handler
      assert_includes [ true, false ], command.permission.call(@user, @room, nil)
    end
  end

  test "command_text? matches slash commands but not escapes or play passthrough" do
    assert SlashCommands::Dispatcher.command_text?("/poll")
    assert SlashCommands::Dispatcher.command_text?("/me waves")
    assert_not SlashCommands::Dispatcher.command_text?("//poll")
    assert_not SlashCommands::Dispatcher.command_text?("hello /poll")
    assert_not SlashCommands::Dispatcher.command_text?("just text")
  end

  test "huddle starts a call when configured" do
    Huddle.stubs(:configured?).returns(true)

    result = dispatch("/huddle")

    assert_equal :start_huddle, result.kind
    assert_equal @room.id, result.payload[:room_id]
  end

  test "huddle errors when unconfigured" do
    Huddle.stubs(:configured?).returns(false)

    result = dispatch("/huddle")

    assert_equal :error, result.kind
    assert_match "not configured", result.message
  end

  test "event opens the prefilled form url" do
    result = dispatch("/event Launch party friday 5pm")

    assert_equal :open_url, result.kind
    uri = URI.parse(result.url)
    query = URI.decode_www_form(uri.query).to_h
    assert_equal "Launch party", query["event[title]"]
    assert_equal "America/New_York", query["event[time_zone]"]
    assert_equal Time.find_zone("America/New_York").local(2026, 9, 25, 17, 0), Time.zone.parse(query["event[starts_at]"])
  end

  test "event without a time prefills the title only" do
    result = dispatch("/event Launch party")

    assert_equal :open_url, result.kind
    assert_includes result.url, "event%5Btitle%5D=Launch+party"
    assert_not_includes result.url, "starts_at"
  end

  test "event rejects past times and blank titles" do
    result = dispatch("/event Retro yesterday")
    assert_equal :open_url, result.kind # "yesterday" is not a time, so it stays in the title

    result = dispatch("/event")
    assert_equal :error, result.kind
    assert_match "Usage", result.message
  end

  test "poll opens the builder in channels but not threads" do
    assert_equal :open_poll, dispatch("/poll").kind

    thread = ChannelThread.create!(room: @room, creator: @user, name: "Side chat")
    result = SlashCommands::Dispatcher.dispatch(user: @user, room: @room, thread: thread, text: "/poll")

    assert_equal :error, result.kind
    assert_match "not in threads", result.message
  end

  test "remind posts and saves with a reminder" do
    result = nil
    assert_difference -> { @room.messages.count }, 1 do
      assert_difference -> { @user.saved_items.count }, 1 do
        result = dispatch("/remind in 20 minutes review the deploy")
      end
    end

    assert_equal :posted, result.kind
    message = @room.messages.ordered.last
    assert_equal "review the deploy", message.plain_text_body
    assert_match "Reminder set", result.notice

    saved_item = @user.saved_items.find_by(message:)
    assert_equal Time.current + 20.minutes, saved_item.remind_at
  end

  test "remind rejects unusable input without posting" do
    assert_no_difference -> { Message.count } do
      result = dispatch("/remind sometime review the deploy")
      assert_equal :error, result.kind
      assert_match "Usage", result.message

      result = dispatch("/remind in 20 minutes")
      assert_equal :error, result.kind
    end
  end

  test "status sets emoji and text until end of day" do
    result = dispatch("/status 🚂 On a train")

    assert_equal :ephemeral, result.kind
    assert_match "On a train", result.message

    @user.reload
    assert_equal "🚂", @user.custom_status_emoji
    assert_equal "On a train", @user.custom_status_text
    assert_in_delta Time.current.in_time_zone("America/New_York").end_of_day.to_f, @user.custom_status_expires_at.to_f, 0.001
  end

  test "status rejects blank arguments" do
    result = dispatch("/status")

    assert_equal :error, result.kind
    assert_match "Usage", result.message
  end

  test "dnd toggles, takes durations, and turns off" do
    assert_equal "Do Not Disturb is on.", dispatch("/dnd").message
    assert @user.reload.dnd_enabled?

    assert_equal "Do Not Disturb is off.", dispatch("/dnd").message
    assert_not @user.reload.dnd_enabled?

    assert_match "until", dispatch("/dnd 2h").message
    assert_equal Time.current + 2.hours, @user.reload.dnd_until

    assert_equal "Do Not Disturb is off.", dispatch("/dnd off").message
    assert_not @user.reload.dnd_enabled?
    assert_nil @user.dnd_until
  end

  test "dnd rejects garbage durations" do
    result = dispatch("/dnd eventually")

    assert_equal :error, result.kind
    assert_match "Usage", result.message
  end

  test "shrug posts with the shrug" do
    result = dispatch("/shrug ship it")

    assert_equal :posted, result.kind
    message = @room.messages.ordered.last
    assert_equal "ship it #{SlashCommands::Dispatcher::SHRUG}", message.markdown_source
    assert_includes message.plain_text_body, "¯_(ツ)_/¯"
  end

  test "slash posts in threads skip the legacy webhook fanout" do
    legacy = User.create_bot!(name: "Legacy Note", webhook_url: "https://example.test/legacy-note")
    @room.memberships.grant_to(legacy)
    thread = ChannelThread.create!(room: @room, creator: @user, name: "Side chat")

    assert_no_enqueued_jobs only: Bot::WebhookJob do
      result = SlashCommands::Dispatcher.dispatch(
        user: @user, room: @room, thread: thread, text: "/shrug Hey @[Legacy Note]"
      )
      assert_equal :posted, result.kind
    end
  end

  test "slash posts in channels fan out to legacy webhooks" do
    legacy = User.create_bot!(name: "Legacy Note", webhook_url: "https://example.test/legacy-note")
    @room.memberships.grant_to(legacy)

    assert_enqueued_jobs 1, only: Bot::WebhookJob do
      result = dispatch("/shrug Hey @[Legacy Note]")
      assert_equal :posted, result.kind
    end
  end

  test "slash posts in threads process attachments once" do
    thread = ChannelThread.create!(room: @room, creator: @user, name: "Side chat")
    Message.any_instance.expects(:process_attachment).once

    result = SlashCommands::Dispatcher.dispatch(user: @user, room: @room, thread: thread, text: "/shrug hi")

    assert_equal :posted, result.kind
  end

  test "me posts an action line" do
    result = dispatch("/me is reviewing the deploy")

    assert_equal :posted, result.kind
    message = @room.messages.ordered.last
    assert message.action?
    assert_equal "is reviewing the deploy", message.plain_text_body
  end

  test "me requires an action" do
    assert_no_difference -> { Message.count } do
      result = dispatch("/me")
      assert_equal :error, result.kind
    end
  end

  test "play posts through the normal message path" do
    result = dispatch("/play tada")

    assert_equal :posted, result.kind
    assert_equal "tada", @room.messages.ordered.last.sound.name
  end

  test "slash posts never start a stream" do
    result = dispatch("/shrug ship it")

    assert_equal :posted, result.kind
    assert_not_predicate @room.messages.ordered.last, :streaming?
  end

  test "unknown commands error with the available list" do
    result = dispatch("/frobnicate")

    assert_equal :error, result.kind
    assert_match "Unknown command", result.message
    assert_match "/poll", result.message
  end

  test "invoking an agent command delivers a slash_command event" do
    agent = agents(:bender_agent)
    AgentSlashCommand.create!(agent: agent, room: @room, name: "deploy", description: "Ship it")

    result = nil
    assert_difference -> { agent.agent_events.count }, 1 do
      result = dispatch("/deploy staging")
    end

    assert_equal :ephemeral, result.kind
    assert_equal "Sent to Bender Bot", result.message

    event = agent.agent_events.ordered.last
    assert_equal "slash_command", event.event_type
    assert_equal @room, event.room
    assert_equal @user, event.actor
    assert_equal "deploy", event.metadata["command"]
    assert_equal "staging", event.metadata["arguments"]
  end

  test "invoking an agent command in a thread records the thread" do
    agent = agents(:bender_agent)
    AgentSlashCommand.create!(agent: agent, room: @room, name: "deploy", description: "Ship it")
    thread = ChannelThread.create!(room: @room, creator: @user, name: "Side chat")

    result = SlashCommands::Dispatcher.dispatch(user: @user, room: @room, thread: thread, text: "/deploy staging")

    assert_equal :ephemeral, result.kind
    event = agent.agent_events.ordered.last
    assert_equal thread.id, event.metadata["thread_id"]
  end

  test "invoking an agent command in the channel records no thread" do
    agent = agents(:bender_agent)
    AgentSlashCommand.create!(agent: agent, room: @room, name: "deploy", description: "Ship it")

    dispatch("/deploy staging")

    event = agent.agent_events.ordered.last
    assert_nil event.metadata["thread_id"]
  end

  test "invoking an agent command requires post_messages" do
    agent = agents(:bender_agent)
    AgentSlashCommand.create!(agent: agent, room: @room, name: "deploy")
    AgentGrant.create!(agent: agent, room: @room, granted_by: users(:david), capability: "read_messages")

    assert_no_difference -> { agent.agent_events.count } do
      result = dispatch("/deploy staging")
      assert_equal :error, result.kind
      assert_match "no longer available", result.message
    end
  end

  test "invoking an agent command is rate limited" do
    agent = agents(:bender_agent)
    AgentSlashCommand.create!(agent: agent, room: @room, name: "deploy")

    SlashCommands::Dispatcher::AGENT_COMMANDS_PER_MINUTE.times do
      dispatch("/deploy staging")
    end

    result = dispatch("/deploy staging")
    assert_equal :error, result.kind
    assert_match "too many", result.message
  end

  private
    def dispatch(text)
      SlashCommands::Dispatcher.dispatch(user: @user, room: @room, text: text)
    end
end
