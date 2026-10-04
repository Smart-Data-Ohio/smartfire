# Additional component combinations, bit-field boundaries and arbitrary Integers.
# These call the pinned model and JSON decoder, not an isolated grammar oracle.
result={reference:ENV.fetch("PARITY_REFERENCE_SHA"),expiry:[],tokens:[]}
years=['-292278','292279','9223372036854775808','-9223372036854775809','9'*110,'-'+'9'*110]
inputs=[]
years.product(['01-01','02-29','12-31'],['',' 23:59:60',' 24:00:00',' 00:00:00 +2359',' 23:59:59 -2359',' BC']).each {|y,date,clock| inputs << "#{y}-#{date}#{clock}"}
['0','31','32','63','64','2147483647','2147483648','4294967295','4294967296','9223372036854775807','9223372036854775808','9'*80].each do |n|
 inputs.concat(["#{n}:01:02","01:#{n}:02","01:02:#{n}","#{n}pm","#{n}th","2030-#{n}-01","2030-01-#{n}","R#{n}.1.2","'#{n}"])
 ['13','16','32'].each {|m| inputs << "2030-#{m}-01 #{n}:01:02"}
 ['+0:'+n,'+23:'+n+':00','+'+n,'+'+n+':00'].each {|z| inputs << "2030-06-15 01:02:03 #{z}"}
end
rng=Random.new(19602)
240.times do
 y=('0'..'9').to_a.sample(random:rng)+(1+rng.rand(110)).times.map {rng.rand(10)}.join
 y='-'+y if rng.rand(2)==0
 inputs << "#{y}/02/#{[1,28,29,31].sample(random:rng)}"
end
inputs=inputs.uniq
['UTC','America/New_York','Australia/Lord_Howe','Pacific/Apia','Asia/Kathmandu','Europe/Dublin'].each do |zone|
 Time.use_zone(zone) do
  inputs.each do |input|
   begin
    value=AgentCredential.new(expires_at:input).expires_at
    result[:expiry] << {zone:,input:,stored:value && ActiveRecord::Base.connection.send(:quoted_date,value)}
   rescue StandardError => error
    result[:expiry] << {zone:,input:,error:error.class.name}
   end
  end
 end
end
raws=['-0','-0.0','1e309','-1e309','1e100000','-1e100000','1e-100000','-1e-100000']
[20,40,80,128,512,4096].each do |width|
 8.times do
  digits='1'+(width-1).times.map{rng.rand(10)}.join
  raws.concat([digits,'-'+digits])
 end
end
raws += raws.map {|raw| '['+raw+']'}
raws.each do |raw|
 params=ActionController::Parameters.new(ActiveSupport::JSON.decode('{"access_token":'+raw+'}'))
 result[:tokens] << {raw:,string:params[:access_token].to_s.strip}
end
puts JSON.pretty_generate(result)
warn "Rails generated extreme corpus: #{result[:expiry].length} dates; #{result[:tokens].length} raw tokens"
