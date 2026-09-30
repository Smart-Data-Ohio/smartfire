require "json"
require "digest"
Rails.logger=ActiveSupport::Logger.new($stderr)
JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"),"reference-tools/users/audit-logs-source-hashes.json"))).each { |path,hash| raise "source drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest==hash }
class AuditLogGoldenController < Accounts::AuditLogsController
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : "GLOBAL"
  end
end
Current.user=User.find(127326141)
AuditLog.delete_all
rows=65.times.map do |n|
  {id:10001+n,action:n.even? ? "user.ban" : "room.create",actor_label:n%3==0 ? "Jason <jason@37signals.com>" : "David <david@37signals.com>",target_type:n.even? ? "User" : "Room",target_label:n.even? ? "Kevin <kevin@37signals.com>" : "HQ <&>",details:{n:n},ip_address:"203.0.113.7",user_agent:"Fixture",created_at:"2026-03-#{n%2==0 ? '01' : '02'}T16:00:00Z"}
end
rows[0].merge!(actor_label:"=formula",target_label:"\tunsafe",details:{name:{before:nil,after:"é <&>"},list:["one","two"],empty:{},flag:false,long:"é"*90},user_agent:"=cmd|'/c calc'!A0")
rows[1].merge!(actor_label:nil,target_label:" ",ip_address:nil,user_agent:nil,details:{})
rows[2].merge!(actor_label:"100%_\\ literal",target_label:"+formula",ip_address:"@formula",user_agent:"\runsafe")
rows.each { |row| AuditLog.insert_all!([row.merge(updated_at:row[:created_at])]) }
queries=[{}, {page:"2"},{page:"999"},{actor:" jason@37signals.com "},{audit_action:"user.ban",target_type:"User"},{audit_action:"user.ban",target_type:"Room"},{audit_action:"room.nuke",target_type:"Spaceship"},{from:"2026-03-02"},{to:"2026-03-01"},{from:"bad",to:"2026-02-30"},{actor:"100%"},{actor:"_"},{actor:"\\"},{actor:"\u00a0"}]
cases=queries.map.with_index do |query,index|
  controller=AuditLogGoldenController.new
  controller.set_request!(ActionDispatch::Request.new(AuditLogGoldenController.renderer.new(http_host:"campfire.test",https:false,"rack.session"=>{},"action_dispatch.content_security_policy_nonce_generator"=>->(_){"NONCE"}).send(:env_for_request)))
  controller.set_response!(ActionDispatch::Response.new)
  controller.params=ActionController::Parameters.new(query)
  filters=controller.send(:audit_filters)
  controller.instance_variable_set(:@filters,filters)
  entries=controller.send(:filtered_entries).order(id: :desc)
  page=GearedPagination::Recordset.new(entries,per_page:50).page(query[:page])
  view=controller.view_context
  view.assign({filters:filters,entries:page.records,page:page,export_truncated:index==0}.stringify_keys)
  html=view.render(template:"accounts/audit_logs/show",layout:false)
  {name:"query_#{index}",query:query,filters:filters,ids:entries.pluck(:id),html:html,nav:view.content_for(:nav).to_s,csv:controller.send(:audit_csv,entries)}
end
dates=["2026-03-02","2026-3-2","2026/03/02","02 Mar 2026","March 2, 2026","20260302","26-03-02","03/02/2026","2026-03-02T12:34:56Z","bad","2026-02-30","2026-13-01"," 2026-03-02 ","\u00a0",""] .map { |s| c=AuditLogGoldenController.new; {input:s,date:c.send(:parse_date,s)&.to_s} }
puts JSON.pretty_generate(reference:"d7c7de92",rows:rows,cases:cases,dates:dates)
warn "Rails audit logs oracle: #{rows.size} rows, #{cases.size} complete HTML/nav/CSV cases, #{dates.size} date parses; reference d7c7de92"
