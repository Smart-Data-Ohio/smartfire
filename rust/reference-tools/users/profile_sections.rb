require 'json'
require 'digest'
Rails.logger=ActiveSupport::Logger.new($stderr)
JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'reference-tools/users/profile-sections-source-hashes.json'))).each {|path,hash|raise "source drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest==hash}
class ProfileSectionsGoldenController < Users::ProfilesController
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
user=User.find(127326141)
Current.reset;Current.user=user
renderer=ProfileSectionsGoldenController.renderer.new(http_host:'campfire.test',https:false,'rack.session'=>{})
cases=[['unconfigured',false,nil],['missing',true,nil],['calendar_only',true,nil,true],['calendar_drive',true,"#{Google::Client::CALENDAR_SCOPE} #{Google::Client::DRIVE_SCOPE}",true],['drive_only',true,Google::Client::DRIVE_SCOPE,true],['openid_email',true,'openid email',true],['rejected_drive',true,Google::Client::DRIVE_SCOPE,true,'401'],['rejected_calendar',true,Google::Client::CALENDAR_SCOPE,true,'invalid_grant'],['retired_metadata',true,"#{Google::Client::CALENDAR_SCOPE} https://www.googleapis.com/auth/drive.metadata.readonly",true],['blank_reason',true,Google::Client::CALENDAR_SCOPE,true,' ']].map do |name,configured,scopes,exists,reason|
  ENV['GOOGLE_CLIENT_ID']=configured ? 'parity-client' : nil
  ENV['GOOGLE_CLIENT_SECRET']=configured ? 'parity-secret' : nil
  GoogleAccount.where(user_id:user.id).delete_all
  if exists
    GoogleAccount.create!(user:user,email:name == 'openid_email' ? 'david@gmail.test' : 'fixture<&>@example.test',scopes:scopes,disconnected_reason:reason)
  end
  user.reload
  account=user.google_account
  input={calendar_configured:Google::Client.configured?,account_exists:!!account,connected:!!account&.connected?,calendar:!!account&.calendar?,drive:!!account&.drive?,email:account&.email || ''}
  {name:name,input:input,scopes:scopes,reason:reason,html:renderer.render(partial:'users/profiles/google_calendar',locals:{user:user})}
end
puts JSON.pretty_generate(reference: ENV.fetch('PARITY_REFERENCE_SHA'),google_calendar:cases)
warn "Rails profile sections oracle: #{cases.size} complete Google Calendar fragments; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
