# Pinned #210 independent shape corpus, adapted into committed runtime inputs.
require 'json'
require 'digest'
require 'uri'
root = File.expand_path(__dir__)
now = '2026-03-02 16:00:00'
room = 486777696; other_room = 654632876; foreign_room = 201306877
ta = 2106000010; tb = 2106000020; tf = 2106000000
ra = 2106010010; ma = 2106020010; mb = 2106030010; mf = 2106001010
secret = 'pr210-independent-array-credential'
sql = <<~SQL
DELETE FROM agent_events WHERE agent_id=773018776;
DELETE FROM agent_grants WHERE agent_id=773018776;
DELETE FROM agent_credentials WHERE agent_id=773018776;
UPDATE agents SET owner_id=127326141,status='idle',suspended_at=NULL,last_seen_at=NULL WHERE id=773018776;
DELETE FROM memberships WHERE user_id=394959859 AND room_id=#{foreign_room};
INSERT OR IGNORE INTO memberships(id,user_id,room_id,created_at,updated_at) VALUES(2106400001,394959859,#{room},'#{now}','#{now}');
INSERT OR IGNORE INTO memberships(id,user_id,room_id,created_at,updated_at) VALUES(2106400002,394959859,#{other_room},'#{now}','#{now}');
INSERT INTO agent_credentials(id,agent_id,created_by_id,name,token_digest,token_last_four,created_at,updated_at) VALUES(2106500001,773018776,127326141,'Independent arrays','#{Digest::SHA256.hexdigest(secret)}','#{Digest::SHA256.hexdigest(secret)[0,4]}','#{now}','#{now}');
UPDATE sqlite_sequence SET seq=2106050000 WHERE name='boosts';
SQL
[[tf,foreign_room,'Foreign'],[ta,room,'Alpha'],[tb,room,'Beta']].each do |id,r,name|
  sql << "INSERT INTO channel_threads(id,name,room_id,creator_id,messages_count,last_activity_at,created_at,updated_at) VALUES(#{id},'PR210 #{name}',#{r},127326141,4,'#{now}','#{now}','#{now}');\n"
end
[[ra,room,nil,'Root'],[ma,room,ta,'Alpha'],[mb,room,tb,'Beta'],[mf,foreign_room,tf,'Foreign']].each do |base,r,t,label|
  4.times do |n|
    id = base+n*10; text = "PR210 #{label} message #{n}"
    sql << "INSERT INTO messages(id,room_id,creator_id,thread_id,client_message_id,markdown_source,created_at,updated_at) VALUES(#{id},#{r},127326141,#{t || 'NULL'},'pr210-array-#{id}','#{text}','#{now}','#{now}');\n"
    sql << "INSERT INTO action_text_rich_texts(id,name,record_type,record_id,body,created_at,updated_at) VALUES(#{2106600000+id%1000000},'body','Message',#{id},'<p>#{text}</p>','#{now}','#{now}');\n"
  end
end

cases = []
add = ->(name,tool,args,setup='',repeat=1) do
  cases << {name:name,surface:tool,method:'POST',path:'/agents/mcp',body:JSON.generate(jsonrpc:'2.0',id:210,method:'tools/call',params:{name:tool,arguments:args}),setup_sql:setup,repeat:repeat}
end
rest = ->(name,args,setup='',repeat=1) do
  query = args.flat_map { |key,value| value.is_a?(Array) ? value.map { |v| ["#{key}[]",v] } : [[key,value]] }
  cases << {name:name,surface:'rest_context',method:'GET',path:'/agents/context?'+URI.encode_www_form(query),body:nil,setup_sql:setup,repeat:repeat}
end
[
  ['read_reverse_valid','read_messages',{thread_id:[tb,ta],limit:3}],
  ['read_foreign_global_first','read_messages',{thread_id:[ta,tf,tb],limit:3}],
  ['read_foreign_with_invalid_limit','read_messages',{thread_id:[ta,tf],limit:{bad:true}}],
  ['read_member_scoped_rooms','read_messages',{room_id:[foreign_room,room],limit:3}],
  ['read_member_room_reverse','read_messages',{room_id:[other_room,room],limit:3}],
  ['read_duplicate_nested_strings','read_messages',{thread_id:[tb,[ta.to_s,ta],-10,nil],limit:3}],
  ['read_nonbreaking_space','read_messages',{thread_id:["\u00a0#{ta}",tb],limit:3}],
  ['read_underscore_id','read_messages',{thread_id:'2_106_000_010',limit:3}],
  ['read_only_missing','read_messages',{thread_id:[-8,-1,nil,'bogus'],limit:3}],
  ['before_scope_multiple','read_messages',{thread_id:ta,before:[mf,ra,mb,ma+30,ma+10],limit:3}],
  ['after_scope_multiple','read_messages',{thread_id:ta,after:[mf,ra,mb,ma+20,ma],limit:3}],
  ['before_after_original_conversation','read_messages',{thread_id:ta,before:[ma+10],after:[ma+30],limit:3}],
  ['before_after_window','read_messages',{thread_id:ta,before:[ma+30],after:[ma+10],limit:3}],
  ['before_root_scope','read_messages',{room_id:room,before:[mf,ma,ra+30,ra+10],limit:3}],
  ['after_root_scope','read_messages',{room_id:room,after:[mf,ma,ra+20,ra],limit:3}],
  ['cursor_foreign_only','read_messages',{thread_id:ta,before:[mf,mb,ra],limit:3}],
  ['context_message_reverse','get_context',{message_id:[ma+30,ma],limit:3}],
  ['context_thread_reverse','get_context',{thread_id:[tb,ta],limit:3}],
  ['context_message_global_foreign_first','get_context',{message_id:[ma,mf],limit:3}],
  ['context_thread_global_foreign_first','get_context',{thread_id:[ta,tf],limit:3}],
  ['context_message_precedence','get_context',{message_id:[ma+20,ma],thread_id:ta,limit:3}],
  ['context_array_thread_constraint','get_context',{message_id:[ma],thread_id:[ta],limit:3}],
  ['context_hash_mixed_valid','get_context',{message_id:[{bad:true},ma],limit:3}],
  ['read_hash_mixed_valid','read_messages',{thread_id:[{bad:true},ta],limit:3}],
  ['read_bool_nil_mixed_valid','read_messages',{thread_id:[true,false,nil,ta],limit:3}],
  ['react_reverse_valid','react',{message_id:[ma+30,ma],content:'👍'}],
  ['react_global_foreign_first','react',{message_id:[ma,mf],content:'👍'}],
  ['react_duplicate_nested','react',{message_id:[ma,[ma.to_s,-8,ma]],content:'👍'}],
  ['react_only_missing','react',{message_id:[nil,false,-8,'bad'],content:'👍'}],
  ['react_hash_mixed_valid','react',{message_id:[{bad:true},ma],content:'👍'}]
].each { |name,tool,args|add.call(name,tool,args,'',tool=='react' ? 2 : 1) }
revoked="INSERT INTO agent_grants(id,agent_id,capability,granted_by_id,revoked_at,created_at,updated_at) VALUES(2106700001,773018776,'read_messages',127326141,'#{now}','#{now}','#{now}');"
add.call('revoked_before_invalid_limit','read_messages',{thread_id:[ta,tb],limit:{bad:true}},revoked)
add.call('react_revoked_grant','react',{message_id:[ma+30,ma],content:'👍'},revoked,2)
add.call('inactive_before_missing','read_messages',{thread_id:[-8,ta],limit:{bad:true}},"UPDATE agents SET suspended_at='#{now}' WHERE id=773018776;")
rest.call('rest_context_message_reverse',{message_id:[ma+30,ma],limit:3})
rest.call('rest_context_thread_reverse',{thread_id:[tb,ta],limit:3})
rest.call('rest_context_foreign_first',{message_id:[ma,mf],limit:3})
rest.call('rest_context_duplicate_strings',{message_id:[ma.to_s,ma,-8],limit:3})
rest.call('rest_context_revoked',{thread_id:[ta,tb],limit:3},revoked)
%w[thread before after react context_message context_thread rest_context_message rest_context_thread].each do |label|
  [5,50].each do |size|
    ids=(-size..-1).to_a+[label.include?('thread') ? ta : label=='before' ? ma+30 : ma]
    if label.start_with?('rest_')
      rest.call("count_#{label}_#{size}",{label.end_with?('thread') ? :thread_id : :message_id=>ids,limit:3})
    else
      tool,args=case label
      when 'thread' then ['read_messages',{thread_id:ids,limit:3}]
      when 'before','after' then ['read_messages',{thread_id:ta,label=>ids,limit:3}]
      when 'react' then ['react',{message_id:ids,content:'👍'}]
      when 'context_message' then ['get_context',{message_id:ids,limit:3}]
      else ['get_context',{thread_id:ids,limit:3}]
      end
      add.call("count_#{label}_#{size}",tool,args)
    end
    cases.last.merge!(warmups:2,count_label:label,candidate_count:size)
  end
end
# Sweep candidate-array adapters, preserving each lookup's association scope.
setup = <<~SQL
UPDATE channel_threads SET work_status='in_progress',work_owner_id=394959859,locked_at='#{now}' WHERE id IN (#{ta},#{tb});
UPDATE messages SET creator_id=394959859 WHERE id=#{ma};
INSERT INTO agent_steps(id,agent_id,message_id,name,status,position,created_at,updated_at) VALUES(2106700001,773018776,#{ma},'Existing','running',0,'#{now}','#{now}');
INSERT INTO agent_approvals(id,agent_id,action,summary,expires_at,created_at,updated_at,room_id) VALUES(2106700002,773018776,'deploy','Reader','2026-03-03 16:00:00','#{now}','#{now}',#{room});
INSERT INTO polls(id,message_id,created_at,updated_at) VALUES(2106700003,#{ra},'#{now}','#{now}');
SQL
%w[read_messages post_messages manage_threads react external_action].each_with_index do |cap,i|
 setup << "INSERT INTO agent_grants(id,agent_id,capability,granted_by_id,created_at,updated_at) VALUES(2106700100+#{i},773018776,'#{cap}',127326141,'#{now}','#{now}');\n"
end
specs=[
 ['pin_message','message_id',ma,{}], ['unpin_message','message_id',ma,{}],
 ['open_dm','user_id',394959859,{body:'Probe'}],
 ['create_poll','room_id',room,{question:'Probe',options:['Only']}],
 ['get_poll','room_id',room,{poll_id:2106700003}], ['get_poll','poll_id',2106700003,{room_id:room}],
 ['register_slash_command','room_id',room,{name:'bad name'}], ['unregister_slash_command','room_id',room,{name:'missing'}],
 ['get_approval','approval_id',2106700002,{}], ['request_approval','room_id',room,{action:'INVALID',summary:'Probe'}],
 ['add_step','message_id',ma,{name:'x'*201}], ['add_step','thread_id',ta,{name:'x'*201}],
 ['update_step','step_id',2106700001,{status:'invalid'}],
 ['post_message','room_id',room,{body:'Probe',thread_id:-9}], ['post_message','thread_id',ta,{room_id:room,body:'Probe'}],
 ['start_stream','room_id',room,{markdown_source:'Probe',thread_id:-9}], ['start_stream','thread_id',ta,{room_id:room,markdown_source:'Probe'}],
 ['append_stream','message_id',ma,{append:'Probe'}], ['finalize_stream','message_id',ma,{}],
 ['list_board_posts','room_id',room,{}], ['create_board_post','room_id',room,{title:'Probe'}],
 ['update_work','work_id',ta,{work_status:'invalid'}], ['update_board_post','post_id',ta,{work_status:'invalid'}],
 ['set_result','post_id',ta,{}], ['handoff_work','work_id',ta,{receiver_agent_id:773018776,summary:'Probe'}],
 ['handoff_work','receiver_agent_id',773018776,{work_id:ta,summary:'Probe'}]
]
specs.each do |tool,key,target,args|
 [['nested_only',[[target]]],['object_valid',[{'bad'=>true},target]]].each do |shape,value|
  pin_note=tool=='pin_message' ? "INSERT INTO messages(id,room_id,creator_id,system_note,client_message_id,markdown_source,created_at,updated_at) VALUES(2106740001,#{room},394959859,1,'recent-pin-note','pinned a message: [jump to message](/rooms/#{room}?message_id=#{ma}&thread=#{ta})','#{now}','#{now}');" : ''
  add.call("sweep_#{tool}_#{key}_#{shape}",tool,args.merge(key=>value),setup+pin_note)
 end
end
# Each acknowledgement member is itself a scoped find_by candidate, and keeps
# the submitted value in its envelope. Earlier individual acknowledgments commit.
event_setup=setup+"INSERT INTO agent_events(id,agent_id,room_id,event_type,outcome,metadata,created_at) VALUES(2106750001,773018776,#{room},'work_assigned','delivered','{}','#{now}');"
add.call('ack_nested_member','ack_events',{event_ids:[[[2106750001]]]},event_setup)
add.call('ack_object_valid_member','ack_events',{event_ids:[[{bad:true},2106750001]]},event_setup)
add.call('ack_nested_multi_member','ack_events',{event_ids:[[2106750001,[-10],nil]]},event_setup)
# Context is the REST candidate reader; nested objects arrive through Rack params.
['message_id','thread_id'].each do |key|
 target=key=='message_id' ? ma : ta
 cases << {name:"rest_#{key}_object_valid",surface:'rest_context',method:'GET',path:'/agents/context?'+URI.encode_www_form([["#{key}[][bad]",'true'],["#{key}[]",target],['limit',3]]),body:nil,setup_sql:'',repeat:1}
end
# Soft-deleted member rooms do not participate in the user.rooms association.
dead_room_setup=setup+"UPDATE rooms SET deleted_at='#{now}' WHERE id=#{foreign_room}; INSERT INTO memberships(id,user_id,room_id,created_at,updated_at) VALUES(2106400003,394959859,#{foreign_room},'#{now}','#{now}');"
[
 ['post_message',{body:'Probe',thread_id:-9}], ['start_stream',{markdown_source:'Probe',thread_id:-9}],
 ['create_board_post',{title:'Probe'}], ['create_poll',{question:'Probe',options:['Only']}],
 ['register_slash_command',{name:'bad name'}], ['get_poll',{poll_id:2106700003}]
].each do |tool,args|
 add.call("dead_member_room_#{tool}",tool,args.merge(room_id:[foreign_room,room]),dead_room_setup)
end
# GitHub's PR lookup is global before the room-thread association check.
pr_setup=setup+"INSERT INTO github_pull_requests(id,owner,repo,number,created_at,updated_at) VALUES(2106760001,'fixture','array',1,'#{now}','#{now}'); INSERT INTO github_pull_request_threads(id,channel_thread_id,room_id,github_pull_request_id,created_at,updated_at) VALUES(2106760002,#{ta},#{room},2106760001,'#{now}','#{now}');"
[['nested_only',[[2106760001]]],['object_valid',[{bad:true},2106760001]]].each do |shape,value|
 cases << {name:"rest_github_pr_#{shape}",surface:'rest_github',method:'POST',path:"/rooms/#{room}/agents/github/pull_request_actions",body:JSON.generate(pull_request_id:value,kind:'invalid'),setup_sql:pr_setup,repeat:1}
end
# PR214 compatibility regressions and empty-parent validation (no source/job writes).
["\v#{ta}", "0d#{ta}", "0D#{ta}", "+0d#{ta}", "-0D#{ta}", "0x#{ta}", "0b#{ta}", "0o#{ta}"].each_with_index do |value,i|
 add.call("pr214_scalar_#{i}",'read_messages',{thread_id:value,limit:3})
 add.call("pr214_position_#{i}",'read_messages',{thread_id:[tb,value],limit:3})
end
['message_id','thread_id'].each do |key|
 add.call("pr214_step_empty_#{key}",'add_step',{name:'Probe',key=>[]})
 cases << {name:"pr214_rest_step_empty_#{key}",surface:'rest_step',method:'POST',path:'/agents/steps',body:JSON.generate(name:'Probe',key=>[]),setup_sql:'',repeat:1}
end
manifest={frozen_at:'2026-03-02T16:00:00Z',request_headers:{'Accept'=>'application/json','Content-Type'=>'application/json','X-Forwarded-For'=>'203.0.113.210','X-Array-Fixture'=>'ws11api-next3'},secret:secret,projection_tables:%w[agents agent_credentials agent_grants users rooms memberships channel_threads messages action_text_rich_texts boosts thread_memberships agent_events audit_logs activity_items agent_steps agent_approvals polls message_pins github_pull_requests github_pull_request_threads],cases:cases}
[sql,manifest]
