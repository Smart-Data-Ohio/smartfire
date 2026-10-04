require "json"
require "active_support/testing/time_helpers"

class HuddleQueryAssertions
  include ActiveSupport::Testing::TimeHelpers
  NOW = Time.utc(2026,1,1,12)
  def reset
    travel_to NOW
    ActiveRecord::Schema.verbose=false
    load Rails.root.join("db/schema.rb")
    ActiveRecord::FixtureSet.reset_cache
    ActiveRecord::FixtureSet.create_fixtures(Rails.root.join("test/fixtures"),%w[accounts users rooms memberships sessions push/subscriptions])
    ENV["LIVEKIT_API_SECRET"]="ws13b-fixture-api-secret"
    ENV.delete("LIVEKIT_URL"); ENV.delete("LIVEKIT_GATEWAY_SECRET")
    @david,@jason,@kevin,@jz=%w[david jason kevin jz].map { |k| User.find(ActiveRecord::FixtureSet.identify(k)) }
    ActionCable.server.define_singleton_method(:broadcast) { |*_| }
    [Huddle::PushInvitationJob,Huddle::BroadcastPresenceJob,Huddle::JoinNoticeJob].each { |job|job.define_singleton_method(:perform_later) { |*_| } }
    Rails.configuration.x.web_push_pool.define_singleton_method(:queue) { |*_| }
    @grant=nil
  end
  def stage
    @room=Rooms::Stage.create!(id:9001,name:"Town Hall",creator:@david)
    [@david,@jason].each_with_index { |u,i| @room.memberships.create!(id:9011+i,user:u,stage_role:i==0 ? "host" : "listener") }
    @host=@room.memberships.find_by!(user:@david)
  end
  def input
    {room:@room.attributes,memberships:@room.memberships.order(:id).map(&:attributes),users:User.order(:id).map { |u|u.attributes.slice("id","name","role","status","inbox_preferences") },sessions:Session.order(:id).map(&:attributes),grants:HuddleGrant.order(:id).map { |g|g.attributes.merge("identity"=>"ws13b-query-#{g.id}") },items:ActivityItem.order(:id).map(&:attributes),streams:Stream.order(:id).map(&:attributes)}
  end
  def capture
    queries=[]
    subscription=ActiveSupport::Notifications.subscribe("sql.active_record") { |*,p|queries << {sql:p[:sql],transaction:ActiveRecord::Base.connection.transaction_open?} unless p[:cached] || p[:name]=="SCHEMA" }
    ActiveRecord::Base.connection.clear_query_cache
    value=yield
    [value,queries]
  ensure
    ActiveSupport::Notifications.unsubscribe(subscription)
  end
  def run
    cases=[]
    [4,8].each do |size|
      reset
      people=[@david,@jason,@kevin,@jz]
      (size-4).times { |i|people << User.create!(id:10000+i,name:"Query member #{i}",email_address:"ws13b-query-#{i}@example.test",password:"ws13b-fixture-password") }
      @room=Rooms::Direct.create!(id:9001,creator:@david)
      people.each_with_index { |u,i|@room.memberships.create!(id:9011+i,user:u,involvement:"everything") }
      @grant=HuddleGrant.issue!(session:Session.find(ActiveRecord::FixtureSet.identify(:david_safari)),membership:@room.memberships.find_by!(user:@david))
      @grant.update_columns(last_seen_at:Time.current)
      peer=HuddleGrant.issue!(session:@jason.sessions.create!(token:"ws13b-query-jason"),membership:@room.memberships.find_by!(user:@jason))
      peer.update_columns(last_seen_at:Time.current)
      ActivityItem.where(event_type:"huddle_started").update_all(handled_at:Time.current)
      @grant.user
      state=input
      _,queries=capture { Huddle::JoinNotifier.notify_join(@grant) }
      user_selects=queries.select { |q|q[:sql].match?(/FROM "users"/) }
      ring_selects=queries.select { |q|q[:sql].match?(/FROM "activity_items"/) }
      raise "incorrect Rails query count" unless user_selects.size==2 && ring_selects.size==1
      cases << {name:"notifier_#{size}",input:state,queries:queries,user_selects:user_selects,ring_selects:ring_selects}
    end
    %w[demotion_lock departure_lock].each do |name|
      reset;stage;state=input;events=[]
      original_lock=Room.instance_method(:lock!)
      Room.define_method(:lock!) { |*args| events << "lock";original_lock.bind_call(self,*args) }
      original_exists=ActiveRecord::Relation.instance_method(:exists?)
      ActiveRecord::Relation.define_method(:exists?) do |*args|
        events << (ActiveRecord::Base.connection.transaction_open? ? "check_in_transaction" : "check_outside_transaction") if klass==Membership
        original_exists.bind_call(self,*args)
      end
      begin
        error=nil
        _,queries=capture do
          if name=="departure_lock"
            @host.destroy!
          else
            begin;@host.change_stage_role!("listener");rescue ActiveRecord::RecordInvalid=>e;error=e.record.errors.full_messages.to_sentence;end
          end
        end
      ensure
        Room.define_method(:lock!,original_lock)
        ActiveRecord::Relation.define_method(:exists?,original_exists)
      end
      raise "incorrect Rails lock order" unless events==%w[lock check_in_transaction]
      cases << {name:name,input:state,error:error,events:events,queries:queries}
    end
    %w[preloaded_live preloaded_ended unloaded_live].each do |name|
      reset;stage
      first=Stream.create!(id:9001,room:@room,membership:@host,user:@david,quality:"1080p15")
      first.end! unless name=="unloaded_live"
      Stream.create!(id:9002,room:@room,membership:@host,user:@david,quality:"1080p15") if name=="preloaded_live"
      state=input
      room=name=="unloaded_live" ? Rooms::Stage.find(@room.id) : Rooms::Stage.includes(:live_streams).find(@room.id)
      preloaded_ids = room.live_streams.map(&:id) unless name=="unloaded_live"
      value,queries=capture { room.live_stream&.id }
      raise "preload issued queries" unless name=="unloaded_live" || queries.empty?
      cases << {name:name,input:state,value:value,preloaded_ids:preloaded_ids,queries:queries}
    end
    reset
    @room=Room.find(ActiveRecord::FixtureSet.identify(:watercooler))
    @grant=HuddleGrant.issue!(session:Session.find(ActiveRecord::FixtureSet.identify(:david_safari)),membership:@room.memberships.find_by!(user:@david))
    state=input
    _,queries=capture { @grant.revoke! }
    raise "non-stage queried streams" if queries.any? { |q|q[:sql].match?(/FROM "streams"/) }
    cases << {name:"nonstage_no_stream_query",input:state,queries:queries}
    puts JSON.pretty_generate(reference_pin:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],now:NOW.to_i,cases:cases)
  ensure
    travel_back
  end
end
HuddleQueryAssertions.new.run
