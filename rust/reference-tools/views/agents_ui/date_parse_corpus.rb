# Actual model cast, not Date._parse in isolation: Time.zone.parse and the
# ActiveModel fallback/error boundaries remain part of this oracle.
inputs = []
years = ['0','30','68','69','99','0030','2000','2030','-0001']
months = ['1','02','6','12','13']
days = ['1','15','29','30','31','32']
years.product(months,days).each do |y,m,d|
 inputs.concat(["#{y}-#{m}-#{d}","#{y}/#{m}/#{d}","#{y}.#{m}.#{d}","#{d}.#{m}.#{y}","#{d} Jun #{y}","Jun #{d} #{y}","#{d}-Jun-#{y}","Jun-#{d}-#{y}"])
end
clocks = ['0:00','24:00:00.5','24:00:01','23:59:60.000001','10h20m30s','4pm','4 a.m.','10:20:30,123456789','10:99:00']
zones = ['','Z','GMT','EST','Eastern Daylight Time','UTC+05','+05:30:45','-5','+053045','+24:00','+99','+05.5','+5.33333335','bogus']
clocks.product(zones).each do |clock,offset|
 ['2030-06-15','2030/06','15 June 30',''].each { |date| inputs << "#{date} #{clock} #{offset}" }
end
inputs.concat(['2030-W24-6','30W246','-W-6','--06-15','--0615','2030-166','-166',"'30",'15th','Fri','next friday','tomorrow','June','June 2030','2030 June 15','June 2030 15','10:20:30 June 15, 2030','June BC 15','15 June 2030 BCE'])
(2..14).each {|n| ['12345678901234','00000000000000','20300615102030'].each {|s| ['','T102030Z','.123456789',' 102030.12 [530:TEST]'].each {|endpart| inputs << s[0,n]+endpart}}}
['M1.1.1','T10.6.15','S20.6.15','H42.6.15','R12.6.15','2030年06月15日','２０３０-０６-１５',"June\n15, 2030",'2030-06-15 garbage','x'*128,'x'*129,'2030-06-15'+' '*118,"2030-06-15\0",'not a date',''].each {|s| inputs << s}
inputs = inputs.uniq
result = {reference:'d7c7de92', generator:'date_parse_corpus.rb', expiry:[], floats:[]}
['UTC','America/New_York','Australia/Lord_Howe','Pacific/Apia'].each do |zone|
 Time.use_zone(zone) do
  inputs.each do |input|
   value = AgentCredential.new(expires_at:input).expires_at
   result[:expiry] << {zone:,input:,stored:value && ActiveRecord::Base.connection.send(:quoted_date,value)}
  end
 end
end
# Cover Float#to_s's thresholds, signed zero, subnormals, maximums, and
# shortest-roundtrip digits. Parse the request JSON before Rails' to_s.
bits = [0,0x8000000000000000,1,0x0010000000000000,0x7fefffffffffffff]
rng = Random.new(196)
4096.times {bits << rng.rand(0...(1<<64))}
floats = bits.map {|b| [b].pack('Q>').unpack1('G')}.select(&:finite?)
(-10..20).each {|e| [1.0,1.2345678901234567,9.99999999999999].each {|f| floats << f*10.0**e}}
floats.each do |float|
 input = JSON.parse(JSON.generate(float))
 result[:floats] << {input:, string:input.to_s}
end
puts JSON.pretty_generate(result)
warn "Rails generated cast corpus: #{result[:expiry].size} expiry cases; #{result[:floats].size} floats"
