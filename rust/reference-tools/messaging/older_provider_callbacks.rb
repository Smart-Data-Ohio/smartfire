# Actual after-update callbacks and FetchPullRequestJob for references older than PAGE_SIZE.
require 'json'
require 'net/http'
Current.user = user = User.find(127326141)
room = Room.find(699448326)
ActiveJob::Base.queue_adapter = :test
FIXTURE_TOKEN = 'fixture-workspace-token'
ENV['GITHUB_TOKEN'] = FIXTURE_TOKEN
frames = []
ActionCable.server.define_singleton_method(:broadcast) { |stream, html, **| frames << { stream: stream, html: html } }
class OlderProviderHTTP
  def initialize(routes) = (@routes = routes)
  def get(path, headers)
    raise 'missing fixture credential' unless headers['Authorization'] == "Bearer #{FIXTURE_TOKEN}"
    body = @routes.fetch(path)
    response = Net::HTTPOK.new('1.1', '200', 'fixture')
    response.instance_variable_set(:@read, true)
    response.body = body.to_json
    response
  end
end
module OlderProviderNetwork
  def start(host, port, **options)
    raise 'unexpected network' unless host == 'api.github.com' && port == 443 && options[:use_ssl]
    yield Thread.current.fetch(:older_provider_http)
  end
end
Net::HTTP.singleton_class.prepend(OlderProviderNetwork)
groups = []
[false, true, nil].each_with_index do |privacy, mode|
 [4,16].each do |size|
  Current.user = user
  prefix = "older-provider-#{mode}-#{size}"
  pr = Github::PullRequest.create!(owner:'older-owner', repo:prefix, number:1, title:'Old title', state:'open', private:privacy, fetched_at:Time.current, fetch_requested_at:Time.current)
  generic = LinkEmbed.create!(normalized_url:"https://old.example.test/#{prefix}", title:'Old embed', fetched_at:Time.current, expires_at:1.day.from_now)
  linkedin = LinkEmbed.create!(normalized_url:"https://www.linkedin.com/feed/update/urn:li:activity:#{mode+100}#{size}", title:'Old LinkedIn', fetched_at:Time.current, expires_at:1.day.from_now)
  messages = (0...size).map do |i|
   m = room.messages.create!(creator:user, markdown_source:"#{prefix} reference", client_message_id:"#{prefix}-#{i}")
   m.update_columns(created_at:1.day.ago, updated_at:1.day.ago, embeds_suppressed:i==size-1)
   Github::PullRequestReference.create!(message:m, pull_request:pr)
   [generic, linkedin].each_with_index { |embed, position| LinkEmbedReference.create!(message:m,link_embed:embed,url:"#{embed.normalized_url}#reference-#{i}",position:position) }
   m
  end
  fillers=(0...Message::PAGE_SIZE).map { |i| room.messages.create!(creator:user,markdown_source:'newer filler',client_message_id:"#{prefix}-filler-#{i}") }
  raise 'old roots reached current window' unless (room.messages.last_page.pluck(:id) & messages.map(&:id)).empty?
  ids=(messages+fillers).map(&:id).join(',')
  selects={'messages'=>"id IN (#{ids})",'action_text_rich_texts'=>"record_type='Message' AND record_id IN (#{ids})",'github_pull_requests'=>"id=#{pr.id}",'github_pull_request_references'=>"github_pull_request_id=#{pr.id}",'link_embeds'=>"id IN (#{generic.id},#{linkedin.id})",'link_embed_references'=>"link_embed_id IN (#{generic.id},#{linkedin.id})"}
  rows=selects.to_h { |table, where| [table, ActiveRecord::Base.connection.select_all("SELECT * FROM #{table} WHERE #{where} ORDER BY id").to_a] }
  steps=[]
  operations=[['github',pr,{title:'Fresh <&> title',state:'merged',review_decision:'approved',check_status:'passing'}],['embed',generic,{title:'Fresh <&> embed',description:'Description "quoted"',site_name:'Site',image_url:'https://images.example.test/image'}],['linkedin',linkedin,{title:'Fresh LinkedIn',description:'Fresh post'}],['negative',generic,{fetch_error:'HTTP 404'}],['empty',generic,{title:nil,description:nil,site_name:nil,image_url:nil,fetch_error:nil}]]
  operations.each do |kind, model, attributes|
   frames.clear; queries=[]
   observer=->(*args) { p=args.last; queries << p[:sql] if !p[:cached] && p[:name]!='SCHEMA' && p[:sql].match?(/\A\s*(SELECT|WITH)\b/i) }
   ActiveSupport::Notifications.subscribed(observer,'sql.active_record') { model.update!(attributes) }
   steps << {kind:kind,id:model.id,attributes:attributes,reads:queries.size,frames:frames.select { |f| f[:stream]=="#{room.to_gid_param}:messages" }.dup}
  end
  payload={'title'=>'Job <&> title','user'=>{'login'=>'job-author','avatar_url'=>'https://images.example.test/avatar'},'state'=>'open','draft'=>true,'merged_at'=>nil,'base'=>{'ref'=>'main','repo'=>{'private'=>privacy}},'head'=>{'ref'=>'feature','sha'=>'abc'},'html_url'=>"https://github.com/older-owner/#{prefix}/pull/1",'updated_at'=>'2026-03-02T15:00:00Z','changed_files'=>0}
  routes={"/repos/older-owner/#{prefix}/pulls/1"=>payload,"/repos/older-owner/#{prefix}/pulls/1/reviews?per_page=100"=>[],"/repos/older-owner/#{prefix}/commits/abc/check-runs?per_page=100"=>{'check_runs'=>[]},"/repos/older-owner/#{prefix}/commits/abc/status"=>{'state'=>'pending','total_count'=>0}}
  Thread.current[:older_provider_http] = OlderProviderHTTP.new(routes)
  frames.clear
  Github::FetchPullRequestJob.perform_now(pr)
  groups << {privacy:privacy,size:size,rows:rows,old_ids:messages.map(&:id),steps:steps,job:{pull_request_id:pr.id,routes:routes,frames:frames.select { |f| f[:stream]=="#{room.to_gid_param}:messages" }.dup}}
 end
end
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],groups:groups)+"\n")
puts "WS8bm2 older-provider Rails oracle: #{groups.size} groups; #{groups.sum { |g| g[:steps].size }} real updates; #{groups.size} real fetch jobs; #{groups.sum { |g| g[:steps].sum { |s| s[:frames].size }+g[:job][:frames].size }} socket frames; all roots outside 40-message windows"
