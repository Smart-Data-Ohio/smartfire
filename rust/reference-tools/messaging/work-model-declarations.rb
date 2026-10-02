# Direct-model declarations in channel_threads_controller_test.rb:387-411.
require 'json'
require 'digest'
ActiveJob::Base.queue_adapter=:test
room=Room.find(654632876);actor=User.find(773523953);owner=User.find(712064548)
thread=ChannelThread.create!(room:,creator:actor,name:'Omitted owner model')
thread.update!(work_status:'planned',work_owner_id:owner.id)
forbidden=false
begin
  thread.update_work!(actor:owner,work_status:nil)
rescue ChannelThread::WorkUpdateForbidden
  forbidden=true
end
thread.reload
omitted={forbidden:,status:thread.work_status,owner:thread.work_owner_id,event_count:thread.work_thread_events.count}
thread=ChannelThread.create!(room:,creator:actor,name:'Independent stale model')
thread.update!(work_status:'planned')
first=ChannelThread.find(thread.id);second=ChannelThread.find(thread.id)
first.update_work!(actor:,work_status:'in_progress')
second.update_work!(actor:,work_status:'blocked')
stale={status:thread.reload.work_status,owner:thread.work_owner_id,events:thread.work_thread_events.ordered.pluck(:actor_id,:event_type,:from_status,:to_status)}
puts JSON.pretty_generate(reference:'d7c7de92',sources:%w[app/models/channel_thread.rb].to_h{|path|[path,Digest::SHA256.file(Rails.root.join(path)).hexdigest]},omitted:,stale:)
warn 'WS8bm Rails direct-model declarations: 2 cases; omitted-owner refusal and independent stale writes; real callbacks'
