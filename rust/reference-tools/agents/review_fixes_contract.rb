# Independent probes for PR #176 against Rails d7c7de92; real callbacks, failure triggers and HTTP client.
require 'active_support/testing/time_helpers'
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter = :test
travel_to Time.utc(2026, 3, 2, 16)
agent = Agent.find(773018776)
room = Room.find(486777696)
human = User.find(127326141)
AgentGrant.delete_all
room.memberships.grant_to([agent.user])
result = {}
# Walk link order rather than assuming the review's alphabetical order matches Rails.
result[:repositories] = []
[['z','a','m'], ['a','m','z'], ['a','z','a']].each do |order|
  [nil, 0, 1, 2].each do |public_index|
    (0..2).each do |disconnect_index|
      next if disconnect_index == public_index
      next if order.uniq.size != order.size && disconnect_index != 1
      Rails.cache = ActiveSupport::Cache::MemoryStore.new
      agent.update_columns(owner_id: human.id)
      agent.reload
      GithubConnectedAccount.where(user_id:human.id).delete_all
      account = GithubConnectedAccount.create!(user:human, github_login:'ws11-review-owner', access_token:'ws11-public-fixture-token')
      thread = ChannelThread.create!(room:room, creator:human, name:'Repository batch')
      order.each_with_index do |owner,index|
        pr = Github::PullRequest.create!(owner:owner,repo:'private',number:thread.id * 10 + index,title:"Private #{owner}",head_branch:'private-head',base_branch:'private-base',private:index != public_index)
        WorkThreadLink.create!(channel_thread:thread,kind:'pull_request',github_pull_request:pr,created_by:human)
      end
      paths = []
      http = Object.new
      http.define_singleton_method(:get) do |path,headers|
        paths << path
        status = path == "/repos/#{order[disconnect_index]}/private" ? 401 : 200
        response = Net::HTTPResponse::CODE_TO_OBJ.fetch(status.to_s).new('1.1',status.to_s,'fixture')
        response.define_singleton_method(:body) { '{}' }
        response
      end
      Net::HTTP.define_singleton_method(:start) { |*args,**kwargs,&block| block.call(http) }
      payload = Agents::WorkPayload.for(thread,agent:agent)
      result[:repositories] << {order:order,public_index:public_index,disconnect_index:disconnect_index,paths:paths,disconnected_reason:account.reload.disconnected_reason,details:payload[:links].map{|link|link[:pull_request].slice(:title,:head_branch,:base_branch)}}
    end
  end
end
# Event polling repeats the same thread after a different repository disconnects.
Rails.cache = ActiveSupport::Cache::MemoryStore.new
agent.update_columns(owner_id:human.id)
GithubConnectedAccount.where(user_id:human.id).delete_all
account = GithubConnectedAccount.create!(user:human,github_login:'ws11-poll-owner',access_token:'ws11-public-fixture-token')
agent.reload
threads = ['a','z'].map do |owner|
  thread = ChannelThread.create!(room:room,creator:human,name:"Polling #{owner}")
  pr = Github::PullRequest.create!(owner:owner,repo:'private',number:thread.id * 10,title:"Private #{owner}",head_branch:'private-head',base_branch:'private-base',private:true)
  WorkThreadLink.create!(channel_thread:thread,kind:'pull_request',github_pull_request:pr,created_by:human)
  thread
end
agent.agent_events.delete_all
ids = [threads[0],threads[1],threads[0]].map do |thread|
  agent.agent_events.create!(event_type:'work_assigned',room:room,outcome:'delivered',metadata:{thread_id:thread.id}).id
end
paths=[]
http=Object.new
http.define_singleton_method(:get) do |path,headers|
  paths << path
  status = path == '/repos/z/private' ? 401 : 200
  response=Net::HTTPResponse::CODE_TO_OBJ.fetch(status.to_s).new('1.1',status.to_s,'fixture')
  response.define_singleton_method(:body) {'{}'}
  response
end
Net::HTTP.define_singleton_method(:start) {|*args,**kwargs,&block|block.call(http)}
payload=Agents::EventPolling.poll(agent:agent,since:ids.first-1,limit:50,presenter:->(message) {{id:message.id}})
result[:poll_batch]={paths:paths,details:payload[:events].map{|event|event[:work][:links].first[:pull_request].slice(:title,:head_branch,:base_branch)},cursor_advanced:payload[:next_since] == ids.last}
# Failure after destruction must leave the deletion committed.
Webhook.where(user_id:agent.user_id).delete_all
thread = ChannelThread.create!(room:room,creator:human,name:'Deleted ledger failure',work_status:'in_progress')
thread.update_columns(work_owner_id:agent.user_id)
conn = ActiveRecord::Base.connection
conn.execute("CREATE TEMP TRIGGER ws11_reject_deleted_event BEFORE INSERT ON agent_events WHEN NEW.event_type='work_unassigned' BEGIN SELECT RAISE(ABORT,'deletion event rejected'); END")
error = nil
begin
  thread.destroy!
rescue => e
  error = e.class.name
end
result[:deletion] = {error:error,thread_exists:ChannelThread.exists?(thread.id),events:AgentEvent.where(event_type:'work_unassigned').where("json_extract(metadata,'$.thread_id')=?",thread.id).count}
conn.execute('DROP TRIGGER ws11_reject_deleted_event')
Webhook.create!(user:agent.user,url:'https://bots.example.test/hook')
thread = ChannelThread.create!(room:room,creator:human,name:'Deliverable deleted ledger failure',work_status:'in_progress')
thread.update_columns(work_owner_id:agent.user_id)
conn.execute("CREATE TEMP TRIGGER ws11_reject_deleted_event BEFORE INSERT ON agent_events WHEN NEW.event_type='work_unassigned' BEGIN SELECT RAISE(ABORT,'deletion event rejected'); END")
error = nil
begin
  thread.destroy!
rescue => e
  error = e.class.name
end
result[:deletion_with_webhook] = {error:error,thread_exists:ChannelThread.exists?(thread.id),events:AgentEvent.where(event_type:'work_unassigned').where("json_extract(metadata,'$.thread_id')=?",thread.id).count}
conn.execute('DROP TRIGGER ws11_reject_deleted_event')
# Capture callback execution, including actual reference syncs, without replacing them.
order = []
wrapper = Module.new
%i[sync_github_pull_request_references sync_fizzy_card_references sync_twitter_post_references sync_event_references sync_message_references sync_link_embed_references].each do |method|
  wrapper.define_method(method) do |*args,**kwargs|
    order << method.to_s
    super(*args,**kwargs)
  end
end
Message.prepend(wrapper)
source = room.messages.create!(creator:human,body:'Quote source')
stream = room.messages.create!(creator:agent.user,streaming:true,markdown_source:"/rooms/#{room.id}/@#{source.id} https://github.com/a/b/pull/0",client_message_id:'ws11-review-reference-order')
order.clear
error = nil
begin
  stream.finalize_stream!
rescue => e
  error = e.class.name
end
result[:stream_failure] = {error:error,streaming:stream.reload.streaming?,quote_references:MessageReference.where(message_id:stream.id).count,order:order.dup}
stream = room.messages.create!(creator:agent.user,streaming:true,markdown_source:"/rooms/#{room.id}/@#{source.id}",client_message_id:'ws11-review-reference-success')
order.clear
stream.finalize_stream!
result[:stream_success] = {streaming:stream.reload.streaming?,quote_references:MessageReference.where(message_id:stream.id).count,order:order.dup}
# Owner-first independent transactions: failure at second recipient preserves the first.
owner = User.find(712064548)
agent.update!(owner:owner)
User.active.without_bots.each{|u|u.update_column(:inbox_preferences,{agent_approvals:true}.to_json)}
conn.execute("CREATE TEMP TRIGGER ws11_reject_second_recipient BEFORE INSERT ON activity_items WHEN NEW.source_type='AgentApproval' AND EXISTS(SELECT 1 FROM activity_items WHERE source_type='AgentApproval' AND source_id=NEW.source_id) BEGIN SELECT RAISE(ABORT,'later recipient rejected'); END")
error = nil
begin
  AgentApproval.create!(agent:agent,action:'deploy',summary:'Owner-first fanout',external_id:'ws11-review-owner-first')
rescue => e
  error = e.class.name
end
approval = AgentApproval.find_by!(external_id:'ws11-review-owner-first')
result[:approval] = {error:error,owner:owner.id,deciders:approval.deciders.map(&:id),notified_users:ActivityItem.where(source:approval).order(:id).pluck(:user_id),status:approval.status}
conn.execute('DROP TRIGGER ws11_reject_second_recipient')
# A captured belongs_to can outlive an Agent deleted in the outer transaction.
# Rails writes the orphan ledger row (there is no FK), rather than reloading it.
Webhook.where(user_id:agent.user_id).delete_all
thread=ChannelThread.create!(room:room,creator:human,name:'Outer agent removal ledger')
thread.update_columns(work_owner_id:agent.user_id)
error=nil
begin
  ActiveRecord::Base.transaction do
    thread.destroy!
    agent.delete
  end
rescue => e
  error=e.class.name
end
result[:deletion_removed_agent]={error:error,thread_exists:ChannelThread.exists?(thread.id),agent_exists:Agent.exists?(agent.id),events:AgentEvent.where("json_extract(metadata,'$.thread_id')=?",thread.id).count}
puts JSON.pretty_generate({reference:'d7c7de92',results:result}.as_json)
