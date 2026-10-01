require "json"
Rails.logger = ActiveSupport::Logger.new($stderr)
Rails.logger.level = Logger::FATAL
ActionController::Base.allow_forgery_protection = true
client = ActionDispatch::Integration::Session.new(Rails.application)
client.host! "campfire.test"
client.get "/first_run"
raise "unexpected first run" unless client.response.status == 200
token = Nokogiri::HTML(client.response.body).at_css('meta[name="csrf-token"]')["content"]
# Reject the attachment save inside the room/user save. Account.create! is a prior commit.
ActiveRecord::Base.connection.execute("CREATE TRIGGER ws8br2_reject_attachment BEFORE INSERT ON active_storage_attachments BEGIN SELECT RAISE(ABORT,'deliberate attachment failure'); END")
file = Rack::Test::UploadedFile.new(Rails.root.join("test/fixtures/files/moon.jpg"), "image/jpeg")
client.post "/first_run", params:{user:{name:"Rollback person",email_address:"attachment@example.test",password:"fixture-password",avatar:file}}, headers:{"X-CSRF-Token"=>token}
puts JSON.pretty_generate(reference:"d7c7de92", status:client.response.status, counts:[Account.count, User.count, ActiveStorage::Blob.count, ActiveStorage::Attachment.count], rooms:Room.count, sessions:Session.count)
warn "Rails first-run attachment failure oracle: status #{client.response.status}; counts #{[Account.count,User.count,ActiveStorage::Blob.count,ActiveStorage::Attachment.count].inspect}; rooms #{Room.count}; sessions #{Session.count}; reference d7c7de92"
