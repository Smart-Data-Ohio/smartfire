require 'json'
require 'active_record/fixtures'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
ActiveRecord::Base.logger=nil
travel_to Time.utc(2026,2,10,12)
fixtures=Rails.root.join('test/fixtures')
ActiveRecord::FixtureSet.create_fixtures(fixtures,Dir[fixtures.join('**/*.yml')].map{|p|p.delete_prefix("#{fixtures}/").delete_suffix('.yml')},{'twitter_posts'=>Twitter::Post,'twitter_post_references'=>Twitter::PostReference})
ActiveJob::Base.queue_adapter=:test
class EventPageGoldenController < ApplicationController
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action&&method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
renderer=EventPageGoldenController.renderer.new(http_host:'campfire.test',https:false,'HTTP_USER_AGENT'=>'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36','rack.session'=>{},'action_dispatch.content_security_policy'=>Rails.application.config.content_security_policy,'action_dispatch.content_security_policy_nonce_generator'=>->(_){'NONCE'})
room=Room.find(ActiveRecord::FixtureSet.identify(:designers));david=User.find(ActiveRecord::FixtureSet.identify(:david));jason=User.find(ActiveRecord::FixtureSet.identify(:jz))
def form_fact(e)
  h=ApplicationController.helpers;zone=e.time_zone.presence||'UTC'
  {room_id:e.room_id,room_name:e.room.name,id:e.persisted? ? e.id : nil,title:e.title.to_s,title_value:e.title,description:e.description,starts_at:e.starts_at&.in_time_zone(zone)&.strftime('%Y-%m-%dT%H:%M'),ends_at:e.ends_at&.in_time_zone(zone)&.strftime('%Y-%m-%dT%H:%M'),time_zone:zone,venue_room_id:e.venue_room_id,recurrence_rule:e.recurrence_rule,recurrence_until:e.recurrence_until&.iso8601,meet_link_requested:e.meet_link_requested,meet_link:h.safe_meet_link(e),series:e.series?,head:e.series_head?,errors:e.errors.full_messages,error_fields:e.errors.attribute_names.map(&:to_s),venues:h.event_venue_options(e).flat_map{|label,options|options.map{|name,id|{id:,name:,stage:label=='Stage'}}}}
end
out=[]
travel_to Time.utc(2026,9,22,12) do
  voice=Rooms::Voice.create!(creator:david,name:'Calls & chat');voice.memberships.create!(user:david)
  stage=Rooms::Stage.create!(creator:david,name:'Presentation <&>');stage.memberships.create!(user:david,stage_role:'host')
  single=room.events.create!(organizer:david,title:'Planning <&>',starts_at:Time.utc(2026,10,5,9),ends_at:Time.utc(2026,10,5,10),time_zone:'Eastern Time (US & Canada)',description:"One line\nsecond line",venue:voice)
  head=room.events.create!(organizer:david,title:'Repeating',starts_at:Time.utc(2026,10,6,9),time_zone:'UTC',recurrence_rule:'weekly',recurrence_until:Date.new(2026,10,20),venue:stage)
  single.update_column(:meet_link,'https://meet.google.com/a?x=1&y=2')
  [david,jason].each do |u|
    ['UTC','Hawaii'].each do |zone|
      Time.use_zone(zone) do
        Current.user=u
        candidates=[room.events.build(time_zone:'UTC'),room.events.build(title:'Prefilled <&>',starts_at:Time.utc(2026,10,5,9),time_zone:'Eastern Time (US & Canada)',meet_link_requested:true),Event.find(single.id),*head.series_events.map{|e|Event.find(e.id)}]
        invalid=room.events.build(title:'',time_zone:'UTC',starts_at:nil,recurrence_rule:'weekly');invalid.valid?;candidates << invalid
        bad_edit=Event.find(single.id);bad_edit.assign_attributes(title:'',ends_at:Time.utc(2026,10,4,9));bad_edit.valid?;candidates << bad_edit
        candidates.each do |e|
          kind=e.persisted? ? 'edit' : 'new'
          html=renderer.render(template:"rooms/events/#{kind}",assigns:{room:,event:e})
          out << {kind:,user:u.name,zone:,view:form_fact(e),html:}
        end
        Current.reset
      end
    end
  end
end
# Differential the actual private controller methods and ActiveRecord casts.
def input_fact(e)
  {title:e.title.to_s,description:e.description,starts_at:e.starts_at&.utc&.iso8601(6),ends_at:e.ends_at&.utc&.iso8601(6),time_zone:e.time_zone,venue_room_id:e.venue_room_id,recurrence_rule:e.recurrence_rule,recurrence_until:e.recurrence_until&.iso8601,meet_link_requested:e.meet_link_requested}
end
inputs=[nil,{},false,true,'scalar',[],{title:'  spaced  '},{title:'é'*260,starts_at:'2026-10-05T09:00',time_zone:'Eastern Time (US & Canada)'},{title:false,description:true},{title:['hidden'],starts_at:{bad:'shape'},time_zone:['UTC']},{starts_at:''},{starts_at:'junk'},{starts_at:'tomorrow'},{starts_at:'9:00'},{starts_at:'2026-03-08T02:30',time_zone:'Eastern Time (US & Canada)'},{starts_at:'2026-11-01T01:30',time_zone:'Eastern Time (US & Canada)'},{starts_at:'2026-10-05T09:00:00+02:00',time_zone:'Hawaii'},{starts_at:'2026-10-05T09:00',time_zone:'Unknown/Zone'},{starts_at:'2026-10-05T09:00',time_zone:''},{title:'  ',starts_at:'2026-02-30T09:00'},{recurrence_until:'bad',venue_room_id:'junk',meet_link_requested:'off'},{recurrence_until:'2026-02-30',venue_room_id:'',meet_link_requested:'0'}]
input_vectors=[]
travel_to Time.utc(2026,9,22,12) do
  Time.use_zone('Hawaii') do
    Current.user=david
    %w[create update prefill].each do |mode|
      inputs.each do |input|
        c=Rooms::EventsController.new;c.params=ActionController::Parameters.new(event:input)
        e=room.events.build(title:'Before',time_zone:'Eastern Time (US & Canada)',starts_at:Time.utc(2026,10,5,9),description:'Before description')
        c.instance_variable_set(:@event,e) if mode=='update'
        begin
          attrs=c.send(mode=='prefill' ? :new_prefill : :event_attributes)
          result=(mode=='update' ? e.tap{|x|x.assign_attributes(attrs)} : room.events.build(attrs))
          input_vectors << {mode:,input:,fact:input_fact(result)}
        rescue => error
          input_vectors << {mode:,input:,status:error.is_a?(ActionController::ParameterMissing) ? 400 : 500,exception:error.class.name}
        end
      end
    end
    Current.reset
  end
end
File.write('/rails/storage/db/event-input.json',JSON.pretty_generate(input_vectors)+"\n")
puts "Rails event input: #{input_vectors.size} states"
out.each do |state|
  Time.use_zone(state[:zone]) do
    Current.user=User.find_by!(name:state[:user])
    state[:logo]=renderer.render(inline:'<%= fresh_account_logo_path %>',layout:false)
    state[:avatar]=renderer.render(inline:'<%= fresh_user_avatar_path(Current.user) %>',layout:false)
    Current.reset
  end
end
File.write('/rails/storage/db/event-forms.json',JSON.pretty_generate(out)+"\n")
puts "Rails event forms: #{out.size} states"
