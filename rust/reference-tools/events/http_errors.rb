require 'json'
require 'active_record/fixtures'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
ActiveRecord::Base.logger=nil
travel_to Time.utc(2026,2,10,12)
fixtures=Rails.root.join('test/fixtures')
ActiveRecord::FixtureSet.create_fixtures(fixtures,Dir[fixtures.join('**/*.yml')].map{|p|p.delete_prefix("#{fixtures}/").delete_suffix('.yml')},{'twitter_posts'=>Twitter::Post,'twitter_post_references'=>Twitter::PostReference})
ActiveJob::Base.queue_adapter=:test
out=[]
travel_to Time.utc(2026,3,2,16) do
  david=User.find(ActiveRecord::FixtureSet.identify(:david));kevin=User.find(ActiveRecord::FixtureSet.identify(:kevin))
  designers=Room.find(ActiveRecord::FixtureSet.identify(:designers));all_talk=Room.find(ActiveRecord::FixtureSet.identify(:watercooler));pets=Room.find(ActiveRecord::FixtureSet.identify(:pets))
  launch=Event.find(ActiveRecord::FixtureSet.identify(:launch_party))
  [[david,"/rooms/#{designers.id}/events/unknown"],[david,"/rooms/#{designers.id}/events/999999"],[david,"/rooms/#{pets.id}/events/#{launch.id}"],[david,"/rooms/#{all_talk.id}/events/999999/attendance"],[kevin,"/rooms/#{all_talk.id}/events"]].each do |user,path|
    session=Session.create!(user:,user_agent:'oracle',ip_address:'127.0.0.1',two_factor_verified_at:Time.current)
    request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'example.com','rack.url_scheme'=>'http','REQUEST_METHOD'=>'GET'))
    jar=ActionDispatch::Cookies::CookieJar.build(request,{})
    jar.signed[:session_token]={value:session.token}
    %w[text/html application/json].each do |accept|
      client=ActionDispatch::Integration::Session.new(Rails.application)
      client.get(path,headers:{'HTTP_COOKIE'=>"session_token=#{URI.encode_www_form_component(jar[:session_token])}",'HTTP_ACCEPT'=>accept})
      response=client.response
      raise "oracle authentication failed: #{response.status} #{path}" unless response.status==404
      out << {user_id:user.id,path:,accept:,status:response.status,content_type:response.headers['content-type'],body:response.body}
    end
  end
end
File.write('/rails/storage/db/event-http-errors.json',JSON.pretty_generate(out)+"\n")
puts "Rails event rescued 404s: #{out.size} HTML/JSON states"
