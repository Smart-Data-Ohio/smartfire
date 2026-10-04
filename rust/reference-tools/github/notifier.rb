# Our pinned Notifier: persisted posts, claims, references, inbox rows and callback routing.
require "json"
require "digest"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
JSON.parse(File.read(ENV.fetch("GITHUB_REFERENCE_HASHES"))).each do |path, hash|
  raise "Reference drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash
end
ActiveJob::Base.queue_adapter = :test
ActionCable.server.instance_variable_set(:@pubsub, ActionCable::SubscriptionAdapter::Test.new(ActionCable.server))
ActiveRecord::Schema.verbose = false
load Rails.root.join("db/schema.rb")
travel_to Time.utc(2026, 1, 1, 12)
Account.create!(name: "Notifier oracle")
owner = User.create!(id: 811, name: "Oracle", email_address: "oracle@example.test", password: "fixture-password", role: :administrator)
reviewer = User.create!(id: 812, name: "Kevin", email_address: "reviewer@example.test", password: "fixture-password")
room = Rooms::Closed.create!(id: 815, name: "Notifications", creator: owner)
other_room = Rooms::Closed.create!(id: 825, name: "Verified", creator: owner)
Rooms::Open.create!(id: 835, name: "Unsubscribed open", creator: owner)
Membership.find_or_create_by!(user:owner,room:)
Membership.find_or_create_by!(user:owner,room:other_room)
module NotifierBroadcastCapture
  def broadcast_create
    Thread.current.fetch(:notifier_broadcasts) << {room_id:, thread: thread_id.present?}
  end
end
Message.prepend(NotifierBroadcastCapture)
def pr(action = "opened", number: 12, title: "Fix login", **fields)
  {"action"=>action, "sender"=>{"login"=>action == "review_requested" ? "bob" : "alice"},
   "repository"=>{"full_name"=>"rails/rails", "private"=>false},
   "pull_request"=>{"number"=>number, "title"=>title, "html_url"=>"https://github.com/evil/other/pull/99", "merged"=>false, "merged_by"=>{"login"=>"alice"}, "closed_at"=>"2026-09-17T12:00:00Z", "base"=>{"repo"=>{"full_name"=>"rails/rails"}}}.merge(fields.transform_keys(&:to_s)),
   "requested_reviewer"=>{"login"=>"Kevin-GH"}}
end
def review(state, id)
  pr.merge("action"=>"submitted", "review"=>{"id"=>id,"state"=>state,"user"=>{"login"=>"carol"}})
end
def check(event = "check_run", sha: "abc123", conclusion: "failure", numbers: [12], name: "ci / test")
  {"repository"=>{"full_name"=>"rails/rails","private"=>false}, event=>{"head_sha"=>sha,"conclusion"=>conclusion,"pull_requests"=>numbers.map { |n| {"number"=>n} },"name"=>name,"app"=>{"name"=>"CI"}}}
end
def status(state="failure", sha: "abc123", branches: ["shiny"])
  {"repository"=>{"full_name"=>"rails/rails","private"=>false},"state"=>state,"sha"=>sha,"branches"=>branches.map { |n| {"name"=>n} },"context"=>"ci / test"}
end
cases = [
  {name:"opened", deliveries:[["pull_request",pr]]},
  {name:"mention_security", deliveries:[["pull_request",pr(title:"Ping @[Everyone]\r\nplease").merge("sender"=>{"login"=>"@[Kevin]\nAlice"})]]},
  {name:"opened_reopen_dedupe", deliveries:%w[opened reopened ready_for_review synchronize labeled].map { |a| ["pull_request",pr(a)] }},
  {name:"reopened", deliveries:[["pull_request",pr("reopened")]]},
  {name:"ready", deliveries:[["pull_request",pr("ready_for_review")]]},
  {name:"closed_merged", deliveries:[["pull_request",pr("closed",merged:true)],["pull_request",pr("closed",number:13)]]},
  {name:"closed_twice", deliveries:[["pull_request",pr("closed")],["pull_request",pr("closed")],["pull_request",pr("closed",closed_at:"later")]]},
  {name:"review_requested", deliveries:[["pull_request",pr("review_requested")]]},
  {name:"review_stranger", deliveries:[["pull_request",pr("review_requested").merge("requested_reviewer"=>{"login"=>"stranger"})]]},
  {name:"review_nonmember", member:false, deliveries:[["pull_request",pr("review_requested")]]},
  {name:"review_off", preference:false, deliveries:[["pull_request",pr("review_requested")]]},
  {name:"review_nothing", involvement:"nothing", deliveries:[["pull_request",pr("review_requested")]]},
  {name:"review_invisible", involvement:"invisible", deliveries:[["pull_request",pr("review_requested")]]},
  {name:"review_inactive", inactive:true, deliveries:[["pull_request",pr("review_requested")]]},
  {name:"review_bot", reviewer_bot:true, deliveries:[["pull_request",pr("review_requested")]]},
  {name:"review_strip", deliveries:[["pull_request",pr("review_requested").merge("requested_reviewer"=>{"login"=>"  Kevin-GH  "})]]},
  {name:"team_review", deliveries:[["pull_request",pr("review_requested").except("requested_reviewer")]]},
  {name:"reviews", deliveries:["approved","approved","changes_requested","commented","dismissed"].each_with_index.map { |s,i| ["pull_request_review",review(s,i < 2 ? 7 : i+7)] }},
  {name:"review_edited", deliveries:[["pull_request_review",review("approved",7).merge("action"=>"edited")]]},
  {name:"checks_dedupe", deliveries:[["check_run",check],["check_run",check(name:"ci / lint")],["check_suite",check("check_suite")],["check_run",check(sha:"def456")]]},
  {name:"checks_success", deliveries:[["check_run",check(conclusion:"success")],["check_suite",check("check_suite",conclusion:"success")]]},
  {name:"checks_timeout_cancel", deliveries:[["check_run",check(conclusion:"timed_out")],["check_suite",check("check_suite",conclusion:"cancelled",sha:"next")]]},
  {name:"checks_multi", stored:true, deliveries:[["check_run",check(numbers:[12,13,12])]]},
  {name:"status_stored", stored:true, deliveries:[["status",status]]},
  {name:"status_no_pr", deliveries:[["status",status],["status",status("success")]]},
  {name:"status_fallback", stored:true, no_url:true, deliveries:[["status",status("error")]]},
  {name:"unsubscribed_keys", events:["opened"], deliveries:[["pull_request",pr("closed",merged:true)],["check_run",check]]},
  {name:"no_subscriptions", no_subscriptions:true, no_bot:true, deliveries:[["pull_request",pr]]},
  {name:"unhandled", no_bot:true, deliveries:[["ping",{}],["push",{"repository"=>{"full_name"=>"rails/rails"}}]]},
  {name:"thread_and_root", thread:true, other:true, deliveries:[["pull_request",pr]]},
  {name:"thread_dedupe", thread:true, deliveries:[["pull_request",pr],["pull_request",pr("reopened")]]},
  {name:"thread_review", thread:true, deliveries:[["pull_request",pr("review_requested")]]},
  {name:"thread_locked", thread:true, locked:true, deliveries:[["pull_request",pr]]},
  {name:"thread_closed", thread:true, closed:true, deliveries:[["pull_request",pr]]},
  {name:"thread_case", thread:true, deliveries:[["pull_request",pr.merge("repository"=>{"full_name"=>"Rails/Rails","private"=>false})]]},
  {name:"check_case", stored:true, deliveries:[["check_run",check.merge("repository"=>{"full_name"=>"Rails/Rails","private"=>false})]]},
  {name:"status_case", stored:true, deliveries:[["status",status.merge("repository"=>{"full_name"=>"Rails/Rails","private"=>false})]]},
  {name:"deleted", deleted:true, deliveries:[["pull_request",pr]]},
  {name:"deleted_review", deleted:true, deliveries:[["pull_request",pr("review_requested")]]},
  {name:"private_unverified", deliveries:[["pull_request",pr(title:"Secret acquisition").merge("repository"=>{"full_name"=>"rails/rails","private"=>true})]]},
  {name:"unknown_unverified", deliveries:[["pull_request",pr("closed",merged:true,title:"Secret acquisition").merge("repository"=>{"full_name"=>"rails/rails"})]]},
  {name:"private_verified", verified:true, deliveries:[["pull_request",pr(title:"Secret acquisition").merge("repository"=>{"full_name"=>"rails/rails","private"=>true})]]},
  {name:"private_checks", stored:true, deliveries:[["check_run",check.merge("repository"=>{"full_name"=>"rails/rails","private"=>true})],["status",status(sha:"next").merge("repository"=>{"full_name"=>"rails/rails","private"=>true})]]},
  {name:"per_subscription", other:true, deliveries:[["pull_request",pr(title:"Secret acquisition").merge("repository"=>{"full_name"=>"rails/rails","private"=>true})]]},
  {name:"lazy_bot", no_bot:true, deliveries:[["pull_request",pr]]},
  {name:"base_repository", deliveries:[["pull_request",pr.except("repository")]]},
  {name:"invalid_repository", deliveries:[["pull_request",pr.merge("repository"=>{"full_name"=>"rails/rails/other"})]]},
  {name:"blank_inline", deliveries:[["pull_request",pr(title:"\n\t").merge("sender"=>{"login"=>"\r\n"})]]},
  {name:"coercion_inline", deliveries:[["pull_request",pr(title:42).merge("sender"=>{"login"=>false})]]}
]
output=cases.map do |test|
  [ActivityItem, Github::Notification, Github::PullRequestReference, Github::PullRequestThread, Github::RepositorySubscription, ThreadMembership, MessageReference, Message, ChannelThread, Github::PullRequest, ActionText::RichText].each(&:delete_all)
  Membership.where(user_id: [810,812]).delete_all
  User.where(name:"GitHub").delete_all
  reviewer.update_columns(github_login:"kevin-gh", role:test[:reviewer_bot] ? 2 : 0, status:test[:inactive] ? 1 : 0, inbox_preferences: test.key?(:preference) ? {github_review_requests:test[:preference]} : {})
  Membership.create!(user:reviewer, room:, involvement:test.fetch(:involvement,"everything")) unless test[:member] == false
  room.update_columns(deleted_at:test[:deleted] ? Time.current : nil)
  bot=nil
  bot=User.create!(id:810,name:"GitHub",role: :bot,skip_open_room_grant:true) unless test[:no_bot]
  unless test[:no_subscriptions]
    # Skip creation callbacks here to exercise Notifier's lazy bot path independently.
    Github::RepositorySubscription.insert!({id:819,room_id:815,owner:"rails",repo:"rails",events:test.fetch(:events,Github::RepositorySubscription::EVENT_KEYS),reader_verified:test.fetch(:verified,false),created_at:Time.current,updated_at:Time.current})
    Membership.create!(user:bot,room:) if bot
    if test[:other]
      Github::RepositorySubscription.insert!({id:829,room_id:825,owner:"rails",repo:"rails",events:Github::RepositorySubscription::EVENT_KEYS,reader_verified:true,created_at:Time.current,updated_at:Time.current})
      Membership.create!(user:bot,room:other_room) if bot
    end
  end
  stored=Github::PullRequest.create!(id:816,owner:"rails",repo:"rails",number:12,title:"Secret acquisition",head_branch:"shiny",html_url:test[:no_url] ? "" : "https://github.com/Rails/Rails/pull/12") if test[:stored] || test[:thread]
  if test[:thread]
    parent=room.messages.create!(creator:owner,body:"Discussion",client_message_id:"fixture-parent")
    thread=ChannelThread.create!(id:817,room:,creator:owner,name:"PR chat",parent_message:parent)
    ThreadMembership.join!(thread,owner)
    Github::PullRequestThread.create!(pull_request:stored,room:,channel_thread:thread)
    thread.update_columns(last_activity_at:2.days.ago,locked_at:test[:locked] ? Time.current : nil,closed_at:test[:closed] ? Time.current : nil)
  end
  ActiveJob::Base.queue_adapter.enqueued_jobs.clear
  Thread.current[:notifier_broadcasts]=[]
  test[:deliveries].each { |event,payload| Github::DeliverSubscriptionEventJob.perform_now(event,payload) }
  messages=Message.where.not(client_message_id:"fixture-parent").order(:id).to_a
  test.merge(expected:{messages:messages.map { |m| {room_id:m.room_id,thread:m.thread_id.present?,source:m.markdown_source,bot:m.creator.name,refs:m.github_pull_requests.order(:id).map { |p| [p.owner,p.repo,p.number] }} },
    notifications:Github::Notification.order(:subscription_id,:id).map { |n| {subscription:n.subscription_id,key:n.dedupe_key,message:messages.index { |m| m.id == n.message_id }} },
    activity:ActivityItem.where(event_type:"pr_review_request").order(:id).map { |i| {user_id:i.user_id,message:messages.index { |m| m.id == i.source_id }} },
    broadcasts:Thread.current[:notifier_broadcasts],bot_count:User.active_bots.where(name:"GitHub").count,
    thread:test[:thread] ? {closed:thread.reload.closed?,locked:thread.locked?,fresh:thread.last_activity_at == Time.current} : nil})
end
File.write(ENV.fetch("GITHUB_NOTIFIER_VECTOR_PATH"),JSON.pretty_generate({reference:ENV.fetch("PARITY_REFERENCE_SHA"),cases:output})+"\n")
puts "GitHub Notifier Rails oracle: #{output.size} persisted delivery cases; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
