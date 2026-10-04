# Real model/service outcomes; HTTP/MCP adapters retain their own auth, throttles and renderers.
require "digest"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"),"reference-tools/agents/work-services-source-hashes.json"))).each do |file, hash|
  raise "source drift: #{file}" unless Digest::SHA256.file(Rails.root.join(file)).hexdigest == hash
end
ApplicationJob.queue_adapter = :test
Rails.logger = ActiveSupport::Logger.new($stderr)
travel_to Time.utc(2026,3,2,16)
agent = Agent.find(773018776); human = User.find(127326141)
board = Rooms::Board.create_for({id:900082020,name:"Service board",creator:human},users:[human,agent.user,User.find(712064548)])
receiver_user = User.create_bot!(id:900082030,name:"Receiver")
receiver = Agent.create!(id:900082031,user:receiver_user,owner:human,kind: :workspace)
receiver_user.create_webhook!(url:"https://receiver.example.test/hook")
board.memberships.grant_to(receiver_user)

cases = []
add = ->(operation, name, input={}, flags=[]) { cases << {operation:,name:,input:,flags:} }
[
 ["defaults",{title:"Ship"}], ["brief",{title:"Ship",body:"The brief."}],
 ["human",{title:"Ship",tags:" API, launch,API ",work_status:"planned",owner_id:712064548,run_url:"https://example.test/run"}],
 ["array_tags",{title:"Ship",tags:[" API ","launch","api",""]}],
 ["blank_owner",{title:"Ship",owner_id:" "}], ["null_owner",{title:"Ship",owner_id:nil}],
 ["blank_body",{title:"Ship",body:"  \n "}], ["blank_status",{title:"Ship",work_status:" "}],
 ["missing_title",{}], ["long_title",{title:"x"*101}], ["bad_status",{title:"Ship",work_status:"shipped"}],
 ["many_tags",{title:"Ship",tags:"a,b,c,d,e,f"}], ["bad_tag",{title:"Ship",tags:"Bad tag!"}],
 ["long_tag",{title:"Ship",tags:"x"*31}], ["http_run",{title:"Ship",run_url:"http://example.test"}],
 ["long_run",{title:"Ship",run_url:"https://example.test/#{'x'*500}"}],
 ["outside_owner",{title:"Ship",owner_id:773523953}], ["missing_owner",{title:"Ship",owner_id:999999}],
 ["bad_owner",{title:"Ship",owner_id:"abc"}], ["receiver_owner",{title:"Ship",owner_id:900082030}]
].each { |name,input| add.call("create",name,input) }
add.call("create","nonmember",{title:"Ship"},["nonmember"])
add.call("create","budget_before_validation",{},["budget"])
[
 ["status",{work_status:"in_progress",note:"Digging in"}], ["same_status",{work_status:"planned",note:"No change"}],
 ["tags_array",{tags:[" API ","launch","api"]}], ["tags_string",{tags:"backend,api"}],
 ["clear_tags",{tags:nil}], ["run",{run_url:"https://example.test/run"}], ["clear_run",{run_url:""}],
 ["all",{work_status:"blocked",tags:"api",run_url:"https://example.test/run",note:"Waiting"}],
 ["bad_status",{work_status:"shipping",tags:"api"}], ["blank_status",{work_status:nil}],
 ["no_fields",{}], ["note_only",{note:"Alone"}], ["long_note",{tags:"api",note:"x"*501}],
 ["note_boundary",{work_status:"done",note:"é"*500}], ["invalid_tag_rollback",{work_status:"done",tags:"bad tag!"}],
 ["invalid_run_rollback",{work_status:"done",tags:"api",run_url:"http://example.test"}],
 ["odd_tags",{tags:[nil,false,12,["x"]]}], ["odd_run",{run_url:{url:"https://example.test"}}]
].each { |name,input| add.call("update",name,input) }
%w[nonmember unreadable no_manage other_owner untracked read_other_room suspended].each { |flag|add.call("update",flag,{work_status:"done"},[flag]) }
add.call("update","global_read",{work_status:"done"},["global_read"])
[
 ["replace",{markdown:"## Shipped"}], ["clear",{markdown:""}], ["null",{markdown:nil}],
 ["missing",{}], ["overlong",{markdown:"é"*20001}], ["boundary",{markdown:"é"*20000}],
 ["odd_markdown",{markdown:[false,12]}]
].each { |name,input|add.call("result",name,input) }
%w[nonmember unreadable no_manage other_owner].each { |flag|add.call("result",flag,{},[flag]) }
[
 ["package",{summary:"Halfway",links:"https://example.test/a\r\n https://example.test/a\nHTTP://example.test/b",open_questions:[" Why? ","","Why?","Which?\nNext?"]}],
 ["summary_blank",{summary:" "}], ["summary_cap",{summary:"é"*2001}],
 ["many_links",{summary:"Hi",links:(1..11).map { |i|"https://example.test/#{i}" }}],
 ["bad_links",{summary:"Hi",links:["ftp://example.test","file:///tmp/x"]}],
 ["long_link",{summary:"Hi",links:["https://example.test/#{'x'*500}"]}],
 ["many_questions",{summary:"Hi",open_questions:(1..11).map { |i|"Why #{i}?" }}],
 ["long_question",{summary:"Hi",open_questions:["é"*501]}],
 ["false_links",{summary:"Hi",links:false}], ["false_questions",{summary:"Hi",open_questions:false}],
 ["summary_boundary",{summary:"é"*2000}], ["odd_package",{summary:"Hi",links:{a:false},open_questions:[nil,false,12,["nested"]]}]
].each { |name,input|add.call("handoff",name,input) }
%w[nonmember unreadable no_manage other_owner receiver_outside receiver_suspended receiver_no_post receiver_no_manage receiver_no_read receiver_self receiver_missing].each { |flag|add.call("handoff",flag,{summary:"Hi"},[flag]) }

stamp = ->(time) { time&.utc&.iso8601(3) }
rows = []
cases.each do |row|
  travel_to Time.utc(2026,3,2,16)
  AgentGrant.delete_all
  agent.update!(daily_board_post_cap:nil,suspended_at:nil); agent.user.update!(status: :active)
  receiver.update!(suspended_at:nil)
  board.memberships.grant_to([agent.user,receiver.user])
  [agent,receiver].each do |who|
    %w[read_messages post_messages manage_threads].each { |cap|AgentGrant.create!(agent:who,room:board,capability:cap,granted_by:human) }
  end
  thread = row[:operation]=="create" ? nil : ChannelThread.create_board_post!(room:board,creator:human,name:"Owned",work_status:"planned",owner_id:agent.user_id,tags:"seed",run_url:"https://example.test/initial")
  flags = row[:flags]
  board.memberships.find_by!(user:agent.user).destroy! if flags.include?("nonmember")
  AgentGrant.where(agent:agent,capability:"read_messages").update_all(revoked_at:Time.current) if flags.include?("unreadable")
  AgentGrant.where(agent:agent,capability:"manage_threads").update_all(revoked_at:Time.current) if flags.include?("no_manage")
  thread.update_columns(work_owner_id:human.id) if flags.include?("other_owner")
  thread.update_columns(work_status:nil) if flags.include?("untracked")
  agent.update!(suspended_at:Time.current) if flags.include?("suspended")
  agent.update!(daily_board_post_cap:1) if flags.include?("budget")
  if flags.include?("read_other_room") || flags.include?("global_read")
    AgentGrant.where(agent:agent,capability:"read_messages").delete_all
    AgentGrant.create!(agent:agent,room:flags.include?("global_read") ? nil : Room.find(486777696),capability:"read_messages",granted_by:human)
  end
  board.memberships.find_by!(user:receiver.user).destroy! if flags.include?("receiver_outside")
  receiver.update!(suspended_at:Time.current) if flags.include?("receiver_suspended")
  {"receiver_no_post"=>"post_messages","receiver_no_manage"=>"manage_threads","receiver_no_read"=>"read_messages"}.each do |flag,cap|
    AgentGrant.where(agent:receiver,capability:cap).update_all(revoked_at:Time.current) if flags.include?(flag)
  end
  before_history = thread&.work_thread_events&.maximum(:id) || 0
  before_ledger = AgentEvent.maximum(:id) || 0
  before_threads = ChannelThread.count
  travel_to Time.utc(2026,3,2,16,1)
  input = row[:input]
  result = case row[:operation]
  when "create"
    Agents::BoardPosts.create(agent:agent.reload,room:board,title:input[:title],**input.except(:title))
  when "update"
    Agents::WorkThreads.update(agent:agent.reload,id:thread.id,**input)
  when "result"
    Agents::WorkThreads.set_result(agent:agent.reload,id:thread.id,markdown:input[:markdown],markdown_given:input.key?(:markdown))
  when "handoff"
    target = flags.include?("receiver_self") ? agent.id : flags.include?("receiver_missing") ? 0 : receiver.id
    Agents::WorkHandoffs.create(agent:agent.reload,id:thread.id,receiver_agent_id:target,**input)
  end
  thread = result.payload if result.ok? && row[:operation]=="create"
  thread&.reload
  fields = if thread
    {title:thread.name,creator_id:thread.creator_id,work_status:thread.work_status,owner:thread.work_owner_id,tags:thread.tag_names,
     run_url:thread.run_url,result:thread.result_markdown,result_length:thread.result_markdown&.length,
     result_updated_by:thread.result_updated_by_id,result_updated_at:stamp.call(thread.result_updated_at),
     updated_at:stamp.call(thread.updated_at),work_status_changed_at:stamp.call(thread.work_status_changed_at),
     messages:thread.messages.order(:id).map { |m|{creator_id:m.creator_id,markdown:m.markdown_source,opener:m.board_post_opener} }}
  end
  history = thread ? thread.work_thread_events.where("id > ?",before_history).order(:id).map do |e|
    metadata = e.metadata.deep_dup; metadata["handoff_id"]="handoff" if metadata.key?("handoff_id")
    {kind:e.event_type,actor:e.actor_id,from_owner:e.from_owner_id,to_owner:e.to_owner_id,metadata:}
  end : []
  ledger = AgentEvent.where("id > ?",before_ledger).order(:id).map do |e|
    package=e.metadata["handoff"]&.deep_dup; package["id"]="handoff" if package
    {agent:e.agent_id,kind:e.event_type,actor:e.actor_id,outcome:e.outcome,title:e.metadata["title"],status:e.metadata["work_status"],hop:e.hop,
     handoff:package,webhook_status:e.webhook_status}
  end
  status = result.status==:unprocessable_entity ? 422 : result.status.is_a?(Integer) ? result.status : Rack::Utils::SYMBOL_TO_STATUS_CODE.fetch(result.status)
  rows << row.merge(expected:{status:,error:result.error,failure:result.ok? ? nil : result.failure_body,
      added_threads:ChannelThread.count-before_threads,fields:,history:,ledger:})
end
puts JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],sources:JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"),"reference-tools/agents/work-services-source-hashes.json"))),rows:)
warn "Rails agent work service oracle: #{rows.length} cases; real writes and denials; 0 masks"
