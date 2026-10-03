# The pinned json gem checks its adjusted i32 exponent before the mantissa.
raws=[]
['0','-0','0.0','-0.000','1.0','-1.00','0.00000000','123.45'].each do |mantissa|
 ['2147483646','2147483647','2147483648','2147483649','-2147483648','-2147483649','9223372036854775807','9223372036854775808','-9223372036854775808','-9223372036854775809','00000000000000000000','00000000000000000001','-00000000000000000001'].each do |exponent|
  raw="#{mantissa}e#{exponent}"
  raws.concat([raw,"[#{raw}]",%Q({"a":#{raw}})])
 end
end
rows=raws.map do |raw|
 params=ActionController::Parameters.new(ActionDispatch::Request::Utils.normalize_encode_params(ActiveSupport::JSON.decode('{"access_token":'+raw+'}')))
 {raw:,string:params[:access_token].to_s.strip}
end
puts JSON.pretty_generate(reference:'d7c7de92',tokens:rows)
warn "Rails exponent boundaries: #{rows.length} raw tokens"
