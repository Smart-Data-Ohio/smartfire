# Smartfire states added to the legacy/media corpus. All external cards are recorded rows,
# never fetched; timestamps stay fresh at the frozen clock. Can also be built directly.
based_on "default"
at NOW - 2.hours
david, jason, bender = %i[david jason bender].map { |name| user(name) }
channel = room(:designers)

markdown = label :messages, :markdown, channel.messages.create!(creator: david,
  markdown_source: "# Launch checklist\n\n**Ready** for review. @#{jason.id}\n\n- [x] Write the plan\n- [ ] Ship the port\n\n```rust\nlet ready = true;\n```\n\n| Area | State |\n| --- | --- |\n| Parity | In progress |", client_message_id: next_client_message_id)
label :messages, :reply, channel.messages.create!(creator: jason, markdown_source: "Reply to the **launch checklist**.", reply_to_message: markdown, client_message_id: next_client_message_id)
label :messages, :forward, room(:pets).messages.create!(creator: david, markdown_source: markdown.markdown_source,
  forwarded_from_message: markdown, forwarded_at: NOW - 2.hours, forward_note: "For the release team", forwarded_markdown: true, client_message_id: next_client_message_id)
label :pins, :launch, MessagePin.create!(room: channel, message: markdown, pinner: david)

thread = label :threads, :launch, ChannelThread.create!(room: channel, creator: david, name: "Launch review", parent_message: markdown, last_activity_at: NOW - 1.hour)
ThreadMembership.join!(thread, david)
ThreadMembership.join!(thread, jason)
label :messages, :thread_reply, thread.messages.create!(room: channel, creator: jason, markdown_source: "Thread reply with `code`.", client_message_id: next_client_message_id)
label :threads, :closed, ChannelThread.create!(room: channel, creator: david, name: "Completed discussion", closed_at: NOW - 1.hour, last_activity_at: NOW - 1.hour)
label :threads, :locked, ChannelThread.create!(room: channel, creator: david, name: "Locked discussion", closed_at: NOW - 1.hour, locked_at: NOW - 1.hour, last_activity_at: NOW - 1.hour)

question = channel.messages.create!(creator: david, markdown_source: "Where should the launch happen?", client_message_id: next_client_message_id)
poll = label :polls, :launch, Poll.create_for_message!(message: question, labels: ["Online", "In the office", "Both"], closes_at: NOW + 1.day)
poll.poll_votes.create!(user: david, poll_option: poll.poll_options.first)
closed_question = channel.messages.create!(creator: david, markdown_source: "Ship on Friday?", client_message_id: next_client_message_id)
closed_poll = label :polls, :closed, Poll.create_for_message!(message: closed_question, labels: ["Yes", "No"])
closed_poll.update_columns(closed_at: NOW - 1.hour)

label :saved_items, :launch, SavedItem.create!(user: david, message: markdown, remind_at: NOW + 2.hours)
label :saved_items, :done, SavedItem.create!(user: david, message: message(:plain), status: "done")
label :scheduled_messages, :launch, ScheduledMessage.create!(user: david, room: channel, markdown_source: "Launch reminder for **tomorrow**", send_at: NOW + 1.day)
past = label :scheduled_messages, :past, ScheduledMessage.create!(user: david, room: channel, markdown_source: "Earlier reminder", send_at: NOW + 1.hour)
past.update_columns(send_at: NOW - 1.hour, sent_at: NOW - 1.hour, sent_message_id: markdown.id)

voice = label :rooms, :voice, Rooms::Voice.create_for({ name: "Team lounge", creator: david }, users: [david, jason, user(:kevin)])
stage = label :rooms, :stage, Rooms::Stage.create_for({ name: "Release stage", creator: david }, users: [david, jason, user(:kevin)])
board = label :rooms, :board, Rooms::Board.create_for({ name: "Release board", creator: david }, users: [david, jason, bender])
%w[planned in_progress blocked done].each_with_index do |status, i|
  at NOW - (50 - i).minutes
  post = label :threads, status, ChannelThread.create!(room: board, creator: david, name: "#{status.humanize} work", work_status: status, work_owner: (status == "blocked" ? bender : david), last_activity_at: NOW - (50 - i).minutes, result_markdown: (status == "done" ? "Verified result with **evidence**." : nil))
  post.tag_names = "release, rust"
  post.save!
  ThreadMembership.join!(post, david)
  post.messages.create!(room: board, creator: david, markdown_source: "Board post in #{status} state.", board_post_opener: true, client_message_id: next_client_message_id)
end

at NOW
# Non-default status and inbox preferences plus the already-granted Drive attachment menu.
david.update!(custom_status_text: "Shipping Rust", custom_status_emoji: "🚀", quiet_hours_enabled: true,
  quiet_hours_start_minute: 1320, quiet_hours_end_minute: 420, inbox_preferences: { "huddle_invitations" => false })
category = label :room_categories, :release, david.room_categories.create!(name: "Release", position: 0)
membership(:designers, :david).update!(room_category: category, favorite_position: 0)
GoogleAccount.create!(user: david, email: david.email_address, refresh_token: "parity-offline-google-refresh", access_token: "parity-offline-google-access", access_token_expires_at: NOW + 1.day, scopes: Google::Client::DRIVE_SCOPE)
# Recorded integration states. Credentials are synthetic and the runtime cannot reach the Internet.
pr = label :github_pull_requests, :launch, Github::PullRequest.create!(owner: "smart-data-ohio", repo: "smartfire", number: 42,
  title: "Port the launch checklist", html_url: "https://github.com/Smart-Data-Ohio/smartfire/pull/42", state: "open", private: false,
  author_login: "parity-user", base_branch: "main", head_branch: "rust-parity", check_status: "success", review_decision: "APPROVED", fetched_at: NOW,
  payload: { "base" => { "repo" => { "full_name" => "Smart-Data-Ohio/smartfire" } }, "additions" => 12, "deletions" => 3, "changed_files" => 2 })
pr_message = label :messages, :github, channel.messages.create!(creator: david, markdown_source: pr.html_url, client_message_id: next_client_message_id)
Github::PullRequestReference.find_or_create_by!(message: pr_message, pull_request: pr)

card = label :fizzy_cards, :launch, Fizzy::Card.create!(account_id: "parity", number: 42)
FizzyConnectedAccount.create!(user: david, access_token: "parity-offline-fizzy-token", fizzy_account_id: "parity", fizzy_account_name: "Parity workspace", fizzy_user_id: "1", fizzy_user_name: "David")
Fizzy::CardCache.create!(card: card, user: david, fetched_at: NOW, payload: { "number" => 42, "title" => "Release checklist", "status" => "open", "url" => "https://app.fizzy.do/parity/cards/42", "board" => { "name" => "Release" }, "tags" => [], "assignees" => [] })
fizzy_message = label :messages, :fizzy, channel.messages.create!(creator: david, markdown_source: card.web_url, client_message_id: next_client_message_id)
Fizzy::CardReference.find_or_create_by!(message: fizzy_message, card: card)

embed = label :link_embeds, :launch, LinkEmbed.create!(normalized_url: "https://example.com/parity", title: "Smartfire release notes", description: "Recorded integration card, captured offline.", site_name: "Example", fetched_at: NOW, expires_at: NOW + 1.day)
link_message = label :messages, :link, channel.messages.create!(creator: jason, markdown_source: embed.normalized_url, client_message_id: next_client_message_id)
LinkEmbedReference.find_or_create_by!(message: link_message, link_embed: embed, url: embed.normalized_url)
Twitter::Post.update_all(fetched_at: NOW, fetch_requested_at: NOW)
linkedin_url = "https://www.linkedin.com/posts/parity_release-activity-1234567890123456789-abcd"
linkedin = channel.messages.create!(creator: david, markdown_source: linkedin_url, client_message_id: next_client_message_id)
label :messages, :linkedin, linkedin
LinkEmbed.find_by!(normalized_url: LinkEmbed.normalize_url(linkedin_url)).update!(title: "Smartfire launch", description: "Recorded LinkedIn excerpt.", site_name: "LinkedIn", fetched_at: NOW, expires_at: NOW + 1.day)
quote = channel.messages.create!(creator: david, markdown_source: "Message from another room: http://localhost:3999/rooms/#{room(:pets).id}/@#{message(:forward).id}", client_message_id: next_client_message_id)
label :message_references, :launch, MessageReference.find_or_create_by!(message: quote, referenced_message: message(:forward))
pr_thread = ChannelThread.create!(room: channel, creator: david, name: "Pull request review", last_activity_at: NOW)
label :threads, :github, pr_thread
Github::PullRequestThread.create!(pull_request: pr, room: channel, channel_thread: pr_thread)

agent = Agent.find_by!(user: bender)
label :agents, :bender, agent unless labels.key?("agents.bender")
label :agent_approvals, :launch, AgentApproval.create!(agent: agent, room: channel, action: "deploy.preview", summary: "Approve a preview deployment", expires_at: NOW + 1.day, payload: {})
label :agent_events, :launch, AgentEvent.create!(agent: agent, actor: david, room: channel, message: markdown, event_type: "posted", outcome: "delivered", detail: "Recorded parity event")
ActivityItem.find_or_create_by!(user: david, source: markdown) { |item| item.event_type = "mention" }
ActivityItem.find_or_create_by!(user: david, source: message(:reply)) { |item| item.event_type = "reply"; item.read_at = NOW - 1.hour }
ActivityItem.find_or_create_by!(user: david, source: Event.find(id_for(:events, :launch_party))) { |item| item.event_type = "event_invitation" }

# Stable, verified sessions avoid concurrent captures creating unordered session/inbox/audit rows.
# They are issued by the real cookie jar. Password/remembered-device authentication remains a
# separate interactive state; read-only screenshots start from a known authenticated database.
Session.update_all(last_active_at: NOW, two_factor_verified_at: NOW)
labels.select { |key, _| key.start_with?("users.") }.sort.each do |key, id|
  person = User.find(id)
  next unless person.active? && !person.bot?
  token = Digest::SHA256.hexdigest("smartfire-parity-session-#{id}").first(32)
  session = person.sessions.create!(token: token, two_factor_verified_at: NOW, user_agent: "Mozilla/5.0 (X11; Linux x86_64) Chrome/130.0.0.0 Safari/537.36", ip_address: "127.0.0.1", last_active_at: NOW)
  label :sessions, key.delete_prefix("users."), session
  label :session_cookies, key.delete_prefix("users."), Rack::Utils.escape(signed_cookie(:session_token, token))
end
# Run the real first-factor flow to issue the pending encrypted cookie used by the challenge.
require "action_dispatch/testing/integration"
challenge = ActionDispatch::Integration::Session.new(Rails.application)
challenge.host! "localhost"
challenge.get "/session/new"
csrf = challenge.response.body[/name="authenticity_token" value="([^"]+)"/, 1] || raise("sign-in CSRF missing")
challenge.post "/session", params: { email_address: david.email_address, password: "secret123456", authenticity_token: csrf }
raise "challenge seed did not reach second factor" unless challenge.response.location&.end_with?("/two_factor_challenge")
label :browser_cookies, :challenge, Array(challenge.response.headers["set-cookie"]).map { |cookie| cookie.split(";", 2).first.split("=", 2) }.to_h
# Rails' production executor is deliberately non-nestable. The in-process request completed
# the runner's Current context; begin a fresh one before seed variants render broadcast partials.
ActiveSupport::ExecutionContext.clear
enrolling = label :users, :enrolling, User.create!(name: "New member", email_address: "enrolling@example.com", password: "secret123456")
setup_session = enrolling.sessions.create!(token: "parity-setup-session-token", last_active_at: NOW, user_agent: "Parity setup", ip_address: "127.0.0.1")
TwoFactorSetupSecret.create!(session: setup_session, secret: "JBSWY3DPEHPK3PXP", expires_at: NOW + 30.minutes)
label :session_cookies, :enrolling, Rack::Utils.escape(signed_cookie(:session_token, setup_session.token))

# Stop stale fixture cards queuing unrecorded fetches; jobs cannot leave the isolated network.
LinkEmbed.update_all(fetched_at: NOW, expires_at: NOW + 1.day, fetch_requested_at: NOW)
Membership.update_all(unread_at: nil, connected_at: nil, connections: 0)
