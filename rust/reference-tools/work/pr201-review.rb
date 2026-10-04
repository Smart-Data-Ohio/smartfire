# PR #201: complete large-list bytes and the shared MessagePayloadHelper's brand image paths.
require 'json'
require 'digest'
require 'zlib'
require 'stringio'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
ActiveJob::Base.queue_adapter=:test
ApplicationController.allow_forgery_protection=false
Rails.logger=ActiveSupport::Logger.new(File::NULL)
ApplicationController.logger=Rails.logger
root=ENV.fetch('PARITY_WORK')
JSON.parse(File.read(File.join(root,'reference-tools/work/pr201-source-hashes.json'))).each do |path,hash|
  raise "Rails source drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest==hash
end
conn=ActiveRecord::Base.connection
base=JSON.parse(File.read(File.join(root,'vectors/human_work_http.json')))['rows'][0]['setup']
labels=JSON.parse(File.read(File.join(root,'parity/.seed/default/labels.json')))
headers={'Cookie'=>"session_token=#{labels.fetch('session_cookies.david')}",'User-Agent'=>'Mozilla'}
browser=ActionDispatch::Integration::Session.new(Rails.application)
browser.host! 'campfire.test'
if ARGV[0]=='large'
  size=32767
  setup=base+['UPDATE channel_threads SET work_status=NULL','DELETE FROM work_thread_links','DELETE FROM work_thread_events',"WITH RECURSIVE seq(x) AS (SELECT 0 UNION ALL SELECT x+1 FROM seq WHERE x+1<#{size}) INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,work_owner_id,created_at,updated_at,last_activity_at) SELECT 1000+x,486777696,127326141,'Large work '||x,'planned',127326141,'2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00' FROM seq"]
  setup.each{|sql|conn.execute(sql)}
  browser.get('/work.json?state=all',headers:)
  body=browser.response.body
  raise "HTTP #{browser.response.status}" unless browser.response.status==200
  raise 'missing rows' unless JSON.parse(body).fetch('threads').size==size
  output=StringIO.new
  Zlib::GzipWriter.wrap(output){|gz|gz.mtime=0;gz.write(body)}
  STDOUT.binmode;STDOUT.write(output.string)
  warn "Rails large work JSON: #{size} threads; HTTP #{browser.response.status}; #{body.bytesize} bytes; #{output.string.bytesize} gzip bytes; complete response; 0 masks"
else
  rows=[]
  %w[github brand-collision custom emoji unknown blank].each do |kind|
    ActiveRecord::Base.transaction do
      setup=base+["UPDATE users SET icon_name='github' WHERE id IN (127326141,149087659,394959859)", "UPDATE channel_threads SET work_status='planned',work_owner_id=149087659 WHERE id=91"]
      setup << "INSERT INTO workspace_icons(name,title,creator_id,created_at,updated_at) VALUES('github','Brand collision',127326141,'2026-03-02 16:00:00','2026-03-02 16:00:00')" if kind=='brand-collision'
      icon={'custom'=>'work_review_icon','emoji'=>'wave','unknown'=>'ws12_unknown','blank'=>''}.fetch(kind,'github')
      setup << "INSERT INTO workspace_icons(name,title,creator_id,created_at,updated_at) VALUES('work_review_icon','Review icon',127326141,'2026-03-02 16:00:00','2026-03-02 16:00:00')" if kind=='custom'
      setup << "UPDATE users SET icon_name=#{conn.quote(icon)} WHERE id IN (127326141,149087659,394959859)"
      # Pin IDs instead of creating HTTP writes; these reader fixtures have no callbacks.
      setup << "INSERT INTO messages(id,room_id,thread_id,creator_id,client_message_id,created_at,updated_at) VALUES(970000003,486777696,91,127326141,'brand-thread-message','2026-03-02 16:00:00','2026-03-02 16:00:00')"
      setup << "INSERT INTO action_text_rich_texts(record_type,record_id,name,body,created_at,updated_at) VALUES('Message',970000003,'body','Brand message','2026-03-02 16:00:00','2026-03-02 16:00:00')"
      setup << "INSERT INTO work_thread_events(channel_thread_id,actor_id,event_type,from_status,to_status,metadata,created_at,updated_at) VALUES(91,149087659,'work_update','planned','blocked','{}','2026-03-02 16:00:00','2026-03-02 16:00:00')"
      setup << "INSERT INTO messages(id,room_id,creator_id,client_message_id,created_at,updated_at) VALUES(970000004,486777696,127326141,'brand-parent-message','2026-03-02 16:00:00','2026-03-02 16:00:00')"
      setup << "INSERT INTO action_text_rich_texts(record_type,record_id,name,body,created_at,updated_at) VALUES('Message',970000004,'body','Brand parent','2026-03-02 16:00:00','2026-03-02 16:00:00')"
      setup << "UPDATE channel_threads SET parent_message_id=970000004 WHERE id=91"
      setup << "UPDATE messages SET reply_to_message_id=970000004 WHERE id=970000003"
      setup << "INSERT INTO thread_memberships(thread_id,user_id,involvement,joined_at,created_at,updated_at) VALUES(94,127326141,'everything','2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00')"
      setup.each{|sql|conn.execute(sql)}
      Icons.expire_custom_cache!
      paths=['/work.json?state=all','/rooms/486777696/threads.json','/rooms/486777696/threads/91.json','/rooms/486777696/messages/970000004/actions.json','/rooms/486777696/messages/970000004/forwards/destinations.json','/rooms/486777696/threads/91/messages.json','/rooms/486777696/threads/91/messages/970000003.json','/rooms/486777696/threads/91/messages/970000003/actions.json','/rooms/486777696/threads/91/messages/970000003/forwards/destinations.json']
      paths.each do |path|
        browser.get(path,headers:)
        raise "#{kind} #{path} HTTP #{browser.response.status}" unless browser.response.status==200
        rows << {kind:,setup:,path:,status:browser.response.status,body:browser.response.body}
      end
      Current.session=User.find(127326141).sessions.where.not(two_factor_verified_at:nil).first!
      root_payload=JSON.parse(ActiveSupport::JSON.encode(browser.controller.send(:message_payload,Message.find(970000004))))
      rows << {kind:,setup:,helper:'agent-adapter',payload:root_payload}
      %w[root nested].each do |scope|
        path=scope=='root' ? '/rooms/486777696/messages/970000004/forwards.json' : '/rooms/486777696/threads/91/messages/970000003/forwards.json'
        # Supply the source UUID provider, never rewrite a response.
        client_id="ws12-brand-forward-#{scope}"
        original_uuid=Random.method(:uuid)
        Random.define_singleton_method(:uuid){client_id}
        begin
          input={forward:{destinations:[{room_id:486777696,thread_id:94}],note:'Brand copy'}}
          browser.post(path,params:JSON.generate(input),headers:headers.merge('Content-Type'=>'application/json'))
        ensure
          Random.define_singleton_method(:uuid,original_uuid)
        end
        raise "forward HTTP #{browser.response.status}: #{browser.response.body}" unless browser.response.status==201
        # A standalone fixture has the same new-message ID as the preceding root copy.
        if scope=='root'
          rows << {kind:,setup:,path:,method:'POST',input:,client_id:,status:browser.response.status,body:browser.response.body}
        else
          # Retain the preceding primary row as fixture input, not an independently regenerated UUID.
          preceding=conn.select_all("SELECT * FROM messages WHERE client_message_id='ws12-brand-forward-root'").first
          preceding_body=conn.select_all("SELECT * FROM action_text_rich_texts WHERE record_type='Message' AND record_id=#{preceding.fetch('id')}").first
          extra=["INSERT INTO messages(#{preceding.keys.join(',')}) VALUES(#{preceding.values.map{|v|conn.quote(v)}.join(',')})", "INSERT INTO action_text_rich_texts(#{preceding_body.keys.join(',')}) VALUES(#{preceding_body.values.map{|v|conn.quote(v)}.join(',')})"]
          rows << {kind:,setup:setup+extra,path:,method:'POST',input:,client_id:,status:browser.response.status,body:browser.response.body}
        end
      end
      raise ActiveRecord::Rollback
    end
  end
  puts JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA"),rows:)
  warn "Rails shared message/thread/work icon JSON: #{rows.size} complete responses; brand/collision/custom/emoji/unknown/blank; 0 masks"
end
