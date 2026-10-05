# Original upload/list/delete assertions, including Rails' shortcode audit labels.
require 'json'
Rails.application.config.hosts.clear
Rails.logger=ActiveSupport::Logger.new($stderr)
ActionView::Base.logger=Rails.logger
user=User.find(127326141)
browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test'
request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
request.cookie_jar.signed.permanent[:session_token]={value:user.sessions.first.token,httponly:true,same_site: :lax}
browser.cookies['session_token']=request.cookie_jar[:session_token]
browser.get '/account/icons';ActiveSupport::IsolatedExecutionState.clear
csrf=Nokogiri::HTML(browser.response.body).at_css('meta[name="csrf-token"]')['content']
AuditLog.delete_all
before=WorkspaceIcon.count
browser.post '/account/icons',params:{workspace_icon:{name:'acme',title:'Acme Corp',image:Rack::Test::UploadedFile.new(Rails.root.join('test/fixtures/files/workspace_icons/clean.svg'),'image/svg+xml')}},headers:{'X-CSRF-Token'=>csrf}
ActiveSupport::IsolatedExecutionState.clear
icon=WorkspaceIcon.find_by!(name:'acme')
create={status:browser.response.status,location:browser.response.location,delta:WorkspaceIcon.count-before,title:icon.title,creator:icon.creator_id,attached:icon.image.attached?,audit:AuditLog.where(action:'workspace_icon.create').pluck(:target_type,:target_label)}
browser.get '/account/icons';ActiveSupport::IsolatedExecutionState.clear
body=browser.response.body;dom=Nokogiri::HTML(body)
index={status:browser.response.status,images:dom.css('img[src="/icons/acme"]').size,codes:dom.css('code').count{|n|n.text==':acme:'},title:body.include?('Acme Corp'),uploader:body.include?('Uploaded by David')}
browser.delete "/account/icons/#{icon.id}",headers:{'X-CSRF-Token'=>csrf};ActiveSupport::IsolatedExecutionState.clear
remove={status:browser.response.status,location:browser.response.location,delta:WorkspaceIcon.count-before,audit:AuditLog.where(action:'workspace_icon.destroy').pluck(:target_type,:target_label)}
puts JSON.pretty_generate(reference:ENV.fetch('PARITY_REFERENCE_SHA'),create:create,index:index,remove:remove)
warn 'Rails original icon oracle: 3 routed responses; shortcode labels create and destroy'
