require "json"
require "digest"
Rails.logger = ActiveSupport::Logger.new($stderr)
%w[app/views/users/profiles/_appearance.html.erb app/helpers/users/profiles_helper.rb].each do |file|
  expected = JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"), "reference-tools/users/profiles-source-hashes.json"))).fetch(file)
  raise "source drift: #{file}" unless Digest::SHA256.file(Rails.root.join(file)).hexdigest == expected
end
class AppearanceGoldenController < ApplicationController
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : "GLOBAL"
  end
end
user=User.find(127326141)
cases=[{theme:"system",text_size:"default",time_zone:nil}, {theme:"dark",text_size:"larger",time_zone:"America/New_York"}, {theme:"light",text_size:"small",time_zone:"Pacific Time (US & Canada)"}, {theme:"neon",text_size:"huge",time_zone:"Narnia"}].map do |attrs|
  user.assign_attributes(attrs)
  user.valid?
  Current.user=user
  html=AppearanceGoldenController.renderer.new(http_host:"campfire.test",https:false,"rack.session"=>{},"action_dispatch.content_security_policy_nonce_generator"=>->(_){"NONCE"}).render(partial:"users/profiles/appearance",assigns:{user:user})
  {attributes:attrs,errors:user.errors.to_hash,html:html}
end
Current.reset
choices=ActiveSupport::TimeZone.all.map{|z|[z.to_s,z.tzinfo.identifier]}.uniq{|(_,identifier)|identifier}
puts JSON.pretty_generate(reference:"d7c7de92",choices:choices,cases:cases)
warn "Rails appearance oracle: #{cases.size} bodies, #{choices.size} zone choices; reference d7c7de92"
