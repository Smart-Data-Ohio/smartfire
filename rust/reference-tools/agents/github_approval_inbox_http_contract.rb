require "active_support/testing/time_helpers"
require "rack/mock"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter=:test
ApplicationController.allow_forgery_protection=false
travel_to Time.utc(2026,3,2,16) do
  agent=Agent.find(773018776);owner=User.find(127326141);room=Room.find(486777696)
  agent.update_columns(owner_id:owner.id)
  AgentCredential.create!(agent:agent,created_by:owner,name:"Inbox failure",token_digest:AgentCredential.digest("ws11-public-http-fixture-secret"),token_last_four:"cret")
  AgentGrant.create!(agent:agent,room:room,granted_by:owner,capability:"external_action")
  GithubConnectedAccount.create!(user:agent.user,github_login:"machine",access_token:"ws11-public-github-fixture-token")
  thread=ChannelThread.create!(room:room,creator:owner,name:"HTTP approval")
  pr=Github::PullRequest.create!(owner:"rails",repo:"rails",number:12)
  Github::PullRequestThread.create!(pull_request:pr,room:room,channel_thread:thread)
  Net::HTTP.define_singleton_method(:start) { |*args,**kwargs| raise "Unexpected GitHub request" }
  ApplicationJob.queue_adapter.enqueued_jobs.clear
  external_id="ws11-main-http-inbox-failure"
  body=JSON.generate(pull_request_id:pr.id,kind:"comment",body:"First",external_id:external_id)
  request=-> do
    Rack::MockRequest.new(Rails.application).post("http://example.org/rooms/#{room.id}/agents/github/pull_request_actions",
      "CONTENT_TYPE"=>"application/json","HTTP_AUTHORIZATION"=>["Bearer","ws11-public-http-fixture-secret"].join(" "),input:body)
  end
  conn=ActiveRecord::Base.connection
  conn.execute("CREATE TEMP TRIGGER ws11_http_reject_inbox BEFORE INSERT ON activity_items WHEN NEW.source_type='AgentApproval' BEGIN SELECT RAISE(ABORT,'inbox unavailable'); END")
  rejected=request.call
  approval=AgentApproval.find_by!(external_id:external_id)
  result={status:rejected.status,approvals:AgentApproval.where(external_id:external_id).count,status_after_error:approval.status,
    inbox:ActivityItem.where(source:approval).count,jobs:ApplicationJob.queue_adapter.enqueued_jobs.size}
  conn.execute("DROP TRIGGER ws11_http_reject_inbox")
  replay=request.call
  result[:replay_status]=replay.status
  result[:replay_same_id]=JSON.parse(replay.body)["id"]==approval.id
  result[:approvals_after_replay]=AgentApproval.where(external_id:external_id).count
  result[:inbox_after_replay]=ActivityItem.where(source:approval).count
  puts JSON.pretty_generate(reference_pin:"d7c7de92",result:result)
end
