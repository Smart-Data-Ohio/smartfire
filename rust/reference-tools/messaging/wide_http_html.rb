# Real HTML requests, comparing raw date tags, datetime-local fields and real flash spans.
require 'json'
require 'action_dispatch/testing/integration'
require 'active_support/testing/time_helpers'
require_relative 'oracle-database'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
ActiveJob::Base.queue_adapter = :test
ActionController::Base.allow_forgery_protection=false
Rails.application.config.hosts.clear
Icons.custom_icons
Icons.instance_variable_set(:@custom_cache_at,Float::INFINITY)
source=JSON.parse(File.read('/work/vectors/messaging/older_calendar_execution.json'))
outer=MessagingOracleDatabase.scenarios(ARGV.fetch(0)+'.groups')
groups=[]
source.fetch('groups').each do |group|
 outer.call do
  conn=ActiveRecord::Base.connection
  group.fetch('rows').each do |table,rows|
   rows.each { |row| conn.execute("INSERT INTO #{conn.quote_table_name(table)} (#{row.keys.map { |k| conn.quote_column_name(k) }.join(',')}) VALUES (#{row.values.map { |v| conn.quote(v) }.join(',')})") }
  end
  reset=MessagingOracleDatabase.scenarios(ARGV.fetch(0))
  cases=[]
  %w[UTC America/New_York Australia/Lord_Howe Pacific/Apia].each do |zone|
   ['9999-12-31T23:59:59Z','10000-03-05T09:00:00Z','178956971-03-05T09:00:00Z'].each do |at|
    %w[saved scheduled].each do |kind|
     reset.call do
      conn=ActiveRecord::Base.connection
      user=User.find(127326141);user.update_columns(time_zone:zone)
      request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
      request.cookie_jar.signed[:session_token]=user.sessions.where.not(two_factor_verified_at:nil).first!.token
      headers={'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}",'Accept'=>'text/html','Content-Type'=>'application/json'}
      params=kind=='saved' ? {message_id:group.fetch('old_ids').first,saved_item:{remind_at:at}} : {scheduled_message:{markdown_source:'Wide HTTP <draft> & proof',send_at:at}}
      path=kind=='saved' ? '/saved' : '/rooms/699448326/scheduled_messages'
      page=kind=='saved' ? '/saved' : '/scheduled_messages'
      browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test'
      queries=[]
      observer=->(*args) { p=args.last;queries << p[:sql] if !p[:cached] && p[:name]!='SCHEMA' && p[:sql].match?(/\A\s*(SELECT|WITH)\b/i) }
      post=nil;get=nil
      ActiveSupport::Notifications.subscribed(observer,'sql.active_record') do
       browser.post(path,params:JSON.generate(params),headers:)
       post={status:browser.response.status,location:browser.response.headers['Location']}
       browser.get(page,headers:{'Accept'=>'text/html'})
       section=browser.response.body[/<section class="#{kind=='saved' ? 'saved-items' : 'scheduled-messages'}__page".*?<\/section>/m]
       notice=browser.response.body[/<span class="for-screen-reader" role="alert" aria-atomic="true">.*?<\/span>/m]
       raise "missing real page #{kind} #{at}: #{browser.response.status}" unless browser.response.status==200 && section && notice
       get={status:browser.response.status,content_type:browser.response.headers['Content-Type'],date_tags:section.scan(/<time\b[^>]*>.*?<\/time>/m),date_fields:section.scan(/<input\b[^>]*type="datetime-local"[^>]*>/m),notice:}
      end
      state=%w[saved_items scheduled_messages].to_h { |table| [table,conn.select_all("SELECT * FROM #{table} ORDER BY id").to_a] }
      cases << {kind:,zone:,at:,path:,page:,params:,post:,get:,state:,reads:queries.length}
     end
    end
   end
  end
  groups << group.slice('size','rows').merge(cases:)
 end
end
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],groups:)+"\n")
puts "WS8bm2 wide HTTP Rails: #{groups.sum { |g| g[:cases].size }} real POST/GET pairs; raw date tags/fields, flash spans and persisted rows"
