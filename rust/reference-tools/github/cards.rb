require "json"
require "digest"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
JSON.parse(File.read(ENV.fetch("GITHUB_REFERENCE_HASHES"))).each do |path, hash|
 raise "Reference drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash
end
ActiveJob::Base.queue_adapter=:test
ActionCable.server.instance_variable_set(:@pubsub, ActionCable::SubscriptionAdapter::Test.new(ActionCable.server))
ActiveRecord::Schema.verbose=false
load Rails.root.join("db/schema.rb")
travel_to Time.utc(2026,1,1,12)
Account.create!(name:"Card oracle")
owner=User.create!(id:811,name:"Oracle",email_address:"oracle@example.test",password:"fixture-password")
room=Rooms::Closed.create!(id:815,creator:owner,name:"Cards")
message=room.messages.create!(id:818,creator:owner,markdown_source:"Discussion",client_message_id:"card-parent")
thread=ChannelThread.create!(id:817,room:,creator:owner,parent_message:message,name:"Discussion")
base={owner:"rails",repo:"rails",number:12,fetched_at:Time.current}
full={title:"Fix login",state:"open",private:false,author_login:"alice",author_avatar_url:"https://example.test/avatar.png",base_branch:"main",head_branch:"shiny",review_decision:"approved",check_status:"passing",html_url:"https://github.com/Rails/Rails/pull/12",github_updated_at:Time.utc(2025,12,31,10),payload:{base:{repo:{full_name:"Rails/Rails"}}},changed_files:JSON.generate({files:[{filename:"app/a.rb",status:"modified",additions:4,deletions:2}],total_count:3})}
cases=[{name:"loading",attrs:{private:false}}, {name:"public",attrs:full}, {name:"private",attrs:full.merge(private:true,title:"Secret title")}, {name:"unknown",attrs:full.merge(private:nil,title:"Secret title")}, {name:"error",attrs:full.merge(fetch_error:"Fetch <failed> & blocked")}, {name:"discuss_link",attrs:full,mapped:true}, {name:"reply",attrs:full,reply:true},
 {name:"escape",attrs:full.merge(owner:"a&b",repo:"r<q>",title:'<script>"&\'@[]',author_login:"<alice>",author_avatar_url:"https://example.test/a?x=1&y=2",base_branch:"<main>",head_branch:"&shiny",html_url:"https://example.test/?a=1&b=2")},
 {name:"blank_optional",attrs:full.merge(title:" ",state:"",html_url:"",github_updated_at:nil)},
 {name:"no_author_avatar",attrs:full.merge(author_avatar_url:nil)}, {name:"no_author",attrs:full.merge(author_login:nil)}, {name:"one_branch",attrs:full.merge(head_branch:nil)}, {name:"empty_files",attrs:full.merge(changed_files:'{"files":[],"total_count":3}')}, {name:"broken_files",attrs:full.merge(changed_files:"bad")}, {name:"loading_files",attrs:full.merge(changed_files:nil)}]
[nil,"merged","closed","draft","other"].each { |v| cases<<{name:"state_#{v}",attrs:full.merge(state:v)} }
[nil,"approved","changes_requested","review_required","other"].each { |v| cases<<{name:"review_#{v}",attrs:full.merge(review_decision:v)} }
[nil,"passing","pending","failing","other"].each { |v| cases<<{name:"checks_#{v}",attrs:full.merge(check_status:v)} }
vectors=cases.map do |c|
 Github::PullRequestReference.delete_all
 Github::PullRequestThread.delete_all
 Github::PullRequest.delete_all
 pr=Github::PullRequest.create!(id:816,**base.merge(c[:attrs]))
 Github::PullRequestReference.create!(message:,pull_request:pr)
 Github::PullRequestThread.create!(pull_request:pr,room:,channel_thread:thread) if c[:mapped]
 message.reload
 message.thread_id=c[:reply] ? thread.id : nil
 {name:c[:name],attributes:pr.attributes.except("created_at","updated_at"),mapped:!!c[:mapped],reply:!!c[:reply],
  card:ApplicationController.render(partial:"github/pull_requests/card",locals:{pull_request:pr,message:}),
  cards:ApplicationController.render(partial:"github/pull_requests/cards",locals:{message:}),
  header:ApplicationController.render(partial:"github/pull_requests/thread_header",locals:{thread:,pull_request:pr}),
  files:ApplicationController.render(partial:"github/pull_requests/files_summary",locals:{pull_request:pr})}
end
File.write("/work/vectors/github_cards.json",JSON.pretty_generate(vectors)+"\n")
puts "GitHub cards Rails oracle: #{vectors.size} cases x card/cards/header/files; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
