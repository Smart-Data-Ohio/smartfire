# Compact, unmasked digests of every real callback frame across Rails find_each batches.
require 'json'
require 'digest'
user = User.find(127326141); room = Room.find(699448326); Current.user = user
ActiveJob::Base.queue_adapter = :test
connection = ActiveRecord::Base.connection
cases = []
[['embed', 32767, true], ['linkedin', 2001, false], ['github', 32767, true]].each do |kind, count, suppressed|
 prefix = "bounded-#{kind}"
 model = if kind == 'github'
  Github::PullRequest.create!(owner:'bounded-owner', repo:'bounded-repo', number:1, title:'before', private:nil, fetched_at:Time.current)
 else
  url = kind == 'linkedin' ? 'https://www.linkedin.com/feed/update/urn:li:activity:900001' : 'https://bounded.example.test/embed'
  LinkEmbed.create!(normalized_url:url, title:'before', expires_at:1.day.from_now)
 end
 base = Message.maximum(:id)
 connection.execute("WITH RECURSIVE ids(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM ids WHERE n<#{count}) INSERT INTO messages (id,room_id,creator_id,client_message_id,embeds_suppressed,created_at,updated_at) SELECT #{base}+n,#{room.id},#{user.id},'#{prefix}-'||n,#{suppressed ? 1 : 0},'2026-03-01 16:00:00','2026-03-01 16:00:00' FROM ids")
 rows = {}
 if kind == 'github'
  connection.execute("INSERT INTO github_pull_request_references (message_id,github_pull_request_id,created_at,updated_at) SELECT id,#{model.id},'2026-03-01 16:00:00','2026-03-01 16:00:00' FROM messages WHERE id>#{base}")
  rows['github_pull_requests'] = connection.select_all("SELECT * FROM github_pull_requests WHERE id=#{model.id}").to_a
 else
  connection.execute("INSERT INTO link_embed_references (message_id,link_embed_id,position,url,created_at,updated_at) SELECT id,#{model.id},0,'#{model.normalized_url}','2026-03-01 16:00:00','2026-03-01 16:00:00' FROM messages WHERE id>#{base}")
  if kind == 'linkedin'
   sibling = LinkEmbed.create!(normalized_url:'https://www.linkedin.com/feed/update/urn:li:activity:900002',title:'stale sibling')
   opposite = LinkEmbed.create!(normalized_url:'https://bounded.example.test/opposite',title:'unrequested opposite')
   [sibling,opposite].each_with_index do |embed,i|
    connection.execute("INSERT INTO link_embed_references (message_id,link_embed_id,position,url,created_at,updated_at) SELECT id,#{embed.id},#{i+1},'#{embed.normalized_url}','2026-03-01 16:00:00','2026-03-01 16:00:00' FROM messages WHERE id>#{base}")
   end
  end
  rows['link_embeds'] = connection.select_all("SELECT * FROM link_embeds WHERE id IN (#{[model.id,defined?(sibling) && sibling&.id,defined?(opposite) && opposite&.id].compact.join(',')}) ORDER BY id").to_a
 end
 frames = 0; digest = Digest::SHA256.new; first = nil; last = nil
 ActionCable.server.define_singleton_method(:broadcast) do |stream,html,**|
  raise 'wrong stream' unless stream == "#{room.to_gid_param}:messages"
  frames += 1
  raise 'wrong frame order' unless html.include?("#{prefix}-#{frames}\"")
  first ||= html; last = html; digest << html << "\n"
 end
 before = ActiveJob::Base.queue_adapter.enqueued_jobs.size
 model.update!(title:'after')
 jobs = ActiveJob::Base.queue_adapter.enqueued_jobs.drop(before)
 raise 'missing frames' unless frames == count
 if kind == 'linkedin'
  raise 'fetches not deduplicated' unless jobs.count { |j| j[:job] == LinkEmbed::FetchJob } == 1
  raise 'opposite fetched' if opposite.reload.fetch_requested_at
 end
 cases << {kind:kind,count:count,suppressed:suppressed,base_id:base,model_id:model.id,rows:rows,frames:frames,sha256:digest.hexdigest,first:first,last:last,fetch_jobs:jobs.count { |j| j[:job] == LinkEmbed::FetchJob }}
 puts "WS8bm2 bounded Rails #{kind}: #{count} references; title=#{model.reload.title}; #{frames} ordered frames; #{cases.last[:fetch_jobs]} fetch jobs; sha256=#{digest.hexdigest}"
end
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],cases:cases)+"\n")
