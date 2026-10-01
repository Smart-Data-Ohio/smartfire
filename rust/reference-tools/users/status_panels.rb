require 'json'
require 'digest'
Rails.logger=ActiveSupport::Logger.new($stderr)
JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'reference-tools/users/status-panels-source-hashes.json'))).each {|path,hash|raise "source drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest==hash}
load File.join(ENV.fetch('PARITY_WORK'),'reference-tools/users/post_pin.rb')
class StatusPanelsGoldenController < Users::ProfilesController
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
user=User.find(127326141)
Current.reset;Current.user=user
renderer=StatusPanelsGoldenController.renderer.new(http_host:'campfire.test',https:false,'rack.session'=>{})
cases=[
  ['unconfigured',false,false,{}],
  ['missing',true,false,{}],
  ['connected',true,true,{}],
  ['enabled',true,true,{meeting_status_enabled:true,ooo_calendar_enabled:true}],
  ['fetch_error',true,true,{meeting_status_enabled:true,ooo_calendar_enabled:true},nil,'<Network & refresh>'],
  ['rejected',true,true,{meeting_status_enabled:true,ooo_calendar_enabled:true},'401'],
  ['drive_only',true,true,{},nil,nil,Google::Client::DRIVE_SCOPE],
  ['manual_ooo',true,false,{ooo_until:Time.utc(2026,3,3,22),ooo_note:'Back <&> soon'}],
  ['calendar_ooo',true,true,{ooo_calendar_enabled:true},nil,nil,nil,true],
  ['overlapping_ooo',true,true,{ooo_calendar_enabled:true,ooo_until:Time.utc(2026,3,3,22)},nil,nil,nil,true],
  ['errors',true,false,{}]
].map do |name,configured,exists,attrs,reason,fetch_error,scopes,calendar_ooo|
  ENV['GOOGLE_CLIENT_ID']=configured ? 'parity-client' : nil
  ENV['GOOGLE_CLIENT_SECRET']=configured ? 'parity-secret' : nil
  GoogleAccount.where(user_id:user.id).delete_all
  Calendar::MeetingCache.where(user_id:user.id).delete_all
  GoogleAccount.create!(user:user,email:'fixture<&>@example.test',scopes:scopes,disconnected_reason:reason) if exists
  if fetch_error || calendar_ooo
    Calendar::MeetingCache.create!(user:user,fetched_at:Time.current,fetch_error:fetch_error,
      ooo_intervals: calendar_ooo ? [[Time.utc(2026,3,2,15).iso8601,Time.utc(2026,3,4,22).iso8601]] : [])
  end
  user.update_columns({presence_setting:'auto',custom_status_emoji:'🚀',custom_status_text:'Shipping Rust',ooo_note:nil,ooo_until:nil,meeting_status_enabled:false,ooo_calendar_enabled:false}.merge(attrs))
  user.reload
  if name=='errors'
    user.errors.add(:ooo_until,'must be in the future')
    user.errors.add(:ooo_note,'is too long (maximum is 140 characters)')
  end
  account=user.google_account
  google={calendar_configured:Google::Client.configured?,account_exists:!!account,connected:!!account&.connected?,calendar:!!account&.calendar?,drive:!!account&.drive?,email:account&.email || ''}
  fields={presence:user.presence_setting,emoji:user.custom_status_emoji,text:user.custom_status_text,ooo_note:user.ooo_note,
    ooo_return:user.out_of_office? ? user.ooo_until_date : nil,manual_ooo:user.manual_ooo_active?,
    meeting_enabled:user.meeting_status_enabled?,ooo_calendar_enabled:user.ooo_calendar_enabled?,
    fetch_error:user.meeting_cache&.fetch_error,errors:user.errors.to_hash}
  {name:name,google:google,fields:fields,html:renderer.render(partial:'users/profiles/status',assigns:{user:user})}
end
puts JSON.pretty_generate(reference:'d7c7de92',status_reference:'2e20b24c',panels:cases)
warn "Rails status panels oracle: #{cases.size} complete status/meeting/OOO fragments; status template 2e20b24c, model files d7c7de92"
