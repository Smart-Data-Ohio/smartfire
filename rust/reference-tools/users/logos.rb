require "json"
require "base64"
require "digest"
Rails.application.config.hosts.clear
Rails.logger=ActiveSupport::Logger.new($stderr)
JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"),"reference-tools/users/logos-source-hashes.json"))).each {|path,hash|raise "source drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest==hash}
user=User.find(127326141)
session=ActionDispatch::Integration::Session.new(Rails.application);session.host! "campfire.test"
request=ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST"=>"campfire.test","rack.input"=>StringIO.new))
request.cookie_jar.signed.permanent[:session_token]={value:user.sessions.first.token,httponly:true,same_site: :lax};session.cookies["session_token"]=request.cookie_jar[:session_token]
session.get "/account/edit";ActiveSupport::IsolatedExecutionState.clear
csrf=Nokogiri::HTML(session.response.body).at_css('meta[name="csrf-token"]')["content"]
account=Account.first;account.logo.destroy
cases=[nil,"moon.jpg","pixel.bmp"].flat_map do |file|
  if file
    session.patch "/account",params:{account:{logo:Rack::Test::UploadedFile.new(Rails.root.join("test/fixtures/files",file),file.end_with?("jpg") ? "image/jpeg" : "image/bmp")}},headers:{"X-CSRF-Token"=>csrf}
    ActiveSupport::IsolatedExecutionState.clear
    raise "upload #{file}: #{session.response.status}" unless session.response.status==302
  end
  [nil,"small","other"].map do |size|
    path="/account/logo#{size ? "?size=#{size}" : ''}"
    session.get path;ActiveSupport::IsolatedExecutionState.clear
    {file:file,size:size,status:session.response.status,body:Base64.strict_encode64(session.response.body),content_type:session.response.headers["Content-Type"],cache_control:session.response.headers["Cache-Control"],etag:session.response.headers["ETag"]}
  end
end
puts JSON.pretty_generate(reference:"d7c7de92",vips:Vips.version_string,cases:cases)
warn "Rails logos oracle: #{cases.size} complete PNG bodies with response/cache headers; libvips #{Vips.version_string}; reference d7c7de92"
