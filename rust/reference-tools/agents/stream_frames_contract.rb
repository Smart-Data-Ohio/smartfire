require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter=:test
travel_to Time.utc(2026,3,2,16) do
  frames=[]
  ActionCable.server.define_singleton_method(:broadcast) { |stream,body,*args,**kwargs| frames << {stream:stream,body:body} }
  room=Room.find(486777696);message=room.root_messages.create!(creator_id:394959859,streaming:true,markdown_source:"Starting",client_message_id:"ws11-stream-frame")
  message.broadcast_stream_start
  start=frames.dup;frames.clear
  message.update!(markdown_source:"Latest draft")
  message.broadcast_stream_update
  update=frames.dup;frames.clear
  message.finalize_stream!
  final=frames.dup
  puts JSON.pretty_generate(reference_pin:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],message_id:message.id,start:start,update:update,final:final)
end
