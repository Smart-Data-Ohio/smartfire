# Actual Fizzy/X callbacks and network fetch jobs for roots and replies outside
# both current windows. Network fakes reject every unlisted endpoint.
require 'json'
require 'net/http'
user = User.find(127326141); room = Room.find(699448326); Current.user = user
ActiveJob::Base.queue_adapter = :test
FIXTURE_TOKEN = 'fixture-fizzy-token'
ENV['FIZZY_API_BASE_URL'] = 'https://app.fizzy.do'
account = FizzyConnectedAccount.find_or_initialize_by(user:user)
account.update!(fizzy_account_id:'fixture-workspace',access_token:FIXTURE_TOKEN)
frames = []
ActionCable.server.define_singleton_method(:broadcast) { |stream,html,**| frames << {stream:stream,html:html} }
class OlderOwnerHTTP
  def initialize(host,path,status,body) = (@host,@path,@status,@body = host,path,status,body)
  def request(request)
    raise 'unexpected path' unless request.path == @path
    if @host == 'app.fizzy.do'
      raise 'missing fixture credential' unless request['Authorization'] == "Bearer #{FIXTURE_TOKEN}"
    end
    response = Net::HTTPResponse::CODE_TO_OBJ.fetch(@status.to_s).new('1.1',@status.to_s,'fixture')
    response.instance_variable_set(:@read,true)
    encoded = @body.to_json
    response.body = encoded
    response.define_singleton_method(:body) { encoded }
    # The X fetcher consumes the streamed response, while Fizzy reads its body.
    response.define_singleton_method(:read_body) { |*_, &block| block ? block.call(encoded) : encoded }
    block_given? ? yield(response) : response
  end
end
module OlderOwnerNetwork
  def start(host,port,**options)
    expected = Thread.current.fetch(:older_owner_route)
    raise 'unexpected network' unless host == expected[:host] && port == 443 && options[:use_ssl]
    yield OlderOwnerHTTP.new(host,expected[:path],expected[:status],expected[:body])
  end
end
Net::HTTP.singleton_class.prepend(OlderOwnerNetwork)
groups=[]
['fizzy','twitter'].each_with_index do |kind,mode|
 [4,16].each do |size|
  Current.user = user
  account.reload.update!(disconnected_reason:nil)
  user.reload
  prefix="older-owner-#{kind}-#{size}"
  thread=ChannelThread.create!(room:room,creator:user,name:prefix)
  model = kind == 'fizzy' ? Fizzy::Card.create!(account_id:'fixture-workspace',number:size) : Twitter::Post.create!(post_id:"9000000000000#{size}",url:"https://x.com/fixture/status/9000000000000#{size}",text:'Old text',author_name:'Original',fetched_at:Time.current)
  sibling = kind == 'fizzy' ? Fizzy::Card.create!(account_id:'a-fixture',number:size) : Twitter::Post.create!(post_id:"8000000000000#{size}",url:"https://x.com/fixture/status/8000000000000#{size}",text:'Sibling text',fetched_at:Time.current)
  messages = size.times.map do |i|
   m=room.messages.create!(creator:user,thread:i.odd? ? thread : nil,markdown_source:'Older owner reference',client_message_id:"#{prefix}-#{i}")
   m.update_columns(created_at:1.day.ago,updated_at:1.day.ago,embeds_suppressed:i==size-1)
   [model,sibling].each do |card|
    if kind == 'fizzy'
     Fizzy::CardReference.create!(message:m,card:card)
    else
     Twitter::PostReference.create!(message:m,post:card)
    end
   end
   m
  end
  fillers = [nil,thread].flat_map do |t|
   Message::PAGE_SIZE.times.map { |i| room.messages.create!(creator:user,thread:t,markdown_source:'Newer filler',client_message_id:"#{prefix}-filler-#{t&.id || 'root'}-#{i}") }
  end
  raise 'old roots reached window' unless (room.root_messages.last_page.pluck(:id)&messages.map(&:id)).empty?
  raise 'old replies reached window' unless (thread.messages.last_page.pluck(:id)&messages.map(&:id)).empty?
  ids=(messages+fillers).map(&:id).join(',')
  selects={'channel_threads'=>"id=#{thread.id}",'messages'=>"id IN (#{ids})",'action_text_rich_texts'=>"record_type='Message' AND record_id IN (#{ids})"}
  if kind == 'fizzy'
   selects.merge!('fizzy_cards'=>"id IN (#{model.id},#{sibling.id})",'fizzy_card_references'=>"message_id IN (#{ids})")
  else
   selects.merge!('twitter_posts'=>"id IN (#{model.id},#{sibling.id})",'twitter_post_references'=>"message_id IN (#{ids})")
  end
  rows=selects.to_h{|table,where|[table,ActiveRecord::Base.connection.select_all("SELECT * FROM #{table} WHERE #{where} ORDER BY id").to_a]}
  streams=["#{room.to_gid_param}:messages","#{thread.to_gid_param}:messages"]
  frames.clear; queries=[]
  observer=->(*args){p=args.last;queries << p[:sql] if !p[:cached] && p[:name]!='SCHEMA' && p[:sql].match?(/\A\s*(SELECT|WITH)\b/i)}
  ActiveSupport::Notifications.subscribed(observer,'sql.active_record') do
   kind == 'fizzy' ? model.broadcast_card_updates : model.update!(text:'Updated <&> text')
  end
  callback={reads:queries.size,frames:frames.select{|f|streams.include?(f[:stream])}.dup}
  # A rollback must publish nothing, including after-commit callbacks.
  frames.clear
  if kind == 'twitter'
   Twitter::Post.transaction { model.update!(text:'Rolled back secret'); raise ActiveRecord::Rollback }
  else
   Fizzy::CardCache.transaction do
    cache=Fizzy::CardCache.for_viewer(card:model,user:user)
    cache.update!(payload:{title:'Rolled back secret'})
    raise ActiveRecord::Rollback
   end
  end
  raise 'rollback broadcast' unless frames.empty?
  jobs=[]
  [200,403,401].each do |status|
   status = 404 if kind == 'twitter' && status == 403
   body = if kind == 'fizzy'
    {id:'fixture-card',title:'Private job <&> title',number:size}
   elsif status == 200
    {tweet:{id:model.post_id,text:'Job <b>text</b> <&>',author:{screen_name:'fixture',name:'Job author'},created_timestamp:1772467200,likes:3,replies:2,retweets:1}}
   else
    {code:status}
   end
   route={host:kind == 'fizzy' ? 'app.fizzy.do' : 'api.fxtwitter.com',path:kind == 'fizzy' ? "/fixture-workspace/cards/#{size}.json" : "/fixture/status/#{model.post_id}",status:status,body:body}
   Thread.current[:older_owner_route] = route
   frames.clear
   kind == 'fizzy' ? Fizzy::FetchCardJob.perform_now(model,user) : Twitter::FetchPostJob.perform_now(model)
   record = kind == 'fizzy' ? Fizzy::CardCache.find_by!(card:model,user:user) : model.reload
   jobs << {route:route,fetch_error:record.fetch_error,frames:frames.select{|f|streams.include?(f[:stream])}.dup}
  end
  groups << {kind:kind,size:size,model_id:model.id,thread_id:thread.id,rows:rows,old_ids:messages.map(&:id),callback:callback,jobs:jobs}
 end
end
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],groups:groups)+"\n")
puts "WS8bm2 older-owner Rails: #{groups.size} groups; #{groups.sum{|g|g[:callback][:frames].size}} callback frames; #{groups.sum{|g|g[:jobs].size}} network jobs; #{groups.sum{|g|g[:jobs].sum{|j|j[:frames].size}}} job frames; 4 silent rollbacks"
