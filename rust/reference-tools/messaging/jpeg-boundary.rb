require 'json'
require_relative 'oracle-database'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
ApplicationController.allow_forgery_protection=false
ActiveJob::Base.queue_adapter=:test
Rails.application.env_config['action_dispatch.show_exceptions']=:all
scenario=MessagingOracleDatabase.scenarios(ARGV.fetch(0))
user=User.find(127326141)
request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
request.cookie_jar.signed[:session_token]=user.sessions.detect(&:two_factor_verified?).token
rows=[]
%w[initial reply root].each do |kind|
  scenario.call do
    browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test'
    headers={'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}
    thread=ChannelThread.create!(room_id:486777696,creator_id:149087659,name:'Review') if kind=='reply'
    thread.close! if thread
    blob=ActiveStorage::Blob.find(1)
    last=Message.maximum(:id)
    path=case kind
    when 'initial' then '/rooms/486777696/threads.json'
    when 'reply' then "/rooms/486777696/threads/#{thread.id}/messages.json"
    else '/rooms/486777696/messages.turbo_stream'
    end
    responses=[]
    2.times do |i|
      input={message:{client_message_id:"jpeg-#{kind}-#{i}",markdown_source:'JPEG boundary',attachment:blob.signed_id}}
      input[:thread]={name:'JPEG boundary'} if kind=='initial'
      browser.post path,params:input,headers:headers.dup,as: :json
      variants=blob.variant_records.order(:id).map do |record|
        image=record.image.blob
        {digest:record.variation_digest,filename:image.filename.to_s,metadata:image.metadata,exists:image.service.exist?(image.key)}
      end
      state={message_count:Message.where('id>?',last).count,variants:,original_exists:blob.service.exist?(blob.key)}
      if thread
        state[:joined]=ThreadMembership.exists?(thread:,user:);state[:closed]=thread.reload.closed_at.present?
      end
      responses << {input:,status:browser.response.status,body:browser.response.body,content_type:browser.response.headers['Content-Type'],cache_control:browser.response.headers['Cache-Control'],location:browser.response.headers['Location'],state:}
    end
    rows << {kind:,path:,responses:}
  end
end
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],rows:)+"\n")
puts 'WS8bm JPEG boundary oracle: 6 actual Rails requests; initial/reply/root; new/reused variants; rows/files/lifecycle after commit'
