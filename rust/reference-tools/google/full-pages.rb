# Whole HTTP pages; only request-generated entropy is fixed before rendering.
require 'json'
require 'digest'
require 'action_dispatch/testing/integration'
ActiveJob::Base.queue_adapter=:test
ActionController::Base.allow_forgery_protection=true
Rails.application.env_config['action_dispatch.show_exceptions']=:all
Rails.application.env_config['action_dispatch.content_security_policy_nonce_generator']=->(_) { 'NONCE' }
ApplicationController.prepend(Module.new do
  def form_authenticity_token(form_options:{})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end)
Net::HTTP.define_singleton_method(:start) { |*|raise 'whole-page fixture attempted live HTTP' }
# Hold unrelated Web Push configuration off on both sides of this Google-page fixture.
ENV['VAPID_PUBLIC_KEY']=''
Rails.configuration.x.vapid.public_key=nil
rows=[]
now=Time.utc(2026,3,2,16)
Time.define_singleton_method(:current) { now }
specs=%w[configured missing_credentials empty_domains].map { |name|{name:"login_#{name}",page:'login',config:name} }
specs+=%w[unconfigured no_account calendar drive retired partial disconnected linked].map { |name|{name:"profile_#{name}",page:'profile',config:name=='unconfigured' ? 'missing_credentials' : 'configured',account:name} }
specs.each do |spec|
  ActiveRecord::Base.transaction(requires_new:true) do
    ENV['GOOGLE_CLIENT_ID']=spec[:config]=='missing_credentials' ? '' : 'test-client-id'
    ENV['GOOGLE_CLIENT_SECRET']=spec[:config]=='missing_credentials' ? '' : 'FAKE-page-secret'
    ENV['GOOGLE_SIGN_IN_DOMAINS']=spec[:config]=='empty_domains' ? '' : 'smartdata.net,cnbssoftware.com'
    user=User.find(127326141)
    GoogleAccount.where(user:).delete_all;GoogleIdentity.where(user:).delete_all
    scope=case spec[:account]
    when 'calendar','disconnected' then Google::Client::CALENDAR_SCOPE
    when 'drive' then "#{Google::Client::CALENDAR_SCOPE} #{Google::Client::DRIVE_SCOPE}"
    when 'retired' then "#{Google::Client::CALENDAR_SCOPE} https://www.googleapis.com/auth/drive.metadata.readonly"
    when 'partial' then Google::Client::DRIVE_SCOPE
    end
    GoogleAccount.create!(user:,email:'david@smartdata.net',access_token:'access-token',refresh_token:'refresh-token',access_token_expires_at:now+3600,scopes:scope,disconnected_reason:spec[:account]=='disconnected' ? 'revoked' : nil) if scope
    GoogleIdentity.create!(user:,subject:'page-fixture',email:'david@smartdata.net',domain:'smartdata.net') if spec[:account]=='linked'
    client=ActionDispatch::Integration::Session.new(Rails.application);client.host! 'campfire.test'
    if spec[:page]=='profile'
      session=Session.create!(user:,user_agent:'Google page fixture',ip_address:'127.0.0.1',two_factor_verified_at:now)
      req=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.url_scheme'=>'http','REQUEST_METHOD'=>'GET'))
      jar=ActionDispatch::Cookies::CookieJar.build(req,{})
      jar.signed[:session_token]={value:session.token};client.cookies['session_token']=jar[:session_token]
    end
    path=spec[:page]=='login' ? '/session/new' : '/users/me/profile'
    client.get(path,headers:{'User-Agent'=>'Mozilla'})
    body=client.response.body
    google_panels=%w[google-calendar-title google-sign-in-title].map { |id|body[/<section[^>]*aria-labelledby="#{id}".*?<\/section>/m] } if spec[:page]=='profile'
    google_settings=body.lines.grep(/<p.*(Google Calendar|Google calendar|Reconnect Google)/).map(&:strip) if spec[:page]=='profile'
    rows << {spec:,scope:,status:client.response.status,body:,google_panels:,google_settings:}
    raise ActiveRecord::Rollback
  end
end
puts JSON.pretty_generate({reference:'d7c7de92',layout_reference:'2e20b24c',sources:%w[app/views/layouts/application.html.erb app/views/users/profiles/_status.html.erb app/views/users/profiles/show.html.erb app/views/sessions/new.html.erb].to_h { |p|[p,Digest::SHA256.file(Rails.root.join(p)).hexdigest] },vapid_public_key:Rails.configuration.x.vapid.public_key,now:now.iso8601,rows:})
