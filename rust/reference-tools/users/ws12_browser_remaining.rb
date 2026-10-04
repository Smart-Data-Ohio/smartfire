# Fixture inputs for the original seven WS12 browser declarations. No UI response
# is synthesized: the driver loads production pages and submits production forms.
require 'json'
require 'digest'
require 'active_support/testing/time_helpers'
extend ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
ActiveJob::Base.queue_adapter=:test
common=["UPDATE rooms SET name='Launch',creator_id=127326141 WHERE id=699448332", "UPDATE users SET name='Board Agent' WHERE id=394959859", "INSERT OR IGNORE INTO memberships(room_id,user_id,involvement,created_at,updated_at) VALUES(699448332,773523953,'mentions','2026-03-02 16:00:00','2026-03-02 16:00:00')", "INSERT OR IGNORE INTO memberships(room_id,user_id,involvement,created_at,updated_at) VALUES(699448332,394959859,'mentions','2026-03-02 16:00:00','2026-03-02 16:00:00')", "DELETE FROM agent_grants WHERE agent_id=773018776", "DELETE FROM board_tag_assignments WHERE room_id=699448332", "DELETE FROM board_sla_rules WHERE room_id=699448332", "DELETE FROM background_jobs"]
%w[read_messages post_messages manage_threads].each{|cap|common<<"INSERT INTO agent_grants(agent_id,room_id,granted_by_id,capability,created_at,updated_at) VALUES(773018776,NULL,127326141,'#{cap}','2026-03-02 16:00:00','2026-03-02 16:00:00')"}
rows=(221..227).map{|i|{id:"c#{i}",setup:common.dup}}
rows.find{|r|r[:id]=='c223'}[:setup]<<"INSERT INTO board_tag_assignments(id,room_id,tag,assignee_id,created_by_id,created_at,updated_at) VALUES(9018970100,699448332,'bug',773523953,127326141,'2026-03-02 16:00:00','2026-03-02 16:00:00')"
rows.find{|r|r[:id]=='c224'}[:setup]<<"UPDATE channel_threads SET work_status='in_progress',work_owner_id=127326141 WHERE id=4"
r=rows.find{|r|r[:id]=='c225'};r[:setup]+=["INSERT INTO board_sla_rules(id,room_id,work_status,nudge_after_minutes,escalate_after_minutes,created_at,updated_at) VALUES(9018970100,699448332,'in_progress',60,240,'2026-03-02 16:00:00','2026-03-02 16:00:00')", "UPDATE channel_threads SET name='Stuck migration',work_status='in_progress',work_owner_id=773523953,work_status_changed_at='2026-03-02 13:00:00' WHERE id=4"]
rows.find{|r|r[:id]=='c226'}[:setup]<<"UPDATE rooms SET name='Parent board' WHERE id=699448332"
rows.find{|r|r[:id]=='c227'}[:setup]<<"DELETE FROM memberships WHERE room_id=699448332 AND user_id<>127326141"
if ENV['WS12_BROWSER_CASE']
 row=rows.find{|r|r[:id]==ENV.fetch('WS12_BROWSER_CASE')};raise 'Unknown browser declaration' unless row
 row[:setup].each{|q|ActiveRecord::Base.connection.execute(q)}
 BoardAutomations::DigestDispatcher.dispatch_due! if row[:id]=='c225'
 warn "WS12_BROWSER_FIXTURE #{row[:id]} production Rails dispatcher and setup ready"
else
 puts JSON.pretty_generate(rows:,sources:%w[test/system/board_automations_test.rb test/system/boards_test.rb].to_h{|p|[p,Digest::SHA256.file(Rails.root.join(p)).hexdigest]})
end
