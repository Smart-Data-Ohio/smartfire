require 'zlib'
input=JSON.parse(Zlib::GzipReader.open(File.join(__dir__, '../../../test-support/agents_ui/extreme_cast_inputs.json.gz'), &:read))
rows=[]
input['zones'].each do |zone|
 Time.use_zone(zone) do
  input['expiry'].each_with_index do |text,index|
   begin
    value=AgentCredential.new(expires_at:text).expires_at
    rows << {zone:zone,input_index:index,stored:value && ActiveRecord::Base.connection.send(:quoted_date,value)}
   rescue StandardError => error
    rows << {zone:zone,input_index:index,error:error.class.name}
   end
  end
 end
end
tokens=input['tokens'].each_with_index.map do |raw,index|
 begin
  params=ActionController::Parameters.new(ActiveSupport::JSON.decode('{"access_token":'+raw+'}'))
  {input_index:index,string:params[:access_token].to_s.strip}
 rescue StandardError => error
  {input_index:index,error:error.class.name}
 end
end
puts JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA"),expiry:rows,tokens:tokens)
warn "Fresh Rails adversarial oracle: #{rows.length} date casts; #{tokens.length} token casts; #{rows.count{|r|r[:error]}} date errors; #{tokens.count{|r|r[:error]}} token errors"
