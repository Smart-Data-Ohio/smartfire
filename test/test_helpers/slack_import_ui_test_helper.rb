# Setup and Slack API stubs for the importer UI tests (setup, OAuth,
# runs, personal pages). Creates SlackImport rows directly and drives
# them with start!/cancel!/undo! as they exist on the base branch.
module SlackImportUiTestHelper
  SLACK_TEAM_ID = "T024BE7LD"
  SLACK_TEAM_NAME = "Acme"
  SLACK_TEAM_DOMAIN = "acme"
  SLACK_USER_ID = "U065VRX1T0"
  SLACK_SCOPES = Slack::OAuth::USER_SCOPES.join(",")
  SLACK_ACCESS_URL = "https://slack.com/api/oauth.v2.access"
  SLACK_TEAM_INFO_URL = "https://slack.com/api/team.info"
  SLACK_REVOKE_URL = "https://slack.com/api/auth.revoke"

  def create_slack_workspace!(attributes = {})
    SlackWorkspace.create!({
      client_id: "test-client-id",
      client_secret: "test-client-secret"
    }.merge(attributes))
  end

  def connect_slack!(user, workspace: nil, **attributes)
    workspace ||= SlackWorkspace.current || create_slack_workspace!
    SlackConnection.create!({
      slack_workspace: workspace,
      user: user,
      slack_user_id: "U#{user.id}TEST",
      access_token: "xoxp-test-#{user.id}",
      scopes: SLACK_SCOPES
    }.merge(attributes))
  end

  def create_slack_import!(attributes = {})
    workspace = attributes.delete(:workspace) || SlackWorkspace.current || create_slack_workspace!
    user = attributes.delete(:user) || users(:david)
    SlackImport.create!({
      slack_workspace: workspace,
      user: user,
      kind: "workspace",
      mode: "dry_run",
      status: "completed"
    }.merge(attributes))
  end

  # The documented oauth.v2.access shape (see
  # https://docs.slack.dev/reference/methods/oauth.v2.access): the user
  # token, its scopes, and the team the grant came from.
  def slack_exchange_body(team_id: SLACK_TEAM_ID, team_name: SLACK_TEAM_NAME,
      scopes: SLACK_SCOPES, slack_user_id: SLACK_USER_ID, token: "xoxp-granted")
    {
      "ok" => true,
      "app_id" => "A0118NQPZZC",
      "team" => { "id" => team_id, "name" => team_name },
      "authed_user" => {
        "id" => slack_user_id,
        "scope" => scopes,
        "access_token" => token,
        "token_type" => "user"
      }
    }
  end

  def stub_slack_code_exchange(body = slack_exchange_body)
    stub_request(:post, SLACK_ACCESS_URL)
      .to_return(status: 200, body: body.to_json, headers: { "Content-Type" => "application/json" })
  end

  def stub_slack_team_info(domain: SLACK_TEAM_DOMAIN, team_id: SLACK_TEAM_ID, team_name: SLACK_TEAM_NAME)
    stub_request(:get, SLACK_TEAM_INFO_URL)
      .to_return(status: 200,
        body: { "ok" => true, "team" => { "id" => team_id, "name" => team_name, "domain" => domain } }.to_json,
        headers: { "Content-Type" => "application/json" })
  end

  def stub_slack_revoke(revoked: true)
    stub_request(:post, SLACK_REVOKE_URL)
      .to_return(status: 200, body: { "ok" => true, "revoked" => revoked }.to_json)
  end

  def slack_conversation_entry(id: "C111", name: "general", type: "public_channel",
      archived: false, members: 12, messages: 100, threads: 4,
      target: { "action" => "create", "room_id" => nil, "room_name" => "general" }, done: true)
    {
      "id" => id, "name" => name, "type" => type, "archived" => archived,
      "members" => members, "messages" => messages, "threads" => threads,
      "target" => target, "done" => done
    }
  end

  def slack_stats_shape(conversations: [ slack_conversation_entry ],
      samples: [ { "conversation" => "general", "slack_text" => "hello *world*",
        "markdown" => "hello **world**" } ],
      overrides: {})
    {
      "phase" => "done",
      "users" => { "matched" => 3, "placeholders" => 2, "deactivated" => 1, "bots" => 1, "total" => 7 },
      "conversations" => conversations,
      "counts" => { "rooms_created" => 1, "rooms_merged" => 0, "messages" => 100,
        "replies" => 8, "threads" => 4, "reactions" => 5, "pins" => 1,
        "files_linked" => 2, "skipped" => 0 },
      "current" => nil,
      "samples" => samples,
      "issues_count" => 0,
      "api_calls" => 42
    }.merge(overrides)
  end
end
