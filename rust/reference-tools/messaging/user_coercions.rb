require "json"
ActionController::Base.allow_forgery_protection=false
Rails.application.config.hosts.clear
user=User.find(127326141)
request=ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST"=>"campfire.test","rack.input"=>StringIO.new))
request.cookie_jar.signed[:session_token]=user.sessions.where.not(two_factor_verified_at:nil).first!.token
headers={"Cookie"=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}
browser=ActionDispatch::Integration::Session.new(Rails.application); browser.host! "campfire.test"
shapes=[nil,false,true,17,1.5,[],["David"],["David","Jason"],{}, {"name"=>"David"},["David",{"name"=>"Jason"}],"David","  "]
cases=[]
%w[query page room_id].each do |key|
 (key=="room_id" ? shapes+[["486777696"],["486777696","699448326"]] : shapes).each do |value|
  input={key=>value}
  input["query"]="David" if key=="page"
  path="/autocompletable/users.json?#{Rack::Utils.build_nested_query(input)}"
  begin
   browser.get(path,headers:headers.dup)
   body=JSON.parse(browser.response.body) rescue nil
   cases << {key:,input:,path:,status:browser.response.status,names:body.is_a?(Array) ? body.map{|u|u["markdown_display_name"]} : nil,links:browser.response.headers["Link"],error_body:browser.response.status>=400 ? browser.response.body : nil}
  rescue => e
   cases << {key:,input:,path:,error:e.class.name}
  end
 end
end
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],cases:)+"\n")
puts "WS8bm2 user coercions: #{cases.size} real Rails requests"
