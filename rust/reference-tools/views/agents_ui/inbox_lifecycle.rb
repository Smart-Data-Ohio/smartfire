# The deferred reminder, recurrence and deleted-source UI cases through real producers.
require 'action_dispatch/testing/integration'
require 'active_support/testing/time_helpers'
extend ActiveSupport::Testing::TimeHelpers
ApplicationController.allow_forgery_protection=false
ActiveRecord::Base.logger=nil
Rails.logger=ActiveSupport::Logger.new($stderr)
labels=JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'parity/.seed/default/labels.json')))
user=User.find(labels.fetch('users.david'))
room=Room.find(labels.fetch('rooms.watercooler'))
message=room.root_messages.where.not(creator_id:user.id).order(:id).first!
SavedItem.delete_all; ActivityItem.delete_all
browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test'
headers={'Cookie'=>"session_token=#{labels.fetch('session_cookies.david')}",'HTTP_USER_AGENT'=>'Mozilla/5.0 Chrome/140.0.0.0','Accept'=>'application/json'}
steps=[]
request=->(name,method,path,params={},accept='application/json') {
 browser.public_send(method,path,params:,headers:headers.merge('Accept'=>accept))
 steps << {name:,method:,path:,params:,accept:,status:browser.response.status,body:browser.response.body,headers:%w[content-type cache-control pragma location].to_h{|key|[key,browser.response.headers[key]]}}
 headers.delete('Cookie')
}
request.call('save reminder','post','/saved.json',{message_id:message.id,saved_item:{remind_at:(Time.current+60).iso8601}})
saved=SavedItem.find_by!(user:,message:)
travel_to(Time.current+60)
SavedItem::ReminderDispatcher.dispatch_due!
item=ActivityItem.find_by!(user:,source:saved)
steps << {name:'dispatch first reminder',operation:'dispatch',advance:60,saved_id:saved.id,item_id:item.id}
request.call('first reminder inbox','get','/activity.json?type=reminders')
request.call('handle reminder','patch',"/activity/#{item.id}/handled.json")
request.call('handled count','get','/activity/unread_count.json')
request.call('rearm reminder','post','/saved.json',{message_id:message.id,saved_item:{remind_at:(Time.current+60).iso8601}})
travel_to(Time.current+60)
 SavedItem::ReminderDispatcher.dispatch_due!
renewed=ActivityItem.find_by!(user:,source:saved)
raise 'recurrence duplicated the inbox row' unless renewed.id==item.id
steps << {name:'dispatch recurrent reminder',operation:'dispatch',advance:60,saved_id:saved.id,item_id:item.id}
request.call('recurrent reminder inbox','get','/activity.json?type=reminders')
request.call('recurrent reminder count','get','/activity/unread_count.json')
request.call('delete source','delete',"/rooms/#{room.id}/messages/#{message.id}",{},'text/vnd.turbo-stream.html')
request.call('deleted source inbox','get','/activity.json?type=reminders')
request.call('deleted source count','get','/activity/unread_count.json')
begin
 request.call('deleted source open','post',"/activity/#{item.id}/open.json")
rescue ActiveRecord::RecordNotFound
 steps << {name:'deleted source open',method:'post',path:"/activity/#{item.id}/open.json",params:{},accept:'application/json',status:404,body:nil}
end
travel_back
puts JSON.pretty_generate(reference:'d7c7de92',message_id:message.id,steps:)
warn "Rails inbox lifecycle: #{steps.count{|s|s[:method]}} HTTP responses; 2 real reminder dispatches; reminder, recurrence and deleted-source"
