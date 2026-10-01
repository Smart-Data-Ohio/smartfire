require "json"
require "active_support/testing/time_helpers"

# Persisted state oracle for Stream.end_stale_live!. Stream's HTML callbacks still execute
# in Rails; their full render parity belongs to the remaining Stage presentation slice.
class HuddleStaleStreamOracle
  include ActiveSupport::Testing::TimeHelpers
  def run
    ActiveRecord::Schema.verbose = false
    load Rails.root.join("db/schema.rb")
    ActiveRecord::FixtureSet.create_fixtures(Rails.root.join("test/fixtures"), %w[accounts users rooms memberships])
    @users=%w[david jason kevin bender].map {|key|User.find(ActiveRecord::FixtureSet.identify(key))}
    ActionCable.server.define_singleton_method(:broadcast) {|*_|}
    cases={
      "no_grants"=>{}, "never_seen"=>{grant:true}, "seen_29"=>{seen:29},
      "seen_exact_30"=>{seen:30}, "seen_31"=>{seen:31}, "revoked_seen"=>{seen:0,revoked:true},
      "other_membership"=>{seen:0,other_member:true}, "other_room"=>{seen:0,other_room:true},
      "second_device_recent"=>{seen:31,second:true}, "ended_history"=>{ended:true},
      "deleted_room"=>{deleted:true}, "inactive_presenter"=>{seen:0,inactive:true},
      "bad_quality"=>{quality:"4k60"}, "missing_membership"=>{missing_member:true},
      "missing_user"=>{missing_user:true}, "missing_room"=>{missing_room:true}
    }.map {|name,options|scenario(name,options)}
    puts JSON.pretty_generate({reference_pin:"d7c7de92",now:Time.current.to_i,cases:cases})
  ensure
    travel_back
  end

  def scenario(name,options)
    travel_to Time.utc(2026,1,1,12)
    room=Rooms::Stage.create!(id:9001,name:"WS13 Stage",creator:@users[0])
    @users.each_with_index {|user,i|room.memberships.create!(id:9011+i,user:user,involvement:"everything",stage_role:i==0 ? "host" : "listener")}
    member=room.memberships.find_by!(user:@users[0])
    stream=Stream.create!(id:40,room:room,membership:member,user:@users[0],quality:"1080p15",started_at:Time.current-120,created_at:Time.current-120)
    stream.update_columns(ended_at:Time.current-100) if options[:ended]
    stream.update_columns(quality:options[:quality]) if options[:quality]
    if options[:grant] || options.key?(:seen)
      user=options[:other_member] ? @users[1] : @users[0]
      grant_room=options[:other_room] ? Room.find(ActiveRecord::FixtureSet.identify(:watercooler)) : room
      HuddleGrant.create!(id:17,identity:"ws13-stale-grant",room_name:"ws13-stale-room",user:user,room:grant_room,membership:grant_room.memberships.find_by!(user:user),session:Session.create!(id:7001,user:user),last_seen_at:options.key?(:seen) ? Time.current-options[:seen] : nil,revoked_at:options[:revoked] ? Time.current : nil)
    end
    if options[:second]
      HuddleGrant.create!(id:18,identity:"ws13-stale-second-device",room_name:"ws13-stale-room",user:@users[0],room:room,membership:member,session:Session.create!(id:7002,user:@users[0]),last_seen_at:Time.current-29)
    end
    room.update_columns(deleted_at:Time.current) if options[:deleted]
    @users[0].update_columns(status: :banned) if options[:inactive]
    member.delete if options[:missing_member]
    # Presence validations can encounter imported orphan coordinates; keep fixtures intact.
    stream.update_columns(user_id:999999999) if options[:missing_user]
    stream.update_columns(room_id:999999999) if options[:missing_room]
    input={room:room.attributes,memberships:room.memberships.reload.map(&:attributes),users:@users.map {|u|u.reload.attributes.slice("id","name","role","status","inbox_preferences")},grants:HuddleGrant.all.map(&:attributes),items:[],streams:Stream.all.map(&:attributes)}
    error=nil
    begin
      Stream.end_stale_live!
    rescue ActiveRecord::RecordInvalid=>exception
      error=exception.record.errors.messages
    end
    {name:name,input:input,streams:Stream.order(:id).map(&:attributes),errors:error}
  ensure
    Stream.delete_all
    HuddleGrant.delete_all
    Membership.where(room_id:9001).delete_all
    Room.where(id:9001).delete_all
    Session.where(id:[7001,7002]).delete_all
    @users[0].update_columns(status: :active)
  end
end
HuddleStaleStreamOracle.new.run
