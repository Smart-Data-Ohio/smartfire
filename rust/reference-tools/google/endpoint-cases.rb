# Real Drive requests, clock boundaries and viewer budgets against pinned Rails.
require 'json'
require 'net/http'
require 'action_dispatch/testing/integration'
load File.join(ENV.fetch("PARITY_WORK"), "reference-tools/google/google_calendar_test_helper.rb")
helper=Object.new.extend(GoogleCalendarTestHelper)
Rails.logger=ActiveSupport::Logger.new($stderr)
ActiveJob::Base.queue_adapter=:test
Rails.application.routes.default_url_options.merge!(host:'campfire.test',protocol:'http')
ENV['GOOGLE_CLIENT_ID']='test-client-id';ENV['GOOGLE_CLIENT_SECRET']='FAKE-google-client-secret'
BASE=Time.utc(2026,3,2,16,0,5)
$drive_now=BASE
Time.define_singleton_method(:current) { $drive_now }
Time.define_singleton_method(:now) { $drive_now }
file=helper.send(:drive_file_payload);list=helper.send(:drive_list_payload)
calls=[];fault=nil
http=Object.new
http.define_singleton_method(:get) do |path,headers|
  calls << {method:'GET',path:,body:'',content_type:headers['Content-Type'],access_token:headers['Authorization']&.delete_prefix('Bearer ')}
  raise Errno::ECONNREFUSED if fault=='transport'
  status=fault.is_a?(Integer) ? fault : 200
  response=Net::HTTPResponse::CODE_TO_OBJ.fetch(status.to_s).new('1.1',status.to_s,'recorded')
  response.define_singleton_method(:body) { JSON.generate(path.start_with?('/drive/v3/files?') ? list : file) }
  response
end
Net::HTTP.define_singleton_method(:start) { |host,*args,**opts,&block| raise 'unrecorded host' unless host=='www.googleapis.com';block.call(http) }
def sign_in(client,id)
  user=User.find(id);session=Session.create!(user:,user_agent:'endpoint-fixture',ip_address:'127.0.0.1',two_factor_verified_at:Time.current)
  request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.url_scheme'=>'http','REQUEST_METHOD'=>'GET'))
  jar=ActionDispatch::Cookies::CookieJar.build(request,{});jar.signed[:session_token]={value:session.token};client.cookies['session_token']=jar[:session_token]
end
specs=%w[show_no_account show_retired show_disconnected show_calendar_only show_transport show_503 show_403 show_404 show_cache show_budget index_blank index_transport index_budget index_no_cache index_query].map { |name|{name:} }
rows=[]
specs.each do |spec|
  ActiveRecord::Base.transaction(requires_new:true) do
    $drive_now=BASE;Rails.cache=ActiveSupport::Cache::MemoryStore.new
    GoogleAccount.delete_all
    david=User.find(127326141);jason=User.find(149087659)
    scopes="#{Google::Client::CALENDAR_SCOPE} #{Google::Client::DRIVE_SCOPE}"
    scopes='https://www.googleapis.com/auth/drive.metadata.readonly' if spec[:name]=='show_retired'
    scopes=Google::Client::CALENDAR_SCOPE if spec[:name]=='show_calendar_only'
    account=GoogleAccount.create!(user:david,email:'david@gmail.test',access_token:'access-token',refresh_token:'refresh-token',access_token_expires_at:BASE+3600,scopes:) unless spec[:name]=='show_no_account'
    account.update_columns(disconnected_reason:'Disconnected') if spec[:name]=='show_disconnected'
    GoogleAccount.create!(user:jason,email:'jason@gmail.test',access_token:'access-token',refresh_token:'refresh-token',access_token_expires_at:BASE+3600,scopes:) if spec[:name].end_with?('budget')
    client=ActionDispatch::Integration::Session.new(Rails.application);client.host! 'campfire.test';sign_in(client,david.id)
    calls.clear;fault=spec[:name].end_with?('transport') ? 'transport' : spec[:name][/_(503|403|404)$/,1]&.to_i
    path=spec[:name].start_with?('show') ? "/google/drive/files/#{file['id']}" : '/google/drive/files'
    path+='?q=%20%20bob%27s%5Cdraft%20%20' if spec[:name]=='index_query'
    offsets=case spec[:name]
    when 'show_cache' then [0,0,299,300,301]
    when 'show_budget' then Array.new(61,0)+[55]
    when 'index_budget' then Array.new(31,0)+[55]
    when 'index_no_cache' then [0,0]
    else [0]
    end
    observations=[]
    offsets.each do |offset|
      $drive_now=BASE+offset;client.get(path,headers:{'Accept'=>'application/json'})
      observations << {status:client.response.status,body:client.response.body.empty? ? '' : JSON.parse(client.response.body),cache_control:client.response.headers['Cache-Control'],calls:calls.length}
    end
    other=nil
    if spec[:name].end_with?('budget')
      sign_in(client,jason.id);client.get(path,headers:{'Accept'=>'application/json'})
      other={status:client.response.status,body:JSON.parse(client.response.body),cache_control:client.response.headers['Cache-Control'],calls:calls.length}
    end
    rows << {spec:,path:,offsets:,fault:,observations:,other:,requests:calls.dup}
    raise ActiveRecord::Rollback
  end
end
puts JSON.pretty_generate({reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],now:BASE.to_i,file:,list:,rows:})
