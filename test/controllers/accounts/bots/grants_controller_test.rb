require "test_helper"

class Accounts::Bots::GrantsControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
    @bot = users(:bender)
    @agent = agents(:bender_agent)
  end

  test "index lists grants and the legacy notice" do
    get account_bot_grants_url(@bot)

    assert_response :ok
    assert_match "Legacy access", response.body
    assert_match "post_messages", response.body
  end

  test "index renders grant timestamps for the local-time controller" do
    AgentGrant.create!(agent: @agent, granted_by: users(:david), capability: "post_messages")

    get account_bot_grants_url(@bot)

    assert_response :ok
    assert_select "time[data-local-time-target='datetime'][datetime]", minimum: 1
  end

  test "index lists existing grants with scope and enforcement state" do
    AgentGrant.create!(agent: @agent, room: rooms(:watercooler), granted_by: users(:david), capability: "post_messages")
    AgentGrant.create!(agent: @agent, granted_by: users(:david), capability: "external_action")

    get account_bot_grants_url(@bot)

    assert_response :ok
    assert_match "All Talk", response.body
    assert_match "Workspace-wide", response.body
    assert_match "enforced", response.body
    assert_no_match "not yet enforced", response.body
    assert_no_match "Legacy access", response.body
  end

  test "room picker includes direct rooms under their display names" do
    direct_room = rooms(:bender_and_kevin)

    get account_bot_grants_url(@bot)

    assert_response :ok
    assert_select "select[name='agent_grant[room_id]'] option[value='#{direct_room.id}']", text: "Bender Bot and Kevin"
  end

  test "index names direct-room grants after the other participants" do
    AgentGrant.create!(agent: @agent, room: rooms(:bender_and_kevin), granted_by: users(:david), capability: "post_messages")

    get account_bot_grants_url(@bot)

    assert_response :ok
    assert_match "Bender Bot and Kevin", response.body
    assert_no_match "Deleted room", response.body
  end

  test "index renders a fallback for grants whose room was deleted" do
    AgentGrant.create!(agent: @agent, room: rooms(:watercooler), granted_by: users(:david), capability: "post_messages")
    rooms(:watercooler).destroy!

    get account_bot_grants_url(@bot)

    assert_response :ok
    assert_match "Deleted room", response.body
  end

  test "create grants a room capability" do
    assert_difference -> { AgentGrant.count }, +1 do
      post account_bot_grants_url(@bot), params: {
        agent_grant: { capability: "post_messages", room_id: rooms(:watercooler).id }
      }
    end

    assert_redirected_to account_bot_grants_url(@bot)
    grant = AgentGrant.last
    assert_equal @agent, grant.agent
    assert_equal rooms(:watercooler), grant.room
    assert_equal users(:david), grant.granted_by
    assert grant.active?
  end

  test "create with a blank room grants workspace-wide" do
    post account_bot_grants_url(@bot), params: {
      agent_grant: { capability: "react", room_id: "" }
    }

    assert_redirected_to account_bot_grants_url(@bot)
    assert AgentGrant.last.workspace_wide?
  end

  test "create is idempotent for an already active grant" do
    existing = AgentGrant.create!(agent: @agent, room: rooms(:watercooler), granted_by: users(:david), capability: "post_messages")

    assert_no_difference -> { AgentGrant.count } do
      post account_bot_grants_url(@bot), params: {
        agent_grant: { capability: "post_messages", room_id: rooms(:watercooler).id }
      }
    end

    assert_redirected_to account_bot_grants_url(@bot)
    assert_predicate existing.reload, :active?
  end

  test "create regrants a capability after the previous grant was revoked" do
    AgentGrant.create!(agent: @agent, room: rooms(:watercooler), granted_by: users(:david), capability: "post_messages").revoke!

    assert_difference -> { AgentGrant.active.count }, +1 do
      post account_bot_grants_url(@bot), params: {
        agent_grant: { capability: "post_messages", room_id: rooms(:watercooler).id }
      }
    end

    assert_redirected_to account_bot_grants_url(@bot)
  end

  test "create rejects a room-scoped dm_anyone grant" do
    assert_no_difference -> { AgentGrant.count } do
      post account_bot_grants_url(@bot), params: {
        agent_grant: { capability: "dm_anyone", room_id: rooms(:watercooler).id }
      }
    end

    assert_response :unprocessable_entity
    assert_match "dm_anyone is granted workspace-wide only", response.body
  end

  test "create rejects unknown capabilities" do
    assert_no_difference -> { AgentGrant.count } do
      post account_bot_grants_url(@bot), params: {
        agent_grant: { capability: "launch_missiles", room_id: "" }
      }
    end
    assert_response :unprocessable_entity
  end

  test "create reuses the existing grant when the unique index rejects a concurrent create" do
    AgentGrant.create!(agent: @agent, room: rooms(:watercooler), granted_by: users(:david), capability: "post_messages")
    AgentGrant.any_instance.stubs(:save).raises(ActiveRecord::RecordNotUnique)

    assert_no_difference -> { AgentGrant.count } do
      post account_bot_grants_url(@bot), params: {
        agent_grant: { capability: "post_messages", room_id: rooms(:watercooler).id }
      }
    end

    assert_redirected_to account_bot_grants_url(@bot)
  end

  test "destroy revokes immediately and the next post is forbidden" do
    grant = AgentGrant.create!(agent: @agent, room: rooms(:watercooler), granted_by: users(:david), capability: "post_messages")

    delete account_bot_grant_url(@bot, grant)

    assert_redirected_to account_bot_grants_url(@bot)
    assert grant.reload.revoked?
    delete session_url

    post room_bot_messages_url(rooms(:watercooler), bot_key_for(@bot)), params: +"Hello!"
    assert_response :forbidden
  end

  test "index creates an agent for legacy bots missing one" do
    @agent.delete

    get account_bot_grants_url(@bot)

    assert_response :ok
    assert @bot.reload.agent.present?
  end

  test "agent owner without admin rights views and revokes grants but cannot create them" do
    grant = @agent.agent_grants.create!(capability: "post_messages", room: rooms(:watercooler), granted_by: users(:david))
    @agent.update!(owner: users(:kevin))
    sign_in users(:kevin)

    get account_bot_grants_url(@bot)
    assert_response :ok
    assert_select "form[action=?][method=post] select[name=?]", account_bot_grants_path(@bot), "agent_grant[capability]", count: 0

    %w[ read_messages external_action ].each do |capability|
      [ rooms(:watercooler).id, "" ].each do |room_id|
        assert_no_difference -> { AgentGrant.count } do
          post account_bot_grants_url(@bot), params: { agent_grant: { capability: capability, room_id: room_id } }
        end
        assert_response :forbidden
      end
    end

    delete account_bot_grant_url(@bot, grant)
    assert_redirected_to account_bot_grants_url(@bot)
    assert grant.reload.revoked?
  end

  test "an owner demoted from administrator loses grant creation" do
    @agent.update!(owner: users(:jason))
    users(:jason).update!(role: :member)
    sign_in users(:jason)

    assert_no_difference -> { AgentGrant.count } do
      post account_bot_grants_url(@bot), params: { agent_grant: { capability: "external_action", room_id: "" } }
    end
    assert_response :forbidden
  end

  test "a grant for a room that does not exist is refused" do
    assert_no_difference -> { AgentGrant.count } do
      post account_bot_grants_url(@bot), params: { agent_grant: { capability: "post_messages", room_id: Room.maximum(:id).to_i + 1000 } }
    end

    assert_response :unprocessable_entity
    assert_match "Room must be an existing room", response.body
  end

  test "back link goes to the bot editor for admins" do
    get account_bot_grants_url(@bot)

    assert_response :ok
    assert_select "a[href='#{edit_account_bot_path(@bot)}']", 1
  end

  test "back link goes to the bot profile for owners without admin rights" do
    @agent.update!(owner: users(:kevin))
    sign_in users(:kevin)

    get account_bot_grants_url(@bot)

    assert_response :ok
    assert_select "a[href='#{user_path(@bot)}']", 1
    assert_select "a[href='#{edit_account_bot_path(@bot)}']", 0
  end

  test "non-owner cannot manage grants" do
    sign_in users(:kevin)

    get account_bot_grants_url(@bot)
    assert_response :forbidden

    assert_no_difference -> { AgentGrant.count } do
      post account_bot_grants_url(@bot), params: {
        agent_grant: { capability: "post_messages", room_id: "" }
      }
    end
    assert_response :forbidden

    grant = AgentGrant.create!(agent: @agent, granted_by: users(:david), capability: "post_messages")
    delete account_bot_grant_url(@bot, grant)
    assert_response :forbidden
    assert_not grant.reload.revoked?
  end
end
