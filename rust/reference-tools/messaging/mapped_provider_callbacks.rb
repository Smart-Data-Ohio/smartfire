# Mapped PR headers, thread reply windows and real callbacks/jobs, without masks.
require 'json'
require 'net/http'
require 'stringio'
user = User.find(127326141)
Current.user = user
ActiveJob::Base.queue_adapter = :test
ActionController::Base.allow_forgery_protection = false
Rails.application.config.hosts.clear
FIXTURE_TOKEN = 'fixture-workspace-token'
ENV['GITHUB_TOKEN'] = FIXTURE_TOKEN
request = ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test', 'rack.input'=>StringIO.new))
request.cookie_jar.signed[:session_token] = user.sessions.where.not(two_factor_verified_at:nil).first!.token
headers = {'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! 'campfire.test'
renderer = ApplicationController.renderer.new(http_host:'campfire.test', https:false)
frames = []
ActionCable.server.define_singleton_method(:broadcast) { |stream, html, **| frames << {stream:stream, html:html} }
class MappedProviderHTTP
  def initialize(routes) = (@routes = routes)
  def get(path, headers)
    raise 'missing fixture credential' unless headers['Authorization'] == "Bearer #{FIXTURE_TOKEN}"
    response = Net::HTTPOK.new('1.1', '200', 'fixture')
    response.instance_variable_set(:@read, true)
    response.body = @routes.fetch(path).to_json
    response
  end
end
module MappedProviderNetwork
  def start(host, port, **options)
    raise 'unexpected network' unless host == 'api.github.com' && port == 443 && options[:use_ssl]
    yield Thread.current.fetch(:mapped_provider_http)
  end
end
Net::HTTP.singleton_class.prepend(MappedProviderNetwork)
groups = []
[false, true, nil].each_with_index do |privacy, mode|
 [4,16].each do |size|
  Current.user = user
  prefix = "mapped-provider-#{mode}-#{size}"
  pr = Github::PullRequest.create!(owner:'mapped-owner',repo:prefix,number:1,title:'Original <&> title',state:'open',private:privacy,fetched_at:Time.current,fetch_requested_at:Time.current,
    changed_files:{'files'=>[{'filename'=>'src/<&>.rs','status'=>'renamed','additions'=>'2','deletions'=>3}], 'total_count'=>3}.to_json)
  rooms = []; threads = []; messages = []; old = []
  size.times do |i|
   room = Room.create_for({type:'Rooms::Open',name:"#{prefix}-room-#{i}",creator:user},users:[user])
   thread = ChannelThread.create!(room:room,creator:user,name:'Mapped PR discussion')
   Github::PullRequestThread.create!(room:room,pull_request:pr,channel_thread:thread)
   root = room.messages.create!(creator:user,markdown_source:'Mapped root reference',client_message_id:"#{prefix}-root-#{i}")
   reply = thread.messages.create!(room:room,creator:user,markdown_source:'Mapped old reply',client_message_id:"#{prefix}-reply-#{i}")
   [root,reply].each do |m|
    m.update_columns(created_at:1.day.ago,updated_at:1.day.ago)
    Github::PullRequestReference.create!(message:m,pull_request:pr)
   end
   rooms << room; threads << thread; messages.concat([root,reply]); old << reply
  end
  fillers = Message::PAGE_SIZE.times.map do |i|
   threads.first.messages.create!(room:rooms.first,creator:user,markdown_source:'Newer reply',client_message_id:"#{prefix}-filler-#{i}")
  end
  messages.concat(fillers)
  ids = messages.map(&:id).join(','); tids = threads.map(&:id).join(','); rids = rooms.map(&:id).join(',')
  selects = {'rooms'=>"id IN (#{rids})",'memberships'=>"room_id IN (#{rids})",'channel_threads'=>"id IN (#{tids})",'messages'=>"id IN (#{ids})",'action_text_rich_texts'=>"record_type='Message' AND record_id IN (#{ids})",'github_pull_requests'=>"id=#{pr.id}",'github_pull_request_references'=>"github_pull_request_id=#{pr.id}",'github_pull_request_threads'=>"github_pull_request_id=#{pr.id}"}
  rows = selects.to_h { |table,where| [table,ActiveRecord::Base.connection.select_all("SELECT * FROM #{table} WHERE #{where} ORDER BY id").to_a] }
  initial_headers = threads.map { |thread| {thread_id:thread.id,html:renderer.render(partial:'github/pull_requests/thread_header',locals:{thread:thread,pull_request:pr})} }
  reply_cards = renderer.render(partial:'github/pull_requests/cards',locals:{message:Message.with_rendering_details.find(old.first.id)})
  windows = []
  path = "/rooms/#{rooms.first.id}/threads/#{threads.first.id}/messages"
  [['current',path],['before',"#{path}?before=#{fillers.first.id}"],['after',"#{path}?after=#{old.first.id}"]].each do |kind,url|
   windows << {kind:kind,path:url}
  end
  frames.clear; queries=[]
  observer=->(*args){p=args.last; queries << p[:sql] if !p[:cached] && p[:name]!='SCHEMA' && p[:sql].match?(/\A\s*(SELECT|WITH)\b/i)}
  attrs={title:'Updated <&> title',state:'merged',review_decision:'approved',check_status:'passing'}
  ActiveSupport::Notifications.subscribed(observer,'sql.active_record'){pr.update!(attrs)}
  callback_frames = frames.dup
  payload={'title'=>'Job <&> title','user'=>{'login'=>'mapped-author'},'state'=>'open','draft'=>true,'base'=>{'ref'=>'main','repo'=>{'private'=>privacy}},'head'=>{'ref'=>'feature','sha'=>'abc'},'updated_at'=>'2026-03-02T15:00:00Z','changed_files'=>0}
  routes={"/repos/mapped-owner/#{prefix}/pulls/1"=>payload,"/repos/mapped-owner/#{prefix}/pulls/1/reviews?per_page=100"=>[],"/repos/mapped-owner/#{prefix}/commits/abc/check-runs?per_page=100"=>{'check_runs'=>[]},"/repos/mapped-owner/#{prefix}/commits/abc/status"=>{'state'=>'pending','total_count'=>0}}
  routes["/repos/mapped-owner/#{prefix}/pulls/1/files?per_page=100"] = [{filename:'job/<&>.rs',status:'modified',additions:5,deletions:2}]
  Thread.current[:mapped_provider_http] = MappedProviderHTTP.new(routes)
  frames.clear
  Github::FetchPullRequestJob.perform_now(pr)
  groups << {privacy:privacy,size:size,rows:rows,initial_headers:initial_headers,reply_cards:reply_cards,old_reply_id:old.first.id,windows:windows,callback:{pull_request_id:pr.id,attributes:attrs,reads:queries.size,frames:callback_frames},job:{pull_request_id:pr.id,routes:routes,frames:frames.dup}}
 end
end
# Integration requests reset Current/executor state. Run them after all model
# operations rather than leaking request state into a later callback.
groups.each do |group|
 group[:windows].each do |window|
  browser.get(window[:path],headers:headers.dup)
  raise 'window request failed' unless browser.response.successful?
  window[:ids] = browser.response.body.scan(/data-message-id="(\d+)"/).flatten.map(&:to_i)
 end
 raise 'old reply reached current window' if group[:windows].first[:ids].include?(group[:old_reply_id])
end
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],groups:groups)+"\n")
puts "WS8bm2 mapped-provider Rails: #{groups.size} groups; #{groups.sum{|g|g[:initial_headers].size}} headers; #{groups.sum{|g|g[:windows].size}} reply windows; #{groups.sum{|g|g[:callback][:frames].size+g[:job][:frames].size}} callback/job frames"
