# The four remaining MessageStreamingTest projections, through real callbacks and cable renders.
require "active_support/testing/time_helpers"
require "nokogiri"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter = :test
ActionCable.server.instance_variable_set(:@pubsub, ActionCable::SubscriptionAdapter::Test.new(ActionCable.server))
Rails.logger = ActiveSupport::Logger.new($stderr)
kind=ARGV.fetch(0)
raise "unknown case" unless %w[start finalize append trailing].include?(kind)
travel_to Time.utc(2026,3,2,16) do
  room=Room.find(486777696);bot=User.find(394959859);manager=User.find(127326141)
  AgentGrant.delete_all;AgentEvent.delete_all
  watcher_bot=User.create_bot!(id:1901700011,name:"Stream Watcher")
  watcher=watcher_bot.create_agent!(id:1901700012,kind: :workspace,owner:manager)
  legacy=User.create_bot!(id:1901700021,name:"Legacy Stream",webhook_url:"https://example.test/legacy-stream")
  room.memberships.grant_to([bot,watcher_bot,legacy])
  Membership.where(room:room,user_id:[127326141,149087659]).update_all(unread_at:nil)
  ActiveRecord::Base.connection.execute("UPDATE sqlite_sequence SET seq=1901720000 WHERE name='messages'")
  ApplicationJob.queue_adapter.enqueued_jobs.clear
  ActiveRecord::Base.connection.execute("UPDATE sqlite_sequence SET seq=1901730000 WHERE name='agent_events'")
  frames=[]
  ActionCable.server.define_singleton_method(:broadcast) { |stream,body,*args,**kwargs| frames << {stream:stream,body:body} }
  source={"start"=>"Starting","finalize"=>"Hey @[David] and @[Stream Watcher] and @[Legacy Stream] hovercraft","append"=>"Hey @[David]","trailing"=>"One"}.fetch(kind)
  message=room.root_messages.create!(creator:bot,streaming:true,markdown_source:source,client_message_id:"ws11-next-2-#{kind}")
  stream=[room.to_gid_param,:messages].join(":")
  snapshots=[]
  snapshot=-> do
    jobs=ApplicationJob.queue_adapter.enqueued_jobs.map do |job|
      args=job[:args].map { |arg| arg.is_a?(Hash) && arg["_aj_globalid"] ? arg["_aj_globalid"] : arg }
      {class:job[:job].name,args:args}
    end
    snapshots << {streaming:message.reload.streaming?,source:message.markdown_source,
      activity:ActivityItem.where(source:message).order(:user_id).pluck(:user_id,:event_type),
      ledger:AgentEvent.where(message_id:message.id).order(:id).pluck(:agent_id,:event_type,:outcome),
      indexed:room.messages.search("hovercraft").map(&:id).include?(message.id),
      unread:Membership.where(room:room,user_id:[127326141,149087659]).order(:user_id).map{|m|[m.user_id,m.unread_at&.utc&.iso8601(6)]},jobs:jobs.sort_by{|job|job[:class]},
      frames:frames.select{|frame|frame[:stream]==stream}.map{|frame|frame[:body]},
      badges:[127326141,149087659].to_h{|id|[id.to_s,frames.select{|frame|frame[:stream]==UnreadRoomsChannel.stream_name_for(id)}.map{|frame|frame[:body]}]}}
  end
  snapshot.call
  returns=[]
  case kind
  when "start"
    message.broadcast_stream_start;snapshot.call
  when "finalize"
    returns << message.finalize_stream!;snapshot.call
    returns << message.finalize_stream!;snapshot.call
  when "append"
    message.update!(markdown_source:"Hey @[David] hovercraft");message.broadcast_stream_update;snapshot.call
    message.update!(markdown_source:"Hey @[David] hovercraft eels");snapshot.call
    returns << message.finalize_stream!;snapshot.call
  when "trailing"
    returns << message.broadcast_stream_update
    message.update!(markdown_source:"One two")
    returns << message.broadcast_stream_update;snapshot.call
    job=ApplicationJob.queue_adapter.enqueued_jobs.find{|j|j[:job]==Message::StreamTrailingBroadcastJob}
    raise "missing real trailing job" unless job
    travel 0.35.seconds,with_usec:true
    Message::StreamTrailingBroadcastJob.perform_now(*ActiveJob::Arguments.deserialize(job[:args]))
    snapshot.call
  end
  puts JSON.pretty_generate(name:kind,message_id:message.id,returns:returns,snapshots:snapshots)
end
