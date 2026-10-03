# ChannelThread#announce_board_post, observed after a real persisted board creation.
require 'digest'
require 'active_support/testing/time_helpers'
extend ActiveSupport::Testing::TimeHelpers
{'app/channels/unread_rooms_channel.rb'=>'e4458d89b1408c2cb0696985900e774dc19eda079acbeaf921a0b454459093db', 'app/models/channel_thread.rb'=>'89f301160437d1d35b61df1c9a54351fd31d376389517ae6ced9bfcfa3581519'}.each do |path,hash|
  raise "source drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash
end
ApplicationJob.queue_adapter=:test
rows=[]
travel_to Time.utc(2026,3,2,16) do
  david,jz,kevin=%w[david jz kevin].map { |name| User.find(ActiveRecord::FixtureSet.identify(name)) }
  ActiveRecord::Base.connection.execute("UPDATE sqlite_sequence SET seq=699448332 WHERE name='rooms'")
  room=Rooms::Board.create_for({name:'Launch',creator:david},users:[david,jz,kevin])
  room.memberships.find_by!(user:kevin).update!(involvement:'muted')
  observer=->(*args) { payload=args.last; rows << {stream:payload[:broadcasting],payload:payload[:message]} if payload[:message].is_a?(Hash) && payload[:message].key?(:roomId) }
  ActiveSupport::Notifications.subscribed(observer,'broadcast.action_cable') do
    ChannelThread.create!(room:room,creator:jz,name:'Tagged post',work_status:'planned')
  end
  puts JSON.pretty_generate({setup_sql:"UPDATE sqlite_sequence SET seq=699448332 WHERE name='rooms'",room_id:room.id,frames:rows})
end
warn "WS12_BOARD_UNREAD_RAILS #{rows.size} exact stream/payload comparisons"
