# Full HTTP responses and committed rules/audits. No response masks.
require 'json'
require 'digest'
require File.join(ENV.fetch('PARITY_WORK'), 'reference-tools/icon_cache_inputs')
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
cases=[
 ['show','get',base,{},david,[]], ['show-rules','get',base,{},david,[rule,tag]],
 ['show-creator','get',base,{},jason,["UPDATE rooms SET creator_id=#{jason} WHERE id=#{board}","UPDATE users SET role=0 WHERE id=#{jason}"]],
 ['show-member','get',base,{},jason,["UPDATE users SET role=0 WHERE id=#{jason}"]], ['show-nonmember','get',base,{},kevin,[]],
 ['show-admin-nonmember','get',base,{},david,["DELETE FROM memberships WHERE room_id=#{board} AND user_id=#{david}"]],
 ['show-channel','get','/rooms/boards/486777696/automations',{},david,[]],
 ['show-deleted','get',base,{},david,["UPDATE rooms SET deleted_at='2026-03-02 16:00:00' WHERE id=#{board}"]],
 ['show-missing','get','/rooms/boards/0/automations',{},david,[]],
 ['create','patch',base+'/sla_rules',{sla_rules:valid},david,[]],
 ['creator-write','patch',base+'/sla_rules',{sla_rules:valid},jason,["UPDATE rooms SET creator_id=#{jason} WHERE id=#{board}","UPDATE users SET role=0 WHERE id=#{jason}"]],
 ['unchanged','patch',base+'/sla_rules',{sla_rules:{planned:valid[:planned]}},david,[rule]],
 ['update','patch',base+'/sla_rules',{sla_rules:{planned:{nudge_after_minutes:'90',escalate_after_minutes:'360'}}},david,[rule]],
 ['clear','patch',base+'/sla_rules',{sla_rules:{planned:{nudge_after_minutes:' ',escalate_after_minutes:''}}},david,[rule]],
 ['omitted','patch',base+'/sla_rules',{},david,[rule]],
 ['empty','patch',base+'/sla_rules',{sla_rules:{}},david,[rule]],
 ['unknown-only','patch',base+'/sla_rules',{sla_rules:{unknown:{nudge_after_minutes:'60'}}},david,[rule]],
 ['trim','patch',base+'/sla_rules',{sla_rules:{planned:{nudge_after_minutes:' 60 ',escalate_after_minutes:' 240 '}}},david,[]],
 ['invalid','patch',base+'/sla_rules',{sla_rules:{planned:{nudge_after_minutes:'0',escalate_after_minutes:'-1'},blocked:{nudge_after_minutes:'90',escalate_after_minutes:'30'}}},david,[rule]],
 ['validate-all-before-write','patch',base+'/sla_rules',{sla_rules:valid.merge(done:{nudge_after_minutes:'60',escalate_after_minutes:'240'})},david,[]],
 ['half-blank','patch',base+'/sla_rules',{sla_rules:{planned:{nudge_after_minutes:'60',escalate_after_minutes:''}}},david,[rule]],
 ['large-nudge','patch',base+'/sla_rules',{sla_rules:{planned:{nudge_after_minutes:'9'*400,escalate_after_minutes:'240'}}},david,[]],
 ['large-negative-nudge','patch',base+'/sla_rules',{sla_rules:{planned:{nudge_after_minutes:'-'+'9'*400,escalate_after_minutes:'240'}}},david,[]],
 ['large-escalation','patch',base+'/sla_rules',{sla_rules:{planned:{nudge_after_minutes:'60',escalate_after_minutes:'9'*400}}},david,[]],
 ['underscored-nudge','patch',base+'/sla_rules',{sla_rules:{planned:{nudge_after_minutes:'4_2',escalate_after_minutes:'10'}}},david,[]],
 ['write-member','patch',base+'/sla_rules',{sla_rules:valid},jason,["UPDATE users SET role=0 WHERE id=#{jason}"]],
 ['write-nonmember','patch',base+'/sla_rules',{sla_rules:valid},kevin,[]],
 ['tag-create','post',base+'/tag_assignments',{tag:' BUG ',assignee_id:jason},david,[]],
 ['tag-duplicate','post',base+'/tag_assignments',{tag:'BUG',assignee_id:jason},david,[tag]],
 ['tag-duplicate-id-zero','post',base+'/tag_assignments',{tag:'BUG',assignee_id:jason},david,[tag.sub('970000002','0')]],
 ['tag-invalid','post',base+'/tag_assignments',{tag:'bad tag',assignee_id:kevin},david,[]],
 ['tag-agent','post',base+'/tag_assignments',{tag:'agent',assignee_id:bot},david,[]],
 ['tag-remove','delete',base+'/tag_assignments/970000002',{},david,[tag]],
 ['tag-remove-missing','delete',base+'/tag_assignments/0',{},david,[tag]],
 ['tag-remove-id-zero','delete',base+'/tag_assignments/0',{},david,[tag.sub('970000002','0')]],
 ['tag-remove-invalid-id','delete',base+'/tag_assignments/bad-id',{},david,[tag.sub('970000002','0')]],
 ['tag-cross-board','delete',base+'/tag_assignments/970000002',{},david,[tag.sub(board.to_s,'486777696')]],
 ['tag-forbidden','post',base+'/tag_assignments',{tag:'bug',assignee_id:jason},jason,["UPDATE users SET role=0 WHERE id=#{jason}"]],
 ['tag-inactive','post',base+'/tag_assignments',{tag:'bug',assignee_id:jason},david,["UPDATE users SET status=1 WHERE id=#{jason}"]]
]
[10,100].each do |size|
  extra=[]
  size.times do |i|
    id=980100000+i
    extra << "INSERT INTO users(id,name,role,status,created_at,updated_at) VALUES(#{id},'Choice #{i}',0,0,'2026-03-02 16:00:00','2026-03-02 16:00:00')"
    extra << "INSERT INTO memberships(room_id,user_id,created_at,updated_at) VALUES(#{board},#{id},'2026-03-02 16:00:00','2026-03-02 16:00:00')"
    extra << "INSERT INTO board_tag_assignments(room_id,tag,assignee_id,created_by_id,created_at,updated_at) VALUES(#{board},'query-#{i}',#{id},#{david},'2026-03-02 16:00:00','2026-03-02 16:00:00')"
  end
  cases << ["choices-#{size}",'get',base,{},david,extra]
end
# The inactive-assignee validation fixture exercises an expired icon cache.
# Raw old/new SQL identifies its historical extra read as Icons' version stamp.
ICON_CACHE_INPUTS = { 'tag-inactive' => 'expired' }.freeze
rows=[]
cases.each_with_index do |(name,method,path,input,user_id,extra), index|
  ActiveRecord::Base.transaction do
    setup=common+extra; setup.each{|sql|conn.execute(sql)}
    user=User.find(user_id)
    request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
    request.cookie_jar.signed[:session_token]=user.sessions.where.not(two_factor_verified_at:nil).first!.token
    browser=ActionDispatch::Integration::Session.new(Rails.application); browser.host! 'campfire.test'
    icon_cache_state = ICON_CACHE_INPUTS.fetch(name, index.zero? ? 'initial' : 'warm')
    icon_cache_started, icon_cache_state = OracleIconCacheInputs.prepare(icon_cache_state)
    selects=[]
    subscriber=ActiveSupport::Notifications.subscribe('sql.active_record') do |*args|
      event=args.last
      selects << event[:sql] if !event[:cached] && event[:name]!='SCHEMA' && event[:sql].match?(/\A\s*SELECT/i)
    end
    browser.public_send(method,path,params: method=='get' ? nil : input.to_json,headers:{'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}",'Content-Type'=>'application/json','User-Agent'=>'Mozilla'})
    ActiveSupport::Notifications.unsubscribe(subscriber)
    icon_cache_elapsed = OracleIconCacheInputs.verify!(icon_cache_started, name:name)
    if ENV["PARITY_QUERY_TRACE"]
      File.open(ENV.fetch("PARITY_QUERY_TRACE"), "a") { |file| file.puts(JSON.generate(name:name,icon_cache:icon_cache_state,icon_cache_elapsed:icon_cache_elapsed,sql:selects)) }
    end
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
