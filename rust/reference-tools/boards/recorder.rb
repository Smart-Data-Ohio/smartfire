# SQL traces and grouped-item broadcasts from the pinned Rails models.
require 'json'
require 'digest'
hashes=JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'reference-tools/boards/source-hashes.json')))
hashes.merge!(JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'reference-tools/boards/post-source-hashes.json'))))
hashes.merge!(JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'reference-tools/boards/write-source-hashes.json'))))
hashes.each { |file,hash| raise "source drift: #{file}" unless Digest::SHA256.file(Rails.root.join(file)).hexdigest==hash }
results=[]
ActiveJob::Base.queue_adapter=:test
Rails.logger = ActiveSupport::Logger.new($stderr)
[1,10,20].each do |n|
 actor=User.find(127326141)
 room=Rooms::Board.create!(name:"Review #{n}",creator:actor)
 room.memberships.find_or_create_by!(user:actor)
 thread=ChannelThread.create_board_post!(room:room,creator:actor,name:'Review post',work_status:'planned')
 n.times do |i|
  user=User.create!(name:"Review #{n} follower #{i}",skip_open_room_grant:true)
  room.memberships.create!(user:user,involvement:'everything')
  ThreadMembership.join!(thread,user).update!(involvement:'everything')
 end
 queries=[]
 callback=->(*args){payload=args.last;queries<<payload[:sql] unless payload[:cached]}
 ActiveSupport::Notifications.subscribed(callback,'sql.active_record') do
  thread.update_work!(actor:actor,work_status:'done')
 end
 rosters=queries.grep(/SELECT.*FROM "(?:thread_memberships|memberships)"/i)
 users=queries.grep(/SELECT.*FROM "users"/i)
 results << {followers:n,membership_selects:rosters.size,user_selects:users.size,total:queries.size}
end
actor=User.find(127326141)
room=Rooms::Board.create!(name:'Broadcast review',creator:actor)
room.memberships.find_or_create_by!(user:actor)
follower=User.create!(name:'Broadcast reviewer',skip_open_room_grant:true)
room.memberships.create!(user:follower,involvement:'everything')
thread=ChannelThread.create_board_post!(room:room,creator:actor,name:'Broadcast post',work_status:'planned')
ThreadMembership.join!(thread,follower).update!(involvement:'everything')
thread.update_work!(actor:actor,work_status:'in_progress')
initial=ActivityItem.where(user:follower,event_type:'work_update').first!
frames=[]
callback=->(*args){payload=args.last;frames<<payload if payload[:broadcasting]==ActivityChannel.stream_name_for(follower.id)}
ActiveSupport::Notifications.subscribed(callback,'broadcast.action_cable') do
 thread.update_work!(actor:actor,work_status:'blocked')
end
updated=initial.reload
raise 'grouped item was not repointed' unless updated.id==initial.id && updated.source_id==thread.work_thread_events.order(:id).last.id
unread_frames=frames.size
initial.mark_read!
frames.clear
ActiveSupport::Notifications.subscribed(callback,'broadcast.action_cable') do
 thread.update_work!(actor:actor,work_status:'done')
end
raise 'read grouped item was not reopened' unless initial.reload.read_at.nil?
puts JSON.pretty_generate(reference:'d7c7de92 plus approved board drift',fanout:results,unread_repoint_activity_frames:unread_frames,read_repoint_activity_frames:frames.size)
warn "Rails recorder oracle: #{results.size} fanout traces; unread/read repoint activity frames=#{unread_frames}/#{frames.size}"
