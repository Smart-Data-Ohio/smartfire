require 'json'
Rails.application.config.hosts.clear
Rails.logger=ActiveSupport::Logger.new($stderr)
u=User.find(127326141);b=ActionDispatch::Integration::Session.new(Rails.application);b.host! 'campfire.test'
r=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
r.cookie_jar.signed.permanent[:session_token]={value:u.sessions.first.token,httponly:true,same_site: :lax};b.cookies['session_token']=r.cookie_jar[:session_token]
rows=[['initials',"/users/#{User.find(712064548).avatar_token}/avatar"],['invalid','/users/not-a-valid-token/avatar']].map do |name,path|
 b.get path,headers:{'Accept'=>'image/svg+xml'};ActiveSupport::IsolatedExecutionState.clear
 h={name:name,path:path,status:b.response.status};h[:body]=b.response.body if name=='initials';h
end
raise 'original initials changed' unless Nokogiri::XML(rows[0][:body]).css('text').map(&:text).map(&:strip)==['K']
raise 'original not_found changed' unless rows[1][:status]==404
puts JSON.pretty_generate(reference:ENV.fetch('PARITY_REFERENCE_SHA'),cases:rows)
warn 'Rails original avatar oracle: 2 routed assertions; exact Kevin initials SVG'
