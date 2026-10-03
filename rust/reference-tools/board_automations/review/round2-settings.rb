# Run through the actual pinned Strong Parameters and model validations to select a
# balanced, unique 128-shape corpus. The shared producer then records full HTTP bytes
# and committed facts, independently of this classification (including all errors).
require 'json'
statuses = ChannelThread::WORK_STATUSES
valid = { 'nudge_after_minutes'=>'61', 'escalate_after_minutes'=>'241' }
values = [nil, false, true, 0, 60, '', '60', [], [nil], ['60'], [{}], [valid], [[valid]], {}, valid,
  {'0'=>{}}, {'-1'=>valid}, {'00'=>valid, **valid}, {'1'=>[], **valid}, {'a'=>valid, **valid}]
pool = [{'sla_rules'=>{'0'=>{}, 'planned'=>valid}}, {}]
values.each { |v| pool << {'sla_rules'=>v} }
statuses.each do |status|
  values.each do |v|
    pool << {'sla_rules'=>{status=>v}}
    pool << {'sla_rules'=>{'0'=>{}, status=>v}}
    pool << {'sla_rules'=>{status=>{'0'=>{}, **valid}}}
    pool << {'sla_rules'=>{status=>v, 'unrecognized'=>[{}]}}
  end
  # Scalar permits include null and booleans; array/hash fields are dropped.
  values.each do |v|
    pool << {'sla_rules'=>{status=>{'nudge_after_minutes'=>v, 'escalate_after_minutes'=>'241'}}}
    pool << {'sla_rules'=>{status=>{'nudge_after_minutes'=>'61', 'escalate_after_minutes'=>v}}}
  end
  ['0','-1','00','+1','1.0','x'].each do |key|
    values.each { |v| pool << {'sla_rules'=>{key=>v, status=>valid}} }
  end
end
controller = Rooms::Boards::AutomationsController.new
controller.instance_variable_set(:@room, Room.find(699448332))
buckets = {302=>[],422=>[],500=>[]}
pool.uniq.each_with_index do |input, index|
  controller.params = input
  status = begin
    submitted = controller.send(:sla_rule_params)
    updates = statuses.to_h { |s| f=submitted[s] || {}; [s,{nudge:f[:nudge_after_minutes].to_s.strip,escalate:f[:escalate_after_minutes].to_s.strip}] }
    controller.send(:validate_sla_updates, updates).empty? ? 302 : 422
  rescue StandardError
    500
  end
  buckets.fetch(status) << [index, input]
end
quotas = {302=>66,422=>35,500=>27}
selected = quotas.flat_map do |status,count|
  raise "too few #{status} shapes: #{buckets[status].size}" if buckets[status].size<count
  # Spread the selected cases over all four statuses, retaining the lead repro.
  count.times.map { |i| buckets[status][i * (buckets[status].size - 1) / (count - 1)] }
end.sort_by(&:first)
rule = "INSERT INTO board_sla_rules(id,room_id,work_status,nudge_after_minutes,escalate_after_minutes,created_at,updated_at) VALUES(0,699448332,'planned',60,240,'2026-03-02 16:00:00','2026-03-02 16:00:00')"
REVIEW_CASES = selected.map do |index,input|
  { 'name'=>"shape-#{index}", 'method'=>'patch', 'path'=>'/rooms/boards/699448332/automations/sla_rules', 'input'=>input, 'user_id'=>127326141, 'extra'=>[rule] }
end
# Every JSON scalar category, including Ruby float notation and negative zero.
[true,false,nil,0,1,-1,149087659,1.0,1.5,-0.0,0.0001,0.00001,1e20,1e-7,9223372036854775808,'MiXeD'].each_with_index do |value,index|
  REVIEW_CASES << {'name'=>"scalar-tag-#{index}",'method'=>'post','path'=>'/rooms/boards/699448332/automations/tag_assignments','input'=>{'tag'=>value,'assignee_id'=>149087659},'user_id'=>127326141,'extra'=>[]}
end
load File.join(ENV.fetch('PARITY_WORK'),'reference-tools/board_automations/review/settings.rb')
