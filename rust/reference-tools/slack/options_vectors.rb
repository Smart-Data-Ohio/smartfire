require 'json'
inputs=[{}, {'include_private'=>false},{'include_private'=>nil},{'include_private'=>'0'},{'include_private'=>0},
 {'conversation_ids'=>['C1','',' ',nil,false,0,'C1','G2']},{'conversation_ids'=>'C1'}, {'conversation_ids'=>42},{'conversation_ids'=>{'C1'=>'G2'}},{'conversation_ids'=>[['C1','G2']]},
 {'room_targets'=>{'C1'=>'861','G2'=>'skip'},'unknown'=>'ignored'},{'room_targets'=>['new','skip']},{'room_targets'=>'new'},
 {'oldest'=>'2026-01-01T12:13:14Z'},{'oldest'=>'2026-01-01T12:13:14.123456789Z'}, {'oldest'=>'2026-01-01T12:13:14+09:00'}, {'latest'=>'2026-01-01T12:13:14-05:30'},
 {'oldest'=>'2026-01-01T12:13:14'}, {'oldest'=>'20260101T121314Z'}, {'oldest'=>'2026-01-01T12:13:14+0900'},
 {'oldest'=>' '},{'oldest'=>false},{'oldest'=>0},{'oldest'=>nil},{'oldest'=>'2026-01-01'},{'latest'=>'bad'},{'oldest'=>[]},{'oldest'=>{}},{'oldest'=>true},
 {'oldest'=>'2026-02-30T00:00:00Z'},{'oldest'=>'2026-01-01T24:00:00Z'},{'oldest'=>'2026-01-01T12:13:60Z'}, {'oldest'=>'2026-01-01t12:13:14z'}]
rows=inputs.map do |input|
 begin
  {input:,result:SlackImport.send(:normalize_options,input)}
 rescue => e
  {input:,error:e.message,class:e.class.name}
 end
end
File.write(File.join(ENV.fetch('PARITY_WORK'),'vectors/slack/options.json'),JSON.pretty_generate(rows)+"\n")
puts "Slack options oracle: #{rows.size} Ruby coercion and ISO time-bound cases generated"
