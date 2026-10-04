require 'json'
require 'active_support/testing/time_helpers'
extend ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
ActiveJob::Base.queue_adapter=:test
Rails.logger=ActiveSupport::Logger.new(File::NULL)
setup=JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'vectors/ws12_generic_recorder.json')))['setup']
rows=[10,100].map do |size|
 ActiveRecord::Base.connection.execute('PRAGMA foreign_keys=OFF');setup.each{|table,rows|ActiveRecord::Base.connection.execute("DELETE FROM #{table}");rows.each{|row|ActiveRecord::Base.connection.execute("INSERT INTO #{table}(#{row.keys.join(',')}) VALUES(#{row.values.map{|v|ActiveRecord::Base.connection.quote(v)}.join(',')})")}};ActiveRecord::Base.connection.execute('PRAGMA foreign_keys=ON')
 notice=AgentBudgetNotice.find(901840003);notice.agent.update_columns(owner_id:nil)
 recipients=size.times.map{|i|User.create!(id:901870000+i,name:'Budget administrator',email_address:"ws12-budget-#{i}@example.com",role: :administrator,status: :active)}
 reads=0
 records=ActiveSupport::Notifications.subscribed(->(_name,_start,_finish,_id,payload){reads+=1 if payload[:sql].lstrip.start_with?('SELECT') && payload[:name]!='SCHEMA'},'sql.active_record') do
  recorder=ActivityItems::Recorder.new(notice)
  recipients.map{|recipient|recorder.record!(recipient:,event_type:'agent_budget_exceeded')}
 end
 {size:,reads:,items:records.map{|r|[r.user_id,r.event_type,r.unread?]}}
end
puts JSON.pretty_generate(rows:)
warn "WS12_BUDGET_NOTICE_RAILS recipients=10/100 SELECTs=#{rows.map{|r|r[:reads]}.join('/')}"
