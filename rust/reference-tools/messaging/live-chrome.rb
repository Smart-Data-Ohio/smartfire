# The owning live layout must supply real icon names and the viewer's recent-search rows.
require 'json'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
rows=[]
users=[User.find(127326141),User.find(149087659)]
users.each { |user| user.searches.delete_all }
users.each_with_index do |user,idx|
  13.times do |i|
    user.searches.insert_all!([{query:"Chrome #{idx}/#{i} <&> \"quote\"",created_at:Time.current+i,updated_at:Time.current+i}])
  end
end
inputs=Search.where(user:users).order(:id).map { |s| {id:s.id,user_id:s.user_id,query:s.query,created_at:s.created_at.iso8601(6),updated_at:s.updated_at.iso8601(6)} }
[[],['zeta_chrome','alpha_chrome']].each do |custom|
 custom.each { |name| WorkspaceIcon.insert_all!([{creator_id:127326141,name:name,title:"Chrome #{name}",created_at:Time.current,updated_at:Time.current}]) }
 Icons.expire_custom_cache!
 users.each do |user|
  Current.reset; Current.user=user
  renderer=ApplicationController.renderer.new(http_host:'campfire.test',https:false)
  searches=user.searches.ordered.limit(10)
  rows << {custom:custom,user_id:user.id,brand_meta:renderer.render(inline:'<%= tag.meta name: "brand-icon-names", content: Icons.client_icon_names.join(",") %>',layout:false),recent_html:renderer.render(partial:'searches/dropdown_recents',locals:{searches:searches}),search_ids:searches.map(&:id)}
 end
end
Current.reset
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],inputs:inputs,rows:rows)+"\n")
puts "WS8bm live chrome: #{rows.size} icon/recent-search components; ordered custom icons, scoped latest ten, escaped HTML/URLs; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
