require 'json'
cases=[{account_id:'897362094',kind:'comment',number:579,body:'Nice'}, {account_id:'897362094',kind:'create',board_id:'03board1',title:'Ship it'}, {account_id:'897362094',kind:'move',number:579,column_id:'03column1'}, *%w[close reopen].map { |k| {account_id:'897362094',kind:k,number:579} }, {account_id:'897362094',kind:'explode',number:579}, {account_id:'897362094',kind:'comment',number:579}, {account_id:'897362094',kind:'comment',body:'x'}, {account_id:'897362094',kind:'create',title:'x'}, {account_id:'897362094',kind:'move',number:579}, {kind:'close',number:579}, {account_id:'897362094',kind:'comment',number:579,body:'x'*3501}, {account_id:'897362094',kind:'create',board_id:'b',title:'x'*501}, {account_id:'../my',kind:'close',number:579}, {account_id:'897362094',kind:'create',board_id:'a/b',title:'t'}, {account_id:'a',kind:'comment',number:'000',body:'  Nice  '}, {account_id:'a',kind:'comment',number:'-1',body:'x'}, {account_id:'a',kind:'create',board_id:' board ',title:' title ',description:' body '}, {account_id:'a',kind:'comment',number:1,body:'é'*3500}, {account_id:'a',kind:'comment',number:1,body:'é'*3501}]
cases += [{account_id:'a',kind:'comment',number:'123456789012345678901234567890',body:'Big'}, {account_id:'a',kind:'create',board_id:'b',title:'<>&\u2028'}]
vectors=cases.map do |input|
 action=Fizzy::AgentCardAction.new(**input)
 {input:input,valid:action.valid?,errors:action.errors.to_hash,action:action.action_name,summary:action.summary,payload:action.payload_hash,payload_json:action.payload_json}
end
File.write(ARGV.fetch(0),JSON.pretty_generate({reference:'d7c7de92',actions:vectors})+"\n")
puts "WS15e Fizzy agent action Rails oracle: #{vectors.size} action cases"
