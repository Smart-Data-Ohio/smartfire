# Validate rows written by the Rust Notifier with the pinned Rails model validations.
require "json"
require "digest"
JSON.parse(File.read(ENV.fetch("GITHUB_REFERENCE_HASHES"))).each do |path, hash|
  raise "Reference drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash
end
ActiveRecord::Base.establish_connection("sqlite3:/work/scratch/rust-notifier.sqlite3")
bot=User.active_bots.find_by!(name:"GitHub")
message=Message.find_by!(room_id:815,creator:bot)
notification=Github::Notification.find_by!(dedupe_key:"opened:rails/rails#12")
pr=Github::PullRequest.find_by!(owner:"rails",repo:"rails",number:12)
reference=Github::PullRequestReference.find_by!(message:,pull_request:pr)
[bot,message,notification,pr,reference].each do |row|
  raise "Invalid #{row.class}: #{row.errors.full_messages}" unless row.valid?
end
raise "Integration bot unexpectedly owns an agent" unless bot.agent.nil?
raise "Notification lost its message" unless notification.message_id == message.id
raise "Bot granted an unsubscribed open room" if Membership.where(user:bot).exists?
puts "GitHub Notifier Rails rollback: 8 checks passed; 0 failed; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
