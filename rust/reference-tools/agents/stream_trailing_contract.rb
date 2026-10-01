require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter=:test
frames=[]
Message.prepend(Module.new do
  define_method(:broadcast_replace_to) {|*args,**opts|frames << {source:markdown_source,streaming:streaming}}
end)
base=Time.utc(2026,3,2,16)
travel_to base do
  agent=Agent.find(773018776);room=Room.find(486777696)
  message=room.messages.create!(id:900100001,creator:agent.user,markdown_source:"",streaming:true)
  result={}
  message.update!(markdown_source:"A");result[:first]=message.broadcast_stream_update
  travel_to(base+0.1,with_usec:true)
  message.update!(markdown_source:"AB");result[:second]=message.broadcast_stream_update
  travel_to(base+0.249,with_usec:true)
  message.update!(markdown_source:"ABC");result[:third]=message.broadcast_stream_update
  jobs=ApplicationJob.queue_adapter.enqueued_jobs.select{|j|j[:job]==Message::StreamTrailingBroadcastJob}.map{|j|j[:args]}
  result[:jobs]=jobs
  travel_to(base+0.25,with_usec:true)
  Message::StreamTrailingBroadcastJob.perform_now(*jobs.first)
  Message::StreamTrailingBroadcastJob.perform_now(*jobs.last)
  message.reload.finalize_stream_quietly!
  Message::StreamTrailingBroadcastJob.perform_now(*jobs.first)
  result[:frames]=frames
  puts JSON.pretty_generate({reference_pin:"d7c7de92",results:result}.as_json)
end
