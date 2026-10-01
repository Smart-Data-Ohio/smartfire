require "json"
require "digest"
Rails.logger=ActiveSupport::Logger.new($stderr)
Rails.application.config.hosts.clear
JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"),"reference-tools/users/onboarding-source-hashes.json"))).each {|path,hash|raise "source drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest==hash}
class OnboardingGoldenController < ApplicationController
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : "GLOBAL"
  end
end
def render_page(template,assigns={})
  controller=OnboardingGoldenController.new
  controller.set_request!(ActionDispatch::Request.new(OnboardingGoldenController.renderer.new(http_host:"campfire.test",https:false,"rack.session"=>{},"action_dispatch.content_security_policy_nonce_generator"=>->(_){"NONCE"}).send(:env_for_request)))
  controller.set_response!(ActionDispatch::Response.new)
  view=controller.view_context;view.assign(assigns.stringify_keys)
  html=view.render(template:template,layout:false)
  {html:html,sidebar:view.content_for(:sidebar).to_s}
end
if ARGV.first=="welcome"
  user=User.find(127326141);Current.user=user
  page=render_page("welcome/show")
  session=ActionDispatch::Integration::Session.new(Rails.application);session.host! "campfire.test"
  request=ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST"=>"campfire.test","rack.input"=>StringIO.new))
  request.cookie_jar.signed.permanent[:session_token]={value:user.sessions.first.token,httponly:true,same_site: :lax};session.cookies["session_token"]=request.cookie_jar[:session_token]
  session.get "/";ActiveSupport::IsolatedExecutionState.clear;first={status:session.response.status,location:session.response.headers["Location"]}
  session.cookies["last_room"]="486777696";session.get "/";ActiveSupport::IsolatedExecutionState.clear;last={status:session.response.status,location:session.response.headers["Location"]}
  puts JSON.pretty_generate(reference:"d7c7de92",page:page,first:first,last:last)
  warn "Rails welcome oracle: 1 complete body/sidebar, 2 visible-room redirects; reference d7c7de92"
else
  Current.reset
  page=render_page("first_runs/show",{user:User.new})
  inputs=[
    {name:"valid",params:{account:{name:"ignored"},user:{name:"New Person",email_address:"new@37signals.com",password:"secret123456",role:"member"}}},
    {name:"empty_name",params:{user:{name:"",email_address:"new@37signals.com",password:"secret123456"}}},
    {name:"missing_name",params:{user:{email_address:"new@37signals.com",password:"secret123456"}}},
    {name:"missing_password",params:{user:{name:"New Person",email_address:"new@37signals.com"}}},
    {name:"empty_password",params:{user:{name:"New Person",email_address:"new@37signals.com",password:""}}},
    {name:"missing_email",params:{user:{name:"New Person",password:"secret123456"}}},
    {name:"missing_user",params:{account:{name:"ignored"}}}
  ]
  cases=inputs.map do |input|
    ActiveRecord::Base.connection.disable_referential_integrity do
      %w[ memberships sessions users rooms accounts audit_logs ].each {|table| ActiveRecord::Base.connection.execute("DELETE FROM #{table}")}
    end
    Current.reset
    session=ActionDispatch::Integration::Session.new(Rails.application);session.host! "campfire.test"
    session.get "/first_run";ActiveSupport::IsolatedExecutionState.clear
    token=Nokogiri::HTML(session.response.body).at_css('meta[name="csrf-token"]')["content"]
    session.post "/first_run",params:input[:params],headers:{"X-CSRF-Token"=>token};ActiveSupport::IsolatedExecutionState.clear
    {**input,status:session.response.status,location:session.response.headers["Location"],state:{account_names:Account.pluck(:name),users:User.all.map {|u|{name:u.name,email:u.email_address,role:u.role,password_present:u.password_digest.present?}},rooms:Room.pluck(:name,:type),memberships:Membership.count,sessions:Session.count,audits:AuditLog.count}}
  end
  puts JSON.pretty_generate(reference:"d7c7de92",page:page,cases:cases)
  warn "Rails first run oracle: 1 complete body, #{cases.size} HTTP/persisted-state cases; reference d7c7de92"
end
