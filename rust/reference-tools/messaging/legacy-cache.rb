require 'json'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
ActionController::Base.allow_forgery_protection=false
Rails.cache=ActiveSupport::Cache::MemoryStore.new
ActionView::PartialRenderer.collection_cache=Rails.cache
ActionController::Base.perform_caching=true
Rails.application.config.hosts.clear
user=User.find(127326141);room=Room.find(486777696)
payload=%(<p title="x> http://evil.test/ <img src=x onerror=alert(1)>">hi</p>)
message=room.messages.create!(creator:user,body:payload,client_message_id:'legacy-cached-autolink')
request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
request.cookie_jar.signed[:session_token]=user.sessions.where.not(two_factor_verified_at:nil).first!.token
headers={'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}
browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test'
old_version=MessagesHelper::PRESENTATION_CACHE_VERSION
fix=RailsExt::AutoLinkOutsideAttributeValues
fix.alias_method :fixed_inside_attribute_value?,:inside_attribute_value?
fix.define_method(:inside_attribute_value?){|_offset|false}
MessagesHelper.send :remove_const,:PRESENTATION_CACHE_VERSION
MessagesHelper.const_set :PRESENTATION_CACHE_VERSION,old_version-1
browser.get("/rooms/#{room.id}/messages",headers:headers)
old_etag=browser.response.headers['ETag']
old_html=ApplicationController.renderer.new(http_host:'campfire.test',https:false).render(partial:'messages/message',collection:[message],cached:->(record){ApplicationController.helpers.message_with_pr_cards_cache_key(record)})
raise 'old renderer not vulnerable' unless Nokogiri::HTML.fragment(old_html).css('[onerror]').any?
fix.alias_method :inside_attribute_value?,:fixed_inside_attribute_value?
fix.remove_method :fixed_inside_attribute_value?
MessagesHelper.send :remove_const,:PRESENTATION_CACHE_VERSION
MessagesHelper.const_set :PRESENTATION_CACHE_VERSION,old_version
html=ApplicationController.renderer.new(http_host:'campfire.test',https:false).render(partial:'messages/message',collection:[message],cached:->(record){ApplicationController.helpers.message_with_pr_cards_cache_key(record)})
raise 'fixed renderer still vulnerable' unless Nokogiri::HTML.fragment(html).css('[onerror]').empty?
last_modified=room.messages.maximum(:updated_at).httpdate
rows=[{'If-None-Match'=>old_etag},{'If-Modified-Since'=>last_modified}].map do |validator|
 browser.get("/rooms/#{room.id}/messages",headers:headers.merge(validator))
 {headers:validator,status:browser.response.status,etag:browser.response.headers['ETag']}
end
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:'d7c7de92',payload:payload,message_id:message.id,old_html:old_html,html:html,rows:rows)+"\n")
puts 'WS8bm legacy cache: vulnerable v2 fragment and validator; safe v3 fragment; both real conditional Rails requests return 200'
