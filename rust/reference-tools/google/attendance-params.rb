require 'json'
require 'nokogiri'
require 'action_dispatch/testing/integration'
ActiveJob::Base.queue_adapter=:test
ActionController::Base.allow_forgery_protection=false
Rails.application.routes.default_url_options.merge!(host:'campfire.test',protocol:'http')
user=User.find(127326141);room=Room.find(486777696)
event=room.events.create!(id:9_300_000_001,organizer:user,title:'Compound response',starts_at:Time.utc(2026,3,3,9),time_zone:'UTC')
session=Session.create!(user:,user_agent:'compound-fixture',ip_address:'127.0.0.1',two_factor_verified_at:Time.current)
request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.url_scheme'=>'http','REQUEST_METHOD'=>'GET'))
jar=ActionDispatch::Cookies::CookieJar.build(request,{})
jar.signed[:session_token]={value:session.token}
headers={'HTTP_COOKIE'=>"session_token=#{URI.encode_www_form_component(jar[:session_token])}",'HTTP_ACCEPT'=>'text/html','HTTP_TURBO_FRAME'=>'fixture-frame'}
shapes={nested:[['601','602'],['603']],mixed:['601',nil,false,7,[],['602',true]],hash_items:[{a:'601'},{b:['602',nil]}],deep_hash:{a:{b:[nil,false,3.5]}},escaped:["quote\"&<>\n\t",['#{x}']],empty:[],flat:['601','602'],scalar:'601'}
cases=[]
shapes.each do |name,value|
  %w[show patch_top patch_nested].each do |location|
    client=ActionDispatch::Integration::Session.new(Rails.application);client.host! 'campfire.test'
    params= location=='patch_nested' ? {attendance:{response:'invalid',message_id:value}} : {message_id:value,response:'invalid'}
    path="/rooms/#{room.id}/events/#{event.id}/attendance"
    client.public_send(location=='show' ? :get : :patch,path,params:,headers:headers.dup,as: :json)
    fragment=Nokogiri::HTML.fragment(client.response.body)
    cases << {name:,location:,params:,status:client.response.status,frame_id:fragment.at_css('turbo-frame')&.[]('id'),input:fragment.at_css('input[name=message_id]')&.[]('value'),frame_tag:client.response.body[/<turbo-frame\b[^>]*>/],input_tag:client.response.body[/<input\b[^>]*\bname="message_id"[^>]*>/],body:client.response.status==200 ? client.response.body : nil}
  end
end
puts JSON.pretty_generate({reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],event_id:event.id,cases:})
