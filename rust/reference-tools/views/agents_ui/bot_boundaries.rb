require 'action_dispatch/testing/integration'
ApplicationController.allow_forgery_protection = false
Rails.logger = ActiveSupport::Logger.new($stderr)
ActiveRecord::Base.logger = nil
Rails.application.env_config['action_dispatch.show_exceptions'] = :all
Rails.application.env_config['action_dispatch.show_detailed_exceptions'] = false
labels = JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'parity/.seed/default/labels.json')))
owner = User.find(labels.fetch('users.david'))
c = ActiveRecord::Base.connection
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! 'campfire.test'
headers = {'Cookie'=>"session_token=#{labels.fetch('session_cookies.david')}",'Accept'=>'text/html','HTTP_USER_AGENT'=>'Mozilla/5.0 Chrome/140.0.0.0'}
browser.post('/sudo',params:{password:'secret123456'},headers:)
headers.delete('Cookie')
rows=[]
errors={}
scenarios = [
 ['create_user_failure','create','users','INSERT',nil],
 ['create_webhook_failure','create','webhooks','INSERT',nil],
 ['create_agent_failure','create','agents','INSERT',nil],
 ['create_audit_failure','create','audit_logs','INSERT',"NEW.action='agent.create'"],
 ['update_user_failure','update','users','UPDATE',nil],
 ['update_agent_failure','update','agents','UPDATE',nil],
 ['update_webhook_audit_failure','update','audit_logs','INSERT',"NEW.action='agent.webhook_url.change'"],
 ['update_edit_audit_failure','update','audit_logs','INSERT',"NEW.action='agent.update'"],
 ['remove_user_failure','remove','users','UPDATE',nil],
 ['remove_audit_failure','remove','audit_logs','INSERT',"NEW.action='agent.suspend'"],
 ['legacy_agent_failure','legacy','agents','INSERT',nil],
 ['legacy_audit_failure','legacy','audit_logs','INSERT',"NEW.action='agent.create'"],
 ['legacy_missing_params','legacy_post',nil,nil,nil],
 *%w[daily_message_cap daily_board_post_cap daily_external_action_cap].map{|field| ["update_#{field}_overflow",'overflow',nil,nil,nil,field]}
]
scenarios.each do |name,operation,table,event,predicate,cap_field|
 bot = nil
 unless operation=='create'
  bot = User.create_bot!(name:,webhook_url:'https://old.example/hook')
  bot.create_agent!(kind: :workspace,owner:) unless operation.start_with?('legacy')
  bot.agent.update!(provider:'Before input',daily_message_cap:7,daily_board_post_cap:7,daily_external_action_cap:7) if operation=='overflow'
 end
 before = AuditLog.maximum(:id) || 0
 if table
  c.execute("CREATE TRIGGER reject_bot_ui BEFORE #{event} ON #{table} #{predicate && "WHEN #{predicate}"} BEGIN SELECT RAISE(ABORT,'fixture bot boundary rejection'); END;")
 end
 path = operation=='create' ? '/account/bots' : "/account/bots/#{bot.id}"
 case operation
 when 'create' then browser.post(path,params:{user:{name:,webhook_url:'https://new.example/hook'}},headers:)
 when 'update' then browser.patch(path,params:{user:{name:"#{name} changed",webhook_url:'https://new.example/hook'},agent:{provider:'Changed'}},headers:)
 when 'overflow' then browser.patch(path,params:JSON.generate(user:{name:'Submitted bot',icon_name:'openai'},agent:{provider:'Submitted provider',cap_field=>'9223372036854775808'}),headers:headers.merge('Content-Type'=>'application/json'))
 when 'remove' then browser.delete(path,headers:)
 when 'legacy' then browser.get("#{path}/credentials",headers:)
 when 'legacy_post' then browser.post("#{path}/credentials",headers:)
 end
 status=browser.response.status
 errors[status.to_s]={body:browser.response.body,content_type:browser.response.headers['Content-Type']} if status >= 400
 c.execute('DROP TRIGGER reject_bot_ui') if table
 bot = User.find_by(name:) if operation=='create'
 agent=bot&.reload&.agent
 rows << {name:,operation:,table:,event:,predicate:,cap_field:,status:,bot:bot && {name:bot.name,icon_name:bot.icon_name,status:bot.status,webhook_url:bot.webhook_url},agent:agent && {kind:agent.kind,owner_id:agent.owner_id,provider:agent.provider,daily_message_cap:agent.daily_message_cap,daily_board_post_cap:agent.daily_board_post_cap,daily_external_action_cap:agent.daily_external_action_cap},audits:AuditLog.where('id > ?',before).order(:id).pluck(:action)}
end
puts JSON.pretty_generate(reference:'d7c7de92',rows:,errors:)
warn "Rails bot mutation boundaries: #{rows.size} HTTP cases"
