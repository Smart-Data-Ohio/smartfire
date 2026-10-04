# Full owner-picker JSON and SELECT growth at 10/100 eligible workspace agents.
require 'json'
require 'action_dispatch/testing/integration'
require 'active_support/testing/time_helpers'
extend ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
Rails.logger=ActiveSupport::Logger.new(File::NULL)
ActiveJob::Base.queue_adapter=:test
root=ENV.fetch('PARITY_WORK');conn=ActiveRecord::Base.connection
setup=JSON.parse(File.read(File.join(root,'vectors/ws12_work_remaining.json')))['rows'].find{|r|r['id']=='c229'}.fetch('setup')
labels=JSON.parse(File.read(File.join(root,'parity/.seed/default/labels.json')))
tables=%w[users rooms memberships messages action_text_rich_texts channel_threads thread_memberships work_thread_events work_thread_links github_pull_requests events event_attendances event_references thread_tags activity_items agent_grants agents agent_credentials sqlite_sequence]
saved=tables.flat_map{|t|columns=conn.columns(t).map(&:name);["DELETE FROM #{t}"]+conn.select_all("SELECT * FROM #{t}").map{|r|"INSERT INTO #{t}(#{columns.join(',')}) VALUES(#{columns.map{|c|conn.quote(r[c])}.join(',')})"}}
rows=[10,100].map do |size|
 conn.execute('PRAGMA foreign_keys=OFF');conn.execute('DELETE FROM message_search_index');saved.each{|q|conn.execute(q)};setup.each{|q|conn.execute(q)}
 size.times do |i|
  id=901896000+i
  conn.execute("INSERT INTO users(id,name,role,status,created_at,updated_at) VALUES(#{id},'Extra eligible #{i}',2,0,'2026-03-02 16:00:00','2026-03-02 16:00:00')")
  conn.execute("INSERT INTO agents(id,user_id,owner_id,kind,provider,description,status,created_at,updated_at) VALUES(#{id},#{id},127326141,'workspace','TestLab','Does the work','idle','2026-03-02 16:00:00','2026-03-02 16:00:00')")
  conn.execute("INSERT INTO memberships(room_id,user_id,involvement,created_at,updated_at) VALUES(486777696,#{id},'mentions','2026-03-02 16:00:00','2026-03-02 16:00:00')")
 end
 conn.execute('PRAGMA foreign_keys=ON');ActiveSupport::IsolatedExecutionState.clear
 browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test'
 headers={'Cookie'=>"session_token=#{labels.fetch('session_cookies.jz')}",'Accept'=>'application/json'}
 browser.get('/rooms/486777696/threads/91.json',headers:)
 reads=0
 ActiveSupport::Notifications.subscribed(->(_name,_start,_finish,_id,p){reads+=1 if p[:sql].lstrip.start_with?('SELECT') && p[:name]!='SCHEMA'},'sql.active_record'){browser.get('/rooms/486777696/threads/91.json',headers:)}
 raise "owner picker status #{browser.response.status}" unless browser.response.status==200
 options=JSON.parse(browser.response.body).fetch('thread').fetch('work_owner_options').select{|o|o['id']>=901896000 && o['id']<901896000+size}
 raise 'missing eligible owner profile' unless options.size==size && options.all?{|o|o['agent'] && o['provider']=='TestLab' && o['description']=='Does the work'}
 {size:,reads:,body:browser.response.body}
end
puts JSON.pretty_generate(rows:)
warn "WS12_OWNER_PROFILE_RAILS agents=10/100 SELECTs=#{rows.map{|r|r[:reads]}.join('/')}"
