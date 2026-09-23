require "test_helper"

class AuditLog::AgentsAuditTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
    @bot = users(:bender)
    @agent = agents(:bender_agent)
  end

  test "agent create is recorded without the key" do
    assert_difference -> { AuditLog.where(action: "agent.create").count }, +1 do
      post account_bots_url, params: { user: { name: "Audit Bot" } }
    end

    assert_response :created
    entry = AuditLog.where(action: "agent.create").last
    assert_equal users(:david).id, entry.actor_id
    assert_equal User.bot.last.agent.id, entry.target_id
    assert_equal "Agent", entry.target_type
    assert_equal "Audit Bot", entry.details["name"]
    key = response.body[/value="([^"]+)"/, 1]
    assert key.present?
    assert_no_match key, entry.details.to_json
  end

  test "visiting credentials for a legacy bot records its agent creation" do
    legacy = User.create_bot!(name: "Legacy Bot")
    assert_nil legacy.agent

    assert_difference -> { AuditLog.where(action: "agent.create").count }, +1 do
      get account_bot_credentials_url(legacy)
    end

    assert_response :success
    entry = AuditLog.where(action: "agent.create").last
    assert_equal users(:david).id, entry.actor_id
    assert_equal legacy.reload.agent.id, entry.target_id
  end

  test "visiting grants for a legacy bot records its agent creation" do
    legacy = User.create_bot!(name: "Legacy Bot")
    assert_nil legacy.agent

    assert_difference -> { AuditLog.where(action: "agent.create").count }, +1 do
      get account_bot_grants_url(legacy)
    end

    assert_response :success
    assert_equal legacy.reload.agent.id, AuditLog.where(action: "agent.create").last.target_id
  end

  test "agent edit is recorded with bot and agent changes" do
    assert_difference -> { AuditLog.where(action: "agent.update").count }, +1 do
      patch account_bot_url(@bot), params: {
        user: { name: "Bender 2" },
        agent: { provider: "openai", runtime: "cloud", description: "Bending" }
      }
    end

    entry = AuditLog.where(action: "agent.update").last
    assert_equal @agent.id, entry.target_id
    assert_equal({ "before" => "Bender Bot", "after" => "Bender 2" }, entry.details["name"])
    assert_equal({ "before" => nil, "after" => "openai" }, entry.details["provider"])
    assert_equal({ "before" => nil, "after" => "Bending" }, entry.details["description"])
  end

  test "unchanged agent edit writes no row" do
    assert_no_difference -> { AuditLog.where(action: "agent.update").count } do
      patch account_bot_url(@bot), params: { user: { name: @bot.name } }
    end
  end

  test "webhook URL change gets its own row" do
    assert_difference -> { AuditLog.where(action: "agent.webhook_url.change").count }, +1 do
      patch account_bot_url(@bot), params: { user: { webhook_url: "https://example.com/hooked?token=s3cret" } }
    end

    entry = AuditLog.where(action: "agent.webhook_url.change").last
    before = entry.details["webhook_url"]["before"]
    after = entry.details["webhook_url"]["after"]
    assert_equal "http://example.com", before["origin"]
    assert_equal "https://example.com", after["origin"]
    assert_equal 12, after["digest"].length
    assert_not_equal before["digest"], after["digest"]
    assert_no_match "hooked", entry.details.to_json
    assert_no_match "s3cret", entry.details.to_json
  end

  test "removing a bot records the suspension" do
    assert_difference -> { AuditLog.where(action: "agent.suspend").count }, +1 do
      delete account_bot_url(@bot)
    end

    entry = AuditLog.where(action: "agent.suspend").last
    assert_equal @agent.id, entry.target_id
  end

  test "deactivating an owner suspends their agents with the admin as actor" do
    @agent.update!(owner: users(:kevin))

    assert_difference -> { AuditLog.where(action: "agent.suspend").count }, +1 do
      delete account_user_url(users(:kevin))
    end

    entry = AuditLog.where(action: "agent.suspend").last
    assert_equal users(:david).id, entry.actor_id
    assert_equal @agent.id, entry.target_id
  end

  test "banning an owner suspends their agents" do
    @agent.update!(owner: users(:kevin))

    assert_difference -> { AuditLog.where(action: "agent.suspend").count }, +1 do
      post user_ban_url(users(:kevin))
    end
  end

  test "credential create and revoke are recorded without the secret" do
    assert_difference -> { AuditLog.where(action: "agent.credential.create").count }, +1 do
      post account_bot_credentials_url(@bot), params: { agent_credential: { name: "CI runner" } }
    end

    assert_response :created
    credential = AgentCredential.last
    secret = response.body[/value="([^"]+)"/, 1]
    create = AuditLog.where(action: "agent.credential.create").last
    assert_equal credential.id, create.target_id
    assert_equal "AgentCredential", create.target_type
    assert_equal "CI runner", create.details["name"]
    assert_no_match secret, create.details.to_json

    assert_difference -> { AuditLog.where(action: "agent.credential.revoke").count }, +1 do
      delete account_bot_credential_url(@bot, credential)
    end

    revoke = AuditLog.where(action: "agent.credential.revoke").last
    assert_equal credential.id, revoke.target_id
    assert_equal "CI runner", revoke.details["name"]

    assert_no_difference -> { AuditLog.where(action: "agent.credential.revoke").count } do
      delete account_bot_credential_url(@bot, credential)
    end
  end

  test "bot key reset is recorded without the key" do
    assert_difference -> { AuditLog.where(action: "agent.credential.reset").count }, +1 do
      patch account_bot_key_url(@bot)
    end

    key = response.body[/value="([^"]+)"/, 1]
    entry = AuditLog.where(action: "agent.credential.reset").last
    assert_no_match key, entry.details.to_json if key.present?
  end

  test "grant create and revoke are recorded" do
    assert_difference -> { AuditLog.where(action: "agent.grant.create").count }, +1 do
      post account_bot_grants_url(@bot), params: {
        agent_grant: { capability: "post_messages", room_id: rooms(:watercooler).id }
      }
    end

    create = AuditLog.where(action: "agent.grant.create").last
    assert_equal "AgentGrant", create.target_type
    assert_equal "post_messages", create.details["capability"]
    assert_equal "All Talk", create.details["room"]

    grant = AgentGrant.last
    assert_difference -> { AuditLog.where(action: "agent.grant.revoke").count }, +1 do
      delete account_bot_grant_url(@bot, grant)
    end

    revoke = AuditLog.where(action: "agent.grant.revoke").last
    assert_equal grant.id, revoke.target_id
    assert_equal "post_messages", revoke.details["capability"]

    assert_no_difference -> { AuditLog.where(action: "agent.grant.revoke").count } do
      delete account_bot_grant_url(@bot, grant)
    end
  end

  test "re-granting an existing capability writes no second row" do
    post account_bot_grants_url(@bot), params: {
      agent_grant: { capability: "post_messages", room_id: rooms(:watercooler).id }
    }

    assert_no_difference -> { AuditLog.where(action: "agent.grant.create").count } do
      post account_bot_grants_url(@bot), params: {
        agent_grant: { capability: "post_messages", room_id: rooms(:watercooler).id }
      }
    end
  end

  test "webhook secret reset is recorded without the secret" do
    old_secret = @agent.webhook_signing_secret

    assert_difference -> { AuditLog.where(action: "agent.webhook_secret.reset").count }, +1 do
      post account_bot_webhook_secret_url(@bot)
    end

    assert_not_equal old_secret, @agent.reload.webhook_signing_secret
    entry = AuditLog.where(action: "agent.webhook_secret.reset").last
    assert_equal @agent.id, entry.target_id
    assert_equal({}, entry.details)
  end

  test "agent GitHub connect and disconnect are recorded without the token" do
    stub_github_user("bender-machine")

    assert_difference -> { AuditLog.where(action: "agent.github.connect").count }, +1 do
      post account_bot_github_connection_url(@bot), params: { access_token: "agent_pat_pasted" }
    end

    connect = AuditLog.where(action: "agent.github.connect").last
    assert_equal({ "github_login" => "bender-machine" }, connect.details)
    assert_no_match "agent_pat_pasted", connect.details.to_json

    assert_difference -> { AuditLog.where(action: "agent.github.disconnect").count }, +1 do
      delete account_bot_github_connection_url(@bot)
    end

    disconnect = AuditLog.where(action: "agent.github.disconnect").last
    assert_equal({ "github_login" => "bender-machine" }, disconnect.details)
  end

  test "approval decisions are recorded with decision and note" do
    @agent.update!(owner: users(:kevin))
    approval = AgentApproval.create!(agent: @agent, room: rooms(:watercooler), action: "deploy", summary: "Ship it")
    sign_in users(:kevin)

    assert_difference -> { AuditLog.where(action: "agent.approval.decide").count }, +1 do
      patch agent_approval_url(approval, decision: "approved"), as: :json
    end

    entry = AuditLog.where(action: "agent.approval.decide").last
    assert_equal users(:kevin).id, entry.actor_id
    assert_equal approval.id, entry.target_id
    assert_equal({ "before" => "pending", "after" => "approved" }, entry.details["decision"])

    second = AgentApproval.create!(agent: @agent, room: rooms(:watercooler), action: "deploy", summary: "Ship it")
    patch agent_approval_url(second, decision: "denied", decision_note: "not now"), as: :json

    denial = AuditLog.where(action: "agent.approval.decide").last
    assert_equal({ "before" => "pending", "after" => "denied" }, denial.details["decision"])
    assert_equal "not now", denial.details["note"]
  end

  test "deciding an already-decided approval writes no row" do
    @agent.update!(owner: users(:kevin))
    approval = AgentApproval.create!(agent: @agent, room: rooms(:watercooler), action: "deploy", summary: "Ship it")
    sign_in users(:kevin)
    patch agent_approval_url(approval, decision: "approved"), as: :json

    assert_no_difference -> { AuditLog.where(action: "agent.approval.decide").count } do
      patch agent_approval_url(approval, decision: "denied"), as: :json
      assert_response :unprocessable_entity
    end
  end

  private
    def stub_github_user(login)
      stub_request(:get, "https://api.github.com/user")
        .to_return(status: 200, body: { login: login }.to_json)
    end
end
