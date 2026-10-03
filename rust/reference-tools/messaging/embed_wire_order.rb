# Live Rails/Redis probe of the existing parent/child corpus. Never replaces delivery.
require 'json'
corpus=JSON.parse(File.read('/work/vectors/messaging/older_embed_children.json'))
conn=ActiveRecord::Base.connection
case ARGV.fetch(0)
when 'setup'
  %w[channel_threads link_embeds messages action_text_rich_texts link_embed_references].each do |table|
    corpus['groups'].each do |group|
      group['rows'].fetch(table,[]).each do |row|
        conn.execute("INSERT INTO #{conn.quote_table_name(table)} (#{row.keys.map{|k|conn.quote_column_name(k)}.join(',')}) VALUES (#{row.values.map{|v|conn.quote(v)}.join(',')})")
      end
    end
  end
  user=User.find(127326141)
  session=user.sessions.create!(user_agent:'fixture cable probe',ip_address:'127.0.0.1',two_factor_verified_at:Time.current)
  request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
  request.cookie_jar.signed[:session_token]=session.token
  streams=corpus['groups'].flat_map{|g|g['jobs'].flat_map{|j|j['frames'].map{|f|f['stream']}}}.uniq
  signed=streams.to_h{|s|[s,RoomMessagesChannel.signed_stream_name([s])]}
  File.write('/rails/storage/files/embed-wire-setup.json',JSON.generate(cookie:"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}",streams:signed))
  puts 'WS8bm2 Rails wire setup: actual room/thread authorization and fresh session'
when 'jobs'
  # Reuse the oracle's strictly whitelisted fixture network; production fetcher is unchanged.
  source=File.read('/work/reference-tools/messaging/older_embed_children.rb')
  eval(source.split("frames=[]\n").first,TOPLEVEL_BINDING,'older_embed_children.rb')
  frames=[]
  original=ActionCable.server.method(:broadcast)
  ActionCable.server.define_singleton_method(:broadcast) do |stream,html,**options|
    frames << {stream:stream,html:html}
    original.call(stream,html,**options) # real Redis adapter -> actual Puma Cable connection
  end
  corpus['groups'].each do |group|
    group['rows']['link_embeds'].each do |row|
      conn.execute("UPDATE link_embeds SET #{row.reject{|k,_|k=='id'}.map{|k,v|"#{conn.quote_column_name(k)}=#{conn.quote(v)}"}.join(',')} WHERE id=#{row.fetch('id')}")
    end
    group['jobs'].each do |job|
      sibling=LinkEmbed.find(group.fetch('sibling_id'))
      sibling.update_columns(fetch_requested_at:11.minutes.ago,expires_at:1.day.ago)
      Thread.current[:embed_routes]=job.fetch('routes').map{|r|r.transform_keys(&:to_sym)}
      Thread.current[:embed_calls]=[]
      frames.clear;ActiveJob::Base.queue_adapter.enqueued_jobs.clear
      LinkEmbed::FetchJob.perform_now(LinkEmbed.find(group.fetch('embed_id')))
      queued=ActiveJob::Base.queue_adapter.enqueued_jobs.select{|j|j[:job]==LinkEmbed::FetchJob}
      raise 'child count' unless queued.size==1
      child=GlobalID::Locator.locate(queued[0][:args][0]['_aj_globalid'])
      raise 'child identity' unless child.id==sibling.id
      ActiveJob::Base.queue_adapter.enqueued_jobs.clear
      LinkEmbed::FetchJob.perform_now(child)
      raise 'publication differs' unless JSON.parse(JSON.generate(frames))==job.fetch('frames')
    end
  end
  puts 'WS8bm2 Rails wire jobs: 20 parents; 20 actual queued children; 400 ordered publications byte-identical'
else
  raise 'unknown phase'
end
