# Source-backed composite-key probe. Provider fetching is deliberately not exercised here.
require 'json'
Current.user=User.find(127326141)
room=Room.find(699448326)
source=room.messages.create!(creator:Current.user,markdown_source:'cached quote source',client_message_id:'cache-source')
message=room.messages.create!(creator:Current.user,markdown_source:"/rooms/#{room.id}/@#{source.id}",client_message_id:'cache-root')
poll=message.create_poll!(multiple:false,anonymous:false)
poll.poll_options.create!(label:'A',position:0)
poll.poll_options.create!(label:'B',position:1)
pin=MessagePin.create!(message:,room:,pinner:Current.user)
pr=Github::PullRequest.create!(owner:'cache-owner',repo:'cache-repo',number:42,private:true,updated_at:Time.current-3.hours)
Github::PullRequestReference.create!(message:,pull_request:pr)
fizzy=Fizzy::Card.create!(account_id:'cache-account',number:17,updated_at:Time.current-2.hours)
Fizzy::CardReference.create!(message:,card:fizzy)
helper=ApplicationController.helpers
renderer=ApplicationController.renderer.new(http_host:'campfire.test',https:false)
cases=[]
snapshot=->(label) do
  m=Message.with_rendering_details.find(message.id)
  expanded_zones=%w[UTC Hawaii].to_h{|zone|[zone,Time.use_zone(zone){ActiveSupport::Cache.expand_cache_key(helper.message_with_pr_cards_cache_key(Message.with_rendering_details.find(message.id)))}]}
  cases<<{label:,expanded:expanded_zones.fetch('UTC'),expanded_zones:,github:renderer.render(partial:'github/pull_requests/cards',locals:{message:m}),fizzy:renderer.render(partial:'fizzy/cards/cards',locals:{message:m})}
end
snapshot.call('initial')
source.update_columns(edited_at:Time.current+1.second)
snapshot.call('source_edit')
source.creator.update_columns(name:'David renamed')
snapshot.call('source_author_rename')
room.update_columns(name:'Quiet renamed')
snapshot.call('source_room_rename')
poll.update_columns(updated_at:Time.current+2.seconds)
snapshot.call('poll_only')
pin.destroy!
snapshot.call('unpin')
# Restore the initial fixture facts; the oracle cases above record independent transitions.
source.update_columns(edited_at:nil)
source.creator.update_columns(name:'David')
room.update_columns(name:'Quiet Corner')
poll.update_columns(updated_at:Time.current)
pin=MessagePin.create!(message:,room:,pinner:Current.user)
selects={'messages'=>"id IN (#{source.id},#{message.id})",'action_text_rich_texts'=>"record_type='Message' AND record_id IN (#{source.id},#{message.id})",'message_references'=>"message_id=#{message.id}",'polls'=>"id=#{poll.id}",'poll_options'=>"poll_id=#{poll.id}",'message_pins'=>"message_id=#{message.id}",'github_pull_requests'=>"id=#{pr.id}",'github_pull_request_references'=>"message_id=#{message.id}",'fizzy_cards'=>"id=#{fizzy.id}",'fizzy_card_references'=>"message_id=#{message.id}"}
rows=selects.to_h{|table,where|[table,ActiveRecord::Base.connection.select_all("SELECT * FROM #{table} WHERE #{where} ORDER BY id").to_a]}
nil_names=[['David',nil],['Joe','Some room']]
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],rows:,message_id:message.id,source_id:source.id,poll_id:poll.id,pin_id:pin.id,cases:,nil_names_inspect:nil_names.sort.inspect,nil_names_digest:Digest::SHA256.hexdigest(nil_names.sort.inspect))+"\n")
puts "WS8bm2 root cache Rails oracle: #{cases.size} composite keys; #{cases.size*2} provider frame containers; #{rows.size} fixture tables"
