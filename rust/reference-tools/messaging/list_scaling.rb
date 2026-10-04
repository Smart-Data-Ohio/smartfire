# Complete real visible Files sections and mention JSON, with physical HTTP reads.
require 'json'
require 'action_dispatch/testing/integration'
require_relative 'oracle-database'
ActionController::Base.allow_forgery_protection = false
ActiveJob::Base.queue_adapter = :test
Rails.application.config.hosts.clear
Icons.custom_icons
Icons.instance_variable_set(:@custom_cache_at, Float::INFINITY)
source = JSON.parse(File.read('/work/vectors/messaging/links_files.json'))
reset = MessagingOracleDatabase.scenarios(ARGV.fetch(0))
def insert_rows(rows)
 c = ActiveRecord::Base.connection
 rows.each do |table, records|
  records.each { |r| c.execute("INSERT INTO #{c.quote_table_name(table)} (#{r.keys.map { |k| c.quote_column_name(k) }.join(',')}) VALUES (#{r.values.map { |v| c.quote(v) }.join(',')})") }
 end
end
groups = []
[4,16].each do |size|
 %w[UTC America/New_York].each do |zone|
  reset.call do
   rows = Marshal.load(Marshal.dump(source.fetch('rows')))
   rows['active_storage_attachments'] = rows.fetch('active_storage_attachments').first(size)
   rows['drive_attachments'] = rows.fetch('drive_attachments').first(size)
   c = ActiveRecord::Base.connection
   template = c.select_one('SELECT * FROM users WHERE id=127326141')
   rows['users'] = size.times.map do |index|
    template.merge('id'=>918500+index,'name'=>index < size-2 ? "Scaling member #{index.to_s.rjust(2,'0')}" : 'Scaling twin',
      'email_address'=>"fixture-visible-#{index}@list.test",'role'=>0,'status'=>0,'time_zone'=>'UTC')
   end
   rows['memberships'] += rows['users'].map.with_index do |user,index|
    rows['memberships'].first.merge('id'=>918600+index,'user_id'=>user.fetch('id'))
   end
   insert_rows(rows)
   user = User.find(127326141)
   user.update_columns(time_zone:zone)
   request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
   request.cookie_jar.signed[:session_token]=user.sessions.where.not(two_factor_verified_at:nil).first!.token
   headers={'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}
   browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test'
   paths=["/rooms/#{source.fetch('room_id')}/files","/rooms/#{source.fetch('room_id')}/files?type=documents", "/autocompletable/users?room_id=#{source.fetch('room_id')}&query=Scaling"]
   cases=paths.map do |path|
    observations=2.times.map do |pass|
     reads=[]
     observer=->(*args) { p=args.last;reads<<p[:sql] if !p[:cached] && p[:name]!='SCHEMA' && p[:sql].match?(/\A\s*(SELECT|WITH)\b/i) }
     accept=path.start_with?('/autocompletable') ? 'application/json' : 'text/html'
     ActiveSupport::Notifications.subscribed(observer,'sql.active_record') { browser.get(path,headers:headers.merge('Accept'=>accept)) }
     body=browser.response.body
     body=body[/<section class="room-files".*?<\/section>/m] unless path.start_with?('/autocompletable')
     raise "missing actual listing #{path}" unless browser.response.status==200 && body
     {status:browser.response.status,content_type:browser.response.headers['Content-Type'],body:,
      total_count:browser.response.headers['X-Total-Count'],link:browser.response.headers['Link'],reads:reads.length}
    end
    puts "WS8bm2 visible-list Rails size=#{size} zone=#{zone} path=#{path}: #{observations.map { |o| o[:reads] }.join('/')} reads"
    {path:,observations:}
   end
   groups << {size:,zone:,rows:,cases:}
  end
 end
end
File.write(ARGV.fetch(0), JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],groups:)+"\n")
puts "WS8bm2 visible-list Rails: #{groups.sum { |g| g[:cases].size*2 }} complete HTTP captures; 4/16 actual visible upload, Drive and mention rows; no masks"
