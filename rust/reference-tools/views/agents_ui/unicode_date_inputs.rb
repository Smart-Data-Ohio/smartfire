# Onig can case-fold a Unicode character in an ASCII date regex, while the
# C month/era lookup compares ASCII bytes. Keep both stages in the oracle.
inputs=['15 ſep 2030','ſep 15 2030','15-ſep-2030','ſep-15-2030','ſ42.6.15','ſ1.1.1','ſun','ſat 4pm','2030-06-15 10:20:30 eſt','15 Sep 2030']
result={reference:ENV.fetch("PARITY_REFERENCE_SHA"),expiry:[]}
Time.use_zone('UTC') do
 inputs.each do |input|
  value=AgentCredential.new(expires_at:input).expires_at
  result[:expiry] << {zone:'UTC',input:,stored:value && ActiveRecord::Base.connection.send(:quoted_date,value)}
 end
end
puts JSON.pretty_generate(result)
warn "Rails Unicode date corpus: #{result[:expiry].size} cases"
