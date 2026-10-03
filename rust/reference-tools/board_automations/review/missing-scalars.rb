# Every missing/scalar SLA-root category: retain Rails' crash and unchanged rows.
rule="INSERT INTO board_sla_rules(id,room_id,work_status,nudge_after_minutes,escalate_after_minutes,created_at,updated_at) VALUES(0,699448332,'planned',60,240,'2026-03-02 16:00:00','2026-03-02 16:00:00')"
inputs=[{}, {sla_rules:nil}, {sla_rules:false}, {sla_rules:true}, {sla_rules:0}, {sla_rules:60}, {sla_rules:1.5}, {sla_rules:''}, {sla_rules:'60'}]
REVIEW_CASES=inputs.each_with_index.map { |input,i| {name:"missing-scalar-#{i}", method:'patch', path:'/rooms/boards/699448332/automations/sla_rules', input:input, user_id:127326141, extra:[rule]} }.as_json
load File.join(ENV.fetch('PARITY_WORK'),'reference-tools/board_automations/review/settings.rb')
