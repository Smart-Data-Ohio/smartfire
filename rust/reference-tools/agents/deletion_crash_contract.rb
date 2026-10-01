# An actual process exit before Rails after_destroy_commit, then a new process reads the same DB.
if ARGV.first=='stop'
  ApplicationJob.queue_adapter=:test
  agent=Agent.find(773018776);room=Room.find(486777696);human=User.find(127326141)
  AgentGrant.delete_all;agent.agent_events.delete_all;room.memberships.grant_to([agent.user])
  Webhook.find_or_create_by!(user:agent.user) {|w|w.url='https://bots.example.test/hook'}
  thread=ChannelThread.create!(room:room,creator:human,name:'Restart gap',work_status:'planned')
  thread.update_columns(work_owner_id:agent.user_id)
  ChannelThread.prepend(Module.new do
    def emit_deleted_work_unassigned
      raise 'deletion did not commit' if ChannelThread.exists?(id)
      STDOUT.flush
      Process.exit!(73)
    end
  end)
  thread.destroy!
  raise 'stop hook did not run'
else
  agent=Agent.find(773018776)
  page=Agents::EventPolling.poll(agent:agent,since:0,limit:50,presenter:->(m) {{id:m.id}})
  result={thread_exists:ChannelThread.where(name:'Restart gap').exists?,deletion_events:agent.agent_events.where(event_type:'work_unassigned').count,
    polled_deletions:page[:events].count {|e|e[:event_type]=='work_unassigned'}}
  puts JSON.pretty_generate({reference:'d7c7de92',results:result}.as_json)
end
