# Integer ID serialization used by Active Record's ArrayHandler (one array level).
require 'json'
type=Message.type_for_attribute('id')
values=[nil,false,true,0,-10,12,12.9,'','bad','  +12tail','-10tail','1e3','0x12',[],[12],['12',12],{'bad'=>true},[{'bad'=>true},12], [12,['1',1],-10,nil]]
rows=values.map do |value|
 predicate=value
 while predicate.is_a?(Array) && predicate.compact.length==1
  predicate=predicate.compact.first
 end
 values=predicate.is_a?(Array) ? predicate : [predicate]
 candidates=values.filter_map do |candidate|
  type.serialize(candidate)
 rescue ActiveModel::RangeError
  nil
 end.uniq.sort
 {input:value,candidates:candidates}
end
puts JSON.pretty_generate(reference_pin:'d7c7de92',cases:rows)
