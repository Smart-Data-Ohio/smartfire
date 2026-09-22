require "test_helper"

# Deactivating a person stops the agents they own: the agents are
# suspended, their grants revoked, and their tokens and bot keys refused.
class AgentOwnerDeactivationTest < ActionDispatch::IntegrationTest
  setup do
    @agent = agents(:bender_agent)
    @room = rooms(:watercooler)
    @secret = "bender-test-secret-1234"
    @bearer = { "Authorization" => "Bearer #{@secret}" }
    WebMock.stub_request(:post, webhooks(:bender).url).to_return(status: 200)
  end

  test "an active owner's agent authenticates with its token and bot key" do
    get agents_me_url, headers: @bearer
    assert_response :success

    assert_difference -> { Message.count }, 1 do
      post room_bot_messages_url(@room, bot_key_for(users(:bender))), params: +"still here"
    end
  end

  test "deactivating the owner suspends the agent and revokes its grants" do
    grant = AgentGrant.create!(agent: @agent, room: @room, capability: "read_messages", granted_by: users(:jason))

    users(:david).deactivate

    assert @agent.reload.suspended?
    assert grant.reload.revoked?
    assert users(:bender).reload.active?, "the agent's own user is suspended, not deactivated"
  end

  test "a deactivated owner's agent token is refused" do
    users(:david).deactivate

    get agents_me_url, headers: @bearer

    assert_response :unauthorized
  end

  test "a deactivated owner's agent bot key is refused" do
    users(:david).deactivate

    assert_no_difference -> { Message.count } do
      post room_bot_messages_url(@room, bot_key_for(users(:bender))), params: +"should not post"
    end
    assert_response :forbidden
  end

  test "agents owned by someone else keep working" do
    @agent.update!(owner: users(:jason))

    users(:david).deactivate

    assert_not @agent.reload.suspended?
    get agents_me_url, headers: @bearer
    assert_response :success
  end

  test "banning the owner suspends the agent, revokes its grants, and refuses its credentials" do
    grant = AgentGrant.create!(agent: @agent, room: @room, capability: "post_messages", granted_by: users(:jason))

    users(:david).ban

    assert @agent.reload.suspended?
    assert grant.reload.revoked?
    get agents_me_url, headers: @bearer
    assert_response :unauthorized
    assert_no_difference -> { Message.count } do
      post room_bot_messages_url(@room, bot_key_for(users(:bender))), params: +"should not post"
    end
    assert_response :forbidden
  end

  test "banning someone else leaves the agent working" do
    users(:kevin).ban

    assert_not @agent.reload.suspended?
    get agents_me_url, headers: @bearer
    assert_response :success
  end
end
