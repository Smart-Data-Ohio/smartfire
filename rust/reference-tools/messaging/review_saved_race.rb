require 'json'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
user=User.find(127326141);Current.user=user
room=Room.find(699448326)
message=room.messages.create!(creator:user,markdown_source:'Partial updates preserve reminder races',client_message_id:'review-race')
item=SavedItem.create!(user:,message:,remind_at:Time.utc(2026,3,2,16,1))
stale=SavedItem.find(item.id)
travel_to Time.utc(2026,3,2,16,2)
SavedItem::ReminderDispatcher.dispatch_due!(now:Time.current)
claimed=item.reload.reminded_at
stale.update!(status:'done')
after_status=item.reload.attributes.slice('status','remind_at','reminded_at')
controller=SavedItemsController.new
controller.request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.url_scheme'=>'http','rack.input'=>StringIO.new))
controller.response=ActionDispatch::Response.new
after_status_response=JSON.parse(ActiveSupport::JSON.encode(controller.send(:saved_item_payload,stale)))
travel_to Time.utc(2026,3,2,16)
message=room.messages.create!(creator:user,markdown_source:'Concurrent reschedule payload',client_message_id:'review-reschedule-response')
item=SavedItem.create!(user:,message:,remind_at:Time.utc(2026,3,2,16,1))
travel_to Time.utc(2026,3,2,16,2)
stale=SavedItem.find(item.id)
item.update!(remind_at:Time.utc(2026,3,2,18))
stale.update!(status:'done')
after_reschedule=item.reload.attributes.slice('status','remind_at','reminded_at')
after_reschedule_response=JSON.parse(ActiveSupport::JSON.encode(controller.send(:saved_item_payload,stale)))
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],after_status_response:,after_reschedule_response:,claimed:claimed.utc.iso8601(6),after_status:after_status.transform_values{|v|v.is_a?(Time) ? v.utc.iso8601(6) : v},after_reschedule:after_reschedule.transform_values{|v|v.is_a?(Time) ? v.utc.iso8601(6) : v})+"\n")
puts "WS8bm2 Rails saved race: claim preserved=#{after_status['reminded_at']==claimed}; reschedule preserved=#{after_reschedule['remind_at']==Time.utc(2026,3,2,18)}"
