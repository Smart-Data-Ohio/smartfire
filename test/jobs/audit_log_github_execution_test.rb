require "test_helper"
require "minitest/mock"

class AuditLog::GithubExecutionAuditTest < ActiveJob::TestCase
  AGENT_TOKEN = "agent-token-abc"

  setup do
    @room = rooms(:watercooler)
    @bot = users(:bender)
    @agent = agents(:bender_agent)
    WebMock.stub_request(:post, webhooks(:bender).url).to_return(status: 200)

    message = @room.messages.create!(
      creator: users(:david),
      markdown_source: "review https://github.com/rails/rails/pull/12",
      client_message_id: "audit-job-pr-12"
    )
    @pull_request = message.github_pull_requests.first
    thread = ChannelThread.create!(room: @room, creator: users(:david), parent_message: message)
    ThreadMembership.join!(thread, users(:david))
    Github::PullRequestThread.create!(pull_request: @pull_request, room: @room, channel_thread: thread)

    @grant = AgentGrant.create!(agent: @agent, room: @room, granted_by: users(:david), capability: "external_action")
    @account = GithubConnectedAccount.create!(user: @bot, github_login: "bender-machine", access_token: AGENT_TOKEN)
  end

  test "a completed GitHub action is recorded with the decider as actor" do
    stub_request(:post, "https://api.github.com/repos/rails/rails/issues/12/comments")
      .to_return(status: 201, body: { html_url: "https://github.com/rails/rails/pull/12#issuecomment-1" }.to_json)
    approval = approve!(build_approval(kind: "comment", body: "Nice work"))

    assert_difference -> { AuditLog.where(action: "agent.github_action.execute").count }, +1 do
      Github::PerformAgentActionJob.perform_now(approval.id)
    end

    entry = AuditLog.where(action: "agent.github_action.execute").last
    assert_equal users(:david).id, entry.actor_id
    assert_equal approval.id, entry.target_id
    assert_equal "AgentApproval", entry.target_type
    assert_equal "github.comment", entry.details["action"]
    assert_equal "completed", entry.details["status"]
    assert_equal "https://github.com/rails/rails/pull/12#issuecomment-1", entry.details["url"]
  end

  test "a refused execution is recorded as failed without a GitHub request" do
    approval = approve!(build_approval(kind: "comment", body: "Nice work"))
    @account.update!(github_login: "someone-else-machine", access_token: "swapped-token")

    assert_difference -> { AuditLog.where(action: "agent.github_action.execute").count }, +1 do
      Github::PerformAgentActionJob.perform_now(approval.id)
    end

    assert_not_requested :post, %r{api\.github\.com}
    entry = AuditLog.where(action: "agent.github_action.execute").last
    assert_equal "failed", entry.details["status"]
    assert_equal "The agent's GitHub account changed since this was approved", entry.details["message"]
  end

  test "a retried job records no second row" do
    stub_request(:post, "https://api.github.com/repos/rails/rails/issues/12/comments")
      .to_return(status: 201, body: { html_url: "https://github.com/rails/rails/pull/12#issuecomment-1" }.to_json)
    approval = approve!(build_approval(kind: "comment", body: "Nice work"))
    Github::PerformAgentActionJob.perform_now(approval.id)

    assert_no_difference -> { AuditLog.where(action: "agent.github_action.execute").count } do
      Github::PerformAgentActionJob.perform_now(approval.id)
    end
  end

  test "a failing audit write still enqueues the outcome webhook" do
    stub_request(:post, "https://api.github.com/repos/rails/rails/issues/12/comments")
      .to_return(status: 201, body: { html_url: "https://github.com/rails/rails/pull/12#issuecomment-1" }.to_json)
    approval = approve!(build_approval(kind: "comment", body: "Nice work"))

    AuditLog.stub :record!, proc { raise "audit store down" } do
      assert_enqueued_with(job: Agent::EventWebhookJob) do
        Github::PerformAgentActionJob.perform_now(approval.id)
      end
    end

    event = @agent.agent_events.where(event_type: "github_action_completed").last
    assert_equal "completed", event.metadata["status"]
  end

  test "a failing sweep audit write still enqueues the outcome webhook" do
    approval = approve!(build_approval(kind: "comment", body: "Nice work"))
    event = @agent.agent_events.create!(
      event_type: "github_action_completed",
      outcome: "delivered",
      agent_approval_id: approval.id,
      webhook_status: "none",
      metadata: { "approval_id" => approval.id, "action" => approval.action, "status" => "running" }
    )
    event.update_columns(created_at: 20.minutes.ago)

    AuditLog.stub :record!, proc { raise "audit store down" } do
      assert_enqueued_with(job: Agent::EventWebhookJob) do
        Github::PerformAgentActionJob.recover_stuck_claims!
      end
    end

    assert_equal "failed", event.reload.metadata["status"]
  end

  test "a stuck claim recovered by the sweep is recorded as failed" do
    approval = approve!(build_approval(kind: "comment", body: "Nice work"))
    event = @agent.agent_events.create!(
      event_type: "github_action_completed",
      outcome: "delivered",
      agent_approval_id: approval.id,
      webhook_status: "none",
      metadata: { "approval_id" => approval.id, "action" => approval.action, "status" => "running" }
    )
    event.update_columns(created_at: 20.minutes.ago)

    assert_difference -> { AuditLog.where(action: "agent.github_action.execute").count }, +1 do
      Github::PerformAgentActionJob.recover_stuck_claims!
    end

    entry = AuditLog.where(action: "agent.github_action.execute").last
    assert_equal approval.id, entry.target_id
    assert_equal "failed", entry.details["status"]
    assert_equal "GitHub action execution timed out", entry.details["message"]
  end

  private
    def build_approval(kind:, body: nil, reviewers: nil)
      action = Github::AgentPullRequestAction.new(
        pull_request: @pull_request, kind: kind, body: body, reviewers: reviewers
      )
      assert action.valid?, action.errors.full_messages.to_sentence
      AgentApproval.create!(
        agent: @agent, room: @room, action: action.action_name,
        summary: action.summary, payload: action.payload_json,
        github_account_id: @account.id, github_login: @account.github_login
      )
    end

    def approve!(approval)
      approval.decide!(decision: "approved", by: users(:david))
      approval
    end
end
