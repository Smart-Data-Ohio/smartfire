require 'json'
ActiveJob::Base.queue_adapter = :test
ENV['GOOGLE_SIGN_IN_DOMAINS'] = 'example.com'
workspace = SlackWorkspace.create!(client_id: 'fixture-client', client_secret: 'fixture-secret')
owner = User.create!(name: 'Run owner')
existing = User.find_by(email_address: 'kevin@37signals.com') || User.create!(name: 'Existing Kevin', email_address: 'kevin@37signals.com')
existing.update!(name: 'Existing Kevin', status: :deactivated)
connection = SlackConnection.create!(slack_workspace: workspace, user: owner, slack_user_id: 'UADMIN', access_token: 'fixture-user-token')
run = SlackImport.create!(slack_workspace: workspace, slack_connection: connection, user: owner, kind: :workspace, mode: :import)
members = JSON.parse(File.read(Rails.root.join('test/fixtures/files/slack/users.json')))['members']
mapper = Slack::UserMapper.new(workspace: workspace, run: run)
preview = mapper.preview_page(members)
delta = mapper.map_page(members)
rows = run.records.where(slack_kind: 'user').order(:slack_key).map do |record|
  user = User.find(record.record_id)
  {'key'=>record.slack_key, 'created'=>record.created_record?, 'name'=>user.name,
    'email'=>user.email_address, 'status'=>user.status, 'bio'=>user.bio,
    'zone'=>user.time_zone, 'claimable'=>user.google_email_link_allowed}
end
result = {'reference'=>'d7c7de9264c63015be398001d7a1094e7695a6db','preview'=>preview,'stats'=>delta,'users'=>rows,'repeat'=>mapper.map_page(members)}
File.write(File.join(ENV.fetch('PARITY_WORK'),'vectors/slack/users.json'),JSON.pretty_generate(result)+"\n")
puts "Slack mapper vectors: #{rows.size} fixture users; preview, import and repeat recorded"
