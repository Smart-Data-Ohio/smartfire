# Queued bot execution and deletion snapshot delivery. Only the remote HTTP connection is a fixture.
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter=:test
Rails.logger=ActiveSupport::Logger.new($stderr)
kind=ARGV.fetch(0)
travel_to Time.utc(2026,3,2,16) do
  bot=User.find(394959859);agent=Agent.find(773018776);room=Room.find(486777696);human=User.find(127326141)
  AgentGrant.delete_all;AgentEvent.delete_all;room.memberships.grant_to(bot)
  webhook=bot.webhook;webhook.update!(url:"http://93.184.216.34:8080/hook",signing_secret:"ws11-next-2-public-signing-material")
  agent.update!(webhook_signing_secret:"ws11-next-2-public-signing-material")
  received=[]
  # Webhook#post_payload, its guard, timestamp and HMAC run unchanged.
  transport=Object.new
  transport.define_singleton_method(:request) do |request|
    received << {body:request.body,timestamp:request["X-Smartfire-Timestamp"],signature:request["X-Smartfire-Signature"],content_type:request["Content-Type"]}
    response=Net::HTTPOK.new("1.1","200","OK");response.instance_variable_set(:@read,true);response.body="";response
  end
  Net::HTTP.define_singleton_method(:start) do |host,port,**options,&block|
    raise "unpinned connection" unless host=="93.184.216.34" && port==8080 && options[:ipaddr]=="93.184.216.34"
    block.call(transport)
  end
  ApplicationJob.queue_adapter.enqueued_jobs.clear
  if kind=="queued_bot"
    message=Message.find(935961918)
    bot.deliver_webhook_later(message)
    jobs=ApplicationJob.queue_adapter.enqueued_jobs.select{|j|j[:job]==Bot::WebhookJob}
    raise "job missing" unless jobs.length==1 && received.empty?
    jobs.each{|job|job[:job].perform_now(*ActiveJob::Arguments.deserialize(job[:args]))}
    result={jobs:jobs.map{|job|{class:job[:job].name,args:job[:args].map{|arg|arg.fetch("_aj_globalid")}}},requests:received}
  elsif kind=="delete_owned"
    thread=ChannelThread.create!(id:1901820001,room:room,creator:human,name:"Agent work")
    ThreadMembership.join!(thread,human);thread.update_work!(actor:human,work_status:"planned")
    thread.update_work!(actor:human,work_owner_id:bot.id);thread.deleted_by=human;thread.destroy!
    raise "premature transport" unless received.empty?
    jobs=ApplicationJob.queue_adapter.enqueued_jobs.select{|j|j[:job]==Agent::EventWebhookJob}
    before=AgentEvent.order(:id).map{|e|e.attributes.slice("id","event_type","outcome","actor_id","webhook_status").merge("metadata"=>e.metadata)}
    jobs.each{|job|job[:job].perform_now(*ActiveJob::Arguments.deserialize(job[:args]))}
    after=AgentEvent.order(:id).map{|e|e.attributes.slice("id","event_type","outcome","actor_id","webhook_status").merge("metadata"=>e.metadata)}
    result={exists:ChannelThread.exists?(thread.id),jobs:jobs.map{|j|{class:j[:job].name,args:j[:args]}},before:before,after:after,requests:received}
  else
    raise "unknown case"
  end
  puts JSON.pretty_generate(name:kind,result:result)
end
