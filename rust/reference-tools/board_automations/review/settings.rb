# Full HTTP responses and committed rules/audits. No response masks.
require 'json'
require 'digest'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'reference-tools/board_automations/settings-source-hashes.json'))).each do |file,hash|
  raise "source drift: #{file}" unless Digest::SHA256.file(Rails.root.join(file)).hexdigest==hash
end
Rails.application.env_config['action_dispatch.show_exceptions']=:all
Rails.application.env_config['action_dispatch.content_security_policy_nonce_generator']=->(_) {'NONCE'}
ActiveJob::Base.queue_adapter=:test
module AutomationGoldenTokens
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
ApplicationController.prepend(AutomationGoldenTokens)
Rooms::Boards::AutomationsController.skip_before_action :verify_authenticity_token
Rails.logger=ActiveSupport::Logger.new(File::NULL)
ApplicationController.logger=Rails.logger
Rails.application.env_config['action_dispatch.logger']=Rails.logger
conn=ActiveRecord::Base.connection
board=699448332; david=127326141; jason=149087659; kevin=712064548; bot=394959859
base="/rooms/boards/#{board}/automations"
common=["DELETE FROM board_sla_rules", "DELETE FROM board_tag_assignments", "DELETE FROM audit_logs"]
rule="INSERT INTO board_sla_rules(id,room_id,work_status,nudge_after_minutes,escalate_after_minutes,created_at,updated_at) VALUES(970000001,#{board},'planned',60,240,'2026-03-02 16:00:00','2026-03-02 16:00:00')"
tag="INSERT INTO board_tag_assignments(id,room_id,tag,assignee_id,created_by_id,created_at,updated_at) VALUES(970000002,#{board},'bug',#{jason},#{david},'2026-03-02 16:00:00','2026-03-02 16:00:00')"
valid={planned:{nudge_after_minutes:'60',escalate_after_minutes:'240'},in_progress:{nudge_after_minutes:'30',escalate_after_minutes:'90'},blocked:{nudge_after_minutes:'1',escalate_after_minutes:'43200'}}
cases=JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'reference-tools/board_automations/review/settings-cases.json'))).map{|r|r.values_at('name','method','path','input','user_id','extra')}
[10,100].each do |size|
  extra=[]
  size.times do |i|
    id=980100000+i
    extra << "INSERT INTO users(id,name,role,status,created_at,updated_at) VALUES(#{id},'Choice #{i}',#{i%4==0 ? 2 : 0},#{i%4==1 ? 1 : 0},'2026-03-02 16:00:00','2026-03-02 16:00:00')"
    extra << "INSERT INTO memberships(room_id,user_id,created_at,updated_at) VALUES(#{board},#{id},'2026-03-02 16:00:00','2026-03-02 16:00:00')"
    extra << "INSERT INTO board_tag_assignments(room_id,tag,assignee_id,created_by_id,created_at,updated_at) VALUES(#{board},'query-#{i}',#{id},#{david},'2026-03-02 16:00:00','2026-03-02 16:00:00')"
  end
  extra << "INSERT INTO agents(user_id,owner_id,created_at,updated_at) SELECT id,127326141,'2026-03-02 16:00:00','2026-03-02 16:00:00' FROM users WHERE id>=980100000 AND role=2"
  cases << ["choices-#{size}",'get',base,{},david,extra]
end
rows=[]
cases.each do |name,method,path,input,user_id,extra|
  ActiveRecord::Base.transaction do
    setup=common+extra; setup.each{|sql|conn.execute(sql)}
    user=User.find(user_id)
    request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
    request.cookie_jar.signed[:session_token]=user.sessions.where.not(two_factor_verified_at:nil).first!.token
    browser=ActionDispatch::Integration::Session.new(Rails.application); browser.host! 'campfire.test'
    selects=[]
    subscriber=ActiveSupport::Notifications.subscribe('sql.active_record') do |*args|
      event=args.last
      selects << event[:sql] if !event[:cached] && event[:name]!='SCHEMA' && event[:sql].match?(/\A\s*SELECT/i)
    end
    browser.public_send(method,path,params: method=='get' ? nil : input.to_json,headers:{'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}",'Content-Type'=>'application/json','User-Agent'=>'Mozilla'})
    ActiveSupport::Notifications.unsubscribe(subscriber)
    rules=BoardSlaRule.where(room_id:board).order(:work_status).pluck(:work_status,:nudge_after_minutes,:escalate_after_minutes)
    tags=BoardTagAssignment.where(room_id:board).order(:tag).pluck(:tag,:assignee_id,:created_by_id)
    audits=AuditLog.where(action:'board.automation.change').order(:id).map{|a|[a.actor_id,a.target_type,a.target_id,a.details]}
    rows << {name:,method:,path:,user_id:,input:,setup:,reads:selects.size,status:browser.response.status,body:browser.response.body,location:browser.response.headers['location'],content_type:browser.response.headers['content-type'],facts:{rules:,tags:,audits:}}
    ActiveSupport::IsolatedExecutionState.clear
    raise ActiveRecord::Rollback
  end
end
puts JSON.pretty_generate(rows:)
warn "Rails board automation settings oracle: #{rows.size} complete responses and rule/tag/audit facts; 0 masks"
