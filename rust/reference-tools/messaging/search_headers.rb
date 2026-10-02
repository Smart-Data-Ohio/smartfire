require 'json'
require 'stringio'
require 'rack/deflater'
ActionController::Base.allow_forgery_protection=true
Rails.application.config.hosts.clear
user=User.find(127326141)
request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
request.cookie_jar.signed[:session_token]=user.sessions.where.not(two_factor_verified_at:nil).first!.token
cookie="session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"
rows=[]
{'bare'=>Rails.application, 'rack'=>Rack::Deflater.new(Rails.application)}.each do |stack, app|
browser=ActionDispatch::Integration::Session.new(app)
browser.host! 'campfire.test'
['/searches','/searches?q=missingheaderquery','/users/127326141'].each do |path|
 [nil,'text/html','text/vnd.turbo-stream.html, text/html'].each do |accept|
  [false,true].each do |frame|
   headers={'Cookie'=>cookie}
   headers['Accept']=accept if accept
   headers['Turbo-Frame']='test-frame' if frame
   browser.get(path,headers:headers)
   label="#{rows.size}-#{path.split('/')[1]}"
   File.write(File.join(File.dirname(ARGV.fetch(0)),"#{label}.html"),browser.response.body)
   body=browser.response.body
   section=body[/<section id="message-area".*?<\/section>/m] if path.start_with?('/searches')
   rows << {stack:stack, path:path, accept:accept, frame:frame, status:browser.response.status, headers:%w[Vary Content-Type Cache-Control X-Frame-Options].to_h{|h|[h,browser.response.headers[h]]}, section:(path.include?('?') ? section : nil), search_token_inputs:section&.scan(/name="authenticity_token"/)&.size}
  end
 end
end
end
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:'d7c7de92',vapid_public_key:ENV['VAPID_PUBLIC_KEY'],cases:rows)+"\n")
source=File.join(Gem.loaded_specs.fetch('turbo-rails').full_gem_path,'app/controllers/turbo/frames/frame_request.rb')
puts "WS8bm2 Rails search header probe: #{rows.size} bare/wrapped full/frame requests; #{rows.map{|r|r[:headers]['Vary']}.uniq.inspect}"
File.write(File.join(File.dirname(ARGV.fetch(0)),'turbo_frame_gem.rb'),File.read(source))
