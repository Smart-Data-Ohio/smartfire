# The complete activity_items_controller_test.rb:74 sequence on one persisted item.
require 'digest'
require 'action_dispatch/testing/integration'
require 'active_support/testing/time_helpers'
extend ActiveSupport::Testing::TimeHelpers
{
  'test/controllers/activity_items_controller_test.rb'=>'3cfcd2915ff8d6fd504c46534bdca089b31e21ab3b4d0b90617e49edfe4b337b',
  'app/controllers/activity_items_controller.rb'=>'ff508adbf44c7e203da09ad543015e97d93688f2fac082a9f8909a800eba1b08'
}.each { |file,hash| raise "source drift: #{file}" unless Digest::SHA256.file(Rails.root.join(file)).hexdigest == hash }
ApplicationController.allow_forgery_protection = false
labels = JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'parity/.seed/default/labels.json')))
ActivityItem.delete_all
item = ActivityItem.create!(id: 8400000000, user: User.find(127326141), source: Message.find(136976342), event_type: 'mention')
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! 'campfire.test'
headers = {'Cookie'=>"session_token=#{labels.fetch('session_cookies.david')}", 'Accept'=>'application/json', 'HTTP_USER_AGENT'=>'Mozilla/5.0 Chrome/140.0.0.0'}
steps = []
[
  ['handled','patch',"/activity/#{item.id}/handled.json",{state:'handled'}],
  ['handled count','get','/activity/unread_count.json',{}],
  ['unhandled','patch',"/activity/#{item.id}/handled.json",{state:'unhandled'}],
  ['unread','patch',"/activity/#{item.id}/read.json",{state:'unread'}]
].each_with_index do |(name,method,path,params),index|
  travel_to(Time.utc(2026,3,2,16)+index) do
    browser.public_send(method,path,params:,headers:)
    headers.delete('Cookie')
    item.reload
    steps << {name:,method:,path:,params:,status:browser.response.status,body:browser.response.body,
      headers:%w[content-type cache-control pragma location].to_h{|key|[key,browser.response.headers[key]]},
      state:{read_at:item.read_at&.iso8601(3),handled_at:item.handled_at&.iso8601(3),updated_at:item.updated_at.iso8601(3),read:item.read?,unread:item.unread?,handled:item.handled?},unread_count:ActivityItem.where(user_id: 127326141).unread.count}
  end
end
puts JSON.pretty_generate(reference:'activity_items_controller_test.rb:74',steps:)
warn 'WS12_HANDLED_SEQUENCE_RAILS 4 sequential HTTP responses and persisted states; 0 masks'
