require 'action_dispatch/testing/integration'
labels=JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'parity/.seed/default/labels.json')))
ApplicationController.allow_forgery_protection=false
ActiveRecord::Base.logger=nil
browser=ActionDispatch::Integration::Session.new(Rails.application)
browser.host! 'campfire.test'
headers={'Cookie'=>"session_token=#{labels.fetch('session_cookies.david')}",'HTTP_USER_AGENT'=>'Mozilla/5.0 Chrome/140.0.0.0','Accept'=>'text/html'}
browser.post('/sudo',params:{password:'secret123456'},headers:)
headers.delete('Cookie')
bot=User.find(labels.fetch('users.bender'))
module ConcurrentEditProbe
 class << self; attr_accessor :changes; end
end
User.prepend(Module.new do
 def update_bot(...)
  result=super
  Agent.find(agent.id).update!(ConcurrentEditProbe.changes) if result && name=='Review interleaved'
  result
 end
end)
fields=%w[provider runtime description daily_message_cap daily_board_post_cap daily_external_action_cap]
initial=fields.to_h {|f| [f,f.start_with?('daily_') ? 7 : 'Original']}
cases=fields.map {|f| {name:"clean #{f}",submitted:{f=>f.start_with?('daily_') ? '7' : 'Original'},concurrent:{f=>f.start_with?('daily_') ? 99 : 'Concurrent B'}}}
cases += [
 {name:'dirty provider',submitted:{provider:'Request A'},concurrent:{provider:'Concurrent B'}},
 {name:'dirty null cap',submitted:{daily_message_cap:nil},concurrent:{daily_message_cap:99}},
 {name:'dirty runtime with clean provider',submitted:{runtime:'Request A',provider:'Original'},concurrent:{provider:'Concurrent B'}}
]
result={reference:'d7c7de92',initial:,cases:[]}
cases.each do |case_data|
 bot.reload.update!(name:'Bender Bot')
 bot.agent.reload.update!(initial)
 ConcurrentEditProbe.changes=case_data[:concurrent]
 before=AuditLog.maximum(:id) || 0
 browser.patch("/account/bots/#{bot.id}",params:JSON.generate(user:{name:'Review interleaved'},agent:case_data[:submitted]),headers:headers.merge('Content-Type'=>'application/json'))
 result[:cases] << case_data.merge(status:browser.response.status,stored:bot.agent.reload.attributes.slice(*fields),audits:AuditLog.where('id > ?',before).where(action:'agent.update').pluck(:details))
end
puts JSON.pretty_generate(result)
warn "Rails concurrent edits: #{result[:cases].size} real independent-save interleavings"
