require 'active_support/testing/time_helpers'
require 'base64'
require 'json'
extend ActiveSupport::Testing::TimeHelpers
Rails.logger=ActiveSupport::Logger.new($stderr)
ActiveJob::Base.queue_adapter=:test
# Send the small shared case generator, not multi-megabyte literal request
# arrays through an environment variable. It generates the same canonical
# input files used by Rust; expected responses always come from real Rails.
fixture,manifest=eval(File.read('/work/reference-tools/agents/array_shape_inputs.rb'),binding,'array_shape_inputs.rb')
conn=ActiveRecord::Base.connection
restore_tables=conn.tables.reject{|n|n.start_with?('message_search') || %w[schema_migrations ar_internal_metadata].include?(n)}+['sqlite_sequence']
original=restore_tables.to_h{|t|[t,conn.select_all("SELECT * FROM #{conn.quote_table_name(t)}").to_a]}
restore_sql="PRAGMA foreign_keys=OFF;\nBEGIN;\nDELETE FROM message_search_index;\n"
restore_tables.each{|t|restore_sql << "DELETE FROM #{conn.quote_table_name(t)};\n"}
original.each{|t,rows|rows.each{|r|restore_sql << "INSERT INTO #{conn.quote_table_name(t)} (#{r.keys.map{|k|conn.quote_column_name(k)}.join(',')}) VALUES (#{r.values.map{|v|conn.quote(v)}.join(',')});\n"}}
restore_sql << "COMMIT;\nPRAGMA foreign_keys=ON;\n"
tables=manifest.fetch(:projection_tables)
json_columns=tables.to_h{|t|[t,conn.columns(t).select{|c|c.sql_type=='json'}.map(&:name)]}
snapshot=-> do
 tables.to_h do |t|
  rows=conn.select_all("SELECT * FROM #{conn.quote_table_name(t)} ORDER BY id").to_a
  rows.each{|r|json_columns[t].each{|c|r[c]=JSON.parse(r[c]) if r[c].is_a?(String)}}
  [t,rows]
 end
end
jobs=-> {ActiveJob::Base.queue_adapter.enqueued_jobs.map{|job|{class:job[:job].name,args:{raw_args:job[:args]}}}}
results=[]
travel_to Time.utc(2026,3,2,16) do
 manifest.fetch(:cases).each do |item|
  conn.raw_connection.execute_batch(restore_sql)
  execution=Rails.application.executor.run!(reset:true)
  begin
   conn.raw_connection.execute_batch(fixture)
   conn.raw_connection.execute_batch(item.fetch(:setup_sql))
   raise 'Request must not be enclosed in a SQL transaction' unless conn.open_transactions.zero?
   ActiveJob::Base.queue_adapter.enqueued_jobs.clear
   Rails.cache=ActiveSupport::Cache::MemoryStore.new
   session=ActionDispatch::Integration::Session.new(Rails.application)
   session.host! 'campfire.test'
   send_request=-> {session.public_send(item.fetch(:method).downcase,item.fetch(:path),params:item[:body],headers:manifest.fetch(:request_headers).merge("Authorization"=>["Bearer",manifest.fetch(:secret)].join(" ")))}
   warmups=[]
   item.fetch(:warmups,0).times do
    send_request.call
    warmups << {status:session.response.status,body:session.response.body,body_base64:Base64.strict_encode64(session.response.body)}
   end
   before=snapshot.call
   steps=[]
   item.fetch(:repeat).times do |index|
    notifications=[]
    callback=->(*args) {p=args.last; notifications << {sql:p[:sql],cached:!!p[:cached],name:p[:name]} if p[:sql].lstrip.match?(/\ASELECT\b/i)}
    ActiveSupport::Notifications.subscribed(callback,'sql.active_record'){send_request.call}
    reply=session.response
    after=snapshot.call
    headers=%w[content-type cache-control pragma retry-after location content-length].to_h{|h|[h,reply.headers[h]]}
    all_headers=reply.headers.to_h.to_h{|h,v|[h.downcase,v.is_a?(Array) ? v : [v]]}
    steps << {attempt:index+1,status:reply.status,response_body:reply.body,response_body_base64:Base64.strict_encode64(reply.body),response_headers:headers,all_headers:all_headers,state:after,jobs:jobs.call,changed_tables:tables.select{|t|after[t]!=before[t]},selects:notifications.count{|q|!q[:cached]},cache_hits:notifications.count{|q|q[:cached]},sql_notifications:notifications,query_boundary:'SQL notifications, uncached executions counted separately from cache hits',open_transactions:conn.open_transactions}
   end
   result={name:item.fetch(:name),surface:item.fetch(:surface),count_label:item[:count_label],candidate_count:item[:candidate_count],warmups:warmups,before:before,steps:steps}
   results << result
   warn "PR210_RAILS_ARRAY case=#{result[:name]} status=#{steps.map{|s|s[:status]}.join('/')} executions=#{steps.map{|s|s[:selects]}.join('/')} cached=#{steps.map{|s|s[:cache_hits]}.join('/')}"
  ensure
   execution.complete!
  end
 end
end
puts JSON.pretty_generate(reference_pin:'d7c7de92',projection_tables:tables,json_columns:json_columns,cases:results)
