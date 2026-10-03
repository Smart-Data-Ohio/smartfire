# Actual ActiveJob adapter failures during the real after-commit card callback.
# No external queue or network is used. Rust's atomic SQLite contract is a separate
# decision; these observations must not be substituted for its committed output.
require 'json'
require_relative 'oracle-database'
Rails.application.routes.default_url_options.merge!(host: 'campfire.test', protocol: 'http')
Icons.custom_icons
Icons.instance_variable_set(:@custom_cache_at, Float::INFINITY)
class WS8RejectingAdapter
  attr_reader :calls, :queued
  def initialize(mode, fail_at)
    @mode, @fail_at, @calls, @queued = mode, fail_at, [], []
  end
  def enqueue(job)
    raise 'unlisted owner job' unless job.is_a?(LinkEmbed::FetchJob)
    @calls << job.arguments.first.id
    if @calls.length == @fail_at
      case @mode
      when 'return_false' then return false
      when 'enqueue_error' then raise ActiveJob::EnqueueError, 'fixture adapter refusal'
      when 'runtime_error' then raise 'fixture adapter write failure'
      end
    end
    @queued << job.arguments.first.id
  end
  def enqueue_at(*) = raise('unexpected delayed fixture job')
end
frames = []
ActionCable.server.define_singleton_method(:broadcast) { |stream, html, **| frames << {stream:, html:} }
source = JSON.parse(File.read('/work/vectors/messaging/older_embed_jobs.json'))
Current.user = User.find(127326141)
groups = []
source.fetch('groups').each do |group|
  conn = ActiveRecord::Base.connection
  group.fetch('rows').each do |table, rows|
    rows.each do |row|
      conn.execute("INSERT INTO #{conn.quote_table_name(table)} (#{row.keys.map { |k| conn.quote_column_name(k) }.join(',')}) VALUES (#{row.values.map { |v| conn.quote(v) }.join(',')})")
    end
  end
  second = LinkEmbed.find(group.fetch('opposite_id'))
  second.update_columns(normalized_url: group['kind'] == 'linkedin' ? "https://www.linkedin.com/feed/update/urn:li:activity:#{93000 + group['size']}" : "https://embed.example.test/second-#{group['size']}")
  # Keep raw reference URLs honest when turning the cross-provider row into a second sibling.
  LinkEmbedReference.where(link_embed: second).update_all(url: second.normalized_url)
  rows = group.fetch('rows').to_h { |table, original| ids = original.map { |r| r.fetch('id') }; [table, conn.select_all("SELECT * FROM #{table} WHERE id IN (#{ids.join(',')}) ORDER BY id").to_a] }
  reset = MessagingOracleDatabase.scenarios(ARGV.fetch(0))
  cases = []
  %w[return_false enqueue_error runtime_error].each do |mode|
    [1, 2].each do |fail_at|
      reset.call do
        conn = ActiveRecord::Base.connection
        adapter = WS8RejectingAdapter.new(mode, fail_at)
        LinkEmbed::FetchJob.queue_adapter = adapter
        Rails.cache.clear
        frames.clear
        error = nil
        reads = []
        observer = ->(*args) { p = args.last; reads << p[:sql] if !p[:cached] && p[:name] != 'SCHEMA' && p[:sql].match?(/\A\s*(SELECT|WITH)\b/i) }
        ActiveSupport::Notifications.subscribed(observer, 'sql.active_record') do
          begin
            LinkEmbed.find(group.fetch('embed_id')).update!(title: 'After adapter refusal')
          rescue StandardError => e
            raise unless e.message.include?('fixture adapter write failure')
            error = {class: e.class.name, message: e.message}
          end
        end
        raise 'adapter failure path not exercised' unless adapter.calls.length >= fail_at
        cases << {mode:, fail_at:, calls: adapter.calls, queued: adapter.queued, error:, reads: reads.length,
                  state: rows.to_h { |table, original| [table, conn.select_all("SELECT * FROM #{table} WHERE id IN (#{original.map { |r| r.fetch('id') }.join(',')}) ORDER BY id").to_a] },
                  frames: frames.select { |f| ["#{Room.find(699448326).to_gid_param}:messages", "#{ChannelThread.find(group['thread_id']).to_gid_param}:messages"].include?(f[:stream]) }.dup}
      end
    end
  end
  groups << group.slice('kind', 'size', 'embed_id', 'sibling_id', 'opposite_id', 'thread_id').merge(rows:, cases:)
end
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: 'd7c7de92', groups:) + "\n")
puts "WS8bm2 adapter rejection Rails: #{groups.sum { |g| g[:cases].length }} actual callback/adapter executions; complete rows, enqueues, errors and ordered broadcasts"
