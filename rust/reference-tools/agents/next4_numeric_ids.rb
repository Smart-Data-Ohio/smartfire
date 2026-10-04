# Raw JSON lexemes distinguish arbitrary-precision Integers from rounded Floats.
require 'json'
Rails.logger=ActiveSupport::Logger.new($stderr)
alpha=2106000010; beta=2106000020
allowed=[-(2**63),-alpha,0,alpha,beta,2**63-1]
forms=[-(2**63),-(2**63)-1,-(2**63)-2,-(2**63)-512,-(2**63)-1024,
       -(2**63)-65536,-(2**64),2**63-1,2**63,2**63+1,2**64-1,2**64,
       -(10**100),10**100,0,-1,alpha,beta].map(&:to_s)
forms+=%w[-9223372036854775808.0 -9223372036854775809.0 -9.223372036854776e18 9223372036854775807.0 2106000010.9 -0.0]
cases=[]
ActiveRecord::Base.transaction do
 conn=ActiveRecord::Base.connection
 allowed.each do |id|
  conn.execute("INSERT INTO channel_threads(id,room_id,creator_id,name,last_activity_at,created_at,updated_at) VALUES(#{id},486777696,127326141,'Numeric boundary','2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00')")
 end
 forms.each do |s|
  [s,"[#{s}]","[null,#{s},null]","[[#{s}]]","[#{beta},#{s}]",
   "[#{s},#{beta}]","[{\"bad\":true},#{s}]","[#{s},[#{alpha}],-10,null]",
   "[#{beta},[#{s}]]","[null,[#{s}],null]","[#{beta},[#{s},#{s}],-10,null]"].each do |raw|
   input=JSON.parse(raw)
   selected=ChannelThread.where(id:input).where(id:allowed).order(:id).pluck(:id)
   cases << {input_json:raw,selected:}
  end
 end
 raise ActiveRecord::Rollback
end
puts JSON.pretty_generate(reference_pin:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],allowed:,scalar_forms:forms.size,positions:11,cases:)
warn "WS11 next4 Rails numeric IDs: #{forms.size} forms; #{cases.size} executed predicates"
