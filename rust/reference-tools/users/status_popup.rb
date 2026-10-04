require 'json'
require 'digest'
Rails.logger=ActiveSupport::Logger.new($stderr)
JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'reference-tools/users/profile-page-source-hashes.json'))).each {|path,hash|raise "source drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest==hash}
load File.join(ENV.fetch('PARITY_WORK'),'reference-tools/users/post_pin.rb')
class StatusPopupGoldenController < Users::StatusesController
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
user=User.find(127326141)
Current.reset;Current.user=user
renderer=StatusPopupGoldenController.renderer.new(http_host:'campfire.test',https:false,'rack.session'=>{})
cases=[['seed',{}],['invisible',{presence_setting:'invisible',custom_status_emoji:'<',custom_status_text:'A & B'}],['invalid_presence',{presence_setting:'away'}],['invalid_text',{custom_status_emoji:'😀'*9,custom_status_text:'x'*101}],['invalid_expiry',{}]].map do |name,attrs|
  user.reload; user.assign_attributes(attrs)
  user.valid? unless name=='seed'
  user.errors.add(:custom_status_expires_in,'is not valid') if name=='invalid_expiry'
  fields={presence:user.presence_setting,emoji:user.custom_status_emoji,text:user.custom_status_text,errors:user.errors.to_hash}
  html=renderer.render(template:'users/statuses/edit',layout:false,assigns:{user:user})
  {name:name,fields:fields,html:html}
end
# Real authenticated requests, real CSRF. No fixed tokens at the HTTP boundary.
Rails.application.config.hosts.clear
session=ActionDispatch::Integration::Session.new(Rails.application);session.host! 'campfire.test'
request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
request.cookie_jar.signed.permanent[:session_token]={value:user.sessions.first.token,httponly:true,same_site: :lax}
session.cookies['session_token']=request.cookie_jar[:session_token]
session.get '/users/me/profile';ActiveSupport::IsolatedExecutionState.clear
token=Nokogiri::HTML(session.response.body).at_css('meta[name="csrf-token"]')['content']
http=[['frame_save',{presence_setting:'dnd',custom_status_emoji:'🚂',custom_status_text:'On a train',custom_status_expires_in:'hour_1'},true],['page_save',{presence_setting:'invisible'},false],['frame_clear',{clear_custom_status:'1'},true],['frame_bad_presence',{presence_setting:'away'},true],['frame_bad_expiry',{custom_status_expires_in:'wrong'},true],['foreign_path',{custom_status_text:'Self only'},true]].map do |name,params,frame|
  user.update_columns(presence_setting:'auto',custom_status_emoji:'🚀',custom_status_text:'Shipping Rust',custom_status_expires_at:nil)
  path=name=='foreign_path' ? '/users/149087659/status' : '/users/me/status'
  headers={'X-CSRF-Token'=>token,'Accept'=>'text/html'};headers['Turbo-Frame']='user_card' if frame
  session.patch path,params:{user:params},as: :json,headers:headers
  ActiveSupport::IsolatedExecutionState.clear
  {name:name,path:path,params:params,frame:frame,status:session.response.status,location:session.response.headers['location'],state:user.reload.attributes.slice('presence_setting','custom_status_emoji','custom_status_text','custom_status_expires_at')}
end
puts JSON.pretty_generate(reference: ENV.fetch('PARITY_REFERENCE_SHA'),popup:cases,http:http)
warn "Rails status popup oracle: #{cases.size} complete popup bodies, #{http.size} HTTP update/state cases; plain pinned reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
