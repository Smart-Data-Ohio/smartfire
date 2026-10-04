# Execute Active Record's real predicate builder for scalar strings in array positions.
require 'json'
Rails.logger=ActiveSupport::Logger.new($stderr)
alpha=2106000010; beta=2106000020
whitespace=['',' ',"\t","\n","\v","\f","\r", " \t\n\v\f\r", "\u00a0", "\u2003", "\0"]
signs=['','+','-']
prefixes=['','0d','0D','0x','0X','0b','0B','0o','0O']
digits=[alpha.to_s,'2_106_000_010','2106__000010','2106000010_','2106000010tail','9223372036854775807','9223372036854775808','9223372036854775809','']
forms=whitespace.product(signs,prefixes,digits).map{|w,s,p,d|w+s+p+d}
forms += ['bad','_2106000010','0d_2106000010','0D_2106000010','0d','+0d','-0D','0__2106000010','++2106000010','--2106000010',"2106000010\u0000tail"]
forms= forms.uniq
layouts=->(s){[s,[s],[nil,s,nil],[[s]],[beta,s],[s,beta],[{bad:true},s],[s,[alpha],-10,nil],[beta,[s]],[nil,[s],nil],[beta,[s,s],-10,nil]]}
allowed=[-(2**63),-alpha,0,alpha,beta,2**63-1]
cases=[]
ActiveRecord::Base.transaction do
 conn=ActiveRecord::Base.connection
 allowed.each do |id|
  conn.execute("INSERT INTO channel_threads(id,room_id,creator_id,name,last_activity_at,created_at,updated_at) VALUES(#{id},486777696,127326141,'Casting corpus','2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00')")
 end
 forms.each_with_index do |form,i|
  layouts.call(form).each_with_index do |input,j|
   selected=ChannelThread.where(id:input).where(id:allowed).order(:id).pluck(:id)
   cases << {form:i,position:j,input:input,selected:selected}
  end
 end
 raise ActiveRecord::Rollback
end
puts JSON.generate(reference_pin:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],scalar_forms:forms.length,positions:11,allowed:allowed,cases:cases)
warn "PR214 Rails ID corpus: #{forms.length} scalar forms; 11 positions; #{cases.length} executed predicates"
