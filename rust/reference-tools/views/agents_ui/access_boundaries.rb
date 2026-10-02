# Independent Rails saves and audit inserts at the bot credential/grant HTTP boundary.
require 'action_dispatch/testing/integration'
ApplicationController.allow_forgery_protection = false
Rails.logger = ActiveSupport::Logger.new($stderr)
labels = JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'), 'parity/.seed/default/labels.json')))
bot = User.find(labels.fetch('users.bender'))
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! 'campfire.test'
headers = {'Cookie'=>"session_token=#{labels.fetch('session_cookies.david')}", 'HTTP_USER_AGENT'=>'Mozilla/5.0 Chrome/140.0.0.0'}
browser.post('/sudo',params:{password:'secret123456'},headers:)
headers.delete('Cookie')
connection = ActiveRecord::Base.connection
connection.execute("CREATE TRIGGER reject_access_audit BEFORE INSERT ON audit_logs WHEN NEW.action IN ('agent.credential.create','agent.credential.revoke','agent.grant.create','agent.grant.revoke') BEGIN SELECT RAISE(ABORT,'fixture access audit rejection'); END;")
request = ->(method,path,params={}) do
  begin
    browser.public_send(method,path,params:,headers:)
    raise "expected 500, got #{browser.response.status}" unless browser.response.status == 500
  rescue ActiveRecord::StatementInvalid => e
    raise unless e.message.include?('fixture access audit rejection')
  end
end
request.call(:post,"/account/bots/#{bot.id}/credentials",{agent_credential:{name:'Fault credential'}})
credential = bot.agent.agent_credentials.find_by!(name:'Fault credential')
request.call(:delete,"/account/bots/#{bot.id}/credentials/#{credential.id}")
raise 'credential revoke rolled back' unless credential.reload.revoked?
browser.delete("/account/bots/#{bot.id}/credentials/#{credential.id}",headers:)
raise 'credential re-revoke was not idempotent' unless browser.response.status==302
room = Room.find(201306877)
bot.agent.agent_grants.where(capability:'external_action',room:).delete_all
request.call(:post,"/account/bots/#{bot.id}/grants",{agent_grant:{capability:'external_action',room_id:room.id}})
grant = bot.agent.agent_grants.active.find_by!(capability:'external_action',room:)
request.call(:delete,"/account/bots/#{bot.id}/grants/#{grant.id}")
raise 'grant revoke rolled back' unless grant.reload.revoked?
browser.delete("/account/bots/#{bot.id}/grants/#{grant.id}",headers:)
raise 'grant re-revoke was not idempotent' unless browser.response.status==302
raise 'failed audit persisted' if AuditLog.where(action:%w[agent.credential.create agent.credential.revoke agent.grant.create agent.grant.revoke]).exists?
connection.execute('DROP TRIGGER reject_access_audit')
# A genuine uniqueness constraint inside the explicit grant transaction reaches its rescue.
connection.execute("CREATE TRIGGER collide_access_grant BEFORE INSERT ON agent_grants WHEN NEW.capability='external_action' AND NEW.room_id=201306877 BEGIN INSERT INTO agent_grants(agent_id,capability,room_id,granted_by_id,created_at,updated_at) VALUES(NEW.agent_id,NEW.capability,NEW.room_id,NEW.granted_by_id,NEW.created_at,NEW.updated_at); END;")
browser.post("/account/bots/#{bot.id}/grants",params:{agent_grant:{capability:'external_action',room_id:room.id}},headers:)
raise "unique rescue failed #{browser.response.status}" unless browser.response.status==302
raise 'unique failure committed the trigger row' if bot.agent.agent_grants.active.where(capability:'external_action',room:).exists?
puts 'Rails credential/grant boundaries: 4 writes survive rejected audits; 2 re-revokes are idempotent; 1 uniqueness rescue rolls back the grant transaction; 7 requests passed'
# A rejected owner save is distinct from a rejected later audit.
connection.execute('DROP TRIGGER collide_access_grant')
connection.execute("CREATE TRIGGER reject_access_credential BEFORE INSERT ON agent_credentials BEGIN SELECT RAISE(FAIL,'fixture access write rejection'); END;")
connection.execute("CREATE TRIGGER reject_access_grant BEFORE INSERT ON agent_grants BEGIN SELECT RAISE(FAIL,'fixture access write rejection'); END;")
[['credentials', {agent_credential: {name: 'Rolled back'}}], ['grants', {agent_grant: {capability: 'react'}}]].each do |area, params|
  begin
    browser.post("/account/bots/#{bot.id}/#{area}", params:, headers:)
    raise "owner write expected 500, got #{browser.response.status}" unless browser.response.status == 500
  rescue ActiveRecord::StatementInvalid => e
    raise unless e.message.include?('fixture access write rejection')
  end
end
raise 'rejected credential persisted' if AgentCredential.where(name: 'Rolled back').exists?
raise 'rejected grant persisted' if AgentGrant.where(capability: 'react').exists?
puts 'Rails failed credential/grant writes: 2 requests return 500; 0 new rows; 0 audits'
