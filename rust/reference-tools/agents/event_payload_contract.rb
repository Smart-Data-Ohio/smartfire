# Actual pinned Agent::Delivery and WorkPayload, capturing the JSON passed to Webhook.
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16) do
  agent=Agent.find(773018776)
  room=Room.find(486777696)
  actor=User.find(127326141)
  captured=nil
  Webhook.define_method(:post_payload) do |payload,secret: nil|
    captured=payload
    Net::HTTPOK.new("1.1","200","OK")
  end
  payloads={}
  record=lambda do |key,type,**attributes|
    event=agent.agent_events.create!(id:900_000_000+payloads.size,event_type:type,outcome:"delivered",**attributes)
    Agent::Delivery.post_event_webhook!(agent.user.webhook,event,agent:agent)
    payloads[key]=captured
  end
  record.call(:slash,"slash_command",room:room,actor:actor,metadata:{thread_id:321,command:"inspect",arguments:"<>& é\u2028"})
  record.call(:slash_no_actor,"slash_command",metadata:[])
  approval=agent.agent_approvals.create!(id:900_010_001,action:"release",summary:"Ship it",status:"approved",decided_by:actor,decision_note:"<>& yes")
  record.call(:approval,"approval_decided",agent_approval_id:approval.id)
  record.call(:approval_metadata,"approval_decided",metadata:{approval_id:approval.id})
  record.call(:github,"github_action_completed",metadata:{approval_id:approval.id,action:"github.comment",status:"completed",url:"https://example.test/?a=1&b=2",message:""})
  record.call(:fizzy,"fizzy_action_completed",metadata:{approval_id:approval.id,action:"fizzy.close",status:false,url:nil,message:"é"})
  record.call(:github_empty,"github_action_completed",metadata:[])
  thread=room.channel_threads.create!(id:900_020_001,creator:actor,name:"Work <>&",work_status:"in_progress",tag_names:["zulu","alpha"])
  thread.update_columns(work_owner_id:agent.user_id,result_markdown:"Done <>&",result_updated_at:Time.current,run_url:"https://example.test/run")
  thread.reload
  event=Event.create!(id:900_030_001,room:room,organizer:actor,title:"Review <>&",starts_at:Time.current+1.hour,ends_at:Time.current+2.hours,time_zone:"UTC")
  thread.work_thread_links.create!(kind:"drive_file",url:"https://drive.google.com/open?id=first",title:"<>& file",created_by:actor)
  thread.work_thread_links.create!(kind:"event",event:event,created_by:actor)
  pr=Github::PullRequest.create!(id:900_040_001,owner:"org",repo:"repo",number:7,title:"Secret title",head_branch:"secret",base_branch:"main",state:"open",check_status:"success",review_decision:"APPROVED",private:true)
  thread.work_thread_links.create!(kind:"pull_request",github_pull_request:pr,created_by:actor)
  record.call(:assigned,"work_assigned",room:room,metadata:{thread_id:thread.id,assigned_by:actor.name})
  record.call(:unassigned,"work_unassigned",room:room,metadata:{thread_id:thread.id,assigned_by:nil})
  record.call(:handoff,"work_handed_off",room:room,metadata:{thread_id:thread.id,assigned_by:actor.name,handoff:{summary:"<>&",messages:[]}})
  record.call(:snapshot,"work_unassigned",metadata:{thread_id:0,work_snapshot:{id:7,title:"Old",thread_id:7,status:"planned",assigned_by:nil}})
  record.call(:snapshot_handoff,"work_handed_off",metadata:{thread_id:0,work_snapshot:{id:8},handoff:{summary:"<>&"}})
  message=Message.find(136976342)
  message.update_columns(thread_id:thread.id)
  Github::PullRequestThread.create!(channel_thread:thread,room:room,pull_request:pr)
  message.drive_attachments.create!(file_id:"ws11_abc-123")
  payloads[:message_private]=agent.user.webhook.send(:payload,message,agent:agent,delivery_id:123)
  pr.update!(private:false)
  payloads[:message_public]=agent.user.webhook.send(:payload,message.reload,agent:agent,delivery_id:123)
  record.call(:assigned_public,"work_assigned",room:room,metadata:{thread_id:thread.id,assigned_by:actor.name})
  # Probe the injected GitHub domain decision for an owner's readable private repository.
  pr.update!(private:true)
  agent.owner.github_connected_account.define_singleton_method(:can_read_repository?) { |*_args| true } if agent.owner.github_connected_account
  Github::PullRequest.define_method(:details_visible_to_agent?) { |viewer| !!viewer&.owner }
  payloads[:message_owner_readable]=agent.user.webhook.send(:payload,message,agent:agent,delivery_id:123)
  missing={}
  [["mention",{message_id:0}],["approval_decided",{agent_approval_id:0}],["work_assigned",{metadata:{thread_id:0}}],["posted",{}]].each do |kind,attributes|
    row=agent.agent_events.create!(event_type:kind,**attributes)
    begin
      Agent::Delivery.post_event_webhook!(agent.user.webhook,row,agent:agent)
    rescue => error
      missing[kind]="#{error.class}: #{error.message}"
    end
  end
  puts JSON.pretty_generate(reference_pin:"d7c7de92",payloads:payloads,missing:missing)
end
