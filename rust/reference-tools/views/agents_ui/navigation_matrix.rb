# Whole controller responses on the pinned seed, with only the approved status-popup inputs.
require 'action_dispatch/testing/integration'
load File.join(ENV.fetch('PARITY_WORK'),'reference-tools/users/post_pin.rb')
ApplicationController.allow_forgery_protection = true
ApplicationController.prepend(Module.new do
  def verify_authenticity_token = nil
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end)
Rails.application.env_config['action_dispatch.content_security_policy_nonce_generator'] = ->(_request) { 'NONCE' }
ActiveRecord::Base.logger=nil
Rails.logger=ActiveSupport::Logger.new($stderr)
ActionView::Base.logger=Rails.logger
ActionController::Base.logger=Rails.logger
labels=JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'parity/.seed/default/labels.json')))
connection=ActiveRecord::Base.connection
cases=[]
add=->(name,viewer,path,sql=[],frame=true,kind=nil,size=nil){cases << {name:,viewer:,path:,sql:,frame:,kind:,size:}}
%w[david kevin jz].each do |viewer|
  ['/users/me/sidebar','/users/me/profile',"/users/#{labels['users.david']}","/users/#{labels['users.kevin']}","/users/#{labels['users.bender']}"] .each do |path|
    [true,false].each { |frame| add.call("#{viewer} #{path} #{frame ? 'frame' : 'page'}",viewer,path,[],frame) }
  end
end
uid=labels.fetch('users.david')
variants={
 'markup'=>["UPDATE users SET name='<b>Human & name</b>',bio='<script>bio()</script>',custom_status_emoji='<&>',custom_status_text='<img src=x>',ooo_note='<b>Back & soon</b>' WHERE id=#{uid}"],
 'appearance'=>["UPDATE users SET theme='dark',text_size='x-large',time_zone='Asia/Tokyo',voice_mode='push_to_talk',push_to_talk_key='F8' WHERE id=#{uid}"],
 'passwordless'=>["UPDATE users SET password_digest=NULL WHERE id=#{uid}"],
 'status'=>["UPDATE users SET presence_setting='appear_offline',custom_status_emoji='🌴',custom_status_text='Back tomorrow',custom_status_expires_at='2026-03-03 16:00:00',ooo_until='2026-03-04 16:00:00',ooo_note='Back & soon',dnd_enabled=1,dnd_until='2026-03-03 16:00:00' WHERE id=#{uid}"],
 'expired_status'=>["UPDATE users SET custom_status_emoji='🌴',custom_status_text='Old status',custom_status_expires_at='2026-03-01 16:00:00',ooo_until='2026-03-01 16:00:00',ooo_note='Old OOO',dnd_enabled=1,dnd_until='2026-03-01 16:00:00' WHERE id=#{uid}"],
 'preferences'=>["UPDATE users SET inbox_preferences='#{JSON.generate(agent_approvals:false,agent_work:false,github_review_requests:false,event_reminders:false,huddle_invitations:false)}',quiet_hours_enabled=1,quiet_hours_start_minute=1320,quiet_hours_end_minute=420,meeting_dnd_enabled=1,ooo_notify_enabled=0 WHERE id=#{uid}"],
 'no_memberships'=>["DELETE FROM memberships WHERE user_id=#{uid}","DELETE FROM room_categories WHERE user_id=#{uid}"],
 'human_deactivated'=>["UPDATE users SET status=1 WHERE id=#{labels.fetch('users.kevin')}"],
 'human_banned'=>["UPDATE users SET status=2 WHERE id=#{labels.fetch('users.kevin')}"],
 'restricted'=>["UPDATE accounts SET settings='#{JSON.generate(restrict_room_creation_to_administrators:true)}'"]
}
variants.each do |name,sql|
  target=name.start_with?('human_') ? labels.fetch('users.kevin') : uid
 ['/users/me/sidebar','/users/me/profile',"/users/#{target}"].each { |path| [true,false].each { |frame| add.call("#{name} #{path} #{frame ? 'frame' : 'page'}",'david',path,sql,frame) } }
end
add.call('restricted member sidebar','kevin','/users/me/sidebar',variants.fetch('restricted'))
# Real models construct the complete organized sidebar, then raw snapshots reproduce only the fixture in Rust.
snapshot=->(tables){tables.flat_map { |table| ["DELETE FROM #{table}"]+connection.select_all("SELECT * FROM #{table}").map { |row| "INSERT INTO #{table} (#{row.keys.map { |k| connection.quote_column_name(k) }.join(',')}) VALUES (#{row.values.map { |v| connection.quote(v) }.join(',')})" } }}
[2,12].each do |size|
 ActiveRecord::Base.transaction do
  user=User.find(uid)
  Current.user=user
  collapsed=user.room_categories.create!(id:9100000000+size,name:'Collapsed <&>',position:0,collapsed:true)
  user.room_categories.create!(id:9100000100+size,name:'Empty',position:1)
  size.times do |i|
   room=Rooms::Direct.find_or_create_for([user,User.find(labels.fetch('users.kevin'))])
   # Rails merges equivalent directs; vary a third member using actual user and room producers.
   if i>0
    peer=User.create!(id:9200000000+size*100+i,name:"Peer #{i}",email_address:"peer#{size}-#{i}@example.test",password_digest:user.password_digest)
    room=Rooms::Direct.find_or_create_for([user,peer])
   end
   room.update_columns(updated_at:Time.current+i)
  end
  user.memberships.joins(:room).where.not(rooms:{type:'Rooms::Direct'}).order(:id).each.with_index do |m,i|
   m.update!(favorite_position: i<2 ? 2-i : nil,room_category_id:i==2 ? collapsed.id : nil,involvement:i.even? ? 'muted' : 'everything',unread_at:i.odd? ? Time.current : nil)
  end
  sql=snapshot.call(%w[users rooms memberships room_categories])
  ['/users/me/sidebar','/users/me/profile'].each { |path| add.call("organized #{size} #{path}",'david',path,sql,true,path.end_with?('sidebar') ? 'sidebar' : 'profile',size) }
  raise ActiveRecord::Rollback
 end
end
cases.each do |entry|
 ActiveRecord::Base.transaction do
  connection.execute('PRAGMA defer_foreign_keys=ON')
  entry[:sql].each { |sql| connection.execute(sql) }
  Current.reset; Rails.cache.clear
  browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test'
  headers={'Cookie'=>"session_token=#{labels.fetch("session_cookies.#{entry[:viewer]}")}",'Accept'=>'text/html','HTTP_USER_AGENT'=>'Mozilla/5.0 Chrome/140.0.0.0'}
  headers['Turbo-Frame']='ui_matrix' if entry[:frame]
  counts=Hash.new(0)
  callback=->(*args){sql=args.last[:sql]; table=sql[/\bFROM\s+"?([a-z_]+)/i,1];counts[table]+=1 if table && sql.lstrip.start_with?('SELECT') && !args.last[:cached]}
  ActiveRecord::Base.uncached do
   ActiveSupport::Notifications.subscribed(callback,'sql.active_record'){browser.get(entry[:path],headers:)}
  end
  entry[:result]={status:browser.response.status,body:browser.response.body,counts:entry[:kind] ? counts.sort.to_h : nil}
  raise "unexpected #{entry[:name]} status #{browser.response.status}" unless browser.response.status==200
  raise ActiveRecord::Rollback
 end
 ActiveSupport::ExecutionContext.clear
end
puts JSON.pretty_generate(reference:'d7c7de92',status_reference:'2e20b24c',cases:)
warn "Rails navigation matrix: #{cases.size} full responses; sidebar/profile growth at 2/12 rows"
