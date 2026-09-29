# Server-side events for recording reference.json (see ../golden.rs), run inside the recording
# instance: `parity/bin/reference runner --port N .../trigger.rb EVENT ID...`.
event, *ids = ARGV
ids = ids.map(&:to_i)

case event
when "unread"
  Message.find(ids[0]).send(:broadcast_unread_room)
when "remove_message"
  Message.find(ids[0]).broadcast_remove
when "revoke"
  Room.find(ids[0]).memberships.revoke_from(User.find(ids[1]))
when "deactivate"
  User.find(ids[0]).deactivate
when "notifications"
  user = User.find(ids[0])
  ActionCable.server.broadcast "user_#{user.id}_activity", { activityItemId: 42 }
  ActionCable.server.broadcast "user_#{user.id}_huddle_notices", { huddleJoinNotice: { eventType: "huddle_ended", roomId: ids[1] } }
  ActionCable.server.broadcast "user_#{user.id}_unread_threads", { threadId: ids[2], roomId: ids[1] }
  Turbo::StreamsChannel.broadcast_replace_to "agents:all", target: "status_badge_agent_1", html: "<span>ready</span>"
  Turbo::StreamsChannel.broadcast_update_to [user, :status], target: "status_badge_user_#{user.id}", html: "<span>online</span>"
  Turbo::StreamsChannel.broadcast_update_to [user, :ooo_notice], target: "ooo_notice_user_#{user.id}", html: "<span>away</span>"
  Turbo::StreamsChannel.broadcast_remove_to [ChannelThread.find(ids[2]), :messages], target: "message_golden"
  Turbo::StreamsChannel.broadcast_remove_to [Room.find(ids[1]), :threads], target: "channel_thread_golden"
else
  raise "unknown event #{event}"
end
