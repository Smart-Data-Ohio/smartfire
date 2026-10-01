# Actual Rails HTTP bodies for board-post forms, pages and conversation panes.
require 'json'
require 'digest'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
hashes=JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'reference-tools/boards/source-hashes.json')))
hashes.merge!(JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'reference-tools/boards/post-source-hashes.json'))))
hashes.merge!(JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'reference-tools/boards/write-source-hashes.json'))))
hashes.each { |file,hash| raise "source drift: #{file}" unless Digest::SHA256.file(Rails.root.join(file)).hexdigest==hash }
Rails.application.env_config['action_dispatch.show_exceptions']=:all
Rails.application.env_config['action_dispatch.content_security_policy_nonce_generator']=->(_) {'NONCE'}
ActiveJob::Base.queue_adapter=:test
module PostGoldenTokens
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
ApplicationController.prepend(PostGoldenTokens)
Rails.logger = ActiveSupport::Logger.new($stderr)
ApplicationController.logger = Rails.logger
Rails.application.env_config['action_dispatch.logger'] = Rails.logger
ChannelThreadsController.skip_before_action :verify_authenticity_token
rows=[]
board=699448332
admin=127326141
owner=712064548
observer=773523953
base_setup=["INSERT INTO memberships(room_id,user_id,involvement,created_at,updated_at) VALUES(699448332,712064548,'mentions','2026-03-02 16:00:00','2026-03-02 16:00:00')", "INSERT INTO memberships(room_id,user_id,involvement,created_at,updated_at) VALUES(699448332,773523953,'mentions','2026-03-02 16:00:00','2026-03-02 16:00:00')", "UPDATE channel_threads SET work_owner_id=712064548 WHERE id=4"]
create="/rooms/#{board}/threads"
update="#{create}/4"
fixtures=[]
add=->(name,user,path,method,input,setup=[],status=200) {fixtures << {name:,user_id:user,path:,method:,input:,setup:base_setup+setup,status:}}
add.call('create-unowned',admin,create+'.json','post',{thread:{name:'Unowned'}},[],201)
add.call('create-human',admin,create+'.json','post',{thread:{name:'Release plan',work_status:'in_progress',work_owner_id:owner,tags:'Launch, api'}},[],201)
add.call('create-agent',admin,create+'.json','post',{thread:{name:'Agent post',work_owner_id:394959859,tags:'rust'}},[],201)
add.call('create-opening',admin,create+'.json','post',{thread:{name:'Brief',work_owner_id:owner,first_message:'## Plan'}},[],201)
add.call('create-empty-opening',admin,create+'.json','post',{thread:{name:'Empty brief',first_message:'  '}},[],201)
add.call('create-nonmember',773523958,create+'.json','post',{thread:{name:'Private'}},[],404)
add.call('create-html',admin,create,'post',{thread:{name:'HTML post'}},[],302)
add.call('create-empty-title-json',admin,create+'.json','post',{thread:{name:''}},[],422)
add.call('create-empty-title-html',admin,create,'post',{thread:{name:'',work_status:'blocked',tags:'Launch,api',first_message:'Retained brief'}},[],422)
add.call('create-invalid-owner-json',admin,create+'.json','post',{thread:{name:'Rejected',work_owner_id:'bad'}},[],422)
add.call('create-invalid-owner-html',admin,create,'post',{thread:{name:'Rejected',work_owner_id:'bad',first_message:'Retained'}},[],422)
add.call('create-outsider',admin,create+'.json','post',{thread:{name:'Rejected',work_owner_id:773523958}},[],422)
add.call('create-suspended-agent',admin,create+'.json','post',{thread:{name:'Rejected',work_owner_id:394959859}},["UPDATE agents SET suspended_at='2026-03-02 16:00:00' WHERE user_id=394959859"],422)
add.call('create-no-agent',admin,create+'.json','post',{thread:{name:'Rejected',work_owner_id:773523956}},["INSERT INTO memberships(room_id,user_id,involvement,created_at,updated_at) VALUES(699448332,773523956,'mentions','2026-03-02 16:00:00','2026-03-02 16:00:00')"],422)
add.call('create-invalid-status',admin,create+'.json','post',{thread:{name:'Rejected',work_status:'unknown'}},[],422)
add.call('create-tag-limit',admin,create+'.json','post',{thread:{name:'Rejected',tags:'a,b,c,d,e,f'}},[],422)
add.call('create-tag-format-html',admin,create,'post',{thread:{name:'Rejected',tags:'needs work'}},[],422)
%w[planned in_progress blocked done].each { |status| add.call('owner-status-'+status,owner,update+'.json','patch',{thread:{work_status:status}}) }
add.call('owner-explicit-assignment',owner,update+'.json','patch',{thread:{work_owner_id:owner}},[],403)
add.call('owner-clear-assignment',owner,update+'.json','patch',{thread:{work_owner_id:''}},[],403)
add.call('owner-remove-work',owner,update+'.json','patch',{thread:{work_status:'',work_owner_id:''}},[],403)
add.call('manager-remove-board-work',admin,update+'.json','patch',{thread:{work_status:'',work_owner_id:''}},[],422)
add.call('manager-reassign',admin,update+'.json','patch',{thread:{work_owner_id:394959859}})
add.call('manager-unassign',admin,update+'.json','patch',{thread:{work_owner_id:''}})
add.call('manager-invalid-owner',admin,update+'.json','patch',{thread:{work_owner_id:'bogus'}},[],422)
add.call('manager-invalid-owner-hex',admin,update+'.json','patch',{thread:{work_owner_id:'0x2a713e24'}})
add.call('owner-metadata',owner,update+'.json','patch',{thread:{name:'Renamed',tags:'Launch, API'}})
add.call('observer-metadata',observer,update+'.json','patch',{thread:{name:'Denied'}},[],403)
add.call('observer-result',observer,update+'.json','patch',{thread:{result_markdown:'Denied'}},[],403)
add.call('owner-invalid-status',owner,update+'.json','patch',{thread:{work_status:'unknown'}},[],422)
add.call('owner-invalid-tags',owner,update+'.json','patch',{thread:{tags:'invalid!'}},[],422)
add.call('owner-result',owner,update+'.json','patch',{thread:{result_markdown:'## Shipped'}})
add.call('owner-result-html',owner,update,'patch',{thread:{result_markdown:'## Shipped'}},[],302)
add.call('owner-result-clear',owner,update+'.json','patch',{thread:{result_markdown:'  '}},["UPDATE channel_threads SET result_markdown='Existing',result_updated_at='2026-03-02 15:00:00' WHERE id=4"])
add.call('owner-result-noop',owner,update+'.json','patch',{thread:{result_markdown:'Existing'}},["UPDATE channel_threads SET result_markdown='Existing',result_updated_at='2026-03-02 15:00:00' WHERE id=4"])
add.call('owner-result-limit-html',owner,update,'patch',{thread:{result_markdown:'x'*20001}},[],422)
add.call('owner-archive',owner,update+'.json','patch',{thread:{auto_archive_after_minutes:true}},[],422)
add.call('manager-archive-html',admin,update,'patch',{thread:{auto_archive_after_minutes:60}},[],422)
add.call('owner-close',owner,update+'.json','patch',{thread:{status:'closed'}},[],403)
add.call('manager-close',admin,update+'.json','patch',{thread:{status:'closed'}})
add.call('owner-lock',owner,update+'.json','patch',{thread:{status:'locked'}},[],403)
add.call('manager-lock',admin,update+'.json','patch',{thread:{status:'locked'}})
add.call('owner-unlock',owner,update+'.json','patch',{thread:{status:'active'}},["UPDATE channel_threads SET closed_at='2026-03-02 16:00:00',locked_at='2026-03-02 16:00:00' WHERE id=4"],403)
add.call('owner-reopen-joined',owner,update+'.json','patch',{thread:{status:'active'}},["UPDATE channel_threads SET closed_at='2026-03-02 16:00:00' WHERE id=4","INSERT INTO thread_memberships(thread_id,user_id,involvement,joined_at,created_at,updated_at) VALUES(4,712064548,'everything','2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00')"])
add.call('owner-delete',owner,update+'.json','delete',{},[],403)
add.call('manager-delete',admin,update+'.json','delete',{},[],204)
add.call('manager-delete-html',admin,update,'delete',{},[],302)
add.call('ordinary-convert',admin,'/rooms/654632876/threads/1.json','patch',{thread:{work_status:'planned',work_owner_id:owner}})
add.call('ordinary-untrack',admin,'/rooms/654632876/threads/1.json','patch',{thread:{work_status:'',work_owner_id:''}},["UPDATE channel_threads SET work_status='planned',work_owner_id=712064548 WHERE id=1"])
[
 ['create-false-status', {name:'False status',work_status:false},201],
 ['create-false-owner', {name:'False owner',work_owner_id:false},201],
 ['create-true-owner', {name:'True owner',work_owner_id:true},422],
 ['create-float-owner', {name:'Float owner',work_owner_id:712064548.8},201],
 ['create-false-title', {name:false},201],
 ['create-array-tags', {name:'Array tags',tags:['launch','api']},201],
 ['create-array-owner', {name:'Array owner',work_owner_id:[712064548]},201],
 ['update-false-status', {work_status:false},422],
 ['update-array-owner', {work_owner_id:[712064548]},200],
 ['update-boolean-archive-crash', {auto_archive_after_minutes:true},500],
 ['update-number-result-crash', {result_markdown:42},500],
 ['update-false-result', {result_markdown:false},200]
].each do |name,input,status|
 path=name.start_with?('create') ? create+'.json' : update+'.json'
 method=name.start_with?('create') ? 'post' : 'patch'
 add.call(name,admin,path,method,{thread:input},[],status)
 fixtures.last[:encoding]='json'
end
creator_setup=["UPDATE channel_threads SET creator_id=773523953 WHERE id=4"]
add.call('post-creator-status',observer,update+'.json','patch',{thread:{work_status:'done'}},creator_setup)
add.call('post-creator-assign',observer,update+'.json','patch',{thread:{work_owner_id:394959859}},creator_setup)
add.call('post-creator-close-denied',observer,update+'.json','patch',{thread:{status:'closed'}},creator_setup,403)
add.call('post-creator-delete-denied',observer,update+'.json','delete',{},creator_setup,403)
add.call('board-creator-nonadmin-close',admin,update+'.json','patch',{thread:{status:'closed'}},["UPDATE users SET role=0 WHERE id=127326141"])
add.call('other-admin-close',149087659,update+'.json','patch',{thread:{status:'closed'}})
add.call('owner-loss-denies-edit',owner,update+'.json','patch',{thread:{work_status:'done'}},["UPDATE channel_threads SET work_owner_id=127326141 WHERE id=4"],403)
add.call('metadata-tags-error-html',owner,update,'patch',{thread:{name:'Attempted title',tags:'invalid!'}},[],422)
add.call('metadata-name-error-html',owner,update,'patch',{thread:{name:'',tags:'new-tag'}},[],422)
add.call('result-error-after-metadata-html',owner,update,'patch',{thread:{name:'Attempted title',tags:'new-tag',result_markdown:'x'*20001}},[],422)
add.call('work-error-after-result-html',admin,update,'patch',{thread:{name:'Attempted title',tags:'new-tag',result_markdown:'New result',work_status:'unknown'}},[],422)
add.call('invalid-owner-update-html',admin,update,'patch',{thread:{work_owner_id:773523958}},[],422)
add.call('remove-board-work-html',admin,update,'patch',{thread:{work_status:'',work_owner_id:''}},[],422)
[
 ['create-invalid-tags-turbo','post',create,'text/vnd.turbo-stream.html, text/html, application/xhtml+xml'],
 ['create-invalid-tags-stream-only','post',create,'text/vnd.turbo-stream.html'],
 ['create-invalid-tags-json-preferred','post',create,'application/json, text/html'],
 ['create-invalid-tags-html-preferred','post',create,'text/html, application/json'],
 ['update-invalid-tags-turbo','patch',update,'text/vnd.turbo-stream.html, text/html, application/xhtml+xml']
].each do |name,method,path,accept|
 add.call(name,admin,path,method,{thread:{name:'Rejected tag',tags:'invalid!',first_message:'Retained brief'}},[],422)
 fixtures.last[:accept]=accept
end
fixtures.each do |row|
 ActiveRecord::Base.transaction do
  row[:setup].each { |sql| ActiveRecord::Base.connection.execute(sql) }
  user=User.find(row[:user_id])
  request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
  request.cookie_jar.signed[:session_token]=user.sessions.where.not(two_factor_verified_at:nil).first!.token
  browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test'
  options={params:row[:input],headers:{'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}",'User-Agent'=>'Mozilla'}}
  options[:headers]['Accept']=row[:accept] if row[:accept]
  options[:as]=:json if row[:encoding]=='json'
  browser.public_send(row[:method],row[:path],**options)
  raise "unexpected Rails response #{row[:name]}: #{browser.response.status}, #{browser.response.body[0,500]}" unless browser.response.status==row[:status]
  row[:body]=browser.response.body
  row[:location]=browser.response.headers['Location']
  row[:content_type]=browser.response.headers['Content-Type']
  rows << row
  raise ActiveRecord::Rollback
 end
 ActiveSupport::IsolatedExecutionState.clear
end
inputs=[42,42.8,"+42","0x2a","0b101010","052","4_2",nil,false," \n",[],{},true,"42junk","09","4__2","1e1",[42],{"id"=>42},"--42","+-42","0x_2a","42"+0.chr,0.chr+"42"]
coercions=inputs.map do |input|
 begin
  normalized=ChannelThread.send(:normalize_board_post_owner_id!,ChannelThread.new,input)
  {input:,normalized:,invalid:false}
 rescue ActiveRecord::RecordInvalid
  {input:,invalid:true}
 end
end
puts JSON.pretty_generate(reference:'d7c7de92 plus approved board drift',coercions:,rows:)
warn "Rails board write oracle: #{rows.size} complete HTTP responses; no masks"
