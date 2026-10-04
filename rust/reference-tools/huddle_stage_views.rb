require 'json'
require 'active_support/testing/time_helpers'
class HuddleStageViewsOracle
  include ActiveSupport::Testing::TimeHelpers
  def run
    ActiveRecord::Schema.verbose = false
    load Rails.root.join('db/schema.rb')
    ActiveRecord::FixtureSet.create_fixtures(Rails.root.join('test/fixtures'), %w[accounts users rooms memberships])
    Rails.application.routes.default_url_options[:host] = 'example.org'
    ActionCable.server.define_singleton_method(:broadcast) {|*_|}
    travel_to Time.utc(2026,1,1,12)
    users=%w[david jason kevin bender].map {|key|User.find(ActiveRecord::FixtureSet.identify(key))}
    users[0].update_columns(name: 'A <&> "host"', updated_at: Time.current)
    users[1].update_columns(name: 'Beta Speaker', role: :member, updated_at: Time.current)
    users[2].update_columns(name: 'Charlie Listener', updated_at: Time.current)
    users[3].update_columns(name: 'delta Bot', updated_at: Time.current)
    room=Rooms::Stage.create!(id:9001,name:'WS13 Stage',creator:users[0])
    members=users.each_with_index.map {|u,i|room.memberships.create!(id:9011+i,user:u,stage_role:%w[host speaker listener listener][i])}
    cases=[]
    variants={default:{}, raised:{hands:[nil,nil,20.000001,20.000002]}, muted:{muted:[true,true,false,true]}, two_hosts:{roles:%w[host host speaker listener]}, host_is_member:{roles:%w[speaker host listener listener]}, only_host:{count:1}, no_host:{roles:%w[speaker speaker listener listener]}}
    variants.each do |variant,opts|
      members.each_with_index {|m,i|m.update_columns(stage_role:(opts[:roles] || %w[host speaker listener listener])[i],hand_raised_at:opts[:hands]&.[](i) ? Time.current-opts[:hands][i] : nil,server_muted_at:opts[:muted]&.[](i) ? Time.current : nil)}
      included=members.first(opts[:count] || 4)
      Membership.where(room_id:room.id).where.not(id:included.map(&:id)).delete_all
      [false,true].each do |live|
        stream=Stream.create!(id:40,room:room,membership:included.first,user:included.first.user,quality:'1080p15') if live
        # Quiet active identities still label the badge; revoked ones never do.
        HuddleGrant.create!(id:17,identity:'ws13-older-identity',room_name:'ws13-room',room:room,membership:included.first,user:included.first.user,session:Session.create!(id:7001,user:included.first.user,token:'ws13-view-session-7001'),last_issued_at:Time.current-20) if live
        HuddleGrant.create!(id:18,identity:'ws13-latest-identity',room_name:'ws13-room',room:room,membership:included.first,user:included.first.user,session:Session.create!(id:7002,user:included.first.user,token:'ws13-view-session-7002'),last_issued_at:Time.current-10) if live
        HuddleGrant.create!(id:19,identity:'ws13-revoked-identity',room_name:'ws13-room',room:room,membership:included.first,user:included.first.user,session:Session.create!(id:7003,user:included.first.user,token:'ws13-view-session-7003'),last_issued_at:Time.current,revoked_at:Time.current) if live
        room.reload
        included.each do |viewer|
          viewer.reload
          input={room_id:room.id,viewer_id:viewer.id,members:room.memberships.order(:id).map {|m|{id:m.id,user_id:m.user_id,name:m.user.name,avatar_path:Rails.application.routes.url_helpers.fresh_user_avatar_path(m.user),administrator:m.user.administrator?,role:m.stage_role,hand:m.hand_raised_at && (m.hand_raised_at.to_r*1_000_000).to_i,muted:m.server_muted?}},live:stream ? {id:stream.id,membership_id:stream.membership_id,name:stream.user.name,identity:'ws13-latest-identity'} : nil}
          html={}
          %w[live_badge live_dot controls roster panel_body role_event].each {|part|html[part]=ApplicationController.render(partial:"rooms/stage/#{part}",locals:{room:room,membership:viewer,viewer:viewer,rejoin:false})}
          html['venue_live_dot']=ApplicationController.render(partial:'rooms/events/venue_live_dot',locals:{room:room})
          html['stream_event']=ApplicationController.render(partial:'rooms/stage/stream_event',locals:{room_id:room.id})
          cases << {name:"#{variant}_#{live ? 'live' : 'quiet'}_#{viewer.id}",input:input,html:html}
        end
        Stream.delete_all; HuddleGrant.delete_all; Session.where(id:[7001,7002,7003]).delete_all; stream=nil
      end
      # Restore the deliberately omitted memberships for the next scenario.
      members.drop(included.length).each {|m|Membership.create!(id:m.id,room:room,user:m.user,stage_role:m.stage_role)}
    end
    puts JSON.pretty_generate({reference_pin:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],cases:cases})
  ensure
    travel_back
  end
end
HuddleStageViewsOracle.new.run
