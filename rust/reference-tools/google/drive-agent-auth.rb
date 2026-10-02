require 'json'
require 'net/http'
require 'action_dispatch/testing/integration'
ActiveJob::Base.queue_adapter=:test
ActionController::Base.allow_forgery_protection=false
ENV['GOOGLE_CLIENT_ID']='test-client-id'
ENV['GOOGLE_CLIENT_SECRET']='FAKE-drive-agent-secret'
ENV['GOOGLE_PICKER_API_KEY']='FAKE-drive-picker-key'
ENV['GOOGLE_CLOUD_PROJECT_NUMBER']='fixture-app-id'
now=Time.utc(2026,3,2,16)
Time.define_singleton_method(:current) { now }
Net::HTTP.define_singleton_method(:start) { |*args,**options| raise 'unrecorded Google HTTP forbidden' }
user=User.find_by!(name:'JZ');owner=User.find_by!(name:'David');room=Room.find(486777696)
agent=Agent.create!(user:,owner:)
credential,secret=AgentCredential.create_with_secret!(agent:,name:'Drive authorization fixture',created_by:owner)
GoogleAccount.create!(user:,email:'jz@smartdata.net',access_token:'fixture-access',refresh_token:'fixture-refresh',access_token_expires_at:now+3600,scopes:[Google::Client::CALENDAR_SCOPE,Google::Client::DRIVE_SCOPE].join(' '))
paths=[['get','/google/drive/files'],['get','/google/drive/files/1AbcDefGhIjKlMnOpQrSt'],['get',"/rooms/#{room.id}/drive_recipients"],['post',"/rooms/#{room.id}/drive_recipients/validate"]]
cases=paths.map do |method,path|
 credential.update_columns(last_used_at:nil,last_used_ip:nil);agent.update_column(:last_seen_at,nil)
 client=ActionDispatch::Integration::Session.new(Rails.application);client.host! 'campfire.test'
 client.public_send(method,path,params:method=='post' ? {user_ids:[owner.id]} : nil,as: :json,headers:{'Authorization'=>"Bearer #{secret}"})
 {method:,path:,status:client.response.status,body:client.response.body,credential_used:credential.reload.last_used_at.present?,agent_seen:agent.reload.last_seen_at.present?}
end
puts JSON.pretty_generate({reference:'d7c7de92',cases:})
