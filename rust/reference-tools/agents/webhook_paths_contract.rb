require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter = :test
travel_to Time.utc(2026,3,2,16) do
  agent=Agent.find(773018776);webhook=agent.user.webhook;message=Message.find(136976342)
  secret="ws11-public-test-signing-secret"
  agent.update!(webhook_signing_secret: secret)
  captured=[]
  Webhook.define_method(:post_payload) do |body, secret: nil|
    timestamp=Time.current.to_i.to_s
    captured << {body:body,timestamp:timestamp,signature:secret.present? ? "sha256=#{OpenSSL::HMAC.hexdigest('SHA256',secret,"#{timestamp}.#{body}")}" : nil}
    Net::HTTPNoContent.new("1.1","204","No Content")
  end
  cases={}
  capture=->(name,&call) {call.call;cases[name]=captured.pop}
  capture.call(:without_context) {webhook.deliver(message)}
  capture.call(:with_context) {webhook.deliver(message,agent:agent,delivery_id:123)}
  agent.update_columns(owner_id:nil)
  capture.call(:ownerless) {webhook.deliver(message,agent:agent,delivery_id:7)}
  agent.update_columns(owner_id:127326141)
  thread=ChannelThread.create!(id:900191001,room:message.room,creator:User.find(127326141),name:"PR chat",parent_message:message)
  pr=Github::PullRequest.create!(id:900191011,owner:"rails",repo:"rails",number:12,title:"Fix login",state:"open",head_branch:"shiny",base_branch:"main",review_decision:"approved",check_status:"passing",html_url:"https://github.com/rails/rails/pull/12",private:false)
  Github::PullRequestThread.create!(pull_request:pr,room:message.room,channel_thread:thread)
  message.update_columns(thread_id:thread.id);message.association(:thread).reset
  capture.call(:public_pr) {webhook.deliver(message,agent:agent,delivery_id:9)}
  ordinary=ChannelThread.create!(id:900191002,room:message.room,creator:User.find(127326141),name:"Ordinary chat")
  message.update_columns(thread_id:ordinary.id);message.association(:thread).reset
  capture.call(:ordinary_thread) {webhook.deliver(message,agent:agent,delivery_id:10)}
  message.update_columns(thread_id:nil);message.association(:thread).reset
  capture.call(:ordinary_room) {webhook.deliver(message,agent:agent,delivery_id:11)}
  approval=AgentApproval.create!(id:900191101,agent:agent,action:"deploy",summary:"Ship it")
  capture.call(:approval) {Agent::Delivery.post_approval_webhook!(webhook,approval,agent:agent,delivery_id:7)}
  event=agent.agent_events.create!(id:900191201,event_type:"work_assigned",outcome:"delivered")
  capture.call(:work) {Agent::Delivery.post_work_webhook!(webhook,event,work:{"title"=>"Signed work"},agent:agent)}
  event=agent.agent_events.create!(id:900191202,event_type:"github_action_completed",outcome:"delivered",metadata:{"approval_id"=>approval.id,"action"=>"github.comment","status"=>"completed"})
  capture.call(:completion) {Agent::Delivery.post_github_action_webhook!(webhook,event,agent:agent)}
  agent.destroy!
  capture.call(:legacy) {webhook.deliver(message)}
  puts JSON.pretty_generate(reference_pin:"d7c7de92",secret:secret,cases:cases)
end
