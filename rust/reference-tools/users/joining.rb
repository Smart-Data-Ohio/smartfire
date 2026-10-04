require 'json'
require 'digest'
Rails.logger=ActiveSupport::Logger.new($stderr);Rails.application.config.hosts.clear
JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'reference-tools/users/joining-source-hashes.json'))).each {|path,hash|raise "source drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest==hash}
class JoinGoldenController < UsersController
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
Current.reset
join=Account.first.join_code
html=JoinGoldenController.renderer.new(http_host:'campfire.test',https:false,'rack.session'=>{},'action_dispatch.request.path_parameters'=>{controller:'users',action:'new',join_code:join}).render(template:'users/new',layout:false,assigns:{user:User.new})
join=Account.first.join_code
session=ActionDispatch::Integration::Session.new(Rails.application);session.host! 'campfire.test'
session.get "/join/#{join}";ActiveSupport::IsolatedExecutionState.clear
valid_get={status:session.response.status}
token=Nokogiri::HTML(session.response.body).at_css('meta[name="csrf-token"]')['content']
session.get '/join/wrong-code';ActiveSupport::IsolatedExecutionState.clear
wrong_get={status:session.response.status}
count=User.count
session.post "/join/#{join}",params:{user:{name:'Another David',email_address:'david@37signals.com',password:'secret123456'}},headers:{'X-CSRF-Token'=>token};ActiveSupport::IsolatedExecutionState.clear
duplicate={status:session.response.status,location:session.response.headers['Location'],user_delta:User.count-count}
session.post '/join/wrong-code',params:{user:{name:'Must not save',password:'secret123456'}},headers:{'X-CSRF-Token'=>token};ActiveSupport::IsolatedExecutionState.clear
wrong_post={status:session.response.status,user_delta:User.count-count}
session.post "/join/#{join}",params:{user:{name:'New Person',email_address:'new@37signals.com',password:'secret123456',role:'administrator'}},headers:{'X-CSRF-Token'=>token};ActiveSupport::IsolatedExecutionState.clear
user=User.find_by!(email_address:'new@37signals.com')
valid={status:session.response.status,location:session.response.headers['Location'],user_delta:User.count-count,role:user.role,room_ids:user.rooms.order(:id).pluck(:id),open_ids:Rooms::Open.order(:id).pluck(:id),session_count:user.sessions.count}
session.get "/join/#{join}";ActiveSupport::IsolatedExecutionState.clear
signed_get={status:session.response.status,location:session.response.headers['Location']}
puts JSON.pretty_generate(reference: ENV.fetch('PARITY_REFERENCE_SHA'),join:join,html:html,valid_get:valid_get,wrong_get:wrong_get,duplicate:duplicate,wrong_post:wrong_post,valid:valid,signed_get:signed_get)
warn "Rails joining oracle: 1 complete signup body; 6 HTTP cases with user, room and session state; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
