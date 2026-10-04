# Run with parity/bin/reference runner in the plain pinned image; redirect stdout
# to reference-tools/users/profile-page-source-hashes.json before profile_page.rb.
require 'json'
require 'digest'

reference=File.read(File.join(ENV.fetch('PARITY_WORK'),'parity/reference.sha')).strip
raise 'profile source hashes require the pinned reference image' unless ENV['PARITY_REFERENCE_SHA']==reference

sources=%w[
  app/controllers/users/profiles_controller.rb
  app/helpers/users/profiles_helper.rb
  app/models/user.rb
  app/models/user/inbox_preferences.rb
  app/models/user/status_settings.rb
  app/views/layouts/application.html.erb
  app/views/two_factor/_reauth_field.html.erb
  app/views/users/profiles/_appearance.html.erb
  app/views/users/profiles/_fizzy_connection.html.erb
  app/views/users/profiles/_github_connection.html.erb
  app/views/users/profiles/_google_calendar.html.erb
  app/views/users/profiles/_google_sign_in.html.erb
  app/views/users/profiles/_membership.html.erb
  app/views/users/profiles/_notifications.html.erb
  app/views/users/profiles/_sessions.html.erb
  app/views/users/profiles/_slack_import.html.erb
  app/views/users/profiles/_status.html.erb
  app/views/users/profiles/_transfer.html.erb
  app/views/users/profiles/_two_factor.html.erb
  app/views/users/profiles/show.html.erb
  app/views/users/statuses/_fields.html.erb
  config/routes.rb
]
output=JSON.pretty_generate(sources.to_h {|path|[path,Digest::SHA256.file(Rails.root.join(path)).hexdigest]})+"\n"
case ARGV
when []
  print output
  warn "Rails profile source hashes: #{sources.length} files; plain pinned reference #{reference}"
when ['--check']
  ledger=File.join(ENV.fetch('PARITY_WORK'),'reference-tools/users/profile-page-source-hashes.json')
  raise 'profile source hash ledger differs from pinned source bytes' unless File.binread(ledger)==output
  puts "Rails profile source hashes: #{sources.length} files match byte for byte; plain pinned reference #{reference}"
else
  raise 'usage: profile_page_source_hashes.rb [--check]'
end
