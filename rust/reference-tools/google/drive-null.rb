require 'json'
require 'net/http'
require 'action_dispatch/testing/integration'
ENV['GOOGLE_CLIENT_ID']='test-client-id'
ENV['GOOGLE_CLIENT_SECRET']='FAKE-review-client-secret'
http=Object.new
calls=[]
http.define_singleton_method(:get) do |path,headers|
  calls << path
  response=Net::HTTPOK.new('1.1','200','fixture')
  response.define_singleton_method(:body) { 'null' }
  response
end
Net::HTTP.define_singleton_method(:start) { |*args,**kwargs,&block| block.call(http) }
user=User.find(127326141)
GoogleAccount.where(user:).delete_all
GoogleAccount.create!(user:,email:'fixture@example.test',access_token:'fixture-access',refresh_token:'fixture-refresh',access_token_expires_at:1.hour.from_now,scopes:"#{Google::Client::CALENDAR_SCOPE} #{Google::Client::DRIVE_SCOPE}")
vectors=JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'vectors/campfire_sessions.json')))
cookie=vectors.fetch('sessions').find { |s|s['user_name']=='David' }.fetch('cookie_header')
session=ActionDispatch::Integration::Session.new(Rails.application)
session.host! 'campfire.test'
session.get '/google/drive/files/1AbcDefGhIjKlMnOpQrSt', headers:{'Cookie'=>cookie,'Accept'=>'text/html'}
puts JSON.generate({reference:'d7c7de92',status:session.response.status,google_calls:calls.size,body_is_production_500:session.response.body==File.read(Rails.root.join('public/500.html'))})
